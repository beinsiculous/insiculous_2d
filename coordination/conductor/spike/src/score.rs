//! A MIDI file as owned, timed events, and the background player that renders it on its
//! own synthesizer — the owned sequencer the conductor needs, in place of rustysynth's
//! bundled `MidiFileSequencer`, which takes the synthesizer and leaves no room for live
//! triggers.
//!
//! Seeking and wrapping a loop rebuild the channel state at the destination: every note is
//! released, then each channel's bank, program, controllers and pitch bend are folded from
//! the score's start up to the destination tick and re-applied. A note that spans the
//! destination is not re-sounded.

use std::sync::Arc;

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use rustysynth::{SoundFont, Synthesizer, SynthesizerError, SynthesizerSettings};

use common::clock::Instant;

const CHANNEL_COUNT: usize = 16;
const DEFAULT_MICROSECONDS_PER_QUARTER: u32 = 500_000;
const NOTE_OFF: i32 = 0x80;
const NOTE_ON: i32 = 0x90;
const CONTROL_CHANGE: i32 = 0xB0;
const PROGRAM_CHANGE: i32 = 0xC0;
const PITCH_BEND: i32 = 0xE0;
const BANK_SELECT: u8 = 0x00;
const CHANNEL_MODE_FIRST_CONTROLLER: u8 = 120;
const PITCH_BEND_CENTRE: u16 = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreMessage {
    NoteOn { key: u8, velocity: u8 },
    NoteOff { key: u8 },
    Controller { controller: u8, value: u8 },
    Program(u8),
    PitchBend(u16),
    Tempo { microseconds_per_quarter: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreEvent {
    pub tick: u64,
    pub channel: u8,
    pub message: ScoreMessage,
}

/// A whole MIDI file, every track merged in tick order.
#[derive(Debug, Clone)]
pub struct Score {
    pub ticks_per_quarter: u16,
    pub events: Arc<[ScoreEvent]>,
    pub last_tick: u64,
}

/// Parse a Standard MIDI File. SMPTE timing is refused: every tempo calculation here is
/// in ticks per quarter note.
pub fn parse_score(bytes: &[u8]) -> Result<Score, String> {
    let smf = Smf::parse(bytes).map_err(|error| format!("not a MIDI file: {error}"))?;
    let Timing::Metrical(ticks_per_quarter) = smf.header.timing else {
        return Err("SMPTE timing is not supported".to_string());
    };
    let mut events = Vec::new();
    for track in &smf.tracks {
        let mut tick: u64 = 0;
        for track_event in track {
            tick += u64::from(track_event.delta.as_int());
            let (channel, message) = match track_event.kind {
                TrackEventKind::Midi { channel, message } => {
                    let message = match message {
                        MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                            ScoreMessage::NoteOn { key: key.as_int(), velocity: vel.as_int() }
                        }
                        MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                            ScoreMessage::NoteOff { key: key.as_int() }
                        }
                        MidiMessage::Controller { controller, value } => {
                            ScoreMessage::Controller { controller: controller.as_int(), value: value.as_int() }
                        }
                        MidiMessage::ProgramChange { program } => ScoreMessage::Program(program.as_int()),
                        MidiMessage::PitchBend { bend } => ScoreMessage::PitchBend(bend.0.as_int()),
                        MidiMessage::Aftertouch { .. } | MidiMessage::ChannelAftertouch { .. } => continue,
                    };
                    (channel.as_int(), message)
                }
                TrackEventKind::Meta(MetaMessage::Tempo(microseconds_per_quarter)) => {
                    (0, ScoreMessage::Tempo { microseconds_per_quarter: microseconds_per_quarter.as_int() })
                }
                _ => continue,
            };
            events.push(ScoreEvent { tick, channel, message });
        }
    }
    // A stable sort keeps each track's order within a tick and puts earlier tracks first.
    events.sort_by_key(|event| event.tick);
    let last_tick = events.last().map_or(0, |event| event.tick);
    Ok(Score { ticks_per_quarter: ticks_per_quarter.as_int(), events: events.into(), last_tick })
}

/// What a channel looks like at some tick: the state a seek has to restore.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelState {
    pub bank: u8,
    pub program: u8,
    pub controllers: [Option<u8>; 128],
    pub pitch_bend: u16,
}

impl Default for ChannelState {
    fn default() -> Self {
        Self { bank: 0, program: 0, controllers: [None; 128], pitch_bend: PITCH_BEND_CENTRE }
    }
}

/// Counters a measurement reads.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ScoreCounters {
    pub reconstructions: u64,
    pub longest_reconstruction_microseconds: f64,
    pub loop_wraps: u64,
}

/// Plays a [`Score`] on its own synthesizer, honouring program and controller events, with
/// an optional loop.
pub struct BackgroundScore {
    synthesizer: Synthesizer,
    score: Score,
    sample_rate: u32,
    cursor: usize,
    position_ticks: f64,
    ticks_per_frame: f64,
    loop_range_ticks: Option<(u64, u64)>,
    counters: ScoreCounters,
}

impl BackgroundScore {
    pub fn new(sound_font: &Arc<SoundFont>, score: Score, sample_rate: u32, maximum_polyphony: usize) -> Result<Self, SynthesizerError> {
        let mut settings = SynthesizerSettings::new(sample_rate as i32);
        settings.block_size = 64;
        settings.maximum_polyphony = maximum_polyphony;
        settings.enable_reverb_and_chorus = false;
        let mut player = Self {
            synthesizer: Synthesizer::new(sound_font, &settings)?,
            ticks_per_frame: 0.0,
            score,
            sample_rate,
            cursor: 0,
            position_ticks: 0.0,
            loop_range_ticks: None,
            counters: ScoreCounters::default(),
        };
        player.set_tempo(DEFAULT_MICROSECONDS_PER_QUARTER);
        Ok(player)
    }

    /// Loop between two ticks; `None` plays once and then holds silence.
    pub fn set_loop(&mut self, loop_range_ticks: Option<(u64, u64)>) {
        self.loop_range_ticks = loop_range_ticks;
    }

    pub fn position_ticks(&self) -> f64 {
        self.position_ticks
    }

    pub fn counters(&self) -> ScoreCounters {
        self.counters
    }

    /// Jump to `tick`, rebuilding every channel's state there.
    pub fn seek(&mut self, tick: u64) {
        let started = Instant::now();
        let (states, microseconds_per_quarter) = channel_states_at(&self.score.events, tick);
        self.set_tempo(microseconds_per_quarter);
        self.synthesizer.note_off_all(false);
        self.synthesizer.reset_all_controllers();
        for (channel, state) in states.iter().enumerate() {
            let channel = channel as i32;
            self.synthesizer.process_midi_message(channel, CONTROL_CHANGE, i32::from(BANK_SELECT), i32::from(state.bank));
            self.synthesizer.process_midi_message(channel, PROGRAM_CHANGE, i32::from(state.program), 0);
            for (controller, value) in state.controllers.iter().enumerate() {
                if let Some(value) = value {
                    self.synthesizer.process_midi_message(channel, CONTROL_CHANGE, controller as i32, i32::from(*value));
                }
            }
            self.synthesizer.process_midi_message(
                channel,
                PITCH_BEND,
                i32::from(state.pitch_bend & 0x7F),
                i32::from(state.pitch_bend >> 7),
            );
        }
        self.cursor = self.score.events.partition_point(|event| event.tick < tick);
        self.position_ticks = tick as f64;
        let elapsed_microseconds = started.elapsed().as_secs_f64() * 1_000_000.0;
        self.counters.reconstructions += 1;
        self.counters.longest_reconstruction_microseconds =
            self.counters.longest_reconstruction_microseconds.max(elapsed_microseconds);
    }

    /// Render `left.len()` frames (overwriting `left` and `right`). Events land on the
    /// synthesizer's 64-frame block boundaries, so a block is the timing grain.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        let frames = left.len();
        let end_ticks = self.position_ticks + self.ticks_per_frame * frames as f64;
        if let Some((loop_start, loop_end)) = self.loop_range_ticks {
            if end_ticks >= loop_end as f64 {
                self.apply_events_before(loop_end as f64);
                self.seek(loop_start);
                self.counters.loop_wraps += 1;
                self.synthesizer.render(left, right);
                self.position_ticks += self.ticks_per_frame * frames as f64;
                return;
            }
        }
        self.apply_events_before(end_ticks);
        self.synthesizer.render(left, right);
        self.position_ticks = end_ticks;
    }

    fn apply_events_before(&mut self, end_ticks: f64) {
        while let Some(event) = self.score.events.get(self.cursor) {
            if event.tick as f64 >= end_ticks {
                break;
            }
            let event = *event;
            self.cursor += 1;
            self.apply(event);
        }
    }

    fn apply(&mut self, event: ScoreEvent) {
        let channel = i32::from(event.channel);
        match event.message {
            ScoreMessage::NoteOn { key, velocity } => {
                self.synthesizer.process_midi_message(channel, NOTE_ON, i32::from(key), i32::from(velocity));
            }
            ScoreMessage::NoteOff { key } => self.synthesizer.process_midi_message(channel, NOTE_OFF, i32::from(key), 0),
            ScoreMessage::Controller { controller, value } => {
                self.synthesizer.process_midi_message(channel, CONTROL_CHANGE, i32::from(controller), i32::from(value));
            }
            ScoreMessage::Program(program) => {
                self.synthesizer.process_midi_message(channel, PROGRAM_CHANGE, i32::from(program), 0);
            }
            ScoreMessage::PitchBend(bend) => {
                self.synthesizer.process_midi_message(channel, PITCH_BEND, i32::from(bend & 0x7F), i32::from(bend >> 7));
            }
            ScoreMessage::Tempo { microseconds_per_quarter } => self.set_tempo(microseconds_per_quarter),
        }
    }

    fn set_tempo(&mut self, microseconds_per_quarter: u32) {
        let quarters_per_second = 1_000_000.0 / f64::from(microseconds_per_quarter.max(1));
        self.ticks_per_frame = quarters_per_second * f64::from(self.score.ticks_per_quarter) / f64::from(self.sample_rate);
    }
}

/// Every channel's state, and the tempo, just before `tick`: the fold a seek re-applies.
pub fn channel_states_at(events: &[ScoreEvent], tick: u64) -> ([ChannelState; CHANNEL_COUNT], u32) {
    let mut states = [ChannelState::default(); CHANNEL_COUNT];
    let mut tempo = DEFAULT_MICROSECONDS_PER_QUARTER;
    for event in events.iter().take_while(|event| event.tick < tick) {
        let state = &mut states[usize::from(event.channel) % CHANNEL_COUNT];
        match event.message {
            // Channel mode messages (all sound off, reset, all notes off) are actions,
            // not state: replaying one at the destination would cut what it restores.
            ScoreMessage::Controller { controller, .. } if controller >= CHANNEL_MODE_FIRST_CONTROLLER => {}
            ScoreMessage::Controller { controller, value } if controller == BANK_SELECT => state.bank = value,
            ScoreMessage::Controller { controller, value } => state.controllers[usize::from(controller)] = Some(value),
            ScoreMessage::Program(program) => state.program = program,
            ScoreMessage::PitchBend(bend) => state.pitch_bend = bend,
            ScoreMessage::Tempo { microseconds_per_quarter } => tempo = microseconds_per_quarter,
            ScoreMessage::NoteOn { .. } | ScoreMessage::NoteOff { .. } => {}
        }
    }
    (states, tempo)
}
