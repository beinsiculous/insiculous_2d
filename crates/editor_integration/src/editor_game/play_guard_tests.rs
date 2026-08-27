//! Tests for the play-session data-loss guards (issue #22, audit §1.2/§1.3):
//! save/new/open refused mid-simulation, snapshot loss warnings on Play and
//! Stop, and resume-from-pause never re-capturing the snapshot.

use editor::PlayControlAction;
use glam::Vec2;

use engine_core::contexts::GameContext;
use engine_core::Game;

use super::EditorGame;

struct DummyGame;
impl Game for DummyGame {
    fn update(&mut self, _ctx: &mut GameContext) {}
}

/// Texture path stand-in — `AssetManager` needs a GPU device, but
/// `save_scene_with` (the mandatory save choke point) only needs this closure.
fn test_texture_path_fn(handle: u32) -> String {
    if handle == 0 { "#white".to_string() } else { format!("#texture_{}", handle) }
}

// ==================== Save/replace guards during play sessions ====================

#[test]
fn test_save_refused_while_playing() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    world.create_entity();
    editor.editor.mark_dirty();

    editor.handle_play_action(PlayControlAction::Play, &mut world);

    let path = std::env::temp_dir().join("test_save_refused_playing.ron");
    let result = editor.save_scene_with(&world, &test_texture_path_fn, path.clone());

    let err = result.expect_err("saving mid-simulation must be refused");
    assert!(err.contains("stop Play"), "error must tell the user how to proceed: {err}");
    assert!(!path.exists(), "a refused save must not touch the scene file");
    assert!(editor.editor.is_dirty(), "a refused save must not pretend the scene is clean");
}

#[test]
fn test_save_refused_while_paused() {
    // The original bug: the Ctrl+S shortcut path is unreachable while
    // Playing but runs while Paused — the paused world is equally
    // mid-simulation and must not overwrite the authored scene.
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    world.create_entity();
    editor.editor.mark_dirty();

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    assert!(editor.editor.is_paused());

    let path = std::env::temp_dir().join("test_save_refused_paused.ron");
    let result = editor.save_scene_with(&world, &test_texture_path_fn, path.clone());

    assert!(result.is_err(), "saving while Paused must be refused");
    assert!(!path.exists());
    assert!(editor.editor.is_dirty());
}

#[test]
fn test_save_succeeds_after_stop() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    world.create_entity();
    editor.editor.mark_dirty();

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    editor.handle_play_action(PlayControlAction::Stop, &mut world);

    let path = std::env::temp_dir().join("test_save_after_stop.ron");
    let result = editor.save_scene_with(&world, &test_texture_path_fn, path.clone());

    assert!(result.is_ok(), "saving after Stop must work again: {result:?}");
    assert!(path.exists());
    assert!(!editor.editor.is_dirty());
    assert_eq!(editor.editor.status_bar.message(), Some("Scene saved"));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_scene_replace_refused_during_play_session() {
    // Shared guard for New Scene / Open Scene (menu and Ctrl+N / Ctrl+O):
    // replacing the world under a pending play snapshot would make the next
    // Stop resurrect the old scene's entities into the new one.
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();

    assert!(editor.scene_replace_refusal().is_none(), "editing mode must allow new/open");

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(editor.scene_replace_refusal().is_some(), "Playing must refuse new/open");

    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    assert!(editor.scene_replace_refusal().is_some(), "Paused must refuse new/open");

    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    assert!(editor.scene_replace_refusal().is_none(), "Stop must re-allow new/open");
}

#[test]
fn test_new_scene_refused_while_paused() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::new(1.0, 2.0))).ok();

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    editor.handle_play_action(PlayControlAction::Pause, &mut world);

    editor.new_scene(&mut world);

    assert_eq!(world.entity_count(), 1, "the world must not be cleared mid-simulation");
    assert!(editor.world_snapshot.is_some(), "the pending play snapshot must survive");
    assert!(
        editor.editor.status_bar.message().is_some_and(|m| m.contains("stop Play")),
        "the refusal must be surfaced on the status bar"
    );

    // Stop still restores the authored scene cleanly.
    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    let t = world.get::<common::Transform2D>(entity).unwrap();
    assert_eq!(t.position, Vec2::new(1.0, 2.0));
}

// ==================== Unregistered-component loss warnings ====================

#[test]
fn test_play_surfaces_warning_for_unregistered_components() {
    struct CustomBrain;

    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    world.add_component(&entity, CustomBrain).ok();

    editor.handle_play_action(PlayControlAction::Play, &mut world);

    assert!(
        editor.editor.status_bar.message().is_some_and(|m| m.contains("lost on Stop")),
        "entering Play with an unregistered component must warn about the coming loss"
    );
}

#[test]
fn test_play_shows_no_warning_for_registry_only_worlds() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::default()).ok();

    editor.handle_play_action(PlayControlAction::Play, &mut world);

    assert!(
        editor.editor.status_bar.message().is_none(),
        "a fully capturable world must not nag on Play"
    );
}

#[test]
fn test_stop_reports_dropped_component_types() {
    struct CustomBrain;

    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    world.add_component(&entity, CustomBrain).ok();

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    editor.editor.status_bar.clear_message(); // the Play warning may be missed
    editor.handle_play_action(PlayControlAction::Stop, &mut world);

    // The loss happens at Stop-restore, so it must be reported there too.
    assert!(
        editor.editor.status_bar.message().is_some_and(|m| m.contains("dropped")),
        "Stop must report what the restore dropped"
    );
    assert!(world.get::<CustomBrain>(entity).is_none(), "the drop itself is the documented loss");
}

#[test]
fn test_resume_from_pause_does_not_recapture_snapshot() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::new(10.0, 20.0))).ok();

    editor.handle_play_action(PlayControlAction::Play, &mut world);

    // Mutate mid-simulation, pause, resume, mutate again, stop.
    if let Some(t) = world.get_mut::<common::Transform2D>(entity) {
        t.position = Vec2::new(500.0, 500.0);
    }
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    editor.handle_play_action(PlayControlAction::Play, &mut world); // resume
    if let Some(t) = world.get_mut::<common::Transform2D>(entity) {
        t.position = Vec2::new(999.0, 999.0);
    }
    editor.handle_play_action(PlayControlAction::Stop, &mut world);

    // A resume that re-captured would restore the paused mid-simulation
    // state; Stop must return to the ORIGINAL authored state.
    let t = world.get::<common::Transform2D>(entity).unwrap();
    assert_eq!(t.position, Vec2::new(10.0, 20.0));
}
