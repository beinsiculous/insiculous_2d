//! The spike as a game: the worst-case storm under a running frame loop, keyboard
//! triggers for the listen check, and a log line every ten seconds.
//!
//! The source starts on the first frame the audio manager is enabled — natively at once,
//! in the browser on the first click or key (the engine's gesture gate). Until then the
//! frame windows are the "without the conductor" baseline.

use std::path::Path;

use engine_core::prelude::*;

use audio::Bus;
use common::clock::Instant;

use crate::frame_stats::FrameWindow;
use crate::mixer::{SlotTarget, SpikeCommand};
use crate::phrase::{opening_bars, CommandQueue, PhraseNote};
use crate::rig::{load_assets, worst_case_mixer, SpikeAssets, BACKGROUND_LOOP_TICKS};
use crate::sample_slot::TriggerMode;
use crate::spike_source::{spike_channel, SpikeSender};
use crate::storm::{Storm, StormConfig, SAMPLE_SLOT_ONLY, WORST_CASE};

/// Output rate of the source; rodio resamples to the device's rate.
pub const SOURCE_SAMPLE_RATE: u32 = 44_100;
const LOG_WINDOW_SECONDS: f32 = 10.0;
const AUTOMATIC_SEEK_AFTER_SECONDS: f64 = 60.0;
const KEYBOARD_SOURCE_A: u32 = 2_000;
const KEYBOARD_SOURCE_B: u32 = 2_001;
const HELD_KEY: u8 = 60;
const STORM_SEED: u64 = 0x5EED;

#[derive(Default)]
pub struct SpikeGame {
    assets: Option<SpikeAssets>,
    load_error: Option<String>,
    load_milliseconds: f64,
    phrase: Vec<PhraseNote>,
    sender: Option<SpikeSender>,
    started_at: Option<Instant>,
    queue: CommandQueue,
    storm: Option<Storm>,
    storm_config: Option<StormConfig>,
    sample_mode_gated: bool,
    seeked: bool,
    window: FrameWindow,
    program_swapped: bool,
}

impl SpikeGame {
    fn seconds_since_start(&self) -> f64 {
        self.started_at.map_or(0.0, |started| started.elapsed().as_secs_f64())
    }

    fn start_source(&mut self, ctx: &mut GameContext) {
        let Some(assets) = self.assets.as_ref() else {
            return;
        };
        let mode = if self.sample_mode_gated { TriggerMode::Gated } else { TriggerMode::OneShot };
        let mixer = match worst_case_mixer(assets, SOURCE_SAMPLE_RATE, mode) {
            Ok(mixer) => mixer,
            Err(error) => {
                self.load_error = Some(error);
                return;
            }
        };
        let (source, sender) = spike_channel(mixer, SOURCE_SAMPLE_RATE);
        if let Err(error) = ctx.audio.play_source(source, Bus::Music) {
            self.load_error = Some(error.to_string());
            return;
        }
        log::info!("conductor source started ({mode:?} samples)");
        self.sender = Some(sender);
        self.started_at = Some(Instant::now());
        self.storm = self.storm_config.map(|config| Storm::new(config, STORM_SEED));
        self.seeked = false;
    }

    fn stop_source(&mut self) {
        if let Some(sender) = self.sender.take() {
            sender.send(SpikeCommand::Stop);
            log::info!("conductor source stopped");
        }
        self.storm = None;
        self.queue = CommandQueue::default();
    }

    fn handle_keys(&mut self, ctx: &mut GameContext) {
        let now = self.seconds_since_start();
        let input = ctx.input;
        let piano = SlotTarget::Instrument(0);
        let strings = SlotTarget::Instrument(1);
        if input.is_key_just_pressed(KeyCode::Digit1) {
            self.queue.push_phrase(&self.phrase, piano, KEYBOARD_SOURCE_A, now);
        }
        if input.is_key_just_pressed(KeyCode::Digit2) {
            self.queue.push_phrase(&self.phrase, strings, KEYBOARD_SOURCE_B, now);
        }
        if input.is_key_just_pressed(KeyCode::Digit3) {
            self.queue.push_phrase(&self.phrase, piano, KEYBOARD_SOURCE_A, now);
            self.queue.push_phrase(&self.phrase, strings, KEYBOARD_SOURCE_B, now);
        }
        for (key, target, source) in [(KeyCode::KeyA, piano, KEYBOARD_SOURCE_A), (KeyCode::KeyB, strings, KEYBOARD_SOURCE_B)] {
            if input.is_key_just_pressed(key) {
                self.queue.push(now, SpikeCommand::NoteOn { target, source, key: HELD_KEY, velocity: 100 });
            }
            if input.is_key_just_released(key) {
                self.queue.push(now, SpikeCommand::NoteOff { target, source, key: HELD_KEY });
            }
        }
        if input.is_key_just_pressed(KeyCode::KeyT) {
            for offset in [0.0, 0.15] {
                self.queue.push(now + offset, SpikeCommand::NoteOn { target: piano, source: KEYBOARD_SOURCE_A, key: 67, velocity: 110 });
            }
            self.queue.push(now + 1.5, SpikeCommand::NoteOff { target: piano, source: KEYBOARD_SOURCE_A, key: 67 });
        }
        if input.is_key_just_pressed(KeyCode::KeyP) {
            self.program_swapped = !self.program_swapped;
            let program = if self.program_swapped { 6 } else { 0 };
            self.queue.push(now, SpikeCommand::ProgramChange { slot: 0, bank: 0, program });
        }
        for (key, pitch) in [(KeyCode::KeyQ, 60), (KeyCode::KeyW, 67), (KeyCode::KeyE, 48)] {
            if input.is_key_just_pressed(key) {
                self.queue.push(now, SpikeCommand::NoteOn { target: SlotTarget::Sample, source: KEYBOARD_SOURCE_A, key: pitch, velocity: 120 });
            }
            if input.is_key_just_released(key) {
                self.queue.push(now, SpikeCommand::NoteOff { target: SlotTarget::Sample, source: KEYBOARD_SOURCE_A, key: pitch });
            }
        }
        if input.is_key_just_pressed(KeyCode::KeyK) {
            self.queue.push(now, SpikeCommand::Seek { tick: BACKGROUND_LOOP_TICKS.1 / 2 });
        }
        if input.is_key_just_pressed(KeyCode::KeyS) {
            self.storm_config = match self.storm_config {
                None => Some(WORST_CASE),
                Some(_) => None,
            };
            self.storm = self.storm_config.map(|config| Storm::new(config, STORM_SEED));
            self.queue.push(now, SpikeCommand::ReleaseAll);
        }
        let restart = input.is_key_just_pressed(KeyCode::KeyV) || input.is_key_just_pressed(KeyCode::KeyG);
        if input.is_key_just_pressed(KeyCode::KeyV) {
            let sample_only = self.storm_config.is_some_and(|config| config.sample_slot_only);
            self.storm_config = Some(if sample_only { WORST_CASE } else { SAMPLE_SLOT_ONLY });
        }
        if input.is_key_just_pressed(KeyCode::KeyG) {
            self.sample_mode_gated = !self.sample_mode_gated;
        }
        if restart || input.is_key_just_pressed(KeyCode::Space) {
            let was_running = self.sender.is_some();
            self.stop_source();
            if restart || !was_running {
                self.start_source(ctx);
            }
        }
    }

    fn send_due_commands(&mut self) {
        let now = self.seconds_since_start();
        let mut commands = Vec::new();
        if let Some(storm) = self.storm.as_mut() {
            storm.commands_until(now, &mut commands);
        }
        if !self.seeked && now >= AUTOMATIC_SEEK_AFTER_SECONDS {
            self.seeked = true;
            commands.push(SpikeCommand::Seek { tick: BACKGROUND_LOOP_TICKS.1 / 2 });
        }
        self.queue.drain_due(now, &mut commands);
        if let Some(sender) = self.sender.as_ref() {
            for command in commands {
                sender.send(command);
            }
        }
    }

    fn log_window(&mut self) {
        let summary = self.window.finish();
        let conductor = match self.sender.as_ref() {
            None => "off".to_string(),
            Some(sender) => {
                let stats = &sender.stats;
                let elapsed = self.seconds_since_start();
                let rendered_seconds = stats.frames_rendered.load(std::sync::atomic::Ordering::Relaxed) as f64 / f64::from(SOURCE_SAMPLE_RATE);
                format!(
                    "on t={elapsed:.1}s rendered={rendered_seconds:.2}s lag={:.3}s block_mean={:.1}us block_longest={:.1}us drain_gap_longest={:.1}ms seeks_and_wraps={} reconstruction_longest={:.1}us peak_notes={} peak_sample_voices={} non_finite={} rejected={} largest_step={:.4}",
                    elapsed - rendered_seconds,
                    stats.mean_render_microseconds(),
                    stats.longest_render_microseconds(),
                    stats.longest_drain_gap_milliseconds(),
                    stats.reconstructions.load(std::sync::atomic::Ordering::Relaxed),
                    stats.longest_reconstruction_nanoseconds.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1000.0,
                    stats.peak_sounding_notes.load(std::sync::atomic::Ordering::Relaxed),
                    stats.peak_sample_voices.load(std::sync::atomic::Ordering::Relaxed),
                    stats.non_finite_samples.load(std::sync::atomic::Ordering::Relaxed),
                    sender.rejected_commands(),
                    stats.largest_step(),
                )
            }
        };
        log::info!(
            "frames={} mean={:.2}ms p99={:.2}ms longest={:.2}ms memory={:.1}MiB conductor={conductor}",
            summary.frames,
            summary.mean_milliseconds,
            summary.percentile_99_milliseconds,
            summary.longest_milliseconds,
            crate::memory::resident_mebibytes(),
        );
    }

    fn draw(&self, ctx: &mut GameContext) {
        let status = if let Some(error) = &self.load_error {
            format!("error: {error}")
        } else if self.sender.is_some() {
            let storm = match self.storm_config {
                None => "off",
                Some(config) if config.sample_slot_only => "sample slot only",
                Some(_) => "worst case",
            };
            let samples = if self.sample_mode_gated { "gated" } else { "one-shot" };
            format!("conductor on - storm {storm} - samples {samples} - loaded in {:.0} ms", self.load_milliseconds)
        } else if ctx.audio.is_enabled() {
            "conductor off - Space starts it".to_string()
        } else {
            "click or press a key to start audio".to_string()
        };
        let lines = [
            status.as_str(),
            "1 piano phrase  2 strings phrase  3 both  A/B hold middle C on piano/strings",
            "T same-pitch retrigger  P program change on piano  Q/W/E sample at C4/G4/C3",
            "K seek  S storm on/off  V storm variant  G one-shot/gated  Space source on/off",
        ];
        for (index, line) in lines.iter().enumerate() {
            ctx.ui.label(line, Vec2::new(20.0, 30.0 + 24.0 * index as f32));
        }
    }
}

impl Game for SpikeGame {
    fn register_achievements(&self, achievements: &mut AchievementManager, _strings: &Strings) {
        // build_wasm.sh refuses an empty achievements manifest, so the spike registers one.
        achievements.register(Achievement::new(
            "heard_the_conductor",
            "Heard the Conductor".to_string(),
            "Start the K1 spike's conductor source.".to_string(),
        ));
    }

    fn init(&mut self, ctx: &mut GameContext) {
        let font_path = Path::new(ctx.assets.base_path()).join("fonts/font.ttf");
        if let Ok(font) = ctx.ui.load_font_file(&font_path.to_string_lossy()) {
            ctx.ui.set_default_font(font);
        }
        let memory_before = crate::memory::resident_mebibytes();
        let started = Instant::now();
        match load_assets(Path::new(ctx.assets.base_path())) {
            Ok(assets) => {
                self.load_milliseconds = started.elapsed().as_secs_f64() * 1000.0;
                log::info!(
                    "assets loaded and decoded in {:.1} ms (memory {memory_before:.1} -> {:.1} MiB): bank {} bytes, background {} bytes, phrase {} bytes, sample {} bytes",
                    self.load_milliseconds,
                    crate::memory::resident_mebibytes(),
                    assets.sizes_bytes[0],
                    assets.sizes_bytes[1],
                    assets.sizes_bytes[2],
                    assets.sizes_bytes[3],
                );
                self.phrase = opening_bars(&assets.phrase, 2);
                self.assets = Some(assets);
            }
            Err(error) => {
                log::error!("spike assets failed to load: {error}");
                self.load_error = Some(error);
            }
        }
        self.storm_config = Some(WORST_CASE);
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.window.record(ctx.delta_time);
        if self.sender.is_none() && self.started_at.is_none() && ctx.audio.is_enabled() {
            self.start_source(ctx);
            ctx.achievements.unlock("heard_the_conductor");
        }
        self.handle_keys(ctx);
        self.send_due_commands();
        if self.window.elapsed_seconds >= LOG_WINDOW_SECONDS {
            self.log_window();
        }
        self.draw(ctx);
    }
}
