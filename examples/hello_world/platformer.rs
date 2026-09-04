use ecs::GlobalTransform2D;
use engine_core::prelude::*;
use input::{InputMapping, InputSource};
use std::path::Path;

/// Anchor all asset paths to the repository so the example runs from any
/// working directory.
pub const EXAMPLES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");

/// Where the player respawns on reset (matches the scene file's spawn point).
const PLAYER_SPAWN: Vec2 = Vec2::new(-200.0, 100.0);

/// Demo-level debug actions (music/UI/volume toggles, reset, locale cycling).
/// Gameplay input goes through the engine's player-aware layer (`ctx.players`);
/// movement and jumping themselves are driven by the scene's `PlayerPlatformer`
/// behavior.
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

/// Cross-system game state accessible by any system via `world.resource::<GameState>()`.
#[derive(Debug, Clone, Default)]
struct GameState {
    score: u32,
    coins_collected: u32,
}

/// Player behavioral states, driven by physics velocity and input.
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

/// Shared platformer game demonstration, used by both the standalone
/// `hello_world` executable and the `editor_demo`.
pub struct PlatformerGame {
    physics: Option<PhysicsSystem>,
    behaviors: BehaviorRunner,
    scene_instance: Option<SceneInstance>,
    transform_hierarchy: TransformHierarchySystem,
    actions: InputMapping<DemoAction>,
    jump_sound: Option<SoundHandle>,
    music_playing: bool,
    volume: f32,
    show_ui: bool,
    font_loaded: bool,
}

impl Default for PlatformerGame {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformerGame {
    pub fn new() -> Self {
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
    fn reset_player(&mut self, ctx: &mut GameContext) {
        let Some(player) = self.player_entity() else { return };

        if let Some(physics) = &mut self.physics {
            physics.reset_body(player, PLAYER_SPAWN);
        } else if let Some(transform) = ctx.world.get_mut::<Transform2D>(player) {
            transform.position = PLAYER_SPAWN;
        }
    }

    fn toggle_music(&mut self, ctx: &mut GameContext) {
        if self.music_playing {
            ctx.audio.pause_music();
            self.music_playing = false;
            log::info!("Music paused");
        } else {
            ctx.audio.resume_music();
            self.music_playing = true;
            log::info!("Music resumed");
        }
    }

    /// Add Name + GlobalTransform2D to entities that lack them, so the editor
    /// hierarchy and inspector work reliably.
    fn add_editor_names(&self, ctx: &mut GameContext) {
        if let Some(instance) = &self.scene_instance {
            for (name, &entity_id) in &instance.named_entities {
                if ctx.world.get::<Name>(entity_id).is_none() {
                    ctx.world.add_component(&entity_id, Name::new(name)).ok();
                }
                if ctx.world.get::<GlobalTransform2D>(entity_id).is_none() {
                    ctx.world.add_component(&entity_id, GlobalTransform2D::default()).ok();
                }
            }
        }

        for entity_id in ctx.world.entities() {
            if ctx.world.get::<Transform2D>(entity_id).is_some()
                && ctx.world.get::<GlobalTransform2D>(entity_id).is_none()
            {
                ctx.world.add_component(&entity_id, GlobalTransform2D::default()).ok();
            }
        }
    }

    fn load_level(&mut self, ctx: &mut GameContext) {
        let scene_path = Path::new(EXAMPLES_DIR).join("assets/scenes/hello_world.scene.ron");

        match SceneLoader::load_and_instantiate(&scene_path, ctx.world, ctx.assets) {
            Ok(instance) => {
                log::info!("Loaded scene '{}' with {} entities", instance.name, instance.entity_count);
                self.behaviors.set_named_entities(instance.named_entities.clone());

                let physics_config = if let Some(settings) = &instance.physics {
                    PhysicsConfig::new(Vec2::new(settings.gravity.0, settings.gravity.1))
                        .with_scale(settings.pixels_per_meter)
                } else {
                    PhysicsConfig::platformer()
                };

                self.physics = Some(PhysicsSystem::with_config(physics_config));
                self.scene_instance = Some(instance);
            }
            Err(error) => {
                log::warn!("Failed to load scene: {}", error);
                self.spawn_fallback_level(ctx);
            }
        }

        if let Some(physics) = &mut self.physics {
            physics.initialize(ctx.world).ok();
        }
        self.transform_hierarchy.initialize(ctx.world).ok();
        self.add_editor_names(ctx);
        ctx.world.insert_resource(GameState::default());
    }

    fn spawn_fallback_level(&mut self, ctx: &mut GameContext) {
        log::info!("Creating entities programmatically as fallback...");

        let player = ctx.world.create_entity();
        ctx.world.add_component(&player, Transform2D::new(PLAYER_SPAWN)).ok();
        ctx.world.add_component(&player, Sprite::new(0).with_color(Vec4::new(0.2, 0.4, 1.0, 1.0))).ok();
        ctx.world.add_component(&player, RigidBody::player_platformer()).ok();
        ctx.world.add_component(&player, Collider::player_box(80.0, 80.0)).ok();
        ctx.world.add_component(&player, Behavior::PlayerPlatformer {
            move_speed: 120.0,
            jump_impulse: 420.0,
            jump_cooldown: 0.3,
            tag: "player".to_string(),
        }).ok();

        let ground = ctx.world.create_entity();
        ctx.world.add_component(
            &ground,
            Transform2D::new(Vec2::new(0.0, -250.0)).with_scale(Vec2::new(10.0, 0.5)),
        ).ok();
        ctx.world.add_component(&ground, Sprite::new(0).with_color(Vec4::new(0.3, 0.3, 0.3, 1.0))).ok();
        ctx.world.add_component(&ground, RigidBody::new_static()).ok();
        ctx.world.add_component(&ground, Collider::platform(800.0, 40.0)).ok();

        self.physics = Some(PhysicsSystem::with_config(PhysicsConfig::platformer()));
    }

    fn attach_player_state(&mut self, ctx: &mut GameContext) {
        if let Some(player) = self.player_entity() {
            ctx.world.add_component(
                &player,
                HierarchicalStateMachine::new(PlayerState::Idle, player_group),
            ).ok();
        }
    }

    fn load_audio(&mut self, ctx: &mut GameContext) {
        match ctx.audio.load_sound(Path::new(EXAMPLES_DIR).join("assets/sounds/snd_jump.wav")) {
            Ok(handle) => {
                self.jump_sound = Some(handle);
                log::info!("Loaded jump sound effect");
            }
            Err(error) => {
                log::info!("No jump sound loaded ({error}). Audio demo will show API usage.");
                log::info!("To enable audio, add a WAV file at examples/assets/sounds/snd_jump.wav");
            }
        }

        match ctx.audio.play_music(Path::new(EXAMPLES_DIR).join("assets/sounds/music.ogg")) {
            Ok(()) => {
                self.music_playing = true;
                log::info!("Playing background music");
            }
            Err(_error) => {
                log::info!("No background music found. Add music.ogg to examples/assets/sounds/");
            }
        }
    }

    fn load_font(&mut self, ctx: &mut GameContext) {
        let bundled_font = format!("{EXAMPLES_DIR}/assets/fonts/font.ttf");
        if ctx.ui.load_font_file(&bundled_font).is_ok() {
            self.font_loaded = true;
            log::info!("Font loaded - text will render with actual glyphs!");
            return;
        }

        let font_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
            "C:\\Windows\\Fonts\\arial.ttf",
        ];

        for path in font_paths {
            if ctx.ui.load_font_file(path).is_ok() {
                self.font_loaded = true;
                log::info!("System font loaded from: {}", path);
                return;
            }
        }

        log::info!("No font loaded. Text will render as placeholders.");
        log::info!("To enable font rendering, add a .ttf file to examples/assets/fonts/font.ttf");
    }

    fn log_ready(&self, ctx: &GameContext) {
        let total_count = ctx.world.entity_count();
        let root_count = ctx.world.get_root_entities().len();
        let child_count = total_count - root_count;

        log::info!(
            "Game initialized with {} entities ({} root, {} children)",
            total_count, root_count, child_count
        );
        log::info!("Controls: WASD to move, SPACE to jump, R to reset, M to toggle music, H to toggle UI, ESC to exit");
        log::info!("Physics enabled - push the wood boxes around!");
        if child_count > 0 {
            log::info!("Scene Graph: {} child entities will follow their parents!", child_count);
        }
        log::info!("Audio system ready - master volume: {:.0}%", ctx.audio.master_volume() * 100.0);
        log::info!("UI system ready - click buttons and drag sliders!");
        if self.font_loaded {
            log::info!("Font system ready - text renders with actual glyphs!");
        }
    }

    fn handle_debug_actions(&mut self, ctx: &mut GameContext) {
        if ctx.players.just_activated(PlayerId::P1, GameAction::Action1, ctx.input)
            || ctx.players.just_activated(PlayerId::P2, GameAction::Action1, ctx.input)
        {
            if let Some(jump_sound) = &self.jump_sound {
                let settings = SoundSettings::new().with_volume(0.8).with_speed(1.0);
                if let Err(error) = ctx.audio.play_with_settings(jump_sound, settings) {
                    log::warn!("Failed to play jump sound: {}", error);
                }
            }
        }

        if self.actions.just_activated(DemoAction::ToggleMusic, ctx.input) {
            self.toggle_music(ctx);
        }

        if self.actions.just_activated(DemoAction::CycleLocale, ctx.input) {
            ctx.strings.cycle_locale();
            log::info!("Locale: {}", ctx.strings.current_display_name());
        }

        if self.actions.just_activated(DemoAction::VolumeUp, ctx.input) {
            let new_volume = (ctx.audio.master_volume() + 0.1).min(1.0);
            ctx.audio.set_master_volume(new_volume);
            log::info!("Volume: {:.0}%", new_volume * 100.0);
        }
        if self.actions.just_activated(DemoAction::VolumeDown, ctx.input) {
            let new_volume = (ctx.audio.master_volume() - 0.1).max(0.0);
            ctx.audio.set_master_volume(new_volume);
            log::info!("Volume: {:.0}%", new_volume * 100.0);
        }

        if self.actions.just_activated(DemoAction::ResetPlayer, ctx.input) {
            self.reset_player(ctx);
        }

        if self.actions.just_activated(DemoAction::ToggleUi, ctx.input) {
            self.show_ui = !self.show_ui;
        }
    }

    fn step_world(&mut self, ctx: &mut GameContext) {
        self.behaviors.update(
            ctx.world,
            ctx.input,
            ctx.delta_time,
            self.physics.as_mut(),
        );

        if let Some(physics) = &mut self.physics {
            physics.update(ctx.world, ctx.delta_time);
        }

        self.transform_hierarchy.update(ctx.world, ctx.delta_time);
    }

    fn collect_pickups(&mut self, ctx: &mut GameContext) {
        let collected: Vec<EntityCollected> = ctx.world.read_events::<EntityCollected>().to_vec();
        for event in &collected {
            if let Some(state) = ctx.world.resource_mut::<GameState>() {
                state.score += event.score_value;
                state.coins_collected += 1;
            }
            log::info!(
                "Collected! +{} points (total: {})",
                event.score_value,
                ctx.world.resource::<GameState>().map(|state| state.score).unwrap_or(0)
            );
        }
    }

    fn update_player_state(&mut self, ctx: &mut GameContext) {
        let Some(player) = self.player_entity() else { return };

        let velocity = ctx.world.get::<RigidBody>(player)
            .map(|body| body.velocity)
            .unwrap_or(Vec2::ZERO);
        let moving_x = velocity.x.abs() > 5.0;

        let new_state = if velocity.y > 10.0 {
            PlayerState::Jumping
        } else if velocity.y < -10.0 {
            PlayerState::Falling
        } else if moving_x {
            PlayerState::Running
        } else {
            PlayerState::Idle
        };

        if let Some(state_machine) = ctx.world.get_mut::<HierarchicalStateMachine<PlayerState, PlayerGroup>>(player) {
            state_machine.transition_to(new_state);
            state_machine.tick(ctx.delta_time);
        }
    }

    fn draw_panel(&mut self, ctx: &mut GameContext) {
        if !self.show_ui {
            return;
        }

        let panel_rect = UIRect::new(10.0, 10.0, 220.0, 250.0);
        ctx.ui.panel(panel_rect);

        let title = ctx.strings.tr("panel.title").to_string();
        ctx.ui.label(&title, Vec2::new(20.0, 25.0));

        let score = ctx.world.resource::<GameState>().map(|state| state.score).unwrap_or(0);
        let coins = ctx.world.resource::<GameState>().map(|state| state.coins_collected).unwrap_or(0);
        let score_text = format!(
            "{}: {} ({} {})",
            ctx.strings.tr("panel.score"), score, coins, ctx.strings.tr("panel.coins"),
        );
        ctx.ui.label(&score_text, Vec2::new(20.0, 50.0));

        let state_label = ctx.strings.tr("panel.state").to_string();
        let state_text = if let Some(player) = self.player_entity() {
            if let Some(state_machine) = ctx.world.get::<HierarchicalStateMachine<PlayerState, PlayerGroup>>(player) {
                format!("{}: {:?} ({:?})", state_label, state_machine.current(), state_machine.parent())
            } else {
                format!("{}: N/A", state_label)
            }
        } else {
            format!("{}: No player", state_label)
        };
        ctx.ui.label(&state_text, Vec2::new(20.0, 70.0));

        let volume_label = ctx.strings.tr("panel.volume").to_string();
        ctx.ui.label(&volume_label, Vec2::new(20.0, 95.0));
        let slider_rect = UIRect::new(20.0, 110.0, 190.0, 20.0);
        let new_volume = ctx.ui.slider("volume_slider", self.volume, slider_rect);
        if new_volume != self.volume {
            self.volume = new_volume;
            ctx.audio.set_master_volume(self.volume);
        }

        let music_btn_rect = UIRect::new(20.0, 140.0, 90.0, 30.0);
        let music_label = ctx.strings
            .tr(if self.music_playing { "panel.music_pause" } else { "panel.music_play" })
            .to_string();
        if ctx.ui.button("music_btn", &music_label, music_btn_rect) {
            self.toggle_music(ctx);
        }

        let reset_btn_rect = UIRect::new(120.0, 140.0, 90.0, 30.0);
        let reset_label = ctx.strings.tr("panel.reset").to_string();
        if ctx.ui.button("reset_btn", &reset_label, reset_btn_rect) {
            self.reset_player(ctx);
        }

        let bar_label = ctx.strings.tr("panel.volume_bar").to_string();
        ctx.ui.label(&bar_label, Vec2::new(20.0, 185.0));
        let progress_rect = UIRect::new(20.0, 200.0, 190.0, 15.0);
        ctx.ui.progress_bar(self.volume, progress_rect);

        let help = ctx.strings.tr("panel.toggle_ui").to_string();
        ctx.ui.label(&help, Vec2::new(20.0, 225.0));

        let font_status = ctx.strings
            .tr(if self.font_loaded { "panel.font_on" } else { "panel.font_off" })
            .to_string();
        ctx.ui.label(&font_status, Vec2::new(140.0, 225.0));
    }
}

impl Game for PlatformerGame {
    fn init(&mut self, ctx: &mut GameContext) {
        self.load_level(ctx);
        self.attach_player_state(ctx);
        self.load_audio(ctx);
        self.load_font(ctx);
        self.log_ready(ctx);
    }

    fn on_play_stopped(&mut self, _ctx: &mut GameContext) {
        if let Some(physics) = &mut self.physics {
            physics.clear();
        }
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.handle_debug_actions(ctx);
        self.step_world(ctx);
        self.collect_pickups(ctx);
        self.update_player_state(ctx);
        self.draw_panel(ctx);
    }
}
