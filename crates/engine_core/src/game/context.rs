//! Construction of the game context shared by frames and key handlers.

macro_rules! build_context {
    ($runner:expr, $assets:expr, $delta_time:expr, $window_size:expr) => {
        crate::contexts::GameContext {
            input: &$runner.input,
            players: &mut $runner.player_input,
            world: &mut $runner.scene.world,
            assets: $assets,
            audio: &mut $runner.audio_manager,
            sources: &mut $runner.sound_sources,
            emitters: &mut $runner.sound_emitters,
            ui: &mut $runner.ui,
            delta_time: $delta_time,
            window_size: $window_size,
            display_scale: $runner.window_manager.display_scale(),
            ui_scale_for_engine_tail: None,
            chaos_mode: $runner.config.chaos_mode,
            time_scale: $runner.time_scale,
            requests: crate::contexts::FrameRequests::default(),
            achievements: &mut $runner.achievements,
            scores: &mut $runner.scores,
            particles: &mut $runner.particles,
            lines: &mut $runner.lines,
            strings: &mut $runner.localization.strings,
            #[cfg(feature = "physics")]
            scripts: &mut $runner.scripts,
        }
    };
}
pub(crate) use build_context;
