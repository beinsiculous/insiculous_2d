//! Tests for scene-load data-loss protection (issue #50): a corrupt or
//! missing scene file — or one that parses but fails to instantiate — must
//! never cost the user their current world. The world is replaced only
//! after the file has parsed AND dry-run instantiated successfully.

use std::path::PathBuf;

use engine_core::contexts::GameContext;
use engine_core::scene_data::SceneLoadError;
use engine_core::{Game, TextureResolver};
use renderer::TextureHandle;

use super::EditorGame;

struct DummyGame;
impl Game for DummyGame {
    fn update(&mut self, _ctx: &mut GameContext) {}
}

/// Headless stand-in for `AssetManager` (which needs a GPU device): every
/// texture reference resolves to the built-in white texture.
struct StubResolver;
impl TextureResolver for StubResolver {
    fn resolve_texture(&mut self, _texture_ref: &str) -> Result<TextureHandle, SceneLoadError> {
        Ok(TextureHandle::WHITE)
    }
}

/// Temp scene file that removes itself when the test ends (pass or panic).
struct TempScene(PathBuf);
impl Drop for TempScene {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn write_scene(name: &str, content: &str) -> TempScene {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, content).expect("test scene must be writable");
    TempScene(path)
}

const VALID_SCENE: &str = r#"SceneData(
    name: "Loaded",
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
fn session_with_unsaved_work(world: &mut ecs::World) -> EditorGame<DummyGame> {
    let entity = world.create_entity();
    let mut editor = EditorGame::new(DummyGame);
    editor.editor.selection.select(entity);
    editor.editor.set_scene_path(Some(PathBuf::from("previous.scene.ron")));
    editor.editor.set_dirty(true);
    editor
}

#[test]
fn test_load_malformed_ron_preserves_world() {
    let mut world = ecs::World::new();
    let mut editor = session_with_unsaved_work(&mut world);
    let scene = write_scene("test_load_malformed.scene.ron", "SceneData(this is not ron");

    let result = editor.load_scene(&mut world, &mut StubResolver, &scene.0);

    assert!(result.is_err(), "malformed RON must fail the load");
    assert_eq!(world.entities().len(), 1, "the current world must survive a failed parse");
    assert!(!editor.editor.selection.is_empty(), "selection must survive a failed load");
    assert!(editor.editor.is_dirty(), "dirty state must survive a failed load");
    assert_eq!(
        editor.editor.scene_path(),
        Some(std::path::Path::new("previous.scene.ron")),
        "a failed load must not adopt the new path"
    );
}

#[test]
fn test_load_missing_file_preserves_world() {
    let mut world = ecs::World::new();
    let mut editor = session_with_unsaved_work(&mut world);
    let path = std::env::temp_dir().join("test_load_missing_does_not_exist.scene.ron");

    let result = editor.load_scene(&mut world, &mut StubResolver, &path);

    assert!(result.is_err(), "a missing file must fail the load");
    assert_eq!(world.entities().len(), 1, "the current world must survive a missing file");
    assert!(editor.editor.is_dirty());
}

#[test]
fn test_load_instantiate_failure_preserves_world() {
    // The file parses, so the naive fix (parse before wiping) is not enough:
    // the dry-run instantiate must also pass before the world is touched.
    let mut world = ecs::World::new();
    let mut editor = session_with_unsaved_work(&mut world);
    let scene = write_scene("test_load_unknown_prefab.scene.ron", UNKNOWN_PREFAB_SCENE);

    let result = editor.load_scene(&mut world, &mut StubResolver, &scene.0);

    let err = result.expect_err("an unknown prefab must fail the load");
    assert!(err.contains("DoesNotExist"), "error should name the missing prefab: {err}");
    assert_eq!(
        world.entities().len(),
        1,
        "the current world must survive a failed instantiate (not be emptied or half-loaded)"
    );
    assert!(!editor.editor.selection.is_empty());
    assert!(editor.editor.is_dirty());
}

#[test]
fn test_load_valid_scene_replaces_world() {
    let mut world = ecs::World::new();
    let mut editor = session_with_unsaved_work(&mut world);
    let scene = write_scene("test_load_valid.scene.ron", VALID_SCENE);

    let result = editor.load_scene(&mut world, &mut StubResolver, &scene.0);

    assert!(result.is_ok(), "a valid scene must load: {result:?}");
    assert_eq!(world.entities().len(), 2, "old entities replaced by the scene's two");
    assert!(editor.editor.selection.is_empty(), "selection is cleared on successful load");
    assert!(!editor.editor.is_dirty(), "a freshly loaded scene is clean");
    assert_eq!(editor.editor.scene_path(), Some(scene.0.as_path()));
}

#[test]
fn test_save_auto_names_script_targets_through_command_history() {
    // kimi #44 code F1/F2: the save-time auto-naming must be an UNDOABLE
    // CommandHistory entry, never a silent world mutation.
    use ecs::script::{ScriptRef, ScriptValue, Scripts};
    fn test_texture_path_fn(handle: u32) -> String {
        if handle == 0 { "#white".to_string() } else { format!("#texture_{handle}") }
    }
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let target = world.create_entity(); // unnamed
    let owner = world.create_entity();
    world.add_component(&owner, ecs::Name::new("runner")).ok();
    let mut script = ScriptRef::new("chase");
    script
        .params
        .insert("target".to_string(), ScriptValue::Entity(target));
    world.add_component(&owner, Scripts(vec![script])).ok();

    let path = std::env::temp_dir().join("test_autoname_script_targets.ron");
    editor
        .save_scene_with(&mut world, &test_texture_path_fn, path.clone())
        .unwrap();

    let assigned = world
        .get::<ecs::Name>(target)
        .map(|n| n.0.clone())
        .expect("referenced target auto-named on save");
    assert!(assigned.starts_with("script_target_"));
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains(&assigned), "the binding reached the file by name");

    // The naming is one undoable entry.
    assert!(editor.command_history.can_undo());
    editor.command_history.undo(&mut world);
    assert!(
        world.get::<ecs::Name>(target).is_none(),
        "undo removes the auto-assigned name"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_load_scene_publishes_physics_resource_and_save_keeps_the_block() {
    // #53: the old EditorApp bypass load left physics_settings None, so a
    // save silently DROPPED the scene's gravity/scale. Through the real
    // load path the block round-trips, and the settings reach the world as
    // a resource for the host game's lazy physics preview.
    fn tex(handle: u32) -> String {
        if handle == 0 { "#white".to_string() } else { format!("#texture_{handle}") }
    }
    let ron = r#"SceneData(
        name: "PhysScene",
        physics: Some(PhysicsSettings(gravity: (0.0, -420.0), pixels_per_meter: 64.0)),
        entities: [ EntityData(name: Some("thing"), components: []) ],
    )"#;
    let dir = std::env::temp_dir();
    let path = dir.join("test_53_phys_scene.ron");
    std::fs::write(&path, ron).unwrap();

    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    editor.load_scene(&mut world, &mut StubResolver, &path).unwrap();

    assert_eq!(editor.editor.scene_path(), Some(path.as_path()));
    let settings = editor.physics_settings.clone().expect("physics recorded");
    assert_eq!(settings.gravity, (0.0, -420.0));
    let resource = world
        .resource::<engine_core::scene_data::PhysicsSettings>()
        .expect("physics published as a world resource");
    assert_eq!(resource.pixels_per_meter, 64.0);

    // Save after load: the physics block survives.
    let out = dir.join("test_53_phys_scene_out.ron");
    editor.save_scene_with(&mut world, &tex, out.clone()).unwrap();
    let saved = std::fs::read_to_string(&out).unwrap();
    assert!(saved.contains("-420"), "gravity persisted: {saved}");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_new_scene_clears_the_physics_resource() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    world.insert_resource(engine_core::scene_data::PhysicsSettings {
        gravity: (0.0, -1.0),
        pixels_per_meter: 10.0,
        timestep: 1.0 / 60.0,
    });
    editor.new_scene(&mut world);
    assert!(
        world.resource::<engine_core::scene_data::PhysicsSettings>().is_none(),
        "a new scene has no inherited physics settings"
    );
}

#[test]
fn test_find_first_scene_is_sorted_and_ron_only() {
    let dir = std::env::temp_dir().join("test_53_find_first_scene");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("zeta.ron"), "x").unwrap();
    std::fs::write(dir.join("alpha.ron"), "x").unwrap();
    std::fs::write(dir.join("aaa.txt"), "x").unwrap();

    let first = crate::find_first_scene(&dir).expect("found a scene");
    assert_eq!(first.file_name().and_then(|n| n.to_str()), Some("alpha.ron"));

    let empty = std::env::temp_dir().join("test_53_find_first_scene_empty");
    let _ = std::fs::remove_dir_all(&empty);
    std::fs::create_dir_all(&empty).unwrap();
    assert!(crate::find_first_scene(&empty).is_none());
    assert!(crate::find_first_scene(std::path::Path::new("/no/such/dir_53")).is_none());

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&empty);
}
