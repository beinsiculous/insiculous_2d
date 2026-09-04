//! Scene round-trips for the two by-name wire tiers: `ComponentData::Dynamic`
//! (game-registered components, and the audio components the
//! serializer used to drop silently) and `ComponentData::Scripts`
//! (Entity params persist by NAME).

use std::collections::BTreeMap;

use ecs::audio_components::{AudioListener, AudioSource, PlaySoundEffect};
use ecs::script::{ScriptRef, ScriptValue, Scripts};
use ecs::sprite_components::{Name, Transform2D};
use ecs::World;
use glam::Vec2;

use super::world_to_scene_data;
use crate::scene_data::ComponentData;
use crate::scene_loader::SceneLoader;
use crate::script_data::ensure_script_target_names;
use crate::test_support::{load_ron, roundtrip, test_texture_path, StubResolver};

/// A component as serde sees it — the field-for-field comparison for types
/// without `PartialEq`.
fn as_json<T: serde::Serialize>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).expect("component serializes")
}

ecs::define_component! {
    /// Stand-in for a game-registered component (unique name — the global
    /// registry is process-wide across tests).
    pub struct SceneDynSpikeStats {
        pub score: f32 = 0.0,
        pub kills: u32 = 0,
        pub label: String = String::new(),
        pub aim: (f32, f32) = (0.0, 0.0),
    }
}

/// The `Dynamic` rows the serializer writes for `world`'s only entity, by
/// component type name, in wire order.
fn dynamic_types(world: &World) -> Vec<String> {
    let scene = world_to_scene_data(world, "Dynamic", None, &test_texture_path);
    scene.entities[0]
        .components
        .iter()
        .filter_map(|c| match c {
            ComponentData::Dynamic { component_type, .. } => Some(component_type.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn dynamic_payload_survives_ron_round_trip() {
    // The load-bearing risk of the dynamic tier: serde_json::Value → RON → Value must
    // preserve ints, floats, strings and nesting exactly — for a game type
    // and for the engine's own dynamic-tier audio components.
    ecs::register_components(|r| r.register::<SceneDynSpikeStats>());
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(Vec2::ZERO)).ok();
    let stats = SceneDynSpikeStats {
        score: 1234.5,
        kills: 42,
        label: "boss #1 — \"final\"".to_string(),
        aim: (-0.25, 1.0),
    };
    world.add_component(&entity, stats.clone()).ok();
    let source = AudioSource { sound_id: 7, volume: 0.4, spatial: true, max_distance: 640.0, ..AudioSource::default() };
    world.add_component(&entity, source.clone()).ok();
    world.add_component(&entity, AudioListener::default()).ok();

    let (loaded, instance) = roundtrip(&world);

    let entity = instance.entities[0];
    assert_eq!(as_json(&loaded.get::<SceneDynSpikeStats>(entity)), as_json(&Some(&stats)));
    assert_eq!(as_json(&loaded.get::<AudioSource>(entity)), as_json(&Some(&source)));
    assert_eq!(as_json(&loaded.get::<AudioListener>(entity)), as_json(&Some(&AudioListener::default())));
}

#[test]
fn transient_components_are_never_saved() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(Vec2::ZERO)).ok();
    world.add_component(&entity, PlaySoundEffect::new(3)).ok();

    let types = dynamic_types(&world);

    assert!(!types.contains(&"PlaySoundEffect".to_string()), "one-shot requests must not persist: {types:?}");
}

#[test]
fn dynamic_rows_are_name_sorted_so_repeated_saves_diff_clean() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(Vec2::ZERO)).ok();
    world.add_component(&entity, AudioSource::default()).ok();
    world.add_component(&entity, AudioListener::default()).ok();

    let types = dynamic_types(&world);

    assert_eq!(types, vec!["AudioListener".to_string(), "AudioSource".to_string()]);
}

#[test]
fn unknown_dynamic_component_refuses_the_whole_load() {
    // Silently dropping (then resaving without) a game component would
    // corrupt scenes — an unregistered name fails the load, naming itself.
    let parsed = crate::scene_loader::SceneLoader::parse(
        r#"SceneData(name: "Unknown", entities: [EntityData(components: [Dynamic(type: "NeverRegisteredComponent", data: {"x": 1})])])"#,
    )
    .expect("parse is schema-level");
    let mut world = World::new();

    let err = crate::scene_loader::SceneLoader::instantiate(&parsed, &mut world, &mut StubResolver::default())
        .expect_err("unregistered dynamic component must refuse the load");

    assert!(err.to_string().contains("NeverRegisteredComponent"), "error names the component: {err}");
}

fn full_params(target: ecs::EntityId) -> BTreeMap<String, ScriptValue> {
    let mut params = BTreeMap::new();
    params.insert("speed".into(), ScriptValue::F32(240.5));
    params.insert("lives".into(), ScriptValue::I32(-3));
    params.insert("armed".into(), ScriptValue::Bool(true));
    params.insert("word".into(), ScriptValue::Str("héllo".into()));
    params.insert("dir".into(), ScriptValue::Vec2(Vec2::new(1.5, -0.25)));
    params.insert("target".into(), ScriptValue::Entity(target));
    params.insert("tint".into(), ScriptValue::Color([0.1, 0.2, 0.3, 0.9]));
    params
}

fn script(params: BTreeMap<String, ScriptValue>) -> Scripts {
    Scripts(vec![ScriptRef { script_id: "chase".into(), source_path: "src/scripts/chase.rs".into(), params }])
}

#[test]
fn scripts_round_trip_every_param_type_and_remap_entities_by_name() {
    let mut world = World::new();
    let target = world.create_entity();
    world.add_component(&target, Name::new("goal")).ok();
    let owner = world.create_entity();
    world.add_component(&owner, Name::new("runner")).ok();
    world.add_component(&owner, script(full_params(target))).ok();

    let (loaded, instance) = roundtrip(&world);

    let owner = instance.named_entities["runner"];
    let goal = instance.named_entities["goal"];
    let scripts = loaded.get::<Scripts>(owner).expect("Scripts survived");
    assert_eq!(scripts.0.len(), 1);
    assert_eq!(scripts.0[0].script_id, "chase");
    assert_eq!(scripts.0[0].source_path, "src/scripts/chase.rs");
    // Every non-entity param is byte-identical; the Entity param resolved to
    // the FRESH id of the entity named "goal".
    let mut expected = full_params(goal);
    expected.insert("target".into(), ScriptValue::Entity(goal));
    assert_eq!(scripts.0[0].params, expected);
}

#[test]
fn entity_params_resolve_forward_references_and_drop_missing_names() {
    // The target appears AFTER the owner: the deferred post-instantiate pass
    // resolves it. A name no entity carries (hand-edited scene) is dropped —
    // never a dangling id — and everything else still loads.
    let (world, instance) = load_ron(
        r#"SceneData(
            name: "Refs",
            entities: [
                EntityData(
                    name: Some("runner"),
                    components: [Scripts([(
                        script_id: "chase",
                        params: {"target": Entity("goal"), "ghost": Entity("no_such_entity"), "speed": F32(5.0)},
                    )])],
                ),
                EntityData(name: Some("goal"), components: []),
            ],
        )"#,
    );

    let owner = instance.named_entities["runner"];
    let goal = instance.named_entities["goal"];
    let params = &world.get::<Scripts>(owner).expect("Scripts loaded").0[0].params;
    assert_eq!(params.get("target"), Some(&ScriptValue::Entity(goal)));
    assert_eq!(params.get("speed"), Some(&ScriptValue::F32(5.0)));
    assert_eq!(params.get("ghost"), None);
}

#[test]
fn save_auto_names_referenced_unnamed_targets_skipping_taken_names() {
    // A script pointing at an unnamed entity must not lose the binding on
    // save: the editor's save choke point names the target first, stepping
    // past any name already in use.
    let mut world = World::new();
    let squatter = world.create_entity();
    world.add_component(&squatter, Name::new("script_target_1")).ok();
    let target = world.create_entity(); // no Name
    let owner = world.create_entity();
    world.add_component(&owner, Name::new("runner")).ok();
    let mut params = BTreeMap::new();
    params.insert("target".into(), ScriptValue::Entity(target));
    params.insert("named".into(), ScriptValue::Entity(squatter));
    world.add_component(&owner, script(params)).ok();

    let assigned = ensure_script_target_names(&mut world);

    assert_eq!(assigned.len(), 1, "already-named targets are untouched");
    assert_eq!(assigned[0].1, "script_target_2", "the taken name is skipped");
    assert_eq!(world.get::<Name>(target).map(|n| n.0.as_str()), Some("script_target_2"));

    let (loaded, instance) = roundtrip(&world);
    let owner = instance.named_entities["runner"];
    let fresh_target = instance.named_entities["script_target_2"];
    let fresh_squatter = instance.named_entities["script_target_1"];
    let params = &loaded.get::<Scripts>(owner).expect("Scripts survived").0[0].params;
    assert_eq!(params["target"], ScriptValue::Entity(fresh_target));
    assert_eq!(params["named"], ScriptValue::Entity(fresh_squatter));
}

#[test]
fn exclusion_list_drift_guard_saves_every_persistent_type_exactly_once() {
    crate::component_registration::register_engine_components();
    let mut world = World::new();
    let entity = world.create_entity();

    let persistent_names = ecs::with_global_registry(|registry| {
        for name in registry.persistent_names() {
            registry
                .insert_default(&mut world, entity, name)
                .unwrap_or_else(|error| panic!("failed inserting default for {name}: {error}"));
        }
        registry.persistent_names()
    });

    let scene = world_to_scene_data(&world, "DriftGuard", None, &test_texture_path);
    assert_eq!(scene.entities.len(), 1);
    let entity_data = &scene.entities[0];

    assert!(entity_data.name.is_some(), "Name must be recorded on EntityData");

    let rows = super::components::concrete_components();

    ecs::with_global_registry(|registry| {
        for row in &rows {
            assert!(
                registry.is_registered(row.registry_name),
                "row {} is not registered in global component registry",
                row.registry_name
            );
        }
    });

    // Wire name → registry name, derived from what each row's extractor writes
    // for this all-defaults entity — the table carries no wire name of its own.
    let registry_name_by_wire_name: std::collections::HashMap<String, &str> = rows
        .iter()
        .map(|row| {
            let component = (row.extract)(&world, entity, &test_texture_path)
                .unwrap_or_else(|| panic!("extract returned None for row {}", row.registry_name));
            (SceneLoader::component_type_name(&component).to_string(), row.registry_name)
        })
        .collect();

    let mut seen_types = std::collections::HashMap::<String, usize>::new();
    seen_types.insert("Name".to_string(), 1);

    for component in &entity_data.components {
        let registry_name = match component {
            ComponentData::Dynamic { component_type, .. } => {
                assert!(
                    !rows.iter().any(|row| row.registry_name == component_type),
                    "dynamic component '{component_type}' duplicates a concrete row"
                );
                component_type.as_str()
            }
            _ => {
                let wire_name = SceneLoader::component_type_name(component);
                *registry_name_by_wire_name
                    .get(wire_name)
                    .unwrap_or_else(|| panic!("no table row for wire name '{wire_name}'"))
            }
        };
        *seen_types.entry(registry_name.to_string()).or_default() += 1;
    }

    for name in &persistent_names {
        let count = seen_types.get(*name).copied().unwrap_or(0);
        assert_eq!(count, 1, "persistent type {name} must appear exactly once, got {count}");
    }
}

#[test]
fn every_table_row_extracts_its_own_variant_and_no_two_rows_share_one() {
    crate::component_registration::register_engine_components();
    let mut world = World::new();
    let entity = world.create_entity();

    ecs::with_global_registry(|registry| {
        for name in registry.persistent_names() {
            registry
                .insert_default(&mut world, entity, name)
                .unwrap_or_else(|error| panic!("failed inserting default for {name}: {error}"));
        }
    });

    let rows = super::components::concrete_components();
    let mut wire_names = std::collections::HashSet::new();
    for row in &rows {
        let component = (row.extract)(&world, entity, &test_texture_path)
            .unwrap_or_else(|| panic!("extract returned None for row {}", row.registry_name));
        let wire_name = SceneLoader::component_type_name(&component).to_string();
        assert!(
            !matches!(component, ComponentData::Dynamic { .. }),
            "row {} extracted a Dynamic component",
            row.registry_name
        );
        assert!(
            wire_names.insert(wire_name.clone()),
            "row {} extracted '{wire_name}', which another row already produced",
            row.registry_name
        );
    }
    assert_eq!(wire_names.len(), rows.len());
}
