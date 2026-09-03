//! The wrapper's own frame contracts: engine time is frozen outside Play,
//! the OS title is published once per change, and `render` derives the GPU
//! camera and the scene-view scissor from the editor dock.

use ecs::World;
use editor::PlayControlAction;
use glam::Vec2;

use super::test_support::editor_game;

#[test]
fn test_time_scale_is_frozen_while_not_playing() {
    let mut editor = editor_game();
    let mut world = World::new();

    // Editing: engine-side time stops dead.
    assert_eq!(editor.editor_time_scale(1.0), 0.0);
    assert_eq!(editor.editor_time_scale(1.0), 0.0);

    // Play hands the game back the value it was running at.
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert_eq!(editor.editor_time_scale(0.0), 1.0);
    // From then on the game owns it — a game that pauses itself stays paused.
    assert_eq!(editor.editor_time_scale(0.0), 0.0);
    assert_eq!(editor.editor_time_scale(0.5), 0.5);

    // Paused counts as not playing: frozen again, and the game's 0.5 is held.
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    assert_eq!(editor.editor_time_scale(0.5), 0.0);
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert_eq!(editor.editor_time_scale(0.0), 0.5);
}

#[test]
fn test_particles_and_animations_do_not_advance_while_editing() {
    use ecs::sprite_components::{AnimationClip, Sprite, SpriteAnimation};
    use engine_core::particles::{ParticleConfig, ParticleEmitter, ParticleManager, ParticleSystem};

    let mut editor = editor_game();
    let mut world = World::new();
    let mut particles = ParticleManager::with_capacity(64);

    let emitter_entity = world.create_entity();
    world.add_component(&emitter_entity, common::Transform2D::new(Vec2::ZERO)).ok();
    world
        .add_component(&emitter_entity, ParticleEmitter::new(30.0, ParticleConfig::default()))
        .ok();

    let animated = world.create_entity();
    world.add_component(&animated, Sprite::new(0)).ok();
    let mut animation = SpriteAnimation::new(common::SheetGrid::new(4, 1))
        .with_clip("walk", AnimationClip::new(vec![0, 1, 2], 10.0));
    animation.play("walk");
    world.add_component(&animated, animation).ok();

    // One second of frames, exactly as the engine's frame tail runs them.
    let step_frames = |world: &mut World, particles: &mut ParticleManager, scale: f32| {
        for _ in 0..10 {
            let dt = 0.1 * scale;
            ParticleSystem::update(world, particles, dt);
            ecs::System::update(&mut ecs::SpriteAnimationSystem, world, dt);
        }
    };

    let editing_scale = editor.editor_time_scale(1.0);
    step_frames(&mut world, &mut particles, editing_scale);
    assert_eq!(particles.alive_count(), 0, "particles must not emit while editing");
    assert_eq!(
        world.get::<SpriteAnimation>(animated).map(|a| a.current_frame),
        Some(0),
        "animations must not advance while editing"
    );
    assert_eq!(
        world.get::<Sprite>(animated).map(|s| s.tex_region),
        Some([0.0, 0.0, 0.25, 1.0])
    );

    // Play: the same frames now move both.
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    let playing_scale = editor.editor_time_scale(1.0);
    step_frames(&mut world, &mut particles, playing_scale);
    assert!(particles.alive_count() > 0, "particles emit once Playing");
    assert_eq!(
        world.get::<SpriteAnimation>(animated).map(|a| a.current_frame),
        Some(1),
        "10 frame steps over a looping 3-frame clip land on frame 1"
    );
}

#[test]
fn test_pending_title_update_only_on_change() {
    // set_title is a window-system round-trip — the editor must publish the
    // title once per change, not once per frame.
    let mut editor = editor_game();

    let first = editor.pending_title_update();
    assert_eq!(first.as_deref(), Some("Untitled - Insiculous Editor"));
    assert_eq!(editor.pending_title_update(), None, "unchanged title is not re-published");

    editor.editor.set_dirty(true);
    let dirty = editor.pending_title_update();
    assert_eq!(dirty.as_deref(), Some("Untitled* - Insiculous Editor"));
    assert_eq!(editor.pending_title_update(), None);
}

/// Run `Game::render` against a headless render context and return the
/// camera and scissor it wrote.
fn render_with(editor_game: &mut super::EditorGame<super::test_support::DummyGame>, window_size: Vec2)
    -> (common::Camera, Option<common::Rect>) {
    let world = World::new();
    let mut sprites = renderer::sprite::SpriteBatcher::new();
    let mut camera = common::Camera::default();
    let glyph_textures = std::collections::HashMap::new();
    let mut viewport_scissor = None;
    let mut ctx = engine_core::contexts::RenderContext {
        world: &world,
        sprites: &mut sprites,
        camera: &mut camera,
        window_size,
        ui_commands: &[],
        glyph_textures: &glyph_textures,
        viewport_scissor: &mut viewport_scissor,
    };
    engine_core::Game::render(editor_game, &mut ctx);
    (camera, viewport_scissor)
}

#[test]
fn test_render_derives_the_gpu_camera_and_scissor_from_the_dock() {
    // The viewport is the single source of truth for the view: the GPU
    // camera must come from it every frame so sprites land where the
    // overlay (gizmo/picking/grid) expects them, and the game-world passes
    // are bounded to the scene panel.
    let mut editor_game = editor_game();
    let window_size = Vec2::new(1600.0, 900.0);
    editor_game.editor.update_layout(window_size);
    editor_game.editor.viewport.set_viewport_bounds(common::Rect::new(300.0, 100.0, 800.0, 600.0));
    editor_game.editor.viewport.set_camera_position(Vec2::new(120.0, -40.0));
    editor_game.editor.viewport.set_camera_zoom(2.0);

    let (camera, scissor) = render_with(&mut editor_game, window_size);

    let expected = editor_game.editor.viewport.to_window_render_camera(window_size);
    assert_eq!(camera, expected);
    assert_eq!(camera.zoom, 2.0);
    assert_eq!(camera.viewport_size, window_size);
    let scene_panel = editor_game.editor.scene_view_bounds().expect("scene view visible by default");
    assert!(scene_panel.width > 0.0 && scene_panel.height > 0.0);
    assert_eq!(scissor, Some(scene_panel), "the scissor is the DOCK's scene-view bounds");
}

#[test]
fn test_render_writes_zero_scissor_when_scene_panel_hidden() {
    // A hidden/collapsed scene panel means NO game world should draw —
    // a zero-size scissor, never None (which would mean full-window).
    let mut editor_game = editor_game();
    editor_game.editor.dock_area.set_panel_visible(editor::PanelId::SCENE_VIEW, false);

    let (_, scissor) = render_with(&mut editor_game, Vec2::new(1600.0, 900.0));

    let rect = scissor.expect("editor always writes a scissor");
    assert_eq!((rect.width, rect.height), (0.0, 0.0));
}
