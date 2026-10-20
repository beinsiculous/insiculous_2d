//! Conductor Background — a MIDI song's background score played by the conductor.
//!
//! Loads a fixture from `crates/music_midi/fixtures/` (Tchaikovsky's *Napolitan Dance*;
//! its type-0 derivative, whose tambourine is moved to General MIDI percussion, with
//! `--fixture napolitan-type0`; or Beethoven's *Egmont* overture with `--fixture egmont`)
//! with a SoundFont, builds the conductor and plays it on the music bus once audio is
//! enabled. Every ten seconds it
//! prints a `conductor=on` line: render cost per block, the longest gap between two
//! blocks (how long a command can wait), the longest block that rebuilt the channels,
//! frame times, the position and the counters.
//!
//! Controls: Space pauses and resumes, K seeks to bar 8, L turns looping off and on,
//! Escape quits.
//!
//! Run with: cargo run --release --example conductor_background -- [SOUNDFONT]
//!   [--fixture napolitan-type0|egmont] [--seconds N]
//! The SoundFont defaults to the spike's cast bank when its `prepare_assets.sh` has been
//! run, and to the sine test bank otherwise. `--seconds N` quits after N seconds and
//! seeks to bar 8 once halfway, so an unattended run measures a seek as well as the loop
//! wraps.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use common::clock::Instant;
use engine_core::prelude::*;
use music_midi::{
    parse_song_file, BusGains, ChannelStateFold, Conductor, ConductorCommand, ConductorConfig,
    ConductorHandle, LoadWarning, Song, SongData, TempoMap, TransportState, BLOCK_FRAMES,
};

/// Anchor asset paths to the repository so the example runs from any working directory.
const REPOSITORY_DIR: &str = env!("CARGO_MANIFEST_DIR");
const CAST_SOUNDFONT: &str = "coordination/conductor/spike/assets/soundfonts/cast.sf2";
const SINE_SOUNDFONT: &str = "crates/music_midi/fixtures/sine.sf2";
const NAPOLITAN_DANCE: &str = "crates/music_midi/fixtures/dansenapolitaine.mid";
const NAPOLITAN_DANCE_TYPE_ZERO: &str = "crates/music_midi/fixtures/dansenapolitaine-type0.mid";
const EGMONT: &str = "crates/music_midi/fixtures/Egmont.mid";
const SAMPLE_RATE: u32 = 44_100;
const REPORT_SECONDS: f32 = 10.0;
/// Bar 8 as musicians count it; bars count from zero in the tempo map.
const SEEK_BAR: u64 = 7;

/// What the audio thread measures, readable from the game thread.
#[derive(Default)]
struct RenderStats {
    blocks: AtomicU64,
    render_nanoseconds_total: AtomicU64,
    render_nanoseconds_longest: AtomicU64,
    drain_gap_nanoseconds_longest: AtomicU64,
    /// The longest block in which the channels were rebuilt (a seek, a resume, a wrap).
    reconstruction_block_nanoseconds_longest: AtomicU64,
}

/// The conductor as a rodio `Source`: one block rendered at a time, its frames handed out
/// interleaved. A stand-in for the audio crate's own adapter, kept to what this example
/// needs to be heard and measured.
struct ConductorSource {
    conductor: Conductor,
    handle: ConductorHandle,
    stats: Arc<RenderStats>,
    left: [f32; BLOCK_FRAMES],
    right: [f32; BLOCK_FRAMES],
    /// The next interleaved sample to hand out; `2 * BLOCK_FRAMES` means the block is spent.
    next_sample: usize,
    previous_block_at: Option<Instant>,
}

impl ConductorSource {
    fn render_block(&mut self) {
        let started = Instant::now();
        if let Some(previous) = self.previous_block_at.replace(started) {
            let gap = started.duration_since(previous).as_nanos() as u64;
            self.stats
                .drain_gap_nanoseconds_longest
                .fetch_max(gap, Ordering::Relaxed);
        }
        let reconstructions_before = self.handle.position().snapshot().reconstructions;
        self.conductor
            .render(&mut self.left, &mut self.right, BusGains::UNITY);
        let nanoseconds = started.elapsed().as_nanos() as u64;
        let stats = &self.stats;
        stats.blocks.fetch_add(1, Ordering::Relaxed);
        stats
            .render_nanoseconds_total
            .fetch_add(nanoseconds, Ordering::Relaxed);
        stats
            .render_nanoseconds_longest
            .fetch_max(nanoseconds, Ordering::Relaxed);
        if self.handle.position().snapshot().reconstructions != reconstructions_before {
            stats
                .reconstruction_block_nanoseconds_longest
                .fetch_max(nanoseconds, Ordering::Relaxed);
        }
        self.next_sample = 0;
    }
}

impl Iterator for ConductorSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.next_sample == 2 * BLOCK_FRAMES {
            self.render_block();
        }
        let frame = self.next_sample / 2;
        let sample = if self.next_sample.is_multiple_of(2) {
            self.left[frame]
        } else {
            self.right[frame]
        };
        self.next_sample += 1;
        Some(sample)
    }
}

impl rodio::Source for ConductorSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// The command line.
struct Settings {
    sound_font: PathBuf,
    fixture: &'static str,
    quit_after_seconds: Option<f32>,
}

impl Settings {
    fn from_arguments() -> Self {
        let mut sound_font = None;
        let mut fixture = NAPOLITAN_DANCE;
        let mut quit_after_seconds = None;
        let mut arguments = std::env::args().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--fixture" => match arguments.next().as_deref() {
                    Some("egmont") => fixture = EGMONT,
                    Some("napolitan-type0") => fixture = NAPOLITAN_DANCE_TYPE_ZERO,
                    other => println!("unknown fixture {other:?}; playing the Napolitan Dance"),
                },
                "--seconds" => {
                    quit_after_seconds = arguments.next().and_then(|value| value.parse().ok())
                }
                path => sound_font = Some(PathBuf::from(path)),
            }
        }
        let sound_font = sound_font.unwrap_or_else(|| {
            let cast = Path::new(REPOSITORY_DIR).join(CAST_SOUNDFONT);
            if cast.exists() {
                PathBuf::from(CAST_SOUNDFONT)
            } else {
                PathBuf::from(SINE_SOUNDFONT)
            }
        });
        Self {
            sound_font,
            fixture,
            quit_after_seconds,
        }
    }
}

/// Frame times over one report window.
#[derive(Default)]
struct FrameWindow {
    frames: u32,
    seconds_total: f32,
    seconds_longest: f32,
}

struct ConductorBackground {
    settings: Settings,
    source: Option<ConductorSource>,
    handle: Option<ConductorHandle>,
    tempo_map: Option<TempoMap>,
    stats: Arc<RenderStats>,
    seek_tick: u64,
    looping: bool,
    elapsed_seconds: f32,
    seeked_halfway: bool,
    window: FrameWindow,
}

impl ConductorBackground {
    fn new(settings: Settings) -> Self {
        Self {
            settings,
            source: None,
            handle: None,
            tempo_map: None,
            stats: Arc::new(RenderStats::default()),
            seek_tick: 0,
            looping: true,
            elapsed_seconds: 0.0,
            seeked_halfway: false,
            window: FrameWindow::default(),
        }
    }

    fn build(&mut self, base_path: &Path) -> Result<(), String> {
        let read = |path: &Path| {
            common::vfs::read(&base_path.join(path))
                .map_err(|error| format!("{}: {error}", path.display()))
        };
        let sound_font_label = self.settings.sound_font.to_string_lossy().into_owned();
        let sound_font_bytes = read(&self.settings.sound_font)?;
        let midi_bytes = read(Path::new(self.settings.fixture))?;
        let sidecar = format!(r#"SongFile(version: 1, soundfont: "{sound_font_label}")"#);
        let file =
            parse_song_file("conductor_background", &sidecar).map_err(|error| error.to_string())?;
        let (data, _) = SongData::from_smf(&midi_bytes).map_err(|error| error.to_string())?;
        let song_started = Instant::now();
        let song = Song::new(
            file,
            data,
            &[(sound_font_label.as_str(), &sound_font_bytes)],
        )
        .map_err(|error| error.to_string())?;
        println!(
            "song prepared in {:.1} ms: {} events, {} SoundFont bytes",
            song_started.elapsed().as_secs_f64() * 1000.0,
            song.background().events().len(),
            sound_font_bytes.len()
        );

        let tempo_map = song.data().tempo_map();
        self.seek_tick = bar_start_tick(&tempo_map, SEEK_BAR);
        report_fold_costs(&song, self.seek_tick);
        let (mut conductor, handle) =
            Conductor::new(ConductorConfig::new(SAMPLE_RATE)).map_err(|error| error.to_string())?;
        let report = conductor
            .load(Arc::new(song))
            .map_err(|error| error.to_string())?;
        for warning in report.warnings {
            let LoadWarning::BackgroundMayOverflowPool {
                peak_voices,
                tick,
                polyphony,
            } = warning;
            println!("warning: {peak_voices} voices at tick {tick} may overflow the {polyphony}-voice pool");
        }
        handle
            .send(ConductorCommand::Play)
            .map_err(|error| error.to_string())?;
        self.source = Some(ConductorSource {
            conductor,
            handle: handle.clone(),
            stats: Arc::clone(&self.stats),
            left: [0.0; BLOCK_FRAMES],
            right: [0.0; BLOCK_FRAMES],
            next_sample: 2 * BLOCK_FRAMES,
            previous_block_at: None,
        });
        self.handle = Some(handle);
        self.tempo_map = Some(tempo_map);
        Ok(())
    }

    fn send(&self, command: ConductorCommand) {
        if let Some(handle) = &self.handle {
            if let Err(error) = handle.send(command) {
                println!("{command:?} refused: {error}");
            }
        }
    }

    fn report(&mut self) {
        let (Some(handle), Some(tempo_map)) = (&self.handle, &self.tempo_map) else {
            return;
        };
        let position = handle.position().snapshot();
        let musical = tempo_map.position_at_tick(position.tick);
        let stats = &self.stats;
        let blocks = stats.blocks.load(Ordering::Relaxed).max(1);
        let microseconds =
            |nanoseconds: &AtomicU64| nanoseconds.load(Ordering::Relaxed) as f64 / 1000.0;
        let window = std::mem::take(&mut self.window);
        println!(
            "conductor=on t={:.0}s block_mean={:.1}us block_longest={:.1}us drain_gap_longest={:.2}ms \
             reconstruction_block_longest={:.1}us frame_mean={:.2}ms frame_longest={:.2}ms \
             bar={} beat={} transport={:?} wraps={} reconstructions={} non_finite={} applied={} rejected={}",
            self.elapsed_seconds,
            microseconds(&stats.render_nanoseconds_total) / blocks as f64,
            microseconds(&stats.render_nanoseconds_longest),
            microseconds(&stats.drain_gap_nanoseconds_longest) / 1000.0,
            microseconds(&stats.reconstruction_block_nanoseconds_longest),
            window.seconds_total * 1000.0 / window.frames.max(1) as f32,
            window.seconds_longest * 1000.0,
            musical.bar + 1,
            musical.beat_in_bar + 1,
            position.transport,
            position.loop_wraps,
            position.reconstructions,
            position.non_finite_samples,
            position.commands_applied,
            position.commands_rejected,
        );
    }
}

/// The first tick of bar `bar` (from zero), walking beats through every meter change.
fn bar_start_tick(tempo_map: &TempoMap, bar: u64) -> u64 {
    (0..)
        .map(|beat| tempo_map.tick_at_beat(beat))
        .find(|tick| tempo_map.position_at_tick(*tick).bar >= bar)
        .unwrap_or(0)
}

/// Time the fold a loop wrap and a seek to bar 8 each rebuild the channels from, and the
/// costliest one, a seek to the song's last tick.
fn report_fold_costs(song: &Song, seek_tick: u64) {
    let events = song.background().events();
    let end_tick = song.background().end_tick();
    for (label, tick) in [
        ("loop start", 0),
        ("bar 8", seek_tick),
        ("song end", end_tick),
    ] {
        let cursor = events.partition_point(|event| event.tick <= tick);
        let started = Instant::now();
        let mut fold = ChannelStateFold::default();
        fold.fold(&events[..cursor]);
        println!(
            "fold at {label} (tick {tick}, {cursor} events): {:.1} us",
            started.elapsed().as_secs_f64() * 1_000_000.0
        );
    }
}

impl Game for ConductorBackground {
    fn init(&mut self, ctx: &mut GameContext) {
        println!("=== Conductor Background ===");
        println!("SoundFont: {}", self.settings.sound_font.display());
        println!("Song: {}", self.settings.fixture);
        println!("Controls: Space pause/resume, K bar 8, L looping, Escape quit");
        let base_path = PathBuf::from(ctx.assets.base_path());
        if let Err(error) = self.build(&base_path) {
            println!("the conductor cannot start: {error}");
            ctx.request_exit();
        }
    }

    fn update(&mut self, ctx: &mut GameContext) {
        if self.source.is_some() && ctx.audio.is_enabled() {
            if let Some(source) = self.source.take() {
                if let Err(error) = ctx.audio.play_source(source, Bus::Music) {
                    println!("the conductor cannot play: {error}");
                }
            }
        }
        self.elapsed_seconds += ctx.delta_time;
        self.window.frames += 1;
        self.window.seconds_total += ctx.delta_time;
        self.window.seconds_longest = self.window.seconds_longest.max(ctx.delta_time);
        if self.window.seconds_total >= REPORT_SECONDS {
            self.report();
        }
        if let Some(quit_after) = self.settings.quit_after_seconds {
            if !self.seeked_halfway && self.elapsed_seconds >= quit_after / 2.0 {
                self.seeked_halfway = true;
                self.send(ConductorCommand::Seek(self.seek_tick));
            }
            if self.elapsed_seconds >= quit_after {
                self.report();
                ctx.request_exit();
            }
        }
        let lines = [
            "Conductor Background".to_string(),
            "Space pause/resume   K bar 8   L looping".to_string(),
            format!("looping: {}", self.looping),
        ];
        for (row, line) in lines.iter().enumerate() {
            ctx.ui
                .label(line, Vec2::new(20.0, 30.0 + row as f32 * 24.0));
        }
    }

    fn on_key_pressed(&mut self, key: KeyCode, ctx: &mut GameContext) {
        match key {
            KeyCode::Space => {
                let transport = self
                    .handle
                    .as_ref()
                    .map(|handle| handle.position().transport());
                let command = if transport == Some(TransportState::Paused) {
                    ConductorCommand::Resume
                } else {
                    ConductorCommand::Pause
                };
                self.send(command);
            }
            KeyCode::KeyK => self.send(ConductorCommand::Seek(self.seek_tick)),
            KeyCode::KeyL => {
                self.looping = !self.looping;
                self.send(ConductorCommand::SetLooping(self.looping));
            }
            KeyCode::Escape => ctx.request_exit(),
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = GameConfig::new("Conductor Background - Insiculous 2D")
        .with_size(640, 160)
        .with_clear_color(0.08, 0.08, 0.1, 1.0)
        .with_asset_base_path(REPOSITORY_DIR);
    run_game(ConductorBackground::new(Settings::from_arguments()), config)?;
    Ok(())
}
