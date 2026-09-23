//! Shared fixtures for engine_core's tests.
//!
//! Two facts every scene test used to restate: a scene round-trip is
//! `world → world_to_scene_data → RON → parse → instantiate` through a
//! GPU-free [`TextureResolver`], and an input frame is "end the previous
//! frame, queue this frame's events, process them". Both live here once.
//!
//! A third: a game's update loop runs headlessly through [`GameHarness`],
//! which drives the engine's own runner and frame with a GPU-free asset
//! manager, so match flow — start, score, death, round change, game over —
//! is testable in any game crate without a window.
//!
//! Compiled for the crate's own unit tests and, behind the `test-support`
//! feature, for its integration tests (`tests/`) and downstream crates'
//! test builds — never for a shipped game.

use ecs::World;
use glam::Vec2;
use input::InputHandler;
use renderer::TextureHandle;

/// The event type [`GameHarness::step`] and [`frame`] take. Re-exported here
/// because a game crate depends on `engine_core` alone, and the prelude does
/// not carry it — a game never sees raw events, only its tests do.
pub use input::InputEvent;

use crate::achievements::AchievementManager;
use crate::contexts::GameContext;
use crate::game::{Game, GameRunner};
use crate::game_config::GameConfig;
use crate::scene_data::SceneLoadError;
use crate::scene_loader::{SceneInstance, SceneLoader};
use crate::scene_serializer::{serialize_to_ron, world_to_scene_data};
use crate::scores::Scores;
use crate::texture_ref::TextureResolver;

/// The texture-path function a save uses in tests: handle 0 is the built-in
/// white texture, every other handle becomes `#texture_<id>`.
pub fn test_texture_path(handle: u32) -> String {
    if handle == 0 {
        "#white".to_string()
    } else {
        format!("#texture_{handle}")
    }
}

/// GPU-free resolver: every reference resolves to the white texture and no
/// sheet has a sidecar, so a load falls back to the values baked into the
/// scene. Counts cache clears so a test can prove the loader asks for one
/// per load.
#[derive(Debug, Default)]
pub struct StubResolver {
    /// How many times a scene load asked for the sidecar cache to be dropped.
    pub cache_clears: usize,
}

impl TextureResolver for StubResolver {
    fn resolve_texture(&mut self, _texture_ref: &str) -> Result<TextureHandle, SceneLoadError> {
        Ok(TextureHandle::WHITE)
    }

    fn clear_sidecar_cache(&mut self) {
        self.cache_clears += 1;
    }
}

/// Save `world` to RON text the way the editor does.
pub fn save_to_ron(world: &World) -> String {
    let scene = world_to_scene_data(world, "RoundTrip", None, &test_texture_path);
    serialize_to_ron(&scene).expect("scene serializes")
}

/// Save `world` to RON, parse it back and instantiate it into a fresh world
/// through `resolver`.
pub fn roundtrip_with(world: &World, resolver: &mut impl TextureResolver) -> (World, SceneInstance) {
    let ron_text = save_to_ron(world);
    let parsed = SceneLoader::parse(&ron_text).expect("saved scene parses");
    let mut loaded = World::new();
    let instance =
        SceneLoader::instantiate(&parsed, &mut loaded, resolver).expect("saved scene instantiates");
    (loaded, instance)
}

/// [`roundtrip_with`] through a [`StubResolver`].
pub fn roundtrip(world: &World) -> (World, SceneInstance) {
    roundtrip_with(world, &mut StubResolver::default())
}

/// Parse RON scene text and instantiate it into a fresh world through a
/// [`StubResolver`].
pub fn load_ron(ron_text: &str) -> (World, SceneInstance) {
    let parsed = SceneLoader::parse(ron_text).expect("scene text parses");
    let mut world = World::new();
    let instance = SceneLoader::instantiate(&parsed, &mut world, &mut StubResolver::default())
        .expect("scene text instantiates");
    (world, instance)
}

/// One input frame: last frame's just-pressed / just-released edges are
/// cleared, then `events` are queued and processed. Held keys and buttons
/// stay held across frames — a hold is one `KeyPressed` until its
/// `KeyReleased`.
pub fn frame(input: &mut InputHandler, events: &[InputEvent]) {
    input.end_frame();
    for event in events {
        input.queue_event(event.clone());
    }
    input.process_queued_events();
}

/// A game driven headlessly: the real `GameRunner` with a headless asset
/// manager, stepped one frame at a time through the same function the window
/// loop calls, so a test sees exactly the frame a player gets — events
/// flushed, input processed, `init` on the first step, `update`, the engine's
/// tail. Only the timing and the render are left out.
///
/// Gamepad input is not drivable through [`step`](Self::step): the backend
/// is the disabled one, so a controller on the machine cannot reach a test.
/// Keyboard and mouse events are, and a key event also reaches the game's
/// `on_key_pressed` / `on_key_released` as the window loop delivers it.
pub struct GameHarness<G: Game> {
    runner: GameRunner<G>,
    window_size: Vec2,
    delta_time: f32,
}

impl<G: Game> GameHarness<G> {
    /// Build the runner as `run_game` and the window's `resumed` do, at the
    /// config's window size: the game's `register_scripts` and
    /// `register_achievements` run here, as does the scene's lifecycle (a
    /// failure panics, where the window logs it); `init` runs at the first
    /// [`step`](Self::step) or [`context`](Self::context), or on the first key.
    ///
    /// The config's save paths are honoured for real — achievements and
    /// scores write through on unlock and submit — so a test that reuses a
    /// game's shipped config points them at a temp dir or leaves them unset
    /// (in memory).
    pub fn new(game: G, config: GameConfig) -> Self {
        let window_size = Vec2::new(config.width as f32, config.height as f32);
        Self {
            runner: GameRunner::headless(game, config),
            window_size,
            delta_time: 1.0 / 60.0,
        }
    }

    /// A resize: the window manager and the game's `on_resize` see it as
    /// after a window event, and every later frame and context reads it. A
    /// window is whole pixels, so the size is truncated once and every
    /// reader sees the same integers.
    pub fn set_window_size(&mut self, size: Vec2) {
        let (width, height) = (size.x as u32, size.y as u32);
        self.window_size = Vec2::new(width as f32, height as f32);
        self.runner.resize(width, height);
    }

    /// One frame of `delta_time` seconds, with `events` arriving before it.
    /// A held key stays held until its `KeyReleased`. Each event is queued
    /// for the frame's input and, for a key, delivered to the game's handler
    /// at once — the window loop's order, `init` included: a key before the
    /// first frame runs `init` first, there and here.
    pub fn step(&mut self, delta_time: f32, events: &[InputEvent]) {
        for event in events {
            self.runner.queue_input_event(event.clone());
            self.runner.dispatch_key_event(event, self.window_size);
        }
        self.runner.step_frame(delta_time, self.window_size);
    }

    /// `count` frames of `delta_time` seconds with no input.
    pub fn steps(&mut self, count: usize, delta_time: f32) {
        for _ in 0..count {
            self.step(delta_time, &[]);
        }
    }

    /// Call into the game with a context built as a frame builds it, at a
    /// sixtieth of a second and the harness's window size — for a
    /// game's own entry points, which a frame would not reach. What the call
    /// requests (exit, title, time scale) lands as it would in a frame, but no
    /// frame runs around it.
    pub fn context<R>(&mut self, f: impl FnOnce(&mut G, &mut GameContext) -> R) -> R {
        self.runner.initialize_if_needed(self.window_size);
        self.runner.with_context(self.delta_time, self.window_size, f)
    }

    pub fn game(&self) -> &G {
        self.runner.game()
    }

    pub fn game_mut(&mut self) -> &mut G {
        self.runner.game_mut()
    }

    pub fn world(&self) -> &World {
        self.runner.world()
    }

    pub fn world_mut(&mut self) -> &mut World {
        self.runner.world_mut()
    }

    pub fn achievements(&self) -> &AchievementManager {
        self.runner.achievements()
    }

    pub fn scores(&self) -> &Scores {
        self.runner.scores()
    }

    /// Whether the game has asked to exit on any frame or context so far.
    pub fn exit_requested(&self) -> bool {
        self.runner.requests().exit
    }

    /// The title the game last set, in a frame or a lent context, or the
    /// config's.
    pub fn window_title(&self) -> &str {
        self.runner.window_title()
    }
}
