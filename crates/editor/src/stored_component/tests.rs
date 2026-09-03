//! The `editor_component_registry!` macro's outputs agree with each other
//! and with the world: one registry line buys capture, restore, the Add
//! Component popup and the inspector — and a forgotten type breaks here.

use std::collections::HashSet;

use ecs::tilemap::Tilemap;
use glam::Vec2;

use super::*;
use crate::test_support::extras;

/// The registry's hidden entries: captured for snapshots but never
/// surfaced as inspector blocks or API component values.
const HIDDEN_ENTRIES: usize = 2; // GlobalTransform2D, BehaviorState

/// An entity carrying one of every registry type.
fn entity_with_every_registry_type(world: &mut World) -> EntityId {
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::new(1.0, 2.0))).ok();
    world.add_component(&entity, GlobalTransform2D::default()).ok();
    world.add_component(&entity, Name::new("All")).ok();
    world.add_component(&entity, common::Camera::default()).ok();
    world.add_component(&entity, Sprite::new(7)).ok();
    world.add_component(&entity, SpriteAnimation::default()).ok();
    world.add_component(&entity, Tilemap::default()).ok();
    world.add_component(&entity, RigidBody::default()).ok();
    world.add_component(&entity, Collider::default()).ok();
    world.add_component(&entity, AudioSource::default()).ok();
    world.add_component(&entity, AudioListener::default()).ok();
    world.add_component(&entity, Behavior::default()).ok();
    world.add_component(&entity, BehaviorState::default()).ok();
    world.add_component(&entity, EntityTag::default()).ok();
    world.add_component(&entity, UiLabel::default()).ok();
    world.add_component(&entity, UiPanel::default()).ok();
    world.add_component(&entity, UiButton::default()).ok();
    world.add_component(&entity, ecs::script::Scripts::default()).ok();
    world.add_component(&entity, ecs::GridBackdrop::default()).ok();
    entity
}

#[test]
fn test_registry_and_world_type_enumeration_agree_over_every_registry_line() {
    let mut world = World::new();
    let entity = entity_with_every_registry_type(&mut world);

    // The world's type-erased view and the registry's known set agree: an
    // entity carrying one of every registry type diffs to nothing, so the
    // snapshot's unknown-type detection never false-positives on a registry
    // type — and a new ecs component missing its registry line breaks here.
    let known: HashSet<std::any::TypeId> = registered_component_type_ids().into_iter().collect();
    let unknown: Vec<&'static str> = world
        .component_types(entity)
        .into_iter()
        .filter(|(type_id, _)| !known.contains(type_id))
        .map(|(_, name)| name)
        .collect();
    assert_eq!(unknown, Vec::<&str>::new(), "registry misses component types");

    // Capture walks the same registry (the TYPED count — the dynamic tier's
    // global contents vary across tests in this process).
    let captured = capture_all_components(&world, entity);
    assert_eq!(captured.len(), registered_typed_component_type_ids().len());
    assert_eq!(captured.len(), world.component_types(entity).len());

    // The command API's value capture skips the hidden entries and carries
    // serde fields; editable Name IS captured (describe lifts it upstream).
    let values = capture_all_values(&world, entity);
    assert_eq!(values.len(), captured.len() - HIDDEN_ENTRIES);
    let names: Vec<&str> = values.iter().map(|(name, _)| name.as_ref()).collect();
    assert!(names.contains(&"Name"), "editable Name is captured as a value");
    assert!(!names.contains(&"GlobalTransform2D"), "hidden entries are not emitted");
    assert!(!names.contains(&"BehaviorState"), "hidden entries are not emitted");
    let (_, transform) = values.iter().find(|(name, _)| name == "Transform2D").expect("Transform2D");
    assert_eq!(transform["position"][0], 1.0, "serde fields come through");

    // Restoring the capture onto a fresh entity reproduces the values —
    // the delete-undo and clipboard path.
    let fresh = world.create_entity();
    restore_components(&mut world, fresh, &captured);
    assert_eq!(world.component_types(fresh).len(), captured.len());
    assert_eq!(world.get::<common::Transform2D>(fresh).expect("transform").position, Vec2::new(1.0, 2.0));
    assert_eq!(world.get::<Sprite>(fresh).expect("sprite").texture_handle, 7);
    assert_eq!(world.get::<Name>(fresh).map(Name::as_str), Some("All"));
}

#[test]
fn test_inspector_renders_one_block_per_present_component_and_records_no_edit() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::new(1.0, 2.0))).ok();
    world.add_component(&entity, Sprite::new(0)).ok();
    world.add_component(&entity, EntityTag::new("player")).ok();
    let bare = world.create_entity();
    let mut ui = UIContext::new();
    let mut history = CommandHistory::new();
    let inspect_style = InspectorStyle::default();
    let field_style = EditableFieldStyle::default();
    let mut drag_drop = crate::DragDropState::new();
    let mut extras = extras(&mut drag_drop);
    let start_y = 40.0;

    let (y, count) = edit_all_components(
        &mut ui, &mut world, entity, &mut history,
        10.0, 400.0, start_y, &inspect_style, &field_style, 10.0, &mut extras,
    );
    let (_, none_count) = edit_all_components(
        &mut ui, &mut world, bare, &mut history,
        10.0, 400.0, start_y, &inspect_style, &field_style, 10.0, &mut extras,
    );

    assert_eq!(count, 3, "one block per present registry component");
    assert!(y > start_y, "rendering advances the layout cursor");
    assert_eq!(none_count, 0, "an entity with no components renders no blocks");
    assert!(!history.can_undo(), "rendering without input records no edit");
}

#[test]
fn test_component_kind_dispatch_adds_captures_removes_and_lists_every_kind() {
    let mut world = World::new();
    let entity = world.create_entity();

    // The Add Component popup groups by category: every kind sits in
    // exactly one group, under the category it reports.
    let grouped: Vec<(ComponentCategory, ComponentKind)> = categorized_components()
        .into_iter()
        .flat_map(|(category, kinds)| kinds.into_iter().map(move |kind| (category, kind)))
        .collect();
    assert_eq!(grouped.len(), ComponentKind::ALL.len(), "each kind listed exactly once");
    for &kind in ComponentKind::ALL {
        assert!(grouped.contains(&(kind.category(), kind)), "{kind:?} under its own category");
    }

    // Choosing a kind adds its default, capture sees it, and the popup no
    // longer offers it; removing it puts it back on offer.
    for &kind in ComponentKind::ALL {
        kind.add_default(&mut world, entity);
        assert!(kind.is_present(&world, entity), "add_default did not add {kind:?}");
        assert!(kind.capture(&world, entity).is_some(), "capture misses {kind:?}");
        assert!(!available_components(&world, entity).contains(&kind), "{kind:?} still offered");
        kind.remove(&mut world, entity);
        assert!(!kind.is_present(&world, entity), "remove did not delete {kind:?}");
        assert!(kind.capture(&world, entity).is_none(), "capture of absent {kind:?}");
    }
    assert_eq!(available_components(&world, entity), ComponentKind::ALL.to_vec());
    assert_eq!(
        capture_all_components(&world, entity).len(),
        0,
        "a fully stripped entity captures nothing"
    );
}

#[test]
fn test_stored_component_from_json_round_trips_all_settable_types() {
    // The write path mirrors the read path for EVERY settable registry
    // entry: capture a live value → serde value → from_json → same type
    // name. A new registry line is covered automatically or this breaks.
    let mut world = World::new();
    let entity = entity_with_every_registry_type(&mut world);

    let values = capture_all_values(&world, entity);
    for name in settable_component_names() {
        // Dynamic-tier names (e.g. PlaySoundEffect) are settable but not on
        // this entity — the typed set is what the fixture attaches.
        let Some((_, value)) = values.iter().find(|(n, _)| *n == name) else {
            assert!(
                crate::stored_component::dynamic::is_dynamic_component(&name),
                "typed settable {name} missing from capture_all_values"
            );
            continue;
        };
        let stored = stored_component_from_json(&name, value.clone())
            .unwrap_or_else(|e| panic!("{name} round-trip failed: {e}"));
        assert_eq!(stored.type_name(), name);
        assert!(
            capture_component_by_name(&world, entity, &name)
                .expect("known name")
                .is_some(),
            "{name} capturable by name"
        );
    }
    assert!(
        !settable_component_names().iter().any(|n| n == "Name"),
        "Name is set through `rename`, never `set`"
    );
    assert!(stored_component_from_json("Bogus", serde_json::Value::Null).is_err());
}
