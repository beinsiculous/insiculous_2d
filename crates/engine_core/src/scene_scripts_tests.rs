//! Scene round-trip tests for `ComponentData::Scripts` (issue #44 Stage 1):
//! every param type survives save→load, Entity params remap by NAME, and
//! unnamed referenced targets get auto-named at the save choke point.

use std::collections::BTreeMap;

use ecs::script::{ScriptRef, ScriptValue, Scripts};
use ecs::sprite_components::Name;
use ecs::World;
use glam::Vec2;

use crate::scene_loader::SceneLoader;
use crate::scene_serializer::{serialize_to_ron, world_to_scene_data};
use crate::script_data::ensure_script_target_names;
use crate::texture_ref::TextureResolver;
use crate::SceneLoadError;

fn test_texture_path(handle: u32) -> String {
    if handle == 0 { "#white".to_string() } else { format!("#texture_{handle}") }
}

struct StubResolver;
impl TextureResolver for StubResolver {
    fn resolve_texture(
        &mut self,
        _texture_ref: &str,
    ) -> Result<renderer::TextureHandle, SceneLoadError> {
        Ok(renderer::TextureHandle { id: 0 })
    }
}

fn roundtrip(world: &World) -> (World, crate::scene_loader::SceneInstance) {
    let scene = world_to_scene_data(world, "ScriptsRoundTrip", None, &test_texture_path);
    let ron_string = serialize_to_ron(&scene).expect("serialize");
    let parsed = SceneLoader::parse(&ron_string).expect("parse");
    let mut loaded = World::new();
    let instance =
        SceneLoader::instantiate(&parsed, &mut loaded, &mut StubResolver).expect("instantiate");
    (loaded, instance)
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

#[test]
fn test_scripts_scene_round_trip_preserves_every_param_type() {
    let mut world = World::new();
    let target = world.create_entity();
    world.add_component(&target, Name::new("goal")).ok();
    world.add_component(&target, common::Transform2D::new(Vec2::new(9.0, 9.0))).ok();

    let owner = world.create_entity();
    world.add_component(&owner, Name::new("runner")).ok();
    world.add_component(&owner, common::Transform2D::new(Vec2::ZERO)).ok();
    world
        .add_component(
            &owner,
            Scripts(vec![ScriptRef {
                script_id: "chase".into(),
                source_path: "src/scripts/chase.rs".into(),
                params: full_params(target),
            }]),
        )
        .ok();

    let (loaded, instance) = roundtrip(&world);
    let owner = instance.named_entities["runner"];
    let goal = instance.named_entities["goal"];
    let scripts = loaded.get::<Scripts>(owner).expect("Scripts survived");
    assert_eq!(scripts.0.len(), 1);
    let script = &scripts.0[0];
    assert_eq!(script.script_id, "chase");
    assert_eq!(script.source_path, "src/scripts/chase.rs");
    assert_eq!(script.params["speed"], ScriptValue::F32(240.5));
    assert_eq!(script.params["lives"], ScriptValue::I32(-3));
    assert_eq!(script.params["armed"], ScriptValue::Bool(true));
    assert_eq!(script.params["word"], ScriptValue::Str("héllo".into()));
    assert_eq!(script.params["dir"], ScriptValue::Vec2(Vec2::new(1.5, -0.25)));
    assert_eq!(script.params["tint"], ScriptValue::Color([0.1, 0.2, 0.3, 0.9]));
    // The Entity param resolved to the FRESH id of the entity named "goal".
    assert_eq!(script.params["target"], ScriptValue::Entity(goal));
}

#[test]
fn test_entity_param_referencing_missing_name_is_dropped_with_warning() {
    // The wire names an entity that doesn't exist (hand-edited scene):
    // param dropped, everything else loads.
    let ron = r#"SceneData(
        name: "MissingTarget",
        entities: [
            EntityData(
                name: Some("runner"),
                components: [
                    Scripts([(
                        script_id: "chase",
                        params: {"target": Entity("no_such_entity"), "speed": F32(5.0)},
                    )]),
                ],
            ),
        ],
    )"#;
    let parsed = SceneLoader::parse(ron).expect("parse");
    let mut world = World::new();
    let instance =
        SceneLoader::instantiate(&parsed, &mut world, &mut StubResolver).expect("instantiate");
    let owner = instance.named_entities["runner"];
    let scripts = world.get::<Scripts>(owner).expect("Scripts loaded");
    assert_eq!(scripts.0[0].params.get("speed"), Some(&ScriptValue::F32(5.0)));
    assert!(
        !scripts.0[0].params.contains_key("target"),
        "unresolvable Entity param is dropped, never a dangling id"
    );
}

#[test]
fn test_entity_param_can_reference_a_later_entity() {
    // The target appears AFTER the owner in the scene — the deferred
    // post-instantiate pass must still resolve it.
    let ron = r#"SceneData(
        name: "ForwardRef",
        entities: [
            EntityData(
                name: Some("runner"),
                components: [
                    Scripts([(script_id: "chase", params: {"target": Entity("goal")})]),
                ],
            ),
            EntityData(name: Some("goal"), components: []),
        ],
    )"#;
    let parsed = SceneLoader::parse(ron).expect("parse");
    let mut world = World::new();
    let instance =
        SceneLoader::instantiate(&parsed, &mut world, &mut StubResolver).expect("instantiate");
    let owner = instance.named_entities["runner"];
    let goal = instance.named_entities["goal"];
    let scripts = world.get::<Scripts>(owner).expect("Scripts loaded");
    assert_eq!(scripts.0[0].params["target"], ScriptValue::Entity(goal));
}

#[test]
fn test_save_auto_names_referenced_unnamed_targets() {
    // kimi plan R2-F4 (decided: auto-name): a script pointing at an unnamed
    // entity must not lose the binding on save.
    let mut world = World::new();
    let target = world.create_entity(); // NO Name
    world.add_component(&target, common::Transform2D::new(Vec2::ZERO)).ok();
    let owner = world.create_entity();
    world.add_component(&owner, Name::new("runner")).ok();
    let mut params = BTreeMap::new();
    params.insert("target".into(), ScriptValue::Entity(target));
    world
        .add_component(&owner, Scripts(vec![ScriptRef { script_id: "s".into(), source_path: String::new(), params }]))
        .ok();

    // The editor save choke point runs this before serializing.
    let assigned = ensure_script_target_names(&mut world);
    assert_eq!(assigned.len(), 1);
    assert!(assigned[0].1.starts_with("script_target_"));
    assert!(world.get::<Name>(target).is_some());

    // And the whole binding now round-trips.
    let (loaded, instance) = roundtrip(&world);
    let owner = instance.named_entities["runner"];
    let fresh_target = instance.named_entities[&assigned[0].1];
    let scripts = loaded.get::<Scripts>(owner).expect("Scripts survived");
    assert_eq!(scripts.0[0].params["target"], ScriptValue::Entity(fresh_target));
}

#[test]
fn test_auto_name_skips_named_targets_and_avoids_collisions() {
    let mut world = World::new();
    // An entity already using the generated pattern forces the counter on.
    let squatter = world.create_entity();
    world.add_component(&squatter, Name::new("script_target_1")).ok();
    let target = world.create_entity();
    let owner = world.create_entity();
    let mut params = BTreeMap::new();
    params.insert("t".into(), ScriptValue::Entity(target));
    params.insert("named".into(), ScriptValue::Entity(squatter));
    world
        .add_component(&owner, Scripts(vec![ScriptRef { script_id: "s".into(), source_path: String::new(), params }]))
        .ok();

    let assigned = ensure_script_target_names(&mut world);
    assert_eq!(assigned.len(), 1, "already-named targets are untouched");
    assert_eq!(assigned[0].1, "script_target_2", "collision skipped");
}
