//! The dynamic component tier (issue #43): a game-registered component with
//! no typed editor registry entry still survives every editor copy path
//! (snapshot, clipboard, undo/redo) and reaches the command API's JSON seam.

use ecs::World;
use glam::Vec2;

use super::*;
use crate::commands::{AddDynamicComponentCommand, CommandHistory, RemoveDynamicComponentCommand};
use crate::world_snapshot::WorldSnapshot;

ecs::define_component! {
    /// Stand-in for a game-registered component (unique name — the global
    /// registry is process-wide across tests).
    pub struct EditorDynTestStat {
        pub power: f32 = 5.0,
        pub label: String = String::new(),
    }
}

const TYPE_NAME: &str = "EditorDynTestStat";

fn register_test_type() {
    ecs::register_components(|r| r.register::<EditorDynTestStat>());
}

fn world_with_dyn_entity() -> (World, ecs::EntityId) {
    register_test_type();
    let mut world = World::new();
    let entity = world.create_entity();
    world
        .add_component(&entity, common::Transform2D::new(Vec2::new(1.0, 2.0)))
        .ok();
    world
        .add_component(&entity, EditorDynTestStat { power: 42.0, label: "dyn".to_string() })
        .ok();
    (world, entity)
}

fn power(world: &World, entity: ecs::EntityId) -> Option<f32> {
    world.get::<EditorDynTestStat>(entity).map(|s| s.power)
}

#[test]
fn test_dynamic_components_survive_snapshot_restore_and_clipboard_duplicate() {
    let (mut world, entity) = world_with_dyn_entity();

    // Capture sees the value through the dynamic tier.
    let captured = capture_all_components(&world, entity);
    let dynamic = captured.iter().find(|c| c.type_name() == TYPE_NAME).expect("captured");
    let StoredComponent::Dynamic { value, .. } = dynamic else {
        panic!("expected Dynamic, got {dynamic:?}");
    };
    assert_eq!(value["power"], 42.0);
    assert_eq!(value["label"], "dyn");

    // The Play→Stop path: not reported as lost, and restored after gameplay
    // mutates it.
    let snapshot = WorldSnapshot::capture(&world);
    assert!(
        !snapshot.uncaptured_types().iter().any(|t| t.contains(TYPE_NAME)),
        "registered dynamic types are NOT reported as lost"
    );
    if let Some(stat) = world.get_mut::<EditorDynTestStat>(entity) {
        stat.power = -1.0;
    }
    snapshot.restore(&mut world);
    assert_eq!(power(&world, entity), Some(42.0), "restore returns the captured value");

    // The Duplicate/Paste path carries it too.
    let clip = crate::clipboard::capture_entity_tree(&world, entity);
    let spawned = crate::clipboard::spawn_entity_tree(&mut world, &clip, None, Vec2::new(10.0, 0.0), None);
    assert_ne!(spawned, entity);
    assert_eq!(
        world.get::<EditorDynTestStat>(spawned).map(|s| s.label.clone()),
        Some("dyn".to_string())
    );
}

#[test]
fn test_dynamic_components_add_and_remove_through_history_with_undo_redo() {
    register_test_type();
    let mut world = World::new();
    let entity = world.create_entity();
    let mut history = CommandHistory::new();

    history.execute(Box::new(AddDynamicComponentCommand::new(entity, TYPE_NAME.to_string())), &mut world);
    assert_eq!(power(&world, entity), Some(5.0), "the type's default attached");
    if let Some(stat) = world.get_mut::<EditorDynTestStat>(entity) {
        stat.power = 9.0; // user edit after add
    }
    history.undo(&mut world);
    assert_eq!(power(&world, entity), None);
    history.redo(&mut world);
    assert_eq!(power(&world, entity), Some(9.0), "redo restores the value captured at undo, not the default");

    history.execute(Box::new(RemoveDynamicComponentCommand::new(entity, TYPE_NAME.to_string())), &mut world);
    assert_eq!(power(&world, entity), None);
    history.undo(&mut world);
    assert_eq!(power(&world, entity), Some(9.0), "undo restores the removed value");
}

#[test]
fn test_dynamic_names_are_settable_and_from_json_reports_the_type_and_the_candidates() {
    register_test_type();

    assert!(
        settable_component_names().iter().any(|n| n == TYPE_NAME),
        "dynamic names are settable via the command API"
    );
    let stored = stored_component_from_json(TYPE_NAME, serde_json::json!({"power": 3.0, "label": "api"}))
        .expect("valid dynamic value");
    assert_eq!(stored.type_name(), TYPE_NAME);

    let err = stored_component_from_json(TYPE_NAME, serde_json::json!({"power": "NaN-ish string"}))
        .expect_err("bad payload");
    assert!(err.contains(TYPE_NAME), "malformed payloads are rejected with the type's own error: {err}");

    let err = stored_component_from_json("NoSuchThing", serde_json::Value::Null).expect_err("unknown name");
    assert!(err.contains("unknown component"), "{err}");
    assert!(err.contains(TYPE_NAME), "unknown names list the dynamic candidates: {err}");
}
