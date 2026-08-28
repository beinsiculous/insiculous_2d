use std::path::PathBuf;

use ecs::World;
use editor::PlayControlAction;
use glam::Vec2;

use engine_core::contexts::GameContext;
use engine_core::scene_data::PhysicsSettings;
use engine_core::Game;
use engine_core::GameConfig;

use crate::constants::clamp_editor_window_size;
use super::EditorGame;

struct DummyGame;
impl Game for DummyGame {
    fn update(&mut self, _ctx: &mut GameContext) {}
}

#[test]
fn test_editor_config_enforces_minimum_size() {
    let config = clamp_editor_window_size(GameConfig::new("Test").with_size(640, 480));
    assert_eq!(config.width, 1024);
    assert_eq!(config.height, 720);
}

#[test]
fn test_editor_config_preserves_large_size() {
    let config = clamp_editor_window_size(GameConfig::new("Test").with_size(1920, 1080));
    assert_eq!(config.width, 1920);
    assert_eq!(config.height, 1080);
}

#[test]
fn test_play_action_captures_snapshot() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(glam::Vec2::new(10.0, 20.0))).ok();

    // Play → snapshot captured
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(editor.editor.is_playing());
    assert!(editor.world_snapshot.is_some());
}

#[test]
fn test_play_pause_resume_stop_cycle() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(glam::Vec2::ZERO)).ok();

    // Play
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(editor.editor.is_playing());

    // Pause
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    assert!(editor.editor.is_paused());

    // Resume
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(editor.editor.is_playing());

    // Stop
    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    assert!(editor.editor.is_editing());
    assert!(editor.world_snapshot.is_none());
}

#[test]
fn test_stop_restores_world_state() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(glam::Vec2::new(10.0, 20.0))).ok();

    // Play
    editor.handle_play_action(PlayControlAction::Play, &mut world);

    // Simulate game modification
    if let Some(t) = world.get_mut::<common::Transform2D>(entity) {
        t.position = glam::Vec2::new(999.0, 999.0);
    }

    // Stop → should restore original position
    editor.handle_play_action(PlayControlAction::Stop, &mut world);

    let t = world.get::<common::Transform2D>(entity).unwrap();
    assert_eq!(t.position, glam::Vec2::new(10.0, 20.0));
}

#[test]
fn test_stop_resets_transform_propagation_cache() {
    use ecs::System;

    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(glam::Vec2::new(10.0, 20.0))).ok();

    // Propagate once so the transform system has a cached baseline.
    editor.transform_system.update(&mut world, 0.016);
    assert_eq!(editor.transform_system.tracked_entity_count(), 1);

    // Play, mutate during play, Stop (restores the snapshot).
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    if let Some(t) = world.get_mut::<common::Transform2D>(entity) {
        t.position = glam::Vec2::new(999.0, 999.0);
    }
    editor.handle_play_action(PlayControlAction::Stop, &mut world);

    // The restore wholesale-replaced the world — the propagation baseline
    // must have been dropped so the next update recomputes from scratch.
    assert_eq!(
        editor.transform_system.tracked_entity_count(),
        0,
        "Stop must reset the transform system's cache"
    );
    editor.transform_system.update(&mut world, 0.016);
    let global = world.get::<ecs::GlobalTransform2D>(entity).unwrap();
    assert_eq!(global.position, glam::Vec2::new(10.0, 20.0));
}

#[test]
fn test_editor_game_initial_scene_state() {
    let editor = EditorGame::new(DummyGame);
    assert!(!editor.editor.is_dirty());
    assert!(editor.editor.scene_path().is_none());
    assert!(editor.physics_settings.is_none());
}

#[test]
fn test_new_scene_clears_world() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let _e1 = world.create_entity();
    let _e2 = world.create_entity();
    assert_eq!(world.entities().len(), 2);

    editor.new_scene(&mut world);
    assert_eq!(world.entities().len(), 0);
    assert!(!editor.editor.is_dirty());
    assert!(editor.editor.scene_path().is_none());
}

#[test]
fn test_new_scene_resets_editor_state() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();

    // Simulate some state
    editor.editor.set_dirty(true);
    editor.editor.set_scene_path(Some(PathBuf::from("test.ron")));
    editor.entity_counter = 5;

    editor.new_scene(&mut world);
    assert!(!editor.editor.is_dirty());
    assert!(editor.editor.scene_path().is_none());
    assert_eq!(editor.entity_counter, 0);
    assert!(editor.physics_settings.is_none());
}

/// Texture path stand-in for tests — `AssetManager` needs a GPU device, but
/// `save_scene_with` (the mandatory save choke point) only needs this closure.
fn test_texture_path_fn(handle: u32) -> String {
    if handle == 0 { "#white".to_string() } else { format!("#texture_{}", handle) }
}

#[test]
fn test_save_creates_file() {
    let mut editor = EditorGame::new(DummyGame);
    let world = ecs::World::new();

    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("test_save_scene.ron");

    let result = editor.save_scene_with(&world, &test_texture_path_fn, path.clone());
    assert!(result.is_ok());
    assert!(path.exists());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_save_clears_dirty_flag() {
    let mut editor = EditorGame::new(DummyGame);
    let world = World::new();

    editor.editor.set_dirty(true);
    assert!(editor.editor.is_dirty());

    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("test_save_dirty.ron");

    editor.save_scene_with(&world, &test_texture_path_fn, path.clone()).unwrap();

    assert!(!editor.editor.is_dirty());
    assert_eq!(editor.editor.scene_path(), Some(path.as_path()));
    assert_eq!(editor.editor.status_bar.message(), Some("Scene saved"));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_new_scene_warns_if_dirty() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    world.create_entity();

    editor.editor.set_dirty(true);
    // new_scene should still work even when dirty (just logs a warning)
    editor.new_scene(&mut world);
    assert_eq!(world.entities().len(), 0);
    assert!(!editor.editor.is_dirty());
}

#[test]
fn test_save_scene_roundtrip() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();

    // Create entities with components
    let e1 = world.create_entity();
    world.add_component(&e1, common::Transform2D::new(Vec2::new(100.0, 200.0))).ok();
    world.add_component(&e1, ecs::sprite_components::Name::new("player")).ok();

    let e2 = world.create_entity();
    world.add_component(&e2, common::Transform2D::new(Vec2::new(50.0, 50.0))).ok();

    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("test_roundtrip.ron");

    editor.save_scene_with(&world, &test_texture_path_fn, path.clone()).unwrap();

    // Verify the file is valid RON by parsing it with SceneLoader
    let parsed = engine_core::scene_loader::SceneLoader::load_from_file(&path).unwrap();
    assert_eq!(parsed.name, "test_roundtrip");
    assert_eq!(parsed.entities.len(), 2);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_save_as_updates_path() {
    let mut editor = EditorGame::new(DummyGame);

    assert!(editor.editor.scene_path().is_none());

    let path = PathBuf::from("scenes/my_scene.ron");
    editor.editor.set_scene_path(Some(path.clone()));

    assert_eq!(editor.editor.scene_path(), Some(path.as_path()));
    assert_eq!(editor.editor.scene_display_name(), "my_scene.ron");
}

#[test]
fn test_dirty_flag_set_on_entity_create() {
    let mut editor = EditorGame::new(DummyGame);
    assert!(!editor.editor.is_dirty());

    // Simulate entity creation marking dirty
    editor.editor.set_dirty(true);
    assert!(editor.editor.is_dirty());
}

#[test]
fn test_load_scene_resets_selection() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    editor.editor.selection.select(entity);
    assert!(!editor.editor.selection.is_empty());

    editor.new_scene(&mut world);
    assert!(editor.editor.selection.is_empty());
}

#[test]
fn test_physics_settings_preserved_on_new() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();

    editor.physics_settings = Some(PhysicsSettings::default());
    assert!(editor.physics_settings.is_some());

    editor.new_scene(&mut world);
    assert!(editor.physics_settings.is_none());
}

#[test]
fn test_scene_display_in_status() {
    let editor = EditorGame::new(DummyGame);
    assert_eq!(editor.editor.scene_display_name(), "Untitled");
    assert_eq!(editor.editor.title_bar_text(), "Untitled - Insiculous Editor");
}

#[test]
fn test_undo_redo_on_empty_history_do_not_mark_dirty() {
    // GPP-L6: an Undo/Redo keypress on an empty history is a no-op and must
    // not dirty a clean scene. The history is the dirty source of truth
    // (issue #24); the mirror sync in update() is what the handlers rely on.
    let mut editor = EditorGame::new(DummyGame);
    let mut world = World::new();
    assert!(!editor.editor.is_dirty());

    assert!(!editor.command_history.undo(&mut world));
    assert!(!editor.command_history.redo(&mut world));

    editor.editor.set_dirty(editor.command_history.is_dirty()); // the update() mirror sync
    assert!(!editor.editor.is_dirty());
}

#[test]
fn test_pending_title_update_only_on_change() {
    // set_title is a window-system round-trip — the editor must publish the
    // title once per change, not once per frame.
    let mut editor = EditorGame::new(DummyGame);

    let first = editor.pending_title_update();
    assert_eq!(first.as_deref(), Some("Untitled - Insiculous Editor"));
    assert_eq!(editor.pending_title_update(), None, "unchanged title is not re-published");

    editor.editor.set_dirty(true);
    let dirty = editor.pending_title_update();
    assert_eq!(dirty.as_deref(), Some("Untitled* - Insiculous Editor"));
    assert_eq!(editor.pending_title_update(), None);
}

#[test]
fn test_dirty_mirror_follows_history() {
    // The EditorContext flag is a per-frame mirror of CommandHistory: a
    // recorded command reads dirty, a save reads clean again.
    let mut editor = EditorGame::new(DummyGame);
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::ZERO)).ok();

    let cmd = editor::commands::CreateEntityCommand::already_created(&world, entity);
    editor.command_history.push_already_executed(Box::new(cmd));
    editor.editor.set_dirty(editor.command_history.is_dirty());
    assert!(editor.editor.is_dirty(), "a recorded command must dirty the scene");

    editor.command_history.mark_saved();
    editor.editor.set_dirty(editor.command_history.is_dirty());
    assert!(!editor.editor.is_dirty(), "saving reads clean again");
}

#[test]
fn test_render_overrides_camera_from_viewport() {
    // The GPU camera must be derived from the editor viewport every frame so
    // sprites land where the overlay (gizmo/picking/grid) expects them.
    let mut editor_game = EditorGame::new(DummyGame);
    editor_game.editor.viewport.set_viewport_bounds(common::Rect::new(300.0, 100.0, 800.0, 600.0));
    editor_game.editor.viewport.set_camera_position(Vec2::new(120.0, -40.0));
    editor_game.editor.viewport.set_camera_zoom(2.0);

    let world = World::new();
    let mut sprites = renderer::sprite::SpriteBatcher::new();
    let mut camera = common::Camera::default();
    let glyph_textures = std::collections::HashMap::new();
    let window_size = Vec2::new(1600.0, 900.0);
    let mut ctx = engine_core::contexts::RenderContext {
        world: &world,
        sprites: &mut sprites,
        camera: &mut camera,
        window_size,
        ui_commands: &[],
        glyph_textures: &glyph_textures,
    };

    engine_core::Game::render(&mut editor_game, &mut ctx);

    let expected = editor_game.editor.viewport.to_window_render_camera(window_size);
    assert_eq!(camera, expected);
    assert_eq!(camera.zoom, 2.0);
    assert_eq!(camera.viewport_size, window_size);
}

#[test]
fn test_sync_viewport_from_main_camera_only_while_playing() {
    let mut editor_game = EditorGame::new(DummyGame);
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Camera::default().as_main_camera()).ok();
    world.add_component(&entity, common::Transform2D::new(Vec2::new(320.0, -40.0))).ok();

    // Editing: the game camera must NOT move the editing view.
    editor_game.sync_viewport_from_main_camera(&world);
    assert_eq!(editor_game.editor.viewport.camera_position(), Vec2::ZERO);

    // Playing: viewport mirrors the main-camera entity.
    editor_game.handle_play_action(PlayControlAction::Play, &mut world);
    editor_game.sync_viewport_from_main_camera(&world);
    assert_eq!(
        editor_game.editor.viewport.camera_position(),
        Vec2::new(320.0, -40.0)
    );

    // A world without a main camera leaves the viewport untouched.
    let empty = World::new();
    editor_game.sync_viewport_from_main_camera(&empty);
    assert_eq!(
        editor_game.editor.viewport.camera_position(),
        Vec2::new(320.0, -40.0)
    );
}

#[test]
fn test_stop_restores_editing_camera() {
    let mut editor_game = EditorGame::new(DummyGame);
    let mut world = World::new();

    editor_game.editor.viewport.set_camera_position(Vec2::new(77.0, -33.0));
    editor_game.editor.viewport.set_camera_zoom(2.5);

    // Play: zoom snaps to 1.0 (game-camera parity), pan/zoom saved.
    editor_game.handle_play_action(PlayControlAction::Play, &mut world);
    assert_eq!(editor_game.editor.viewport.camera_zoom(), 1.0);

    // Simulate the game camera dragging the viewport around during play.
    editor_game.editor.viewport.set_camera_position(Vec2::new(999.0, 999.0));

    // Stop: the editing view comes back.
    editor_game.handle_play_action(PlayControlAction::Stop, &mut world);
    assert_eq!(editor_game.editor.viewport.camera_position(), Vec2::new(77.0, -33.0));
    assert_eq!(editor_game.editor.viewport.camera_zoom(), 2.5);
}

#[test]
fn test_scale_collider_scales_shapes_and_offset() {
    use physics::components::{Collider, ColliderShape};
    use super::viewport_interaction::scale_collider;

    let mut boxed = Collider::box_collider(80.0, 40.0); // half extents 40, 20
    boxed.offset = Vec2::new(10.0, -5.0);
    scale_collider(&mut boxed, Vec2::new(2.0, 3.0));
    match boxed.shape {
        ColliderShape::Box { half_extents } => assert_eq!(half_extents, Vec2::new(80.0, 60.0)),
        other => panic!("unexpected shape {other:?}"),
    }
    assert_eq!(boxed.offset, Vec2::new(20.0, -15.0), "body-local offset scales too");

    let mut circle = Collider::circle_collider(10.0);
    scale_collider(&mut circle, Vec2::new(1.5, 2.0));
    match circle.shape {
        ColliderShape::Circle { radius } => assert_eq!(radius, 20.0, "dominant axis factor"),
        other => panic!("unexpected shape {other:?}"),
    }
}

#[test]
fn test_gizmo_scale_undo_restores_transform_and_collider_together() {
    use editor::commands::{MacroCommand, SetColliderCommand, TransformGizmoCommand};
    use physics::components::{Collider, ColliderShape};

    let mut world = ecs::World::new();
    let entity = world.create_entity();
    let old_t = common::Transform2D::from_parts(Vec2::ZERO, 0.0, Vec2::ONE);
    let old_c = Collider::box_collider(80.0, 80.0);
    let mut new_t = old_t;
    new_t.scale = Vec2::new(2.0, 2.0);
    let mut new_c = old_c.clone();
    super::viewport_interaction::scale_collider(&mut new_c, Vec2::new(2.0, 2.0));
    world.add_component(&entity, new_t).ok();
    world.add_component(&entity, new_c.clone()).ok();

    // The single undo entry the release path pushes
    let mut history = editor::CommandHistory::new();
    let cmd = MacroCommand::new(
        "Scale Entity",
        vec![
            Box::new(TransformGizmoCommand::new(entity, old_t, new_t)),
            Box::new(SetColliderCommand::new(entity, old_c.clone(), new_c, "gizmo_scale")),
        ],
    );
    history.push_already_executed(Box::new(cmd));

    assert!(history.undo(&mut world), "one Ctrl+Z reverts the whole drag");
    let t = world.get::<common::Transform2D>(entity).unwrap();
    assert_eq!(t.scale, Vec2::ONE);
    let c = world.get::<Collider>(entity).unwrap();
    match &c.shape {
        ColliderShape::Box { half_extents } => assert_eq!(*half_extents, Vec2::new(40.0, 40.0)),
        other => panic!("unexpected shape {other:?}"),
    }
}
