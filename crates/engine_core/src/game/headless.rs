//! The seams `test_support::GameHarness` drives a `GameRunner` through: a
//! constructor with no window and no GPU, the input queue, a lent context,
//! and read-back accessors. Test builds only.
//!
//! Child module of `game` (like `frame_tail`) so it can reach the runner's
//! private fields without widening their visibility.

use ecs::World;
use glam::Vec2;
use input::InputEvent;

use super::{Game, GameRunner};
use crate::achievements::AchievementManager;
use crate::assets::{AssetConfig, AssetManager};
use crate::contexts::{FrameRequests, GameContext};
use crate::game_config::GameConfig;
use crate::scores::Scores;

impl<G: Game> GameRunner<G> {
    /// A runner with a headless asset manager in place of the one a renderer
    /// would create. Mirrors everything `run_game` and the window's `resumed`
    /// do before the first frame: engine components are registered, the
    /// runner is built, the game registers its achievements, and the scene
    /// is initialized and started. The gamepad backend is the disabled one,
    /// so a controller on the machine cannot reach a test's input queue.
    pub(crate) fn headless(game: G, config: GameConfig) -> Self {
        crate::component_registration::register_engine_components();
        let mut runner = Self::new(game, config);
        runner.game.register_achievements(&mut runner.achievements, &runner.localization.strings);
        runner.gamepad_backend = crate::gamepad_backend::GamepadBackend::disabled();
        runner.asset_manager = Some(AssetManager::headless(AssetConfig::from(&runner.config)));
        runner.scene.initialize().expect("the headless scene initializes");
        runner.scene.start().expect("the headless scene starts");
        runner
    }

    /// Queue an input event; the next `step_frame` processes it.
    pub(crate) fn queue_input_event(&mut self, event: InputEvent) {
        self.input.queue_event(event);
    }

    /// A key event reaches the game's `on_key_pressed` / `on_key_released`
    /// the way the window loop delivers it: `init` first if it has not run,
    /// then the handler at once, through a zero-delta context, with what it
    /// wrote absorbed. Other events have no handler and pass through.
    pub(crate) fn dispatch_key_event(&mut self, event: &InputEvent, window_size: Vec2) {
        if matches!(event, InputEvent::KeyPressed(_) | InputEvent::KeyReleased(_)) {
            self.initialize_if_needed(window_size);
        }
        match *event {
            InputEvent::KeyPressed(key) => {
                self.with_context(0.0, window_size, |game, ctx| game.on_key_pressed(key, ctx));
            }
            InputEvent::KeyReleased(key) => {
                self.with_context(0.0, window_size, |game, ctx| game.on_key_released(key, ctx));
            }
            _ => {}
        }
    }

    /// A resize as the window loop applies one: the window manager, the
    /// render manager (which holds no renderer here) and the game's
    /// `on_resize`.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.resize_everything(width, height);
    }

    /// Lend the game and a context built exactly as a frame builds it, then
    /// absorb what the game wrote (exit, title, chaos mode, time scale). No
    /// frame runs around the call.
    pub(crate) fn with_context<R>(
        &mut self,
        delta_time: f32,
        window_size: Vec2,
        f: impl FnOnce(&mut G, &mut GameContext) -> R,
    ) -> R {
        let asset_manager = self
            .asset_manager
            .as_mut()
            .expect("headless runner owns its asset manager");
        let mut ctx = super::build_context!(self, asset_manager, delta_time, window_size);
        let result = f(&mut self.game, &mut ctx);
        let outcome = ctx.into_outcome();
        self.absorb(outcome);
        result
    }

    pub(crate) fn game(&self) -> &G {
        &self.game
    }

    pub(crate) fn game_mut(&mut self) -> &mut G {
        &mut self.game
    }

    pub(crate) fn world(&self) -> &World {
        &self.scene.world
    }

    pub(crate) fn world_mut(&mut self) -> &mut World {
        &mut self.scene.world
    }

    pub(crate) fn achievements(&self) -> &AchievementManager {
        &self.achievements
    }

    pub(crate) fn scores(&self) -> &Scores {
        &self.scores
    }

    /// What the game has requested so far; exit latches.
    pub(crate) fn requests(&self) -> &FrameRequests {
        &self.requests
    }

    /// The title the game last requested — pending from a lent context, or
    /// applied by a frame — else the config's.
    pub(crate) fn window_title(&self) -> &str {
        self.requests
            .window_title
            .as_deref()
            .unwrap_or_else(|| self.window_manager.title())
    }
}
