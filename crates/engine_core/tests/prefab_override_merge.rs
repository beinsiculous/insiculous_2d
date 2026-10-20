//! The override layer's contract: an override names only the fields it
//! changes, presence comes from the text, and every way an override can fail
//! to fit its prefab is a load error naming where — all through the public
//! `SceneLoader::effective_components` and a headless load.

use ecs::World;
use engine_core::scene_data::{BehaviorData, ComponentData, SceneData, SceneLoadError};
use engine_core::scene_loader::SceneLoader;
use engine_core::test_support::StubResolver;

ecs::define_component! {
    /// Stand-in for a game-registered component (unique name — the global
    /// registry is process-wide across tests).
    pub struct OverrideMergeStats {
        pub score: f32 = 0.0,
        pub lives: u32 = 3,
        pub label: String = String::new(),
    }
}

fn register_stats() {
    ecs::register_components(|registry| registry.register::<OverrideMergeStats>());
}

/// A scene with one prefab of `prefab_components` and one entity "thing"
/// instancing it with `overrides`.
fn scene_with(prefab_components: &str, overrides: &str) -> SceneData {
    SceneLoader::parse(&format!(
        r##"SceneData(
            name: "Overrides",
            prefabs: {{ "Thing": PrefabData(components: [{prefab_components}]) }},
            entities: [EntityData(name: Some("thing"), prefab: Some("Thing"), overrides: [{overrides}])],
        )"##
    ))
    .expect("the scene parses")
}

fn effective(scene: &SceneData) -> Result<Vec<ComponentData>, SceneLoadError> {
    SceneLoader::effective_components(scene, &scene.entities[0])
}

fn load_error(scene: &SceneData) -> String {
    let mut world = World::new();
    match SceneLoader::instantiate(scene, &mut world, &mut StubResolver::default()) {
        Ok(_) => panic!("the scene was expected to fail to load"),
        Err(error) => error.to_string(),
    }
}

fn transform(components: &[ComponentData]) -> ((f32, f32), f32, (f32, f32)) {
    components
        .iter()
        .find_map(|component| match component {
            ComponentData::Transform2D { position, rotation, scale } => Some((*position, *rotation, *scale)),
            _ => None,
        })
        .expect("a Transform2D")
}

fn dynamic_data(components: &[ComponentData], component_type: &str) -> serde_json::Value {
    components
        .iter()
        .find_map(|component| match component {
            ComponentData::Dynamic { component_type: name, data } if name == component_type => Some(data.clone()),
            _ => None,
        })
        .expect("the Dynamic component")
}

#[test]
fn an_explicit_default_in_an_override_is_honoured_over_the_prefab() -> Result<(), SceneLoadError> {
    // Presence comes from the text: treating a default-valued field as absent
    // would keep the prefab's (0.8, 0.4) here in silence.
    let scene = scene_with(
        "Transform2D(scale: (0.8, 0.4), rotation: 0.5)",
        "Transform2D(position: (3.0, 4.0), scale: (1.0, 1.0))",
    );

    let (position, rotation, scale) = transform(&effective(&scene)?);

    assert_eq!(scale, (1.0, 1.0), "the named default wins");
    assert_eq!(position, (3.0, 4.0));
    assert_eq!(rotation, 0.5, "the unnamed field stays the prefab's");
    Ok(())
}

#[test]
fn a_dynamic_override_merges_one_key_and_keeps_the_rest() -> Result<(), SceneLoadError> {
    register_stats();
    let scene = scene_with(
        r#"Dynamic(type: "OverrideMergeStats", data: {"score": 7.0, "lives": 9, "label": "boss"})"#,
        r#"Dynamic(type: "OverrideMergeStats", data: {"lives": 1})"#,
    );

    let data = dynamic_data(&effective(&scene)?, "OverrideMergeStats");

    assert_eq!(data, serde_json::json!({"score": 7.0, "lives": 1, "label": "boss"}));
    Ok(())
}

#[test]
fn a_behavior_override_replaces_the_prefabs_whole_behavior() -> Result<(), SceneLoadError> {
    let scene = scene_with(
        r#"Behavior(Collectible(score_value: 10, despawn_on_collect: false, collector_tag: "hero"))"#,
        "Behavior(Collectible(score_value: 5))",
    );

    let components = effective(&scene)?;

    let Some(ComponentData::Behavior(BehaviorData::Collectible { score_value, despawn_on_collect, collector_tag })) =
        components.first()
    else {
        panic!("expected the Collectible behavior, got {components:?}");
    };
    assert_eq!(*score_value, 5);
    assert!(*despawn_on_collect, "the prefab's false is gone: the newtype replaced whole");
    assert_eq!(collector_tag, "player", "serde's default, not the prefab's tag");
    Ok(())
}

#[test]
fn an_override_naming_a_field_the_component_lacks_fails_naming_entity_component_and_field() {
    let scene = scene_with("Transform2D(scale: (0.8, 0.4))", "Transform2D(positon: (3.0, 4.0))");

    let error = load_error(&scene);

    for expected in ["'thing'", "'Transform2D'", "'positon'"] {
        assert!(error.contains(expected), "{expected} missing from: {error}");
    }
}

#[test]
fn an_override_for_a_component_the_prefab_lacks_is_appended_whole() -> Result<(), SceneLoadError> {
    let scene = scene_with("Transform2D(scale: (0.8, 0.4))", r##"Sprite(texture: "#white", depth: 2.0)"##);

    let components = effective(&scene)?;

    assert_eq!(components.len(), 2, "{components:?}");
    let Some(ComponentData::Sprite { depth, scale, visible, .. }) = components.get(1) else {
        panic!("expected the appended Sprite, got {components:?}");
    };
    assert_eq!(*depth, 2.0);
    assert_eq!(*scale, (1.0, 1.0), "an omitted field takes its serde default");
    assert!(*visible);
    assert_eq!(transform(&components).2, (0.8, 0.4), "the prefab's own component is untouched");
    Ok(())
}

#[test]
fn a_typed_and_a_dynamic_entry_of_one_type_name_fail_to_load() {
    let scene = scene_with(
        "Transform2D(scale: (0.8, 0.4))",
        r#"Dynamic(type: "Transform2D", data: {"scale": [1.0, 1.0]})"#,
    );

    let error = load_error(&scene);

    for expected in ["'thing'", "'Transform2D'", "Dynamic"] {
        assert!(error.contains(expected), "{expected} missing from: {error}");
    }
}

#[test]
fn a_dynamic_override_with_a_misspelled_key_fails_to_load() {
    register_stats();
    let scene = scene_with(
        r#"Dynamic(type: "OverrideMergeStats", data: {"score": 7.0})"#,
        r#"Dynamic(type: "OverrideMergeStats", data: {"livez": 1})"#,
    );

    let error = load_error(&scene);

    for expected in ["'thing'", "'OverrideMergeStats'", "'livez'"] {
        assert!(error.contains(expected), "{expected} missing from: {error}");
    }
}

#[test]
fn a_dynamic_payload_that_is_not_an_object_fails_to_load_on_either_side() {
    register_stats();
    let non_object_override = scene_with(
        r#"Dynamic(type: "OverrideMergeStats", data: {"score": 7.0})"#,
        r#"Dynamic(type: "OverrideMergeStats", data: 4)"#,
    );
    let non_object_prefab = scene_with(
        r#"Dynamic(type: "OverrideMergeStats", data: [7.0])"#,
        r#"Dynamic(type: "OverrideMergeStats", data: {"lives": 1})"#,
    );

    for scene in [non_object_override, non_object_prefab] {
        let error = load_error(&scene);
        assert!(error.contains("'thing'") && error.contains("object"), "{error}");
    }
}

/// `(entity name, scale, rotation)` for each prefab instance of `scene_file`.
fn instance_transforms(scene_file: &str) -> Vec<(String, (f32, f32), f32)> {
    let path = format!("{}/../../examples/assets/scenes/{scene_file}", env!("CARGO_MANIFEST_DIR"));
    let scene = SceneLoader::load_from_file(&path).expect("the example scene parses");
    let mut instances = Vec::new();
    let mut pending: Vec<&engine_core::scene_data::EntityData> = scene.entities.iter().collect();
    while let Some(entity) = pending.pop() {
        pending.extend(&entity.children);
        if entity.prefab.is_some() {
            let components = SceneLoader::effective_components(&scene, entity).expect("the instance merges");
            let (_, rotation, scale) = transform(&components);
            instances.push((entity.name.clone().unwrap_or_default(), scale, rotation));
        }
    }
    instances.sort_by(|left, right| left.0.cmp(&right.0));
    instances
}

fn named(name: &str, scale: f32, rotation: f32) -> (String, (f32, f32), f32) {
    (name.to_string(), (scale, scale), rotation)
}

fn stretched(name: &str, scale: (f32, f32)) -> (String, (f32, f32), f32) {
    (name.to_string(), scale, 0.0)
}

#[test]
fn hello_world_prefab_instances_load_at_their_prefabs_scale_and_rotation() {
    // The same numbers the re-blessed golden fixture carries: every instance
    // whose override omits scale and rotation keeps its prefab's.
    let mut expected = vec![
        named("player", 1.0, 0.0),
        named("player_indicator", 0.5, 0.0),
        stretched("ground", (10.0, 0.5)),
        stretched("floating_platform", (3.0, 0.5)),
        named("obstacle_1", 1.0, 0.0),
        named("obstacle_2", 1.0, 0.0),
        named("obstacle_3", 1.0, 0.0),
        stretched("ground_east", (10.0, 0.5)),
        stretched("step_1", (2.0, 0.5)),
        stretched("step_2", (2.0, 0.5)),
        stretched("step_3", (2.0, 0.5)),
        named("obstacle_east", 1.0, 0.0),
        named("coin_1", 0.3, 0.785),
        named("coin_2", 0.3, 0.785),
        named("coin_3", 0.3, 0.785),
        named("coin_4", 0.3, 0.785),
        named("coin_5", 0.3, 0.785),
    ];
    expected.sort_by(|left, right| left.0.cmp(&right.0));

    assert_eq!(instance_transforms("hello_world.scene.ron"), expected);
}

#[test]
fn behavior_demo_prefab_instances_load_at_their_prefabs_scale_and_rotation() {
    // Every override here names position alone (the walls also scale), so
    // each instance draws at its prefab's authored size and tilt.
    let mut expected = vec![
        named("player", 0.5, 0.0),
        named("companion", 0.35, 0.0),
        named("chaser_1", 0.4, 0.0),
        named("chaser_2", 0.4, 0.0),
        named("guard_horizontal", 0.45, 0.0),
        named("guard_vertical", 0.45, 0.0),
        named("coin_1", 0.25, 0.785),
        named("coin_2", 0.25, 0.785),
        named("coin_3", 0.25, 0.785),
        named("coin_4", 0.25, 0.785),
        named("coin_5", 0.25, 0.785),
        stretched("wall_top", (10.0, 0.3)),
        stretched("wall_bottom", (10.0, 0.3)),
        stretched("wall_left", (0.3, 7.0)),
        stretched("wall_right", (0.3, 7.0)),
        named("obstacle", 1.5, 0.0),
    ];
    expected.sort_by(|left, right| left.0.cmp(&right.0));

    assert_eq!(instance_transforms("behavior_demo.scene.ron"), expected);
}
