//! Simple game trait for rapid game development.
//!
//! This module provides a `Game` trait that hides all the complexity of
//! window management, event loops, and rendering setup. Game developers
//! just implement a few simple methods and call `run()`.
//!
//! # Example
//! ```no_run
//! use engine_core::prelude::*;
//!
//! struct MyGame {
//!     player_x: f32,
//! }
//!
//! impl Game for MyGame {
//!     fn update(&mut self, ctx: &mut GameContext) {
//!         if ctx.input.is_key_pressed(KeyCode::KeyD) {
//!             self.player_x += 5.0;
//!         }
//!     }
//! }
//!
//! fn main() {
//!     run_game(MyGame { player_x: 0.0 }, GameConfig::default()).unwrap();
//! }
//! ```

use glam::Vec2;
use winit::{event_loop::EventLoop, keyboard::KeyCode};

use audio::AudioManager;
use input::InputHandler;
use renderer::{sprite::SpriteBatcher, texture::TextureHandle};

mod app_handler;
mod frame_tail;
mod locale_font;
mod render;
#[cfg(target_arch = "wasm32")]
mod web;

use crate::{GameLoopManager, UIManager};
use crate::game_config::GameConfig;
use crate::ui_integration::render_ui_commands;
use ui::DrawCommand;
use crate::contexts::{GameContext, RenderContext};
use crate::assets::{AssetConfig, AssetManager};
use crate::achievements::AchievementManager;
use crate::glyph_texture_cache::GlyphTextureCache;
use crate::render_manager::RenderManager;
use crate::window_manager::{WindowConfig, WindowManager};
use crate::Scene;

use ecs::sprite_components::{Sprite as EcsSprite, Transform2D};

/// The main game trait. Implement this to create your game.
///
/// Only `update` is required - all other methods have default implementations.
pub trait Game: Sized + 'static {
    /// Called once when the game starts, after the window and renderer are ready.
    /// Use this to set up your initial game state, create entities, load assets, etc.
    fn init(&mut self, _ctx: &mut GameContext) {}

    /// Called every frame. Update your game logic here.
    /// This is the only required method.
    fn update(&mut self, ctx: &mut GameContext);

    /// Called every frame to render sprites. Add sprites to `ctx.sprites`.
    /// Default implementation extracts sprites from ECS entities with Transform2D and Sprite components,
    /// then renders UI draw commands on top.
    fn render(&mut self, ctx: &mut RenderContext) {
        // Tilemaps first, so equal-depth entity sprites draw over tiles
        crate::tilemap_render::append_tilemap_sprites(ctx.world, ctx.sprites);

        // Default: extract sprites from ECS
        for entity_id in ctx.world.entities() {
            let sprite = ctx.world.get::<EcsSprite>(entity_id);

            if let Some(ecs_sprite) = sprite {
                if !ecs_sprite.visible { continue; }
                // Use GlobalTransform2D if available (for hierarchical entities),
                // otherwise fall back to local Transform2D
                let (position, rotation, scale) = if let Some(global) = ctx.world.get::<ecs::hierarchy::GlobalTransform2D>(entity_id) {
                    (global.position, global.rotation, global.scale)
                } else if let Some(transform) = ctx.world.get::<Transform2D>(entity_id) {
                    (transform.position, transform.rotation, transform.scale)
                } else {
                    continue; // No transform, skip this entity
                };

                // Use the texture handle from the ECS sprite component
                let texture = TextureHandle { id: ecs_sprite.texture_handle };
                // The region carries sheet-cell UVs for animated and
                // sub-image sprites; the default [0, 0, 1, 1] is the full
                // texture, so unanimated content renders exactly as before.
                let region = ecs_sprite.tex_region;
                let renderer_sprite = renderer::Sprite::new(texture)
                    .with_position(position)
                    .with_rotation(rotation)
                    .with_scale(scale * ecs_sprite.scale * crate::RENDER_UNIT)
                    .with_tex_region(region[0], region[1], region[2], region[3])
                    .with_color(ecs_sprite.color)
                    .with_depth(ecs_sprite.depth)
                    .with_emissive(ecs_sprite.emissive);

                ctx.sprites.add_sprite(&renderer_sprite);
            }
        }

        // Render UI draw commands on top
        render_ui_commands(ctx.sprites, ctx.ui_commands, &*ctx.camera, ctx.glyph_textures);
    }

    /// Called by the editor when play mode is stopped and the world has been
    /// restored to its pre-play state. Override this to reset any non-ECS state
    /// (e.g., physics world) that was modified during play.
    fn on_play_stopped(&mut self, _ctx: &mut GameContext) {}

    /// Called when a key is pressed. Override for custom key handling.
    fn on_key_pressed(&mut self, _key: KeyCode, _ctx: &mut GameContext) {}

    /// Called when a key is released. Override for custom key handling.
    fn on_key_released(&mut self, _key: KeyCode, _ctx: &mut GameContext) {}

    /// Called when the window is resized.
    fn on_resize(&mut self, _width: u32, _height: u32) {}

    /// Called when the game is about to exit. Clean up resources here.
    fn on_exit(&mut self) {}
}

/// Run a game with the given configuration.
///
/// This function handles all the window creation, event loop, and rendering
/// boilerplate. Just implement the `Game` trait and call this function.
///
/// # Example
/// ```no_run
/// use engine_core::prelude::*;
///
/// struct MyGame;
///
/// impl Game for MyGame {
///     fn update(&mut self, _ctx: &mut GameContext) {
///         // Game logic here
///     }
/// }
///
/// fn main() {
///     run_game(MyGame, GameConfig::default()).unwrap();
/// }
/// ```
pub fn run_game<G: Game>(game: G, config: GameConfig) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let runner = GameRunner::new(game, config);

    // Native: block until the window closes. Web: hand the runner to the
    // browser's event loop and return immediately — `spawn_app` never blocks
    // (the wasm entry point has nothing left to do after this call).
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut runner = runner;
        event_loop.run_app(&mut runner)?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(runner);
    }
    Ok(())
}

/// Internal game runner that implements ApplicationHandler.
///
/// This struct orchestrates the game loop, delegating specific responsibilities
/// to focused managers:
/// - `WindowManager`: Window creation and size tracking
/// - `RenderManager`: Renderer lifecycle and sprite rendering
/// - `AssetManager`: Texture and asset loading
/// - `AudioManager`: Sound playback
/// - `GameLoopManager`: Frame timing and delta calculation
/// - `UIManager`: UI lifecycle and draw command collection
struct GameRunner<G: Game> {
    /// The user's game implementation
    game: G,
    /// Game configuration (title, size, etc.)
    config: GameConfig,
    /// Window management
    window_manager: WindowManager,
    /// Rendering management
    render_manager: RenderManager,
    /// Asset loading and management
    asset_manager: Option<AssetManager>,
    /// Audio playback management. Falls back to a disabled (no-op) manager
    /// when no audio device is available, so the game always runs.
    audio_manager: AudioManager,
    /// Input handling
    input: InputHandler,
    /// Gamepad hardware backend (gilrs); disabled no-op when unavailable.
    gamepad_backend: crate::gamepad_backend::GamepadBackend,
    /// Player-aware input bindings (universal per-player mapping layer),
    /// loaded from `GameConfig::input_settings_path` when set.
    player_input: input::InputSettings,
    /// True while the last dirty-triggered input-settings save failed —
    /// gates the warning to once per failure streak (retries stay silent).
    input_save_failing: bool,
    /// UI management
    ui_manager: UIManager,
    /// Game loop timing and frame management
    game_loop_manager: GameLoopManager,
    /// Cached glyph textures for text rendering
    glyph_textures: GlyphTextureCache,
    /// Renderer being created asynchronously by a browser task (web only);
    /// drained by the frame driver once the task completes.
    #[cfg(target_arch = "wasm32")]
    pending_renderer: web::PendingRenderer,
    /// H7 gesture-gated audio: failed upgrade attempts so far. Stops
    /// retrying at the cap in `game/web.rs`; never counts successes.
    #[cfg(target_arch = "wasm32")]
    audio_gesture_attempts: u8,
    /// Engine time multiplier mirrored onto `GameContext.time_scale`
    /// (read-write, persisted like chaos_mode). Scales particle stepping.
    time_scale: f32,
    /// Set when the game writes `GameContext.exit_requested` — triggers the
    /// clean shutdown path at the end of the frame.
    exit_requested: bool,
    /// Latched when rendering fails fatally (GPU device lost). The frame
    /// driver stops the loop instead of submitting to a dead queue — the
    /// failure mode that crashed Firefox's parent-process WebGPU.
    render_fatal: bool,
    /// Title requested via `GameContext.window_title`, applied to the OS
    /// window in the frame tail (window-system round-trips happen at most
    /// once per frame, and only when a title was actually requested).
    pending_window_title: Option<String>,
    /// Main game scene containing ECS world
    scene: Scene,
    /// Achievement / trophy manager
    achievements: AchievementManager,
    /// High-score lists (top-N per game-defined mode), persisted when
    /// `GameConfig::score_save_path` is set.
    scores: crate::scores::Scores,
    /// CPU-pooled particle system. Lives across frames so emitters can
    /// accumulate over time and spawn bursts can persist for their lifetime.
    particles: crate::particles::ParticleManager,
    /// Line vertex buffer that the game fills each frame and the engine
    /// uploads to the renderer. Cleared before every `update()`.
    lines: Vec<renderer::line_pipeline::LineVertex>,
    /// Persistent sprite batchers, cleared (capacity retained) each frame —
    /// no per-frame HashMap/Vec churn (GPP-15). Game and UI sprites batch
    /// separately so UI never shares a batch with (and paints over) sprites.
    game_batcher: SpriteBatcher,
    ui_batcher: SpriteBatcher,
    /// Localization tables (loaded from `GameConfig::locales_dir` at startup).
    strings: crate::localization::Strings,
    /// The game's own default font, captured after `init()` so locale font
    /// switches can restore it (`None` = game never loaded a font).
    base_font: Option<ui::FontHandle>,
    /// Locale font path → loaded handle, so cycling locales doesn't reload
    /// font files.
    locale_fonts: std::collections::HashMap<String, ui::FontHandle>,
    /// Button presses from this frame's UI-element pass. The event bus
    /// flushes BEFORE `update()`, so presses are held here and emitted onto
    /// the bus right after the next frame's flush (one-frame latency).
    pending_ui_events: Vec<crate::ui_element_system::UiButtonPressed>,
    /// Whether the game's init() has been called
    initialized: bool,
}

impl<G: Game> GameRunner<G> {
    fn new(game: G, config: GameConfig) -> Self {
        // Create window manager from game config
        let window_config = WindowConfig::new(&config.title)
            .with_size(config.width, config.height)
            .with_resizable(config.resizable);

        // Audio init failure is non-fatal: falls back to a disabled manager
        // whose playback calls are no-ops, so init()/update() always run.
        let audio_manager = AudioManager::new_or_disabled();

        let achievements = match &config.achievement_save_path {
            Some(path) => AchievementManager::with_save_path(path),
            None => AchievementManager::in_memory(),
        };

        let scores = match &config.score_save_path {
            Some(path) => crate::scores::Scores::with_save_path(path),
            None => crate::scores::Scores::in_memory(),
        };

        let mut game_loop_manager = GameLoopManager::new();
        game_loop_manager.set_target_fps(config.target_fps);

        let player_input = match &config.input_settings_path {
            Some(path) => crate::input_settings_io::load_or_create(std::path::Path::new(path)),
            None => input::InputSettings::default_two_player(),
        };

        // Locale tables live under the asset base (default `assets/locales`).
        let asset_base = config.asset_base_path.clone().unwrap_or_else(|| "assets".to_string());
        let locales_dir = std::path::Path::new(&asset_base).join(&config.locales_dir);
        let mut strings = crate::localization::Strings::load_dir(&locales_dir);
        strings.set_locale(config.locale.clone());

        Self {
            game,
            config,
            window_manager: WindowManager::new(window_config),
            render_manager: RenderManager::new(),
            asset_manager: None,
            audio_manager,
            input: InputHandler::new(),
            gamepad_backend: crate::gamepad_backend::GamepadBackend::new_or_disabled(),
            player_input,
            input_save_failing: false,
            ui_manager: UIManager::new(),
            game_loop_manager,
            glyph_textures: GlyphTextureCache::new(),
            #[cfg(target_arch = "wasm32")]
            pending_renderer: web::PendingRenderer::default(),
            #[cfg(target_arch = "wasm32")]
            audio_gesture_attempts: 0,
            time_scale: 1.0,
            exit_requested: false,
            render_fatal: false,
            pending_window_title: None,
            scene: Scene::new("main"),
            achievements,
            scores,
            particles: crate::particles::ParticleManager::default(),
            lines: Vec::new(),
            game_batcher: SpriteBatcher::new(),
            ui_batcher: SpriteBatcher::new(),
            strings,
            base_font: None,
            locale_fonts: std::collections::HashMap::new(),
            pending_ui_events: Vec::new(),
            initialized: false,
        }
    }

    /// Initialize the render manager with the current window (native only —
    /// blocks on wgpu setup; the web path spawns it async, see `game/web.rs`).
    #[cfg(not(target_arch = "wasm32"))]
    fn init_renderer(&mut self) -> Result<(), renderer::RendererError> {
        let window = self.window_manager.window_clone().ok_or_else(|| {
            renderer::RendererError::WindowCreationError("No window".to_string())
        })?;

        // Initialize render manager
        self.render_manager.init(
            window,
            self.config.clear_color,
            renderer::RendererConfig { vsync: self.config.vsync },
        )?;
        self.finish_renderer_setup();
        Ok(())
    }

    /// Shared tail of renderer bring-up (native blocking / web async):
    /// set the initial viewport and create the `AssetManager` from the live
    /// device and queue.
    fn finish_renderer_setup(&mut self) {
        self.render_manager.set_viewport_size(
            self.config.width as f32,
            self.config.height as f32,
        );

        // Create asset manager with renderer's device and queue
        if let (Some(device), Some(queue)) = (self.render_manager.device(), self.render_manager.queue()) {
            let asset_config = AssetConfig::from(&self.config);
            self.asset_manager = Some(AssetManager::with_config(device, queue, asset_config));
            log::info!("Asset manager initialized");
        }
    }

    /// Helper to get window size from window manager.
    fn window_size(&self) -> Vec2 {
        let (w, h) = self.window_manager.size();
        Vec2::new(w as f32, h as f32)
    }


    fn update_and_render(&mut self) {
        // Adopt the async-created renderer once its browser task lands
        // (web only; no-op after initialization).
        #[cfg(target_arch = "wasm32")]
        self.drain_pending_renderer();

        // Update game loop timing
        let delta_time = self.game_loop_manager.update();
        let window_size = self.window_size();

        // Check if managers are available (audio is optional)
        let has_managers = self.asset_manager.is_some();
        if !has_managers {
            return;
        }

        // Flush events from previous frame before processing new input
        self.scene.world.flush_events();

        // Emit last frame's UI-element button presses now that the bus is
        // fresh — games read them in update() one frame after the click.
        for event in self.pending_ui_events.drain(..) {
            self.scene.world.emit_event(event);
        }

        // Drain gamepad hardware events into the same queue as window events,
        // then process everything FIRST so UI and game logic see fresh state
        // with identical frame semantics across devices.
        self.gamepad_backend.pump(&mut self.input);
        self.input.process_queued_events();

        // Update all subsystems
        self.update_audio();
        self.update_ui_begin(window_size, delta_time);
        self.initialize_and_update(delta_time, window_size);
        let ui_commands = self.update_ui_end();
        self.update_input_end();

        // Render frame if ready
        if self.render_manager.is_initialized() {
            self.render_frame(window_size, &ui_commands);
        }
    }

    /// Update audio manager to clean up finished sounds
    fn update_audio(&mut self) {
        self.audio_manager.update();
    }

    /// Begin UI frame and process input
    fn update_ui_begin(&mut self, window_size: Vec2, delta_time: f32) {
        self.ui_manager.begin_frame(&self.input, window_size, delta_time);
    }

    /// Initialize game on first frame, then update game logic.
    fn initialize_and_update(&mut self, delta_time: f32, window_size: Vec2) {
        let Some(asset_manager) = self.asset_manager.as_mut() else {
            log::warn!("initialize_and_update called before asset manager exists; skipping frame");
            return;
        };

        // Clear the line buffer at the start of the frame so games push fresh
        // vertices each update (typical case: grid.build_line_vertices()).
        self.lines.clear();

        let mut ctx = GameContext {
            input: &self.input,
            players: &mut self.player_input,
            world: &mut self.scene.world,
            assets: asset_manager,
            audio: &mut self.audio_manager,
            ui: self.ui_manager.ui_context(),
            delta_time,
            window_size,
            chaos_mode: self.config.chaos_mode,
            time_scale: self.time_scale,
            exit_requested: false,
            window_title: None,
            achievements: &mut self.achievements,
            scores: &mut self.scores,
            particles: &mut self.particles,
            lines: &mut self.lines,
            strings: &mut self.strings,
        };

        let first_frame = !self.initialized;
        if first_frame {
            self.game.init(&mut ctx);
            self.initialized = true;
        }

        self.game.update(&mut ctx);

        // Persist any chaos-mode or time-scale change the game wrote to the
        // context, so both reflect the current runtime selection next frame.
        self.config.chaos_mode = ctx.chaos_mode;
        self.time_scale = ctx.time_scale;
        self.exit_requested |= ctx.exit_requested;
        if let Some(title) = ctx.window_title.take() {
            self.pending_window_title = Some(title);
        }

        // Engine-side frame tail: particles, lines, scene-defined UI,
        // toasts, locale fonts (game/frame_tail.rs).
        self.post_update(delta_time, window_size, first_frame);
    }

    /// End UI frame and return draw commands
    fn update_ui_end(&mut self) -> Vec<DrawCommand> {
        self.ui_manager.end_frame()
    }

    /// Clear input state for next frame
    fn update_input_end(&mut self) {
        self.input.end_frame();
    }

}
