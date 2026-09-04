use std::path::Path;

use ecs::World;
use engine_core::scene_loader::SceneLoader;
use engine_core::scene_serializer::{serialize_to_ron, world_to_scene_data};
use engine_core::test_support::{test_texture_path, StubResolver};

/// Environment variable that makes the test rewrite the fixture instead of
/// comparing against it. A blessed fixture is hand-diffed against the source
/// scene before it is committed: only prefab flattening, the stub's `#white`
/// textures, defaults written out and root order may differ, or the change
/// that produced it is a serializer bug being blessed.
const BLESS_ENVIRONMENT_VARIABLE: &str = "HELLO_WORLD_GOLDEN_BLESS";

#[test]
fn test_hello_world_scene_resaves_byte_identical_to_golden() {
    let source_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/assets/scenes/hello_world.scene.ron"
    );
    let source_content =
        std::fs::read_to_string(source_path).expect("read source hello_world.scene.ron");
    let parsed = SceneLoader::parse(&source_content).expect("parse hello_world.scene.ron");

    let mut world = World::new();
    let mut resolver = StubResolver::default();
    SceneLoader::instantiate(&parsed, &mut world, &mut resolver).expect("instantiate hello world");

    let saved_data = world_to_scene_data(&world, "Hello World", parsed.physics, &test_texture_path);
    let saved_ron = serialize_to_ron(&saved_data).expect("serialize hello world");

    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hello_world_saved.scene.ron");

    if std::env::var_os(BLESS_ENVIRONMENT_VARIABLE).is_some() {
        std::fs::write(&fixture_path, &saved_ron).expect("write the blessed fixture");
        return;
    }

    let fixture_content = std::fs::read_to_string(&fixture_path)
        .expect("read the committed hello_world_saved.scene.ron fixture — bless it deliberately, never by hand");

    assert_eq!(saved_ron, fixture_content, "the serialized scene bytes changed");
}
