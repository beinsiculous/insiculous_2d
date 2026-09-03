//! Hello World - Demonstrates the simplified Game API with Physics, Audio, UI, and Scene Graph
//!
//! This example shows how easy it is to create a game with the Insiculous 2D engine.
//! All the window, event loop, and rendering boilerplate is handled internally.
//!
//! Features demonstrated:
//! - Simple Game API (Game trait, GameConfig, run_game)
//! - RON scene file loading for entity/component definition
//! - ECS for entity/component management
//! - Asset Manager with a config-level base path (cwd-independent asset loading)
//! - **Camera Follow** - a scene-defined main-camera entity with the
//!   `CameraFollow` behavior trails the player across a wide level
//! - **Player-aware input** - gameplay reads `ctx.players` (keyboard AND
//!   gamepad bindings for free); raw keys are reserved for debug toggles
//! - **Audio System** - sound effects and music playback
//! - **UI System** - immediate-mode UI with buttons, sliders, and panels
//! - **Font Rendering** - load TTF/OTF fonts for text display via fontdue
//! - **Action Mapping** - game-defined action enum bound through `InputMapping<A>`
//! - 2D Physics with rapier2d integration
//! - **Scene Graph Hierarchy** - parent-child entity relationships with transform propagation
//!
//! Controls: WASD to move player, SPACE (or pad A) to jump, R to reset,
//!           M to toggle music, +/- to adjust volume, H to toggle UI,
//!           L to cycle language (English/Pirate), ESC to exit
//!           Walk right past the gap — the camera follows! Collect the coins!
//!
//! Scene file: examples/assets/scenes/hello_world.scene.ron
//! Font file: examples/assets/fonts/font.ttf (optional - download any TTF font)

use engine_core::prelude::*;
use input::{InputMapping, InputSource};
use std::path::Path;

/// Anchor all asset paths to the repository so the example runs from any
/// working directory (`GameConfig::with_asset_base_path` resolves textures
/// referenced by the scene file against this directory).
const EXAMPLES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");

/// Where the player respawns on reset (matches the scene file's spawn point).
const PLAYER_SPAWN: Vec2 = Vec2::new(-200.0, 100.0);

// --- Actions: game-defined enum evaluated through the engine's InputMapping ---

/// Demo-level DEBUG actions (music/UI/volume toggles, reset). Gameplay input
/// goes through the engine's player-aware layer (`ctx.players` — see the
/// jump-sound check in `update`); movement and jumping themselves are driven
/// by the scene's `PlayerPlatformer` behavior. Raw keys are the convention
/// for debug toggles only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DemoAction {
    ToggleMusic,
    ToggleUi,
    ResetPlayer,
    VolumeUp,
    VolumeDown,
    CycleLocale,
}

fn demo_actions() -> InputMapping<DemoAction> {
    let mut actions = InputMapping::new();
    actions.bind(DemoAction::ToggleMusic, InputSource::Keyboard(KeyCode::KeyM));
    actions.bind(DemoAction::ToggleUi, InputSource::Keyboard(KeyCode::KeyH));
    actions.bind(DemoAction::ResetPlayer, InputSource::Keyboard(KeyCode::KeyR));
    actions.bind(DemoAction::VolumeUp, InputSource::Keyboard(KeyCode::Equal));
    actions.bind(DemoAction::VolumeDown, InputSource::Keyboard(KeyCode::Minus));
    actions.bind(DemoAction::CycleLocale, InputSource::Keyboard(KeyCode::KeyL));
    actions
}

// --- Resources: typed singleton state stored in the World ---

/// Cross-system game state accessible by any system via `world.resource::<GameState>()`.
/// This is the Repository pattern adapted for ECS — a single source of truth for
/// game-wide state like score and lives.
#[derive(Debug, Clone, Default)]
struct GameState {
    score: u32,
    coins_collected: u32,
}

// --- State Machine: per-entity state for the player ---

/// Player behavioral states, driven by physics velocity and input.
/// Systems read this to decide animations, sounds, UI display, etc.
#[derive(Debug, Clone, PartialEq)]
enum PlayerState {
    Idle,
    Running,
    Jumping,
    Falling,
}

/// Player state groups for shared behavior across related states.
#[derive(Debug, Clone, PartialEq)]
enum PlayerGroup {
    OnGround,
    InAir,
}

fn player_group(state: &PlayerState) -> PlayerGroup {
    match state {
        PlayerState::Idle | PlayerState::Running => PlayerGroup::OnGround,
        PlayerState::Jumping | PlayerState::Falling => PlayerGroup::InAir,
    }
}

/// Our game state - simplified with BehaviorRunner handling player movement
struct HelloWorld {
    physics: Option<PhysicsSystem>,
    /// Behavior runner - processes all entity behaviors (player movement, AI, etc.)
    behaviors: BehaviorRunner,
    /// Scene instance with named entity lookups
    scene_instance: Option<SceneInstance>,
    /// Transform hierarchy system for parent-child relationships
    transform_hierarchy: TransformHierarchySystem,
    /// Action bindings for the demo's manual controls
    actions: InputMapping<DemoAction>,
    /// The jump sound effect handle (if loaded)
    jump_sound: Option<SoundHandle>,
    /// Whether music is currently playing
    music_playing: bool,
    /// Volume slider value (0.0 to 1.0)
    volume: f32,
    /// Whether to show the UI panel
    show_ui: bool,
    /// Whether a font was successfully loaded
    font_loaded: bool,
}

impl HelloWorld {
    fn new() -> Self {
        Self {
            physics: None,
            behaviors: BehaviorRunner::new(),
            scene_instance: None,
            transform_hierarchy: TransformHierarchySystem::new(),
            actions: demo_actions(),
            jump_sound: None,
            music_playing: false,
            volume: 1.0,
            show_ui: true,
            font_loaded: false,
        }
    }

    fn player_entity(&self) -> Option<EntityId> {
        self.scene_instance
            .as_ref()
            .and_then(|scene| scene.get_entity("player"))
    }

    /// Move the player back to spawn and zero its velocity.
    ///
    /// Once a body exists, rapier is authoritative — writing to `Transform2D`
    /// or `RigidBody.velocity` on a live physics entity has no effect, so the
    /// reset goes through the physics API (`reset_body` is also safe for
    /// entities spawned the same frame).
    fn reset_player(&mut self, ctx: &mut GameContext) {
        let Some(player) = self.player_entity() else { return };

        if let Some(physics) = &mut self.physics {
            physics.reset_body(player, PLAYER_SPAWN);
        } else if let Some(transform) = ctx.world.get_mut::<Transform2D>(player) {
            // No physics running: the transform is the source of truth.
            transform.position = PLAYER_SPAWN;
        }
    }

    fn toggle_music(&mut self, ctx: &mut GameContext) {
        if self.music_playing {
            ctx.audio.pause_music();
            self.music_playing = false;
            println!("Music paused");
        } else {
            ctx.audio.resume_music();
            self.music_playing = true;
            println!("Music resumed");
        }
    }
}

impl Game for HelloWorld {
    /// Called once at startup - loads a scene from a .scene.ron file
    fn init(&mut self, ctx: &mut GameContext) {
        // Try to load the scene from RON file
        let scene_path = Path::new(EXAMPLES_DIR).join("assets/scenes/hello_world.scene.ron");

        match SceneLoader::load_and_instantiate(&scene_path, ctx.world, ctx.assets) {
            Ok(instance) => {
                println!("Loaded scene '{}' with {} entities", instance.name, instance.entity_count);

                // Set named entities on the behavior runner (for FollowEntity and other reference-based behaviors)
                self.behaviors.set_named_entities(instance.named_entities.clone());

                // Create a physics system from scene settings
                let physics_config = if let Some(settings) = &instance.physics {
                    PhysicsConfig::new(Vec2::new(settings.gravity.0, settings.gravity.1))
                        .with_scale(settings.pixels_per_meter)
                } else {
                    PhysicsConfig::platformer()
                };

                self.physics = Some(PhysicsSystem::with_config(physics_config));
                self.scene_instance = Some(instance);
            }
            Err(e) => {
                println!("Failed to load scene: {}", e);
                println!("Creating entities programmatically as fallback...");

                // Fallback: create entities manually with a Behavior component
                let player = ctx.world.create_entity();
                ctx.world.add_component(&player, Transform2D::new(PLAYER_SPAWN)).ok();
                ctx.world.add_component(&player, Sprite::new(0).with_color(Vec4::new(0.2, 0.4, 1.0, 1.0))).ok();
                ctx.world.add_component(&player, RigidBody::player_platformer()).ok();
                ctx.world.add_component(&player, Collider::player_box(80.0, 80.0)).ok();
                // Add behavior for player-controlled platformer movement
                ctx.world.add_component(&player, Behavior::PlayerPlatformer {
                    move_speed: 120.0,
                    jump_impulse: 420.0,
                    jump_cooldown: 0.3,
                    tag: "player".to_string(),
                }).ok();

                // Create ground
                let ground = ctx.world.create_entity();
                ctx.world.add_component(&ground,
                    Transform2D::new(Vec2::new(0.0, -250.0))
                        .with_scale(Vec2::new(10.0, 0.5))
                ).ok();
                ctx.world.add_component(&ground,
                    Sprite::new(0).with_color(Vec4::new(0.3, 0.3, 0.3, 1.0))
                ).ok();
                ctx.world.add_component(&ground, RigidBody::new_static()).ok();
                ctx.world.add_component(&ground, Collider::platform(800.0, 40.0)).ok();

                self.physics = Some(PhysicsSystem::with_config(PhysicsConfig::platformer()));
            }
        }

        // Initialize the physics system
        if let Some(physics) = &mut self.physics {
            physics.initialize(ctx.world).ok();
        }

        // Initialize the transform hierarchy system
        self.transform_hierarchy.initialize(ctx.world).ok();

        // --- Resource: insert game-wide state ---
        ctx.world.insert_resource(GameState::default());

        // --- State Machine: attach to player entity ---
        if let Some(player) = self.player_entity() {
            ctx.world.add_component(
                &player,
                HierarchicalStateMachine::new(PlayerState::Idle, player_group),
            ).ok();
        }

        // Count entities with hierarchy relationships
        let root_count = ctx.world.get_root_entities().len();
        let total_count = ctx.world.entity_count();
        let child_count = total_count - root_count;

        // Try to load sound effects
        match ctx.audio.load_sound(Path::new(EXAMPLES_DIR).join("assets/sounds/snd_jump.wav")) {
            Ok(handle) => {
                self.jump_sound = Some(handle);
                println!("Loaded jump sound effect");
            }
            Err(e) => {
                println!("No jump sound loaded ({}). Audio demo will show API usage.", e);
                println!("To enable audio, add a WAV file at examples/assets/sounds/snd_jump.wav");
            }
        }

        // Try to load background music
        match ctx.audio.play_music(Path::new(EXAMPLES_DIR).join("assets/sounds/music.ogg")) {
            Ok(()) => {
                self.music_playing = true;
                println!("Playing background music");
            }
            Err(_) => {
                println!("No background music found. Add music.ogg to examples/assets/sounds/");
            }
        }

        // Try to load a font for text rendering
        let bundled_font = format!("{EXAMPLES_DIR}/assets/fonts/font.ttf");
        if ctx.ui.load_font_file(&bundled_font).is_ok() {
            self.font_loaded = true;
            println!("Font loaded - text will render with actual glyphs!");
        } else {
            // Try fallback paths for common system fonts
            let font_paths = [
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/TTF/DejaVuSans.ttf",
                "/System/Library/Fonts/Helvetica.ttc",
                "C:\\Windows\\Fonts\\arial.ttf",
            ];

            for path in font_paths {
                if ctx.ui.load_font_file(path).is_ok() {
                    self.font_loaded = true;
                    println!("System font loaded from: {}", path);
                    break;
                }
            }

            if !self.font_loaded {
                println!("No font loaded. Text will render as placeholders.");
                println!("To enable font rendering, add a .ttf file to examples/assets/fonts/font.ttf");
            }
        }

        println!("Game initialized with {} entities ({} root, {} children)",
                 total_count, root_count, child_count);
        println!("Controls: WASD to move, SPACE to jump, R to reset, M to toggle music, H to toggle UI, ESC to exit");
        println!("Physics enabled - push the wood boxes around!");
        if child_count > 0 {
            println!("Scene Graph: {} child entities will follow their parents!", child_count);
        }
        println!("Audio system ready - master volume: {:.0}%", ctx.audio.master_volume() * 100.0);
        println!("UI system ready - click buttons and drag sliders!");
        if self.font_loaded {
            println!("Font system ready - text renders with actual glyphs!");
        }
    }

    /// Called every frame - update game logic
    fn update(&mut self, ctx: &mut GameContext) {
        // Play jump sound on the strict press edge (if sound is loaded).
        // The jump itself is handled by the PlayerPlatformer behavior.
        // Single-player convention: listen to BOTH player slots so Space,
        // Enter, and either gamepad's A button all work.
        if ctx.players.just_activated(PlayerId::P1, GameAction::Action1, ctx.input)
            || ctx.players.just_activated(PlayerId::P2, GameAction::Action1, ctx.input)
        {
            if let Some(jump_sound) = &self.jump_sound {
                let settings = SoundSettings::new()
                    .with_volume(0.8)
                    .with_speed(1.0);
                if let Err(e) = ctx.audio.play_with_settings(jump_sound, settings) {
                    eprintln!("Failed to play jump sound: {}", e);
                }
            }
        }

        // Toggle music
        if self.actions.just_activated(DemoAction::ToggleMusic, ctx.input) {
            self.toggle_music(ctx);
        }

        // Cycle the UI language (en → pirate → en; pirate also swaps the
        // default font to BlackSamsGold via the locale file's `font` field)
        if self.actions.just_activated(DemoAction::CycleLocale, ctx.input) {
            ctx.strings.cycle_locale();
            println!("Locale: {}", ctx.strings.current_display_name());
        }

        // Adjust master volume
        if self.actions.just_activated(DemoAction::VolumeUp, ctx.input) {
            let new_volume = (ctx.audio.master_volume() + 0.1).min(1.0);
            ctx.audio.set_master_volume(new_volume);
            println!("Volume: {:.0}%", new_volume * 100.0);
        }
        if self.actions.just_activated(DemoAction::VolumeDown, ctx.input) {
            let new_volume = (ctx.audio.master_volume() - 0.1).max(0.0);
            ctx.audio.set_master_volume(new_volume);
            println!("Volume: {:.0}%", new_volume * 100.0);
        }

        // Process all entity behaviors (player movement, AI, etc.)
        // This single call replaces 40+ lines of hardcoded movement logic!
        self.behaviors.update(
            ctx.world,
            ctx.input,
            ctx.delta_time,
            self.physics.as_mut(),
        );

        // Reset player position (manual action, not a behavior)
        if self.actions.just_activated(DemoAction::ResetPlayer, ctx.input) {
            self.reset_player(ctx);
        }

        // Step physics simulation (drain collisions via take_collision_events())
        if let Some(physics) = &mut self.physics {
            physics.update(ctx.world, ctx.delta_time);
        }

        // Update transform hierarchy - propagates transforms from parents to children
        // This must run after physics so child entities follow their parents
        self.transform_hierarchy.update(ctx.world, ctx.delta_time);

        // --- Events: read collection events emitted by BehaviorRunner ---
        // Any system can read these — audio, particles, scoring all stay decoupled
        let collected: Vec<EntityCollected> = ctx.world.read_events::<EntityCollected>().to_vec();
        for event in &collected {
            if let Some(state) = ctx.world.resource_mut::<GameState>() {
                state.score += event.score_value;
                state.coins_collected += 1;
            }
            println!("Collected! +{} points (total: {})",
                event.score_value,
                ctx.world.resource::<GameState>().map(|s| s.score).unwrap_or(0));
        }

        // --- State Machine: update player state based on physics velocity ---
        if let Some(player) = self.player_entity() {
            // Determine state from physics velocity
            let vel = ctx.world.get::<RigidBody>(player)
                .map(|rb| rb.velocity)
                .unwrap_or(Vec2::ZERO);
            let moving_x = vel.x.abs() > 5.0;

            let new_state = if vel.y > 10.0 {
                PlayerState::Jumping
            } else if vel.y < -10.0 {
                PlayerState::Falling
            } else if moving_x {
                PlayerState::Running
            } else {
                PlayerState::Idle
            };

            if let Some(sm) = ctx.world.get_mut::<HierarchicalStateMachine<PlayerState, PlayerGroup>>(player) {
                sm.transition_to(new_state);
                sm.tick(ctx.delta_time);
            }
        }

        // ==================== UI Demo ====================
        // Toggle UI visibility
        if self.actions.just_activated(DemoAction::ToggleUi, ctx.input) {
            self.show_ui = !self.show_ui;
        }

        // Create UI elements (immediate-mode - describe UI every frame)
        // Labels render with actual fonts if loaded, otherwise as placeholder rectangles
        if self.show_ui {
            // Draw a semi-transparent control panel in the top-left.
            // Every literal goes through ctx.strings.tr(...) so the L key
            // (and the editor's View menu) can swap languages live.
            let panel_rect = UIRect::new(10.0, 10.0, 220.0, 250.0);
            ctx.ui.panel(panel_rect);

            // Title label (renders with font glyphs if font loaded)
            let title = ctx.strings.tr("panel.title").to_string();
            ctx.ui.label(&title, Vec2::new(20.0, 25.0));

            // --- Score display (from Resource) ---
            let score = ctx.world.resource::<GameState>().map(|s| s.score).unwrap_or(0);
            let coins = ctx.world.resource::<GameState>().map(|s| s.coins_collected).unwrap_or(0);
            let score_text = format!(
                "{}: {} ({} {})",
                ctx.strings.tr("panel.score"), score, coins, ctx.strings.tr("panel.coins"),
            );
            ctx.ui.label(&score_text, Vec2::new(20.0, 50.0));

            // --- Player state display (from StateMachine) ---
            let state_label = ctx.strings.tr("panel.state").to_string();
            let state_text = if let Some(player) = self.player_entity() {
                if let Some(sm) = ctx.world.get::<HierarchicalStateMachine<PlayerState, PlayerGroup>>(player) {
                    format!("{}: {:?} ({:?})", state_label, sm.current(), sm.parent())
                } else {
                    format!("{}: N/A", state_label)
                }
            } else {
                format!("{}: No player", state_label)
            };
            ctx.ui.label(&state_text, Vec2::new(20.0, 70.0));

            // Volume slider
            let volume_label = ctx.strings.tr("panel.volume").to_string();
            ctx.ui.label(&volume_label, Vec2::new(20.0, 95.0));
            let slider_rect = UIRect::new(20.0, 110.0, 190.0, 20.0);
            let new_volume = ctx.ui.slider("volume_slider", self.volume, slider_rect);
            if new_volume != self.volume {
                self.volume = new_volume;
                ctx.audio.set_master_volume(self.volume);
            }

            // Music toggle button
            let music_btn_rect = UIRect::new(20.0, 140.0, 90.0, 30.0);
            let music_label = ctx.strings
                .tr(if self.music_playing { "panel.music_pause" } else { "panel.music_play" })
                .to_string();
            if ctx.ui.button("music_btn", &music_label, music_btn_rect) {
                self.toggle_music(ctx);
            }

            // Reset button
            let reset_btn_rect = UIRect::new(120.0, 140.0, 90.0, 30.0);
            let reset_label = ctx.strings.tr("panel.reset").to_string();
            if ctx.ui.button("reset_btn", &reset_label, reset_btn_rect) {
                self.reset_player(ctx);
            }

            // Progress bar showing current volume level
            let bar_label = ctx.strings.tr("panel.volume_bar").to_string();
            ctx.ui.label(&bar_label, Vec2::new(20.0, 185.0));
            let progress_rect = UIRect::new(20.0, 200.0, 190.0, 15.0);
            ctx.ui.progress_bar(self.volume, progress_rect);

            // Help text and status at bottom
            let help = ctx.strings.tr("panel.toggle_ui").to_string();
            ctx.ui.label(&help, Vec2::new(20.0, 225.0));

            // Show font status
            let font_status = ctx.strings
                .tr(if self.font_loaded { "panel.font_on" } else { "panel.font_off" })
                .to_string();
            ctx.ui.label(&font_status, Vec2::new(140.0, 225.0));
        }
    }

    // render() uses the default implementation which extracts sprites from ECS
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create game configuration. The asset base path makes texture paths in
    // the scene file resolve correctly regardless of the working directory.
    let game_config = GameConfig::new("Hello World - Insiculous 2D Physics Demo")
        .with_size(800, 600)
        .with_clear_color(0.1, 0.1, 0.15, 1.0)
        .with_asset_base_path(EXAMPLES_DIR)
        .with_locales_dir("assets/locales");

    // Create and run the game
    let game = HelloWorld::new();
    run_game(game, game_config)
}
