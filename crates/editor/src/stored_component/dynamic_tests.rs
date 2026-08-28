//! Tests for the dynamic component tier (issue #43): game-registered
//! components surviving snapshot, clipboard, undo/redo, and the command API
//! without any typed editor registry entry.

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
        .add_component(
            &entity,
            EditorDynTestStat {
                power: 42.0,
                label: "dyn".to_string(),
            },
        )
        .ok();
    (world, entity)
}

#[test]
fn test_capture_all_components_includes_dynamic_types() {
    let (world, entity) = world_with_dyn_entity();
    let captured = capture_all_components(&world, entity);
    let dynamic = captured
        .iter()
        .find(|c| c.type_name() == "EditorDynTestStat")
        .expect("dynamic component captured");
    match dynamic {
        StoredComponent::Dynamic { value, .. } => {
            assert_eq!(value["power"], 42.0);
            assert_eq!(value["label"], "dyn");
        }
        other => panic!("expected Dynamic, got {other:?}"),
    }

    // apply_to restores it onto a fresh entity.
    let mut world2 = World::new();
    let fresh = world2.create_entity();
    for c in &captured {
        c.apply_to(&mut world2, fresh);
    }
    assert_eq!(
        world2.get::<EditorDynTestStat>(fresh).map(|s| s.power),
        Some(42.0)
    );
}

#[test]
fn test_world_snapshot_round_trips_dynamic_components() {
    // The Play→Stop path: a game-registered component must survive the
    // snapshot restore AND stop appearing in the loss warning.
    let (mut world, entity) = world_with_dyn_entity();
    let snapshot = WorldSnapshot::capture(&world);
    assert!(
        !snapshot
            .uncaptured_types()
            .iter()
            .any(|t| t.contains("EditorDynTestStat")),
        "registered dynamic types are NOT reported as lost"
    );

    // Simulate gameplay mutating it, then Stop.
    if let Some(stat) = world.get_mut::<EditorDynTestStat>(entity) {
        stat.power = -1.0;
    }
    snapshot.restore(&mut world);
    assert_eq!(
        world.get::<EditorDynTestStat>(entity).map(|s| s.power),
        Some(42.0),
        "restore returns the captured value"
    );
}

#[test]
fn test_clipboard_duplicate_carries_dynamic_components() {
    let (mut world, entity) = world_with_dyn_entity();
    let clip = crate::clipboard::capture_entity_tree(&world, entity);
    let spawned =
        crate::clipboard::spawn_entity_tree(&mut world, &clip, None, Vec2::new(10.0, 0.0), None);
    assert_ne!(spawned, entity);
    assert_eq!(
        world.get::<EditorDynTestStat>(spawned).map(|s| s.label.clone()),
        Some("dyn".to_string())
    );
}

#[test]
fn test_add_remove_dynamic_commands_undo_redo() {
    register_test_type();
    let mut world = World::new();
    let entity = world.create_entity();
    let mut history = CommandHistory::new();

    // Add default → undo removes → redo restores (with later edits kept).
    history.execute(
        Box::new(AddDynamicComponentCommand::new(entity, "EditorDynTestStat".to_string())),
        &mut world,
    );
    assert_eq!(
        world.get::<EditorDynTestStat>(entity).map(|s| s.power),
        Some(5.0),
        "default attached"
    );
    if let Some(stat) = world.get_mut::<EditorDynTestStat>(entity) {
        stat.power = 9.0; // user edit after add
    }
    history.undo(&mut world);
    assert!(world.get::<EditorDynTestStat>(entity).is_none());
    history.redo(&mut world);
    assert_eq!(
        world.get::<EditorDynTestStat>(entity).map(|s| s.power),
        Some(9.0),
        "redo restores the value captured at undo, not the default"
    );

    // Remove → undo restores the removed value.
    history.execute(
        Box::new(RemoveDynamicComponentCommand::new(entity, "EditorDynTestStat".to_string())),
        &mut world,
    );
    assert!(world.get::<EditorDynTestStat>(entity).is_none());
    history.undo(&mut world);
    assert_eq!(
        world.get::<EditorDynTestStat>(entity).map(|s| s.power),
        Some(9.0)
    );
}

#[test]
fn test_dynamic_names_reach_settable_and_from_json() {
    register_test_type();
    assert!(
        settable_component_names().iter().any(|n| n == "EditorDynTestStat"),
        "dynamic names are settable via the command API"
    );

    let stored = stored_component_from_json(
        "EditorDynTestStat",
        serde_json::json!({"power": 3.0, "label": "api"}),
    )
    .expect("valid dynamic value");
    assert_eq!(stored.type_name(), "EditorDynTestStat");

    // Malformed payloads are rejected with the type's own error.
    let err = stored_component_from_json(
        "EditorDynTestStat",
        serde_json::json!({"power": "NaN-ish string"}),
    )
    .expect_err("bad payload");
    assert!(err.contains("EditorDynTestStat"));

    // Unknown names list typed AND dynamic candidates.
    let err = stored_component_from_json("NoSuchThing", serde_json::Value::Null)
        .expect_err("unknown name");
    assert!(err.contains("unknown component"));
    assert!(err.contains("EditorDynTestStat"));
}
