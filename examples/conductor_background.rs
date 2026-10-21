//! Conductor Background — a MIDI song played by the audio manager's conductor, with keys
//! that trigger its parts.
//!
//! Plays a fixture from `crates/music_midi/fixtures/` with a SoundFont through
//! `ctx.audio.play_song`, its background on the music bus. The default is the *Napolitan
//! Dance*'s cast derivative (flute, two piano hands and the tambourine, each on its own
//! General MIDI program); `--fixture napolitan` plays the Mutopia file itself,
//! `--fixture napolitan-type0` its type-0 derivative and `--fixture egmont` Beethoven's
//! *Egmont* overture.
//!
//! **The song on disk.** A song is a MIDI file with its `.mid.ron` sidecar beside it, so at
//! start the example copies the fixture into a directory of its own under the system's
//! temporary directory and writes the sidecar there, and removes the directory once the
//! song has loaded; the sidecar's SoundFont and sample paths are relative to the
//! repository, the example's asset base.
//!
//! **Triggers.** For the cast fixture the sidecar has two parts — the flute's melody
//! (channel 2) and the piano's left hand (channel 5), which then leave the background —
//! two instruments, the piano and the drum kit, and four bindings: keys 1 and 2 walk the
//! melody on the piano and on the kit, one shared cursor between them; keys 3 and 4 walk
//! the left hand the same way, each through `ctx.audio.trigger`. `--sample` adds a third
//! instrument, a gated sample instrument made from the middle-C sine fixture (root note
//! 60), and key 5 walks the left hand on it: its notes play at their written pitches,
//! higher ones shorter and lower ones longer. `--background-only` writes no parts, so the
//! whole score plays by itself.
//!
//! Every ten seconds it prints a `conductor=on` line from `song_position` and
//! `song_snapshot`: render cost per block, the longest gap between two blocks (how long a
//! command can wait), frame times, the position and the counters, triggers, sample voices
//! and the limiter's included.
//!
//! Controls (a held key acts once): 1–4 trigger (5 with `--sample`), Space pauses and
//! resumes, K seeks to bar 8, L turns looping off and on, M mutes and unmutes the music
//! bus (the triggers keep sounding), S mutes and unmutes the sfx bus (the background keeps
//! playing), Escape quits.
//!
//! Run with: cargo run --release --example conductor_background -- [SOUNDFONT]
//!   [--fixture napolitan|napolitan-type0|egmont] [--background-only] [--sample]
//!   [--seconds N]
//! The SoundFont defaults to the spike's cast bank when its `prepare_assets.sh` has been
//! run, and to the sine test bank otherwise; the cast bank has no flute, so its flute
//! channel falls back to the piano — any General MIDI bank, such as
//! `/usr/share/sounds/sf2/TimGM6mb.sf2`, plays it as a flute. The sine bank has no kit
//! either, and the kit bindings play its one preset. `--seconds N` quits after N seconds
//! and seeks to bar 8 once halfway, so an unattended run measures a seek as well as the
//! loop wraps.

use std::path::{Path, PathBuf};

use engine_core::prelude::*;
use music_midi::{SongData, SourceId, TempoMap, TransportState};

/// Anchor asset paths to the repository so the example runs from any working directory.
const REPOSITORY_DIR: &str = env!("CARGO_MANIFEST_DIR");
const CAST_SOUNDFONT: &str = "coordination/conductor/spike/assets/soundfonts/cast.sf2";
const SINE_SOUNDFONT: &str = "crates/music_midi/fixtures/sine.sf2";
const NAPOLITAN_DANCE: &str = "crates/music_midi/fixtures/dansenapolitaine.mid";
const NAPOLITAN_DANCE_TYPE_ZERO: &str = "crates/music_midi/fixtures/dansenapolitaine-type0.mid";
const NAPOLITAN_DANCE_CAST: &str = "crates/music_midi/fixtures/dansenapolitaine-cast.mid";
const EGMONT: &str = "crates/music_midi/fixtures/Egmont.mid";
const SINE_SAMPLE: &str = "crates/music_midi/fixtures/sine_48000_mono.wav";
const REPORT_SECONDS: f32 = 10.0;
/// Bar 8 as musicians count it; bars count from zero in the tempo map.
const SEEK_BAR: u64 = 7;
/// The keyboard is one source for the whole run: it never leaves the screen.
const KEYBOARD: SourceId = SourceId {
    id: 1,
    generation: 0,
    epoch: 1,
};
/// The five keys' actions, in key order; the fifth is bound only with `--sample`.
const ACTIONS: [&str; 5] = [
    "melody-piano",
    "melody-kit",
    "bass-piano",
    "bass-kit",
    "bass-sample",
];

/// The command line.
struct Settings {
    sound_font: PathBuf,
    fixture: &'static str,
    background_only: bool,
    sample: bool,
    quit_after_seconds: Option<f32>,
}

impl Settings {
    fn from_arguments() -> Self {
        let mut sound_font = None;
        let mut fixture = NAPOLITAN_DANCE_CAST;
        let mut background_only = false;
        let mut sample = false;
        let mut quit_after_seconds = None;
        let mut arguments = std::env::args().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--fixture" => match arguments.next().as_deref() {
                    Some("egmont") => fixture = EGMONT,
                    Some("napolitan") => fixture = NAPOLITAN_DANCE,
                    Some("napolitan-type0") => fixture = NAPOLITAN_DANCE_TYPE_ZERO,
                    other => println!("unknown fixture {other:?}; playing the cast"),
                },
                "--background-only" => background_only = true,
                "--sample" => sample = true,
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
            background_only,
            sample,
            quit_after_seconds,
        }
    }

    /// Whether this run plays the sample instrument: asked for, and the cast's parts
    /// written.
    fn plays_sample(&self) -> bool {
        self.sample && !self.background_only && self.fixture == NAPOLITAN_DANCE_CAST
    }

    /// The sidecar this run plays: the cast fixture's parts and bindings unless the
    /// background plays alone.
    fn sidecar(&self, sound_font_label: &str) -> String {
        if self.background_only || self.fixture != NAPOLITAN_DANCE_CAST {
            return format!(r#"SongFile(version: 1, soundfont: "{sound_font_label}")"#);
        }
        // The sine test bank has one preset, so there the kit is that preset too.
        let kit_bank = if self.sound_font == Path::new(SINE_SOUNDFONT) {
            0
        } else {
            128
        };
        let (sample_instrument, sample_binding) = if self.plays_sample() {
            (
                format!(
                    r#"(name: "sample", source: Sample(path: "{SINE_SAMPLE}", root_note: 60, gain: 1.0, playback: Gated)),"#
                ),
                r#"(action: "bass-sample", part: "bass", instrument: "sample"),"#,
            )
        } else {
            (String::new(), "")
        };
        format!(
            r#"SongFile(
                version: 1,
                soundfont: "{sound_font_label}",
                parts: [
                    (name: "melody", track: Name("channel 2")),
                    (name: "bass", track: Name("channel 5")),
                ],
                instruments: [
                    (name: "piano", source: SoundFont(path: "{sound_font_label}", bank: 0, preset: 0)),
                    (name: "kit", source: SoundFont(path: "{sound_font_label}", bank: {kit_bank}, preset: 0)),
                    {sample_instrument}
                ],
                bindings: [
                    (action: "melody-piano", part: "melody", instrument: "piano"),
                    (action: "melody-kit", part: "melody", instrument: "kit"),
                    (action: "bass-piano", part: "bass", instrument: "piano"),
                    (action: "bass-kit", part: "bass", instrument: "kit"),
                    {sample_binding}
                ],
            )"#
        )
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
    seek_beat: u64,
    looping: bool,
    music_muted: bool,
    sfx_muted: bool,
    elapsed_seconds: f32,
    seeked_halfway: bool,
    window: FrameWindow,
    /// Keys down now: the window system repeats a held key's press, and a held trigger
    /// key would fire at the keyboard's repeat rate.
    keys_down: Vec<KeyCode>,
}

impl ConductorBackground {
    fn new(settings: Settings) -> Self {
        Self {
            settings,
            seek_beat: 0,
            looping: true,
            music_muted: false,
            sfx_muted: false,
            elapsed_seconds: 0.0,
            seeked_halfway: false,
            window: FrameWindow::default(),
            keys_down: Vec::new(),
        }
    }

    /// Copy the fixture and write its sidecar into a directory of the example's own; the
    /// path of the copied MIDI file.
    fn write_song(&self, base_path: &Path) -> Result<PathBuf, String> {
        let directory =
            std::env::temp_dir().join(format!("insiculous_conductor_background_{}", std::process::id()));
        let stem = Path::new(self.settings.fixture)
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("song.mid"));
        let midi_path = directory.join(stem);
        let midi = common::vfs::read(&base_path.join(self.settings.fixture))
            .map_err(|error| format!("{}: {error}", self.settings.fixture))?;
        let sound_font_label = self.settings.sound_font.to_string_lossy().into_owned();
        let mut sidecar_path = midi_path.clone().into_os_string();
        sidecar_path.push(".ron");
        let write = |path: &Path, bytes: &[u8]| {
            common::vfs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()))
        };
        write(&midi_path, &midi)?;
        write(
            Path::new(&sidecar_path),
            self.settings.sidecar(&sound_font_label).as_bytes(),
        )?;
        println!("Song written to {}", midi_path.display());
        Ok(midi_path)
    }

    fn start(&mut self, ctx: &mut GameContext) -> Result<(), String> {
        let base_path = PathBuf::from(ctx.assets.base_path());
        let midi_path = self.write_song(&base_path)?;
        let midi = common::vfs::read(&midi_path).map_err(|error| error.to_string())?;
        let (data, _) = SongData::from_smf(&midi).map_err(|error| error.to_string())?;
        let tempo_map = data.tempo_map();
        self.seek_beat = tempo_map
            .position_at_tick(bar_start_tick(&tempo_map, SEEK_BAR))
            .beat;
        ctx.audio.set_asset_base(&base_path);
        let started = common::clock::Instant::now();
        let played = ctx.audio.play_song(&midi_path);
        // Every byte of the song is in memory once it loads, so its files can go.
        if let Some(directory) = midi_path.parent() {
            common::vfs::remove_prefix(directory).ok();
        }
        played.map_err(|error| error.to_string())?;
        println!(
            "song loaded in {:.1} ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
        Ok(())
    }

    fn trigger(&self, ctx: &mut GameContext, key_index: usize) {
        if let Err(error) = ctx.audio.trigger(ACTIONS[key_index], KEYBOARD) {
            println!("{} refused: {error}", ACTIONS[key_index]);
        }
    }

    fn report(&mut self, ctx: &GameContext) {
        let (Some(position), Some(snapshot)) = (ctx.audio.song_position(), ctx.audio.song_snapshot())
        else {
            return;
        };
        let (conductor, source) = (snapshot.conductor, snapshot.source);
        let blocks = conductor.blocks_rendered.max(1);
        let window = std::mem::take(&mut self.window);
        println!(
            "conductor=on t={:.0}s block_mean={:.1}us block_longest={:.1}us drain_gap_longest={:.2}ms \
             frame_mean={:.2}ms frame_longest={:.2}ms \
             bar={} beat={} latency_samples={} transport={:?} wraps={} reconstructions={} non_finite={} \
             applied={} rejected={} \
             triggers_accepted={} triggers_rejected={} triggers_late={} triggers_stale={} triggers_silent={} pending={} \
             sample_voices={} limited_samples={}",
            self.elapsed_seconds,
            source.render_nanoseconds_total as f64 / 1000.0 / blocks as f64,
            source.render_nanoseconds_longest as f64 / 1000.0,
            source.drain_gap_nanoseconds_longest as f64 / 1_000_000.0,
            window.seconds_total * 1000.0 / window.frames.max(1) as f32,
            window.seconds_longest * 1000.0,
            position.bar + 1,
            position.beat_in_bar + 1,
            position.latency_samples,
            conductor.transport,
            conductor.loop_wraps,
            conductor.reconstructions,
            conductor.non_finite_samples,
            conductor.commands_applied,
            conductor.commands_rejected,
            conductor.accepted_triggers,
            conductor.rejected_triggers,
            conductor.late_triggers,
            conductor.stale_triggers,
            conductor.silent_triggers,
            conductor.pending_events,
            conductor.sample_voices,
            source.limited_samples,
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

impl Game for ConductorBackground {
    fn init(&mut self, ctx: &mut GameContext) {
        println!("=== Conductor Background ===");
        println!("SoundFont: {}", self.settings.sound_font.display());
        println!("Song: {}", self.settings.fixture);
        println!("Controls: 1-5 trigger, Space pause/resume, K bar 8, L looping, M music, S sfx, Escape quit");
        if let Err(error) = self.start(ctx) {
            println!("the song cannot start: {error}");
            ctx.request_exit();
        }
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.elapsed_seconds += ctx.delta_time;
        self.window.frames += 1;
        self.window.seconds_total += ctx.delta_time;
        self.window.seconds_longest = self.window.seconds_longest.max(ctx.delta_time);
        if self.window.seconds_total >= REPORT_SECONDS {
            self.report(ctx);
        }
        if let Some(quit_after) = self.settings.quit_after_seconds {
            if !self.seeked_halfway && self.elapsed_seconds >= quit_after / 2.0 {
                self.seeked_halfway = true;
                ctx.audio.seek_song(self.seek_beat);
            }
            if self.elapsed_seconds >= quit_after {
                self.report(ctx);
                ctx.request_exit();
            }
        }
        let counters = ctx
            .audio
            .song_snapshot()
            .map(|snapshot| snapshot.conductor)
            .unwrap_or_default();
        let lines = [
            "Conductor Background".to_string(),
            "1 melody/piano  2 melody/kit  3 bass/piano  4 bass/kit  5 bass/sample".to_string(),
            "Space pause/resume   K bar 8   L looping   M music   S sfx".to_string(),
            format!(
                "looping: {}   music {}   sfx {}   triggers played {} refused {} late {} silent {}",
                self.looping,
                if self.music_muted { "off" } else { "on" },
                if self.sfx_muted { "off" } else { "on" },
                counters.accepted_triggers,
                counters.rejected_triggers,
                counters.late_triggers,
                counters.silent_triggers
            ),
        ];
        for (row, line) in lines.iter().enumerate() {
            ctx.ui
                .label(line, Vec2::new(20.0, 30.0 + row as f32 * 24.0));
        }
    }

    fn on_key_pressed(&mut self, key: KeyCode, ctx: &mut GameContext) {
        if self.keys_down.contains(&key) {
            return;
        }
        self.keys_down.push(key);
        match key {
            KeyCode::Space => {
                let paused = ctx
                    .audio
                    .song_snapshot()
                    .is_some_and(|snapshot| snapshot.conductor.transport == TransportState::Paused);
                if paused {
                    ctx.audio.resume_song();
                } else {
                    ctx.audio.pause_song();
                }
            }
            KeyCode::Digit1 => self.trigger(ctx, 0),
            KeyCode::Digit2 => self.trigger(ctx, 1),
            KeyCode::Digit3 => self.trigger(ctx, 2),
            KeyCode::Digit4 => self.trigger(ctx, 3),
            KeyCode::Digit5 => self.trigger(ctx, 4),
            KeyCode::KeyK => ctx.audio.seek_song(self.seek_beat),
            KeyCode::KeyL => {
                self.looping = !self.looping;
                ctx.audio.set_song_looping(self.looping);
            }
            KeyCode::KeyM => {
                self.music_muted = !self.music_muted;
                ctx.audio
                    .set_music_volume(if self.music_muted { 0.0 } else { 1.0 });
            }
            KeyCode::KeyS => {
                self.sfx_muted = !self.sfx_muted;
                ctx.audio
                    .set_sfx_volume(if self.sfx_muted { 0.0 } else { 1.0 });
            }
            KeyCode::Escape => ctx.request_exit(),
            _ => {}
        }
    }

    fn on_key_released(&mut self, key: KeyCode, _ctx: &mut GameContext) {
        self.keys_down.retain(|down| *down != key);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = GameConfig::new("Conductor Background - Insiculous 2D")
        .with_size(640, 190)
        .with_clear_color(0.08, 0.08, 0.1, 1.0)
        .with_asset_base_path(REPOSITORY_DIR);
    run_game(ConductorBackground::new(Settings::from_arguments()), config)?;
    Ok(())
}
