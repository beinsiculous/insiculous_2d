use std::path::Path;

use ecs::behavior::Behavior;
use ecs::sprite_components::Name;
use ecs::{World, WorldHierarchyExt};
use engine_core::scene_data::ComponentData;
use engine_core::scene_loader::SceneLoader;
use engine_core::scene_serializer::{serialize_to_ron, world_to_scene_data};
use engine_core::test_support::{test_texture_path, StubResolver};

fn build_golden_behavior_world() -> World {
    let mut world = World::new();
    let root = world.create_entity();
    world.add_component(&root, Name::new("root")).ok();

    for variant_index in 0..Behavior::VARIANT_NAMES.len() {
        let child = world.create_entity();
        let name = format!("entity_{}", Behavior::VARIANT_NAMES[variant_index]);
        world.add_component(&child, Name::new(name)).ok();
        world
            .add_component(&child, Behavior::default_for_variant(variant_index))
            .ok();
        world.set_parent(child, root).ok();
    }

    world
}

#[test]
fn test_behavior_scene_fixture_load_equal_and_resave_byte_identical() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/behavior_scene.ron");

    let fixture_content = std::fs::read_to_string(&fixture_path)
        .expect("read the committed behavior_scene.ron fixture — never regenerate it here");

    // The fixture was written by the serializer before `BehaviorData` became an
    // alias of `ecs::Behavior`; today's serializer must still produce those bytes.
    let golden = world_to_scene_data(
        &build_golden_behavior_world(),
        "BehaviorScene",
        None,
        &test_texture_path,
    );
    let golden_ron = serialize_to_ron(&golden).expect("serialize golden behavior scene");
    assert_eq!(golden_ron, fixture_content, "the behavior wire shape changed");

    // 1. Parse scene data
    let scene_data = SceneLoader::parse(&fixture_content)
        .expect("parse behavior_scene.ron");
    assert_eq!(scene_data.name, "BehaviorScene");
    assert_eq!(scene_data.entities.len(), 1, "root entity exists");

    let children = &scene_data.entities[0].children;
    assert_eq!(
        children.len(),
        Behavior::VARIANT_NAMES.len(),
        "all behavior variants represented"
    );

    for (variant_index, child_data) in children.iter().enumerate() {
        let behavior_component = child_data
            .components
            .iter()
            .find_map(|component| match component {
                ComponentData::Behavior(behavior_data) => Some(behavior_data),
                _ => None,
            })
            .expect("child entity has Behavior component");

        let expected_behavior = Behavior::default_for_variant(variant_index);
        let behavior_json = serde_json::to_value(behavior_component).expect("serializes to json");
        let expected_json = serde_json::to_value(&expected_behavior).expect("serializes to json");
        assert_eq!(
            behavior_json, expected_json,
            "parsed behavior matches default_for_variant({variant_index})"
        );
    }

    // 2. Instantiate into world and verify loaded entities have exact behaviors
    let mut world = World::new();
    let mut resolver = StubResolver::default();
    SceneLoader::instantiate(&scene_data, &mut world, &mut resolver)
        .expect("instantiate behavior scene");

    let roots = world.get_root_entities();
    assert_eq!(roots.len(), 1, "one root in instantiated world");
    let root = roots[0];

    let child_ids = world.get_children(root).expect("root has children").to_vec();
    assert_eq!(child_ids.len(), Behavior::VARIANT_NAMES.len());

    for (variant_index, &child_id) in child_ids.iter().enumerate() {
        let behavior = world
            .get::<Behavior>(child_id)
            .expect("child has Behavior in world");
        let expected_behavior = Behavior::default_for_variant(variant_index);
        let behavior_json = serde_json::to_value(behavior).expect("serializes to json");
        let expected_json = serde_json::to_value(&expected_behavior).expect("serializes to json");
        assert_eq!(
            behavior_json, expected_json,
            "world behavior matches default_for_variant({variant_index})"
        );
    }

    // 3. Re-save from world and assert byte-identical to fixture
    let resaved_scene_data =
        world_to_scene_data(&world, "BehaviorScene", None, &test_texture_path);
    let resaved_ron =
        serialize_to_ron(&resaved_scene_data).expect("serialize resaved behavior scene");

    assert_eq!(
        resaved_ron, fixture_content,
        "resaved RON must be byte-identical to original fixture"
    );
}

#[test]
fn test_example_scenes_parse_successfully() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let behavior_demo_path = manifest_dir
        .join("../../examples/assets/scenes/behavior_demo.scene.ron");
    let hello_world_path = manifest_dir
        .join("../../examples/assets/scenes/hello_world.scene.ron");

    let behavior_demo = SceneLoader::load_from_file(&behavior_demo_path)
        .expect("behavior_demo.scene.ron must parse");
    assert_eq!(behavior_demo.name, "Behavior Demo");

    let hello_world = SceneLoader::load_from_file(&hello_world_path)
        .expect("hello_world.scene.ron must parse");
    assert_eq!(hello_world.name, "Hello World");
}
