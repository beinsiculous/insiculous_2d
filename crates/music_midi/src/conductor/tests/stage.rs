//! A conductor on a test's own clock, and songs with trigger parts on the sine bank.

use std::sync::Arc;
use std::time::Duration;

use common::clock::Instant;

use super::*;
use crate::commands::CommandRejected;
use crate::pcm::OwnedPcm;
use crate::test_support::Step;

/// The sine bank's preset as an instrument named `name`.
pub(super) fn sine_instrument(name: &str) -> String {
    format!(r#"(name: "{name}", source: SoundFont(path: "{SINE_PATH}", bank: 0, preset: 0))"#)
}

/// A song whose control track ends at `end_tick` and whose other tracks are named trigger
/// tracks; `sidecar` holds its parts, instruments and bindings. Nothing plays in the
/// background, so the output is the slots' alone.
/// A sample instrument named `name` playing `path`, rooted on middle C, as the sine WAVs
/// are.
pub(super) fn sample_instrument(name: &str, path: &str, playback: &str) -> String {
    format!(
        r#"(name: "{name}", source: Sample(path: "{path}", root_note: 60, gain: 1.0, playback: {playback}))"#
    )
}

pub(super) fn trigger_song(
    tracks: Vec<(&'static str, Vec<Step>)>,
    end_tick: u32,
    looping: bool,
    sidecar: &str,
) -> Arc<Song> {
    trigger_song_with_samples(tracks, end_tick, looping, sidecar, &[])
}

/// [`trigger_song`] given the decoded samples its sample instruments play.
pub(super) fn trigger_song_with_samples(
    tracks: Vec<(&'static str, Vec<Step>)>,
    end_tick: u32,
    looping: bool,
    sidecar: &str,
    samples: &[(&str, OwnedPcm)],
) -> Arc<Song> {
    let mut midi_tracks = vec![vec![
        (0, tempo(500_000)),
        (end_tick, controller(15, 7, 100)),
    ]];
    for (name, steps) in tracks {
        let mut track = vec![(0, track_name(name))];
        track.extend(steps);
        midi_tracks.push(track);
    }
    let midi = smf(&midi_tracks);
    let file = parse_song_file(
        "test.mid.ron",
        &format!(
            r#"SongFile(version: 1, soundfont: "{SINE_PATH}", looping: {looping}, {sidecar})"#
        ),
    )
    .expect("the sidecar parses");
    let (data, _) = SongData::from_smf(&midi).expect("the MIDI reads");
    Arc::new(Song::new(file, data, &[(SINE_PATH, SINE_FONT)], samples).expect("the song builds"))
}

/// Notes one after another on channel 0: each `(key, velocity, length in ticks)` starts
/// where the previous one ended.
pub(super) fn melody(notes: &[(u8, u8, u32)]) -> Vec<Step> {
    let mut steps = Vec::new();
    for &(key, velocity, length) in notes {
        steps.push((0, note_on(0, key, velocity)));
        steps.push((length, note_off(0, key)));
    }
    steps
}

pub(super) fn source(id: u64) -> SourceId {
    SourceId {
        id,
        generation: 0,
        epoch: 1,
    }
}

/// A loaded conductor rendered in whole blocks on a clock that advances one second per
/// second of output, plus whatever time the test lets pass without rendering.
pub(super) struct Stage {
    pub conductor: Conductor,
    pub handle: ConductorHandle,
    started: Instant,
    pub frames: u64,
    stalled: Duration,
}

impl Stage {
    pub(super) fn new(song: Arc<Song>, config: ConductorConfig) -> Self {
        let (mut conductor, handle) = Conductor::new(config).expect("the config builds");
        conductor.load(song).expect("the song loads");
        Self {
            conductor,
            handle,
            started: Instant::now(),
            frames: 0,
            stalled: Duration::ZERO,
        }
    }

    /// The frozen config, playing.
    pub(super) fn playing(song: Arc<Song>) -> Self {
        let stage = Self::new(song, ConductorConfig::new(SAMPLE_RATE));
        stage.send(ConductorCommand::Play);
        stage
    }

    pub(super) fn now(&self) -> Instant {
        self.started
            + Duration::from_secs_f64(self.frames as f64 / f64::from(SAMPLE_RATE))
            + self.stalled
    }

    /// Let `duration` pass with nothing rendered: a stalled consumer.
    pub(super) fn stall(&mut self, duration: Duration) {
        self.stalled += duration;
    }

    /// Render `frames` frames, block by block, each drained at its own time.
    pub(super) fn render(&mut self, frames: usize) -> (Vec<f32>, Vec<f32>) {
        self.render_with(frames, BusGains::UNITY)
    }

    pub(super) fn render_with(&mut self, frames: usize, buses: BusGains) -> (Vec<f32>, Vec<f32>) {
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];
        for (left_block, right_block) in left
            .chunks_mut(BLOCK_FRAMES)
            .zip(right.chunks_mut(BLOCK_FRAMES))
        {
            let now = self.now();
            self.conductor
                .render_at(now, left_block, right_block, buses);
            self.frames += left_block.len() as u64;
        }
        (left, right)
    }

    /// Render `frames` frames in one call, as a host pulling a whole device period does.
    pub(super) fn render_in_one_call(&mut self, frames: usize) -> (Vec<f32>, Vec<f32>) {
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];
        let now = self.now();
        self.conductor
            .render_at(now, &mut left, &mut right, BusGains::UNITY);
        self.frames += frames as u64;
        (left, right)
    }

    pub(super) fn send(&self, command: ConductorCommand) {
        self.handle.send(command).expect("the command is queued");
    }

    /// Trigger `action` for `source`, stamped now.
    pub(super) fn trigger(&self, action: &str, source: SourceId) -> Result<(), CommandRejected> {
        self.trigger_stamped(action, source, self.now())
    }

    pub(super) fn trigger_stamped(
        &self,
        action: &str,
        source: SourceId,
        stamp: Instant,
    ) -> Result<(), CommandRejected> {
        let binding = self
            .conductor
            .song()
            .and_then(|song| song.binding_index(action))
            .expect("a declared action");
        self.handle.send(ConductorCommand::Trigger {
            binding,
            source,
            stamp,
        })
    }

    pub(super) fn sample_at_tick(&self, tick: u64) -> u64 {
        self.conductor
            .song()
            .map(|song| {
                song.background()
                    .tempo_map()
                    .sample_at_tick(tick, SAMPLE_RATE)
            })
            .expect("loaded")
    }

    pub(super) fn snapshot(&self) -> PositionSnapshot {
        self.handle.position().snapshot()
    }
}
