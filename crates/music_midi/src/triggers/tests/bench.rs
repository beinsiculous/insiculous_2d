//! A trigger engine on the conductor's block loop, with a real ledger behind every slot
//! and a log of what each was told.

use std::sync::Arc;

use super::super::*;
use crate::conductor::{LedgerAction, VoiceLedger};
use crate::song_data::SongData;
use crate::song_file::parse_song_file;
use crate::test_support::*;
use crate::BLOCK_FRAMES;

pub(super) const SAMPLE_RATE: u32 = 44_100;
pub(super) const SINE_FONT: &[u8] = include_bytes!("../../../fixtures/sine.sf2");
pub(super) const SINE_PATH: &str = "soundfonts/sine.sf2";
pub(super) const QUARTER: u32 = 384;

/// What a slot was told, as a listener would hear it: the song sample of the block it
/// was applied in, the slot, the lane and the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Heard {
    pub(super) sample: u64,
    pub(super) slot: usize,
    pub(super) lane: usize,
    pub(super) key: u8,
    /// `None` for a note-off.
    pub(super) velocity: Option<u8>,
}

/// A real ledger per slot behind a log of everything it told its synthesizer.
pub(super) struct Recorder {
    pub(super) ledgers: Vec<VoiceLedger>,
    pub(super) sample: u64,
    pub(super) heard: Vec<Heard>,
}

impl Recorder {
    pub(super) fn record(heard: &mut Vec<Heard>, sample: u64, slot: usize, action: LedgerAction) {
        heard.push(match action {
            LedgerAction::NoteOn {
                lane,
                key,
                velocity,
            } => Heard {
                sample,
                slot,
                lane,
                key,
                velocity: Some(velocity),
            },
            LedgerAction::NoteOff { lane, key } => Heard {
                sample,
                slot,
                lane,
                key,
                velocity: None,
            },
        });
    }
}

impl TriggerSink for Recorder {
    fn note_on(
        &mut self,
        slot: usize,
        source: SourceId,
        key: u8,
        velocity: u8,
    ) -> Option<NoteTicket> {
        let (sample, heard) = (self.sample, &mut self.heard);
        let start = self
            .ledgers
            .get_mut(slot)?
            .note_on(source, key, velocity, |action| {
                Recorder::record(heard, sample, slot, action)
            });
        Some(start.ticket)
    }

    fn note_off(&mut self, slot: usize, ticket: NoteTicket) {
        let (sample, heard) = (self.sample, &mut self.heard);
        if let Some(ledger) = self.ledgers.get_mut(slot) {
            ledger.note_off(ticket, |action| {
                Recorder::record(heard, sample, slot, action)
            });
        }
    }

    fn is_sounding(&self, slot: usize, ticket: NoteTicket) -> bool {
        self.ledgers
            .get(slot)
            .is_some_and(|ledger| ledger.is_sounding(ticket))
    }

    fn release_source(&mut self, source: SourceId) {
        let (sample, heard) = (self.sample, &mut self.heard);
        for (slot, ledger) in self.ledgers.iter_mut().enumerate() {
            ledger.release_source(source, |action| {
                Recorder::record(heard, sample, slot, action)
            });
        }
    }
}

/// The conductor's block loop without a synthesizer: a trigger engine draining at the
/// start of each 64-frame block and firing what falls due in it.
pub(super) struct Bench {
    pub(super) song: Arc<Song>,
    pub(super) engine: TriggerEngine,
    pub(super) recorder: Recorder,
    pub(super) started: Instant,
    pub(super) playing: bool,
}

impl Bench {
    pub(super) fn new(song: Arc<Song>) -> Self {
        Self::with_pending_capacity(song, 64)
    }

    pub(super) fn with_pending_capacity(song: Arc<Song>, pending_events: usize) -> Self {
        Self::with_shape(
            song,
            EngineShape {
                sources: 16,
                pending_events,
                sounding_cap: 8,
                sample_voices: 8,
            },
        )
    }

    pub(super) fn with_shape(song: Arc<Song>, shape: EngineShape) -> Self {
        let engine = TriggerEngine::new(&song, shape).expect("the song's parts resolve");
        let slots = song.file().instruments.len();
        Self {
            song,
            engine,
            recorder: Recorder {
                ledgers: (0..slots).map(|_| VoiceLedger::new(15, 8)).collect(),
                sample: 0,
                heard: Vec::new(),
            },
            started: Instant::now(),
            playing: true,
        }
    }

    pub(super) fn sample(&self) -> u64 {
        self.recorder.sample
    }

    /// The wall clock at the current block's drain: one sample of song per sample of
    /// time since the bench started.
    pub(super) fn now(&self) -> Instant {
        self.started + self.duration(self.sample())
    }

    pub(super) fn duration(&self, samples: u64) -> Duration {
        Duration::from_secs_f64(samples as f64 / f64::from(SAMPLE_RATE))
    }

    /// Trigger `action` for `source`, stamped `age_samples` before this block's drain.
    pub(super) fn trigger(&mut self, action: &str, source: SourceId, age_samples: u64) {
        let stamp = self.now() - self.duration(age_samples);
        self.trigger_stamped(action, source, stamp);
    }

    pub(super) fn trigger_stamped(&mut self, action: &str, source: SourceId, stamp: Instant) {
        let binding = self.song.binding_index(action).expect("a declared action");
        let clock = DrainClock {
            song: &self.song,
            tempo_map: self.song.background().tempo_map(),
            sample_rate: SAMPLE_RATE,
            block_frames: BLOCK_FRAMES,
            drained_at: self.now(),
            position_sample: self.sample(),
            playing: self.playing,
        };
        let trigger = Trigger {
            binding,
            source,
            stamp,
        };
        self.engine.trigger(trigger, &clock, &mut self.recorder);
    }

    /// Fire what falls due in the current block, then move to the next one.
    pub(super) fn next_block(&mut self) {
        let due_before = self.sample() + BLOCK_FRAMES as u64;
        self.engine.fire_due(due_before, &mut self.recorder);
        self.recorder.sample = due_before;
    }

    /// The song looped: back to its first sample, the engine restarted there.
    pub(super) fn wrap(&mut self) {
        self.recorder.sample = 0;
        let song = Arc::clone(&self.song);
        self.engine.restart_at(0, &song);
    }

    pub(super) fn run(&mut self, blocks: usize) {
        for _ in 0..blocks {
            self.next_block();
        }
    }

    /// Run blocks until the block holding `sample` is the current one.
    pub(super) fn run_to(&mut self, sample: u64) {
        while self.sample() + (BLOCK_FRAMES as u64) <= sample {
            self.next_block();
        }
    }

    pub(super) fn note_ons(&self) -> Vec<(usize, u8)> {
        self.recorder
            .heard
            .iter()
            .filter(|heard| heard.velocity.is_some())
            .map(|heard| (heard.slot, heard.key))
            .collect()
    }

    pub(super) fn sample_at_tick(&self, tick: u64) -> u64 {
        self.song
            .background()
            .tempo_map()
            .sample_at_tick(tick, SAMPLE_RATE)
    }
}

pub(super) fn source(id: u64) -> SourceId {
    SourceId {
        id,
        generation: 0,
        epoch: 1,
    }
}

/// The start of the block holding `sample`.
pub(super) fn block_of(sample: u64) -> u64 {
    sample / BLOCK_FRAMES as u64 * BLOCK_FRAMES as u64
}

/// One trigger track named `melody` beside a control track, at 384 ticks a quarter.
pub(super) fn song_of(melody: Vec<Step>, mode: &str, quantize: &str) -> Arc<Song> {
    let mut track = vec![(0, track_name("melody"))];
    track.extend(melody);
    let midi = smf(&[vec![(0, tempo(500_000))], track]);
    let sidecar = format!(
        r#"SongFile(version: 1, soundfont: "{SINE_PATH}",
            parts: [(name: "melody", track: Name("melody"), mode: {mode}, quantize: {quantize})],
            instruments: [
                (name: "piano", source: SoundFont(path: "{SINE_PATH}", bank: 0, preset: 0)),
                (name: "bells", source: SoundFont(path: "{SINE_PATH}", bank: 0, preset: 0)),
            ],
            bindings: [
                (action: "slime", part: "melody", instrument: "piano"),
                (action: "skeleton", part: "melody", instrument: "bells"),
            ])"#
    );
    let file = parse_song_file("test.mid.ron", &sidecar).expect("the sidecar parses");
    let (data, _) = SongData::from_smf(&midi).expect("the MIDI reads");
    Arc::new(Song::new(file, data, &[(SINE_PATH, SINE_FONT)], &[]).expect("the song builds"))
}

/// Four quarter notes, C D E F, each an eighth long.
pub(super) fn four_notes() -> Vec<Step> {
    let mut steps = Vec::new();
    for key in [60, 62, 64, 65] {
        steps.push((0, note_on(0, key, 100)));
        steps.push((QUARTER / 2, note_off(0, key)));
        steps.push((QUARTER / 2, controller(0, 7, 100)));
    }
    steps
}
