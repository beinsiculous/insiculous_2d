//! Scene I/O through the editor: the mandatory save choke point (file
//! written, parses back, dirty cleared, script targets auto-named through
//! the history), the load dry-run that never costs the live world,
//! the physics block reaching the world as a resource, and New Scene
//! resetting every piece of session state.

use std::path::PathBuf;

use ecs::World;
use engine_core::scene_data::PhysicsSettings;
use engine_core::test_support::{test_texture_path, StubResolver};
use glam::Vec2;

use super::test_support::{editor_game, spawn_at, DummyGame};
use super::EditorGame;

const VALID_SCENE: &str = r#"SceneData(
    name: "Loaded",
    physics: Some(PhysicsSettings(gravity: (0.0, -420.0), pixels_per_meter: 64.0)),
    entities: [
        EntityData(name: Some("a"), components: [Transform2D(position: (1.0, 2.0))]),
        EntityData(name: Some("b"), components: [Transform2D(position: (3.0, 4.0))]),
    ],
)"#;

/// Parses fine, but instantiation fails: the entity references a prefab
/// that the scene never defines.
const UNKNOWN_PREFAB_SCENE: &str = r#"SceneData(
    name: "Broken",
    entities: [
        EntityData(name: Some("a"), components: [Transform2D(position: (0.0, 0.0))]),
        EntityData(name: Some("b"), prefab: Some("DoesNotExist")),
    ],
)"#;

/// An editor session with one unsaved entity, a selection, and a recorded
/// scene path — everything a failed load must leave untouched.
fn session_with_unsaved_work(world: &mut World) -> EditorGame<DummyGame> {
    let entity = world.create_entity();
    let mut editor = editor_game();
    editor.editor.selection.select(entity);
    editor.editor.set_scene_path(Some(PathBuf::from("previous.scene.ron")));
    editor.editor.set_dirty(true);
    editor
}

/// The world, selection, dirty flag and path survived a failed load.
fn assert_session_untouched(editor: &EditorGame<DummyGame>, world: &World, case: &str) {
    assert_eq!(world.entities().len(), 1, "{case}: the current world must survive");
    assert!(!editor.editor.selection.is_empty(), "{case}: selection must survive");
    assert!(editor.editor.is_dirty(), "{case}: dirty state must survive");
    assert_eq!(
        editor.editor.scene_path(),
        Some(std::path::Path::new("previous.scene.ron")),
        "{case}: a failed load must not adopt the new path"
    );
}

#[test]
fn test_save_writes_a_file_that_parses_back_and_clears_dirty() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let mut editor = editor_game();
    let mut world = World::new();
    let player = spawn_at(&mut world, Vec2::new(100.0, 200.0));
    world.add_component(&player, ecs::Name::new("player")).ok();
    spawn_at(&mut world, Vec2::new(50.0, 50.0));
    editor.command_history.push_already_executed(Box::new(
        editor::commands::CreateEntityCommand::already_created(&world, player),
    ));
    assert!(editor.command_history.is_dirty());
    let path = dir.path().join("roundtrip.ron");

    editor
        .save_scene_with(&mut world, &test_texture_path, path.clone())
        .expect("save succeeds");

    let parsed = engine_core::scene_loader::SceneLoader::load_from_file(&path).expect("valid RON");
    assert_eq!(parsed.name, "roundtrip", "the scene is named after the file stem");
    assert_eq!(parsed.entities.len(), 2);
    assert!(!editor.command_history.is_dirty(), "save marks the watermark");
    assert!(!editor.editor.is_dirty(), "the same frame's mirror already reads clean");
    assert_eq!(editor.editor.scene_path(), Some(path.as_path()));
    assert_eq!(editor.editor.status_bar.message(), Some("Scene saved"));
    Ok(())
}

#[test]
fn test_save_auto_names_script_targets_through_command_history() -> std::io::Result<()> {
    // The save-time auto-naming must be an UNDOABLE
    // CommandHistory entry, never a silent world mutation.
    use ecs::script::{ScriptRef, ScriptValue, Scripts};
    let dir = tempfile::tempdir()?;
    let mut editor = editor_game();
    let mut world = World::new();
    let target = world.create_entity(); // unnamed
    let owner = world.create_entity();
    world.add_component(&owner, ecs::Name::new("runner")).ok();
    let mut script = ScriptRef::new("chase");
    script.params.insert("target".to_string(), ScriptValue::Entity(target));
    world.add_component(&owner, Scripts(vec![script])).ok();
    let path = dir.path().join("autoname.ron");

    editor
        .save_scene_with(&mut world, &test_texture_path, path.clone())
        .expect("save succeeds");

    let assigned = world
        .get::<ecs::Name>(target)
        .map(|n| n.0.clone())
        .expect("referenced target auto-named on save");
    assert!(assigned.starts_with("script_target_"));
    let saved = std::fs::read_to_string(&path)?;
    assert!(saved.contains(&assigned), "the binding reached the file by name");

    assert!(editor.command_history.undo(&mut world), "the naming is one undoable entry");
    assert!(world.get::<ecs::Name>(target).is_none(), "undo removes the auto-assigned name");
    Ok(())
}

#[test]
fn test_failed_parse_or_missing_file_preserves_the_live_world() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let malformed = dir.path().join("malformed.scene.ron");
    std::fs::write(&malformed, "SceneData(this is not ron")?;
    let missing = dir.path().join("does_not_exist.scene.ron");

    for (case, path) in [("malformed RON", malformed), ("missing file", missing)] {
        let mut world = World::new();
        let mut editor = session_with_unsaved_work(&mut world);

        let result = editor.load_scene(&mut world, &mut StubResolver::default(), &path);

        assert!(result.is_err(), "{case} must fail the load");
        assert_session_untouched(&editor, &world, case);
    }
    Ok(())
}

#[test]
fn test_load_instantiate_failure_preserves_the_live_world() -> std::io::Result<()> {
    // The file parses, so the naive fix (parse before wiping) is not enough:
    // the dry-run instantiate must also pass before the world is touched.
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("unknown_prefab.scene.ron");
    std::fs::write(&path, UNKNOWN_PREFAB_SCENE)?;
    let mut world = World::new();
    let mut editor = session_with_unsaved_work(&mut world);

    let result = editor.load_scene(&mut world, &mut StubResolver::default(), &path);

    let err = result.expect_err("an unknown prefab must fail the load");
    assert!(err.to_string().contains("DoesNotExist"), "error should name the missing prefab: {err}");
    assert_session_untouched(&editor, &world, "instantiate failure");
    Ok(())
}

#[test]
fn test_load_replaces_the_world_publishes_physics_and_save_keeps_the_block() -> std::io::Result<()> {
    // The old EditorApp bypass load left physics_settings None, so a
    // save silently DROPPED the scene's gravity/scale. Through the real
    // load path the block round-trips, and the settings reach the world as
    // a resource for the host game's lazy physics preview.
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("loaded.scene.ron");
    std::fs::write(&path, VALID_SCENE)?;
    let mut world = World::new();
    let mut editor = session_with_unsaved_work(&mut world);

    editor
        .load_scene(&mut world, &mut StubResolver::default(), &path)
        .expect("a valid scene loads");

    assert_eq!(world.entities().len(), 2, "old entities replaced by the scene's two");
    assert!(editor.editor.selection.is_empty(), "selection is cleared on load");
    assert!(!editor.editor.is_dirty(), "a freshly loaded scene is clean");
    assert_eq!(editor.editor.scene_path(), Some(path.as_path()));
    assert_eq!(editor.physics_settings.as_ref().map(|p| p.gravity), Some((0.0, -420.0)));
    assert_eq!(
        world.resource::<PhysicsSettings>().map(|p| p.pixels_per_meter),
        Some(64.0),
        "physics published as a world resource"
    );

    let out = dir.path().join("resaved.scene.ron");
    editor
        .save_scene_with(&mut world, &test_texture_path, out.clone())
        .expect("save succeeds");
    let saved = std::fs::read_to_string(&out)?;
    assert!(saved.contains("-420"), "gravity persisted: {saved}");
    Ok(())
}

#[test]
fn test_new_scene_resets_world_and_editor_state() {
    let mut editor = editor_game();
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    spawn_at(&mut world, Vec2::ONE);
    editor.editor.selection.select(entity);
    editor.command_history.push_already_executed(Box::new(
        editor::commands::CreateEntityCommand::already_created(&world, entity),
    ));
    editor.editor.set_dirty(true);
    editor.editor.set_scene_path(Some(PathBuf::from("test.ron")));
    editor.entity_counter = 5;
    editor.physics_settings = Some(PhysicsSettings::default());
    world.insert_resource(PhysicsSettings::default());

    // Dirty or not, new_scene proceeds — the confirm dialog gates upstream.
    editor.new_scene(&mut world);

    assert_eq!(world.entities().len(), 0);
    assert!(editor.editor.selection.is_empty());
    assert!(!editor.command_history.can_undo(), "the history is fresh");
    assert!(!editor.editor.is_dirty());
    assert_eq!(editor.editor.scene_path(), None);
    assert_eq!(editor.entity_counter, 0);
    assert!(editor.physics_settings.is_none());
    assert!(!world.has_resource::<PhysicsSettings>(), "no inherited physics settings");
}
