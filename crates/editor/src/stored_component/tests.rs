//! Tests for the editor component registry (`editor_component_registry!` output).

use super::*;
use glam::Vec2;

#[test]
fn test_edit_all_components_covers_present_components_and_advances_y() {
    let mut world = World::new();
    let entity = world.create_entity();
    world
        .add_component(&entity, common::Transform2D::new(Vec2::new(1.0, 2.0)))
        .unwrap();
    world.add_component(&entity, Sprite::new(0)).unwrap();
    world.add_component(&entity, EntityTag::new("player")).unwrap();

    let mut ui = UIContext::new();
    let mut history = CommandHistory::new();
    let inspect_style = InspectorStyle::default();
    let field_style = EditableFieldStyle::default();

    let start_y = 40.0;
    let mut drag_drop = crate::DragDropState::new();
    let mut extras = crate::InspectorExtras { drag_drop: &mut drag_drop, texture_display: None };
    let (y, count) = edit_all_components(
        &mut ui, &mut world, entity, &mut history,
        10.0, 400.0, start_y, &inspect_style, &field_style, 10.0, &mut extras,
    );

    assert_eq!(count, 3, "one block per present registry component");
    assert!(y > start_y, "rendering must advance the layout cursor");
    assert!(
        !history.can_undo(),
        "rendering without input must not record any edit"
    );
    // Registry order is builtin-then-removable, so absent components
    // (RigidBody etc.) contribute nothing.
    let bare = world.create_entity();
    let (_, none_count) = edit_all_components(
        &mut ui, &mut world, bare, &mut history,
        10.0, 400.0, start_y, &inspect_style, &field_style, 10.0, &mut extras,
    );
    assert_eq!(none_count, 0, "an entity with no components renders no blocks");
}

#[test]
fn test_capture_empty_entity() {
    let mut world = World::new();
    let entity = world.create_entity();
    let captured = capture_all_components(&world, entity);
    assert!(captured.is_empty());
}

#[test]
fn test_capture_and_restore_round_trip() {
    let mut world = World::new();
    let entity = world.create_entity();
    let pos = Vec2::new(42.0, 99.0);
    world.add_component(&entity, common::Transform2D::new(pos)).ok();
    world.add_component(&entity, GlobalTransform2D::default()).ok();
    world.add_component(&entity, Name::new("TestEntity")).ok();
    world.add_component(&entity, Sprite::new(5)).ok();
    world.add_component(&entity, RigidBody::default()).ok();

    let captured = capture_all_components(&world, entity);
    assert_eq!(captured.len(), 5);

    // Create a fresh entity and restore onto it
    let new_entity = world.create_entity();
    restore_components(&mut world, new_entity, &captured);

    let t = world.get::<common::Transform2D>(new_entity).unwrap();
    assert_eq!(t.position, pos);
    assert!(world.get::<Name>(new_entity).is_some());
    assert!(world.get::<Sprite>(new_entity).is_some());
    assert!(world.get::<RigidBody>(new_entity).is_some());
    assert!(world.get::<GlobalTransform2D>(new_entity).is_some());
}

#[test]
fn test_capture_includes_all_component_types() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::default()).ok();
    world.add_component(&entity, GlobalTransform2D::default()).ok();
    world.add_component(&entity, Name::new("All")).ok();
    world.add_component(&entity, common::Camera::default()).ok();
    world.add_component(&entity, Sprite::default()).ok();
    world.add_component(&entity, SpriteAnimation::default()).ok();
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

    let captured = capture_all_components(&world, entity);
    assert_eq!(captured.len(), 16);
}

#[test]
fn test_gameplay_components_registered_under_gameplay_category() {
    assert_eq!(ComponentKind::Behavior.category(), ComponentCategory::Gameplay);
    assert_eq!(ComponentKind::EntityTag.category(), ComponentCategory::Gameplay);

    let categories = categorized_components();
    let (_, gameplay_kinds) = categories
        .iter()
        .find(|(c, _)| *c == ComponentCategory::Gameplay)
        .expect("Gameplay category present");
    assert!(gameplay_kinds.contains(&ComponentKind::Behavior));
    assert!(gameplay_kinds.contains(&ComponentKind::EntityTag));
}

// ==================== ComponentKind dispatch ====================

#[test]
fn test_add_default_creates_each_component_kind() {
    let mut world = World::new();
    let entity = world.create_entity();

    for &kind in ComponentKind::ALL {
        kind.add_default(&mut world, entity);
        assert!(
            kind.is_present(&world, entity),
            "add_default did not add {:?}",
            kind
        );
    }
}

#[test]
fn test_remove_deletes_each_component_kind() {
    let mut world = World::new();
    let entity = world.create_entity();

    for &kind in ComponentKind::ALL {
        kind.add_default(&mut world, entity);
        kind.remove(&mut world, entity);
        assert!(
            !kind.is_present(&world, entity),
            "remove did not delete {:?}",
            kind
        );
    }
}

#[test]
fn test_remove_absent_component_is_safe() {
    let mut world = World::new();
    let entity = world.create_entity();
    // Should not panic
    ComponentKind::Sprite.remove(&mut world, entity);
    assert!(!ComponentKind::Sprite.is_present(&world, entity));
}

#[test]
fn test_capture_returns_value_when_present() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Sprite::new(7)).ok();

    let stored = ComponentKind::Sprite.capture(&world, entity);
    assert!(matches!(stored, Some(StoredComponent::Sprite(s)) if s.texture_handle == 7));
    assert!(ComponentKind::Camera.capture(&world, entity).is_none());
}

#[test]
fn test_display_names_match_variant_names() {
    assert_eq!(ComponentKind::Camera.display_name(), "Camera");
    assert_eq!(ComponentKind::SpriteAnimation.display_name(), "SpriteAnimation");
    for &kind in ComponentKind::ALL {
        assert!(!kind.display_name().is_empty());
    }
}

#[test]
fn test_available_components_filters_present() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Sprite::default()).ok();
    world.add_component(&entity, RigidBody::default()).ok();

    let available = available_components(&world, entity);
    assert!(!available.contains(&ComponentKind::Sprite));
    assert!(!available.contains(&ComponentKind::RigidBody));
    assert!(available.contains(&ComponentKind::Camera));
    assert!(available.contains(&ComponentKind::Collider));
    assert!(available.contains(&ComponentKind::AudioSource));
}

#[test]
fn test_categorized_components_covers_all_kinds() {
    let categories = categorized_components();
    let all: Vec<ComponentKind> = categories
        .iter()
        .flat_map(|(_, kinds)| kinds.iter().copied())
        .collect();
    assert_eq!(all.len(), ComponentKind::ALL.len());
    for &kind in ComponentKind::ALL {
        assert!(all.contains(&kind), "{:?} missing from categories", kind);
    }
}

#[test]
fn test_every_kind_has_consistent_category() {
    for &kind in ComponentKind::ALL {
        let category = kind.category();
        let categories = categorized_components();
        let (_, kinds) = categories
            .iter()
            .find(|(c, _)| *c == category)
            .expect("category present");
        assert!(kinds.contains(&kind));
    }
}

#[test]
fn test_registered_type_ids_match_world_enumeration() {
    use ecs::tilemap::Tilemap;

    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::default()).ok();
    world.add_component(&entity, GlobalTransform2D::default()).ok();
    world.add_component(&entity, Name::new("All")).ok();
    world.add_component(&entity, common::Camera::default()).ok();
    world.add_component(&entity, Sprite::default()).ok();
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

    // The world's type-erased view and the registry's known set must agree:
    // an entity carrying one of every registry type diffs to nothing, so the
    // snapshot's unknown-type detection never false-positives on registry
    // types (and this test breaks if the registry and the enumeration drift).
    let known: std::collections::HashSet<std::any::TypeId> =
        registered_component_type_ids().into_iter().collect();
    let unknown: Vec<&'static str> = world
        .component_types(entity)
        .into_iter()
        .filter(|(type_id, _)| !known.contains(type_id))
        .map(|(_, name)| name)
        .collect();
    assert!(unknown.is_empty(), "registry misses component types: {:?}", unknown);

    // And the captured set covers the same count, Tilemap included. (The
    // TYPED count — registered_component_type_ids also unions the dynamic
    // tier, whose global contents vary across tests in this process.)
    let captured = capture_all_components(&world, entity);
    assert_eq!(captured.len(), registered_typed_component_type_ids().len());
    assert_eq!(captured.len(), 17);

    // The command API's value capture walks the same registry, minus the
    // 2 hidden entries (GlobalTransform2D, BehaviorState) — a new registry
    // line is covered automatically or this count breaks.
    let values = capture_all_values(&world, entity);
    assert_eq!(values.len(), captured.len() - 2);
    let transform = values
        .iter()
        .find(|(name, _)| *name == "Transform2D")
        .expect("builtin Transform2D captured as a value");
    assert!(transform.1.get("position").is_some(), "serde fields come through");
    // Name is a registry component since #32 (editable), so it IS captured
    // here; the describe query filters it because the API surfaces the name
    // as the record's top-level field (covered in command_api tests).
    assert!(
        values.iter().any(|(name, _)| *name == "Name"),
        "editable Name is captured as a value"
    );
    assert!(
        !values.iter().any(|(name, _)| *name == "GlobalTransform2D"),
        "hidden entries are not emitted as components"
    );
}

#[test]
fn test_stored_component_from_json_round_trips_all_settable_types() {
    // The write path mirrors the read path for EVERY settable registry
    // entry: capture a live default → serde value → from_json → same type
    // name. A new registry line is covered automatically or this breaks.
    use ecs::tilemap::Tilemap;

    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::default()).ok();
    world.add_component(&entity, common::Camera::default()).ok();
    world.add_component(&entity, Sprite::default()).ok();
    world.add_component(&entity, SpriteAnimation::default()).ok();
    world.add_component(&entity, Tilemap::default()).ok();
    world.add_component(&entity, RigidBody::default()).ok();
    world.add_component(&entity, Collider::default()).ok();
    world.add_component(&entity, AudioSource::default()).ok();
    world.add_component(&entity, AudioListener::default()).ok();
    world.add_component(&entity, Behavior::default()).ok();
    world.add_component(&entity, EntityTag::default()).ok();
    world.add_component(&entity, UiLabel::default()).ok();
    world.add_component(&entity, UiPanel::default()).ok();
    world.add_component(&entity, UiButton::default()).ok();

    let values = capture_all_values(&world, entity);
    for name in settable_component_names() {
        // Dynamic-tier names (e.g. PlaySoundEffect) are settable but not on
        // this entity — the typed set is what this test attaches above.
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
