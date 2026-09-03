//! Entity CRUD operations for the editor.
//!
//! Pure functions operating on `&mut World` + `&mut Selection` with no UI
//! dependency — fully testable headlessly.

use ecs::sprite_components::{Name, Sprite};
use ecs::hierarchy::GlobalTransform2D;
use ecs::ui_components::{UiButton, UiLabel, UiPanel};
use ecs::{EntityId, World, WorldHierarchyExt};
use editor::Selection;
use glam::Vec2;
use physics::components::{Collider, RigidBody, RigidBodyType};

// Component add/remove and the add-component popup are driven by
// `editor::ComponentKind` — the registry in editor/src/stored_component.rs
// is the single source of truth for editor-visible component types.

/// Create a base entity with Transform2D, GlobalTransform2D, and Name, then select it.
fn create_base_entity(
    world: &mut World,
    selection: &mut Selection,
    position: Vec2,
    label: &str,
    counter: &mut u32,
) -> EntityId {
    *counter += 1;
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(position)).ok();
    world.add_component(&entity, GlobalTransform2D::default()).ok();
    world.add_component(&entity, Name::new(format!("{} {}", label, counter))).ok();
    selection.select(entity);
    entity
}

/// Create an empty entity with Transform2D, GlobalTransform2D, and Name.
pub fn create_empty_entity(
    world: &mut World,
    selection: &mut Selection,
    position: Vec2,
    counter: &mut u32,
) -> EntityId {
    create_base_entity(world, selection, position, "Entity", counter)
}

/// Create a sprite entity (empty + Sprite).
pub fn create_sprite_entity(
    world: &mut World,
    selection: &mut Selection,
    position: Vec2,
    counter: &mut u32,
) -> EntityId {
    let entity = create_base_entity(world, selection, position, "Sprite", counter);
    world.add_component(&entity, Sprite::new(0)).ok();
    entity
}

/// Create a camera entity (empty + Camera).
pub fn create_camera_entity(
    world: &mut World,
    selection: &mut Selection,
    position: Vec2,
    counter: &mut u32,
) -> EntityId {
    let entity = create_base_entity(world, selection, position, "Camera", counter);
    world.add_component(&entity, common::Camera::default()).ok();
    entity
}

/// Create a physics body entity (empty + Sprite + RigidBody + Collider).
pub fn create_physics_body(
    world: &mut World,
    selection: &mut Selection,
    position: Vec2,
    body_type: RigidBodyType,
    counter: &mut u32,
) -> EntityId {
    let type_label = match body_type {
        RigidBodyType::Static => "StaticBody",
        RigidBodyType::Dynamic => "DynamicBody",
        RigidBodyType::Kinematic => "KinematicBody",
    };
    let entity = create_base_entity(world, selection, position, type_label, counter);
    world.add_component(&entity, Sprite::new(0)).ok();
    world.add_component(&entity, RigidBody::default().with_body_type(body_type)).ok();
    world.add_component(&entity, Collider::default()).ok();
    entity
}

/// Create a UI entity: Name only — screen-space elements place themselves
/// via anchor + offset, so no Transform2D (a transform would suggest the
/// gizmo/world position matters, and it doesn't).
fn create_ui_entity<T: ecs::Component>(
    world: &mut World,
    selection: &mut Selection,
    label: &str,
    component: T,
    counter: &mut u32,
) -> EntityId {
    *counter += 1;
    let entity = world.create_entity();
    world.add_component(&entity, Name::new(format!("{} {}", label, counter))).ok();
    world.add_component(&entity, component).ok();
    selection.select(entity);
    entity
}

/// Create a UI label entity (Name + UiLabel, no Transform2D).
pub fn create_ui_label(
    world: &mut World,
    selection: &mut Selection,
    counter: &mut u32,
) -> EntityId {
    let label = UiLabel { text: "New Label".to_string(), ..Default::default() };
    create_ui_entity(world, selection, "UiLabel", label, counter)
}

/// Create a UI panel entity (Name + UiPanel, no Transform2D).
pub fn create_ui_panel(
    world: &mut World,
    selection: &mut Selection,
    counter: &mut u32,
) -> EntityId {
    create_ui_entity(world, selection, "UiPanel", UiPanel::default(), counter)
}

/// Create a UI button entity (Name + UiButton, no Transform2D).
pub fn create_ui_button(
    world: &mut World,
    selection: &mut Selection,
    counter: &mut u32,
) -> EntityId {
    let button = UiButton { text: "Button".to_string(), id: "button".to_string(), ..Default::default() };
    create_ui_entity(world, selection, "UiButton", button, counter)
}

/// Dispatch a menu action string to the appropriate create function.
///
/// Returns `Some(entity_id)` if an entity was created, `None` if the action
/// is not recognized as a create action.
pub fn handle_create_action(
    action: &str,
    world: &mut World,
    selection: &mut Selection,
    position: Vec2,
    counter: &mut u32,
) -> Option<EntityId> {
    match action {
        "Create Empty" => Some(create_empty_entity(world, selection, position, counter)),
        "Create Sprite" => Some(create_sprite_entity(world, selection, position, counter)),
        "Create Camera" => Some(create_camera_entity(world, selection, position, counter)),
        "Create Static Body" => Some(create_physics_body(world, selection, position, RigidBodyType::Static, counter)),
        "Create Dynamic Body" => Some(create_physics_body(world, selection, position, RigidBodyType::Dynamic, counter)),
        "Create Kinematic Body" => Some(create_physics_body(world, selection, position, RigidBodyType::Kinematic, counter)),
        "Create UI Label" => Some(create_ui_label(world, selection, counter)),
        "Create UI Panel" => Some(create_ui_panel(world, selection, counter)),
        "Create UI Button" => Some(create_ui_button(world, selection, counter)),
        _ => None,
    }
}

/// Assign a texture handle to an entity's Sprite, recording an undo entry.
/// Returns false (and records nothing) when the entity has no Sprite or the
/// texture is unchanged. Used by asset-browser click-to-assign and drops.
pub fn assign_sprite_texture(
    world: &mut World,
    entity: EntityId,
    texture_handle: u32,
    history: &mut editor::CommandHistory,
) -> bool {
    let Some(old) = world.get::<Sprite>(entity).cloned() else {
        return false;
    };
    if old.texture_handle == texture_handle {
        return false;
    }
    let mut new = old.clone();
    new.texture_handle = texture_handle;
    if let Some(sprite) = world.get_mut::<Sprite>(entity) {
        sprite.texture_handle = texture_handle;
    }
    // Discrete assignments are discrete undo entries — no merging.
    history.push_already_executed(Box::new(editor::commands::SetSpriteCommand::new(
        entity, old, new, "texture_drop",
    )));
    true
}

/// Create a sprite entity showing `texture_handle` at `position`, named from
/// the asset's file stem, and record its creation for undo. Used when a
/// texture is dropped on empty viewport space.
pub fn create_sprite_entity_with_texture(
    world: &mut World,
    selection: &mut Selection,
    position: Vec2,
    texture_handle: u32,
    name_stem: &str,
    counter: &mut u32,
    history: &mut editor::CommandHistory,
) -> EntityId {
    let entity = create_base_entity(world, selection, position, name_stem, counter);
    world.add_component(&entity, Sprite::new(texture_handle)).ok();
    history.push_already_executed(Box::new(
        editor::commands::CreateEntityCommand::already_created(world, entity),
    ));
    entity
}

/// Every entity Ctrl+A selects — today all world entities, matching what
/// the hierarchy shows. This helper is the single place a future
/// editor-only-entity filter would go.
pub fn selectable_entities(world: &World) -> Vec<EntityId> {
    world.entities()
}

/// Selected entities with no selected ancestor — the set a multi-entity
/// drag or nudge operates on. Moving a parent already moves its children
/// through hierarchy propagation, so operating on a selected child of a
/// selected parent would double-move it.
///
/// Ordering contract: the current primary comes FIRST when it is a root
/// (it anchors grid snapping, so the anchor must be deterministic); the
/// remaining roots keep selection insertion order.
pub fn selection_roots(world: &World, selection: &Selection) -> Vec<EntityId> {
    let has_selected_ancestor = |entity: EntityId| {
        let mut current = world.get_parent(entity);
        while let Some(parent) = current {
            if selection.contains(parent) {
                return true;
            }
            current = world.get_parent(parent);
        }
        false
    };

    let mut roots: Vec<EntityId> = selection
        .selected()
        .filter(|&entity| !has_selected_ancestor(entity))
        .collect();
    if let Some(primary) = selection.primary() {
        if let Some(index) = roots.iter().position(|&e| e == primary) {
            let primary = roots.remove(index);
            roots.insert(0, primary);
        }
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor::CommandHistory;

    #[test]
    fn test_world_factories_place_name_and_select_the_new_entity() {
        // Every archetype the Create menu and the command API spawn, keyed
        // by the menu label they dispatch on.
        let (mut world, mut selection, mut counter) = (World::new(), Selection::new(), 0);
        let position = Vec2::new(100.0, -50.0);
        let expectations: [(&str, bool, bool, Option<RigidBodyType>); 6] = [
            ("Create Empty", false, false, None),
            ("Create Sprite", true, false, None),
            ("Create Camera", false, true, None),
            ("Create Static Body", true, false, Some(RigidBodyType::Static)),
            ("Create Dynamic Body", true, false, Some(RigidBodyType::Dynamic)),
            ("Create Kinematic Body", true, false, Some(RigidBodyType::Kinematic)),
        ];
        let mut names = std::collections::HashSet::new();

        for (action, has_sprite, has_camera, body_type) in expectations {
            let entity = handle_create_action(action, &mut world, &mut selection, position, &mut counter)
                .unwrap_or_else(|| panic!("{action} is a create action"));

            assert_eq!(selection.primary(), Some(entity), "{action}: the new entity is selected");
            assert_eq!(
                world.get::<common::Transform2D>(entity).map(|t| t.position),
                Some(position),
                "{action}: spawned where asked"
            );
            assert!(world.get::<GlobalTransform2D>(entity).is_some(), "{action}: pickable from frame one");
            assert_eq!(world.get::<Sprite>(entity).is_some(), has_sprite, "{action}: Sprite");
            assert_eq!(world.get::<common::Camera>(entity).is_some(), has_camera, "{action}: Camera");
            assert_eq!(world.get::<RigidBody>(entity).map(|b| b.body_type), body_type, "{action}: body");
            assert_eq!(world.get::<Collider>(entity).is_some(), body_type.is_some(), "{action}: Collider");
            let name = world.get::<Name>(entity).map(|n| n.as_str().to_string()).expect("auto-named");
            assert!(names.insert(name.clone()), "auto-names are unique: {name}");
        }
        assert_eq!(
            handle_create_action("Create Nonsense", &mut world, &mut selection, position, &mut counter),
            None,
            "an unknown label creates nothing"
        );
    }

    #[test]
    fn test_ui_factories_give_a_name_but_no_world_transform() {
        // Screen-space elements place themselves by anchor + offset; a
        // Transform2D would suggest the gizmo and world position matter.
        let (mut world, mut selection, mut counter) = (World::new(), Selection::new(), 0);

        let label = create_ui_label(&mut world, &mut selection, &mut counter);
        let panel = create_ui_panel(&mut world, &mut selection, &mut counter);
        let button = create_ui_button(&mut world, &mut selection, &mut counter);

        assert!(world.get::<UiLabel>(label).is_some());
        assert!(world.get::<UiPanel>(panel).is_some());
        assert!(world.get::<UiButton>(button).is_some());
        for entity in [label, panel, button] {
            assert!(world.get::<Name>(entity).is_some(), "UI entities are named");
            assert!(world.get::<common::Transform2D>(entity).is_none(), "no world transform");
            assert!(world.get::<GlobalTransform2D>(entity).is_none(), "never pickable in the viewport");
        }
        assert_eq!(selection.primary(), Some(button), "the last created is selected");
    }

    #[test]
    fn test_select_all_reaches_every_entity_sprite_or_not() {
        // Ctrl+A matches what the hierarchy shows: a camera-like entity
        // without a sprite is still selectable.
        let (mut world, mut selection, mut counter) = (World::new(), Selection::new(), 0);
        let sprite = create_sprite_entity(&mut world, &mut selection, Vec2::ZERO, &mut counter);
        let camera = create_camera_entity(&mut world, &mut selection, Vec2::ZERO, &mut counter);
        let bare = world.create_entity();

        let all = selectable_entities(&world);

        assert_eq!(all.len(), 3);
        for entity in [sprite, camera, bare] {
            assert!(all.contains(&entity), "{entity:?} is selectable");
        }
    }

    #[test]
    fn test_texture_assignment_records_one_undo_entry_and_skips_no_ops() {
        let (mut world, mut selection, mut counter) = (World::new(), Selection::new(), 0);
        let sprite = create_sprite_entity(&mut world, &mut selection, Vec2::ZERO, &mut counter);
        let bare = create_empty_entity(&mut world, &mut selection, Vec2::ZERO, &mut counter);
        let mut history = CommandHistory::new();

        assert!(assign_sprite_texture(&mut world, sprite, 7, &mut history));
        assert_eq!(world.get::<Sprite>(sprite).map(|s| s.texture_handle), Some(7));
        assert!(history.undo(&mut world));
        assert_eq!(world.get::<Sprite>(sprite).map(|s| s.texture_handle), Some(0), "undo restores the old texture");

        assert!(!assign_sprite_texture(&mut world, sprite, 0, &mut history), "same handle is a no-op");
        assert!(!assign_sprite_texture(&mut world, bare, 7, &mut history), "no Sprite is a no-op");
        assert!(!history.can_undo(), "no-ops record nothing");
    }

    #[test]
    fn test_texture_dropped_on_empty_space_spawns_an_undoable_named_sprite() {
        let (mut world, mut selection, mut counter) = (World::new(), Selection::new(), 0);
        let mut history = CommandHistory::new();

        let entity = create_sprite_entity_with_texture(
            &mut world, &mut selection, Vec2::new(50.0, -20.0), 9, "crate", &mut counter, &mut history,
        );

        assert_eq!(world.get::<Sprite>(entity).map(|s| s.texture_handle), Some(9));
        assert_eq!(world.get::<common::Transform2D>(entity).map(|t| t.position), Some(Vec2::new(50.0, -20.0)));
        assert!(
            world.get::<Name>(entity).is_some_and(|n| n.as_str().starts_with("crate")),
            "named after the asset's file stem"
        );
        assert_eq!(selection.primary(), Some(entity));
        assert!(history.undo(&mut world));
        assert!(world.get::<Sprite>(entity).is_none(), "undo deletes the spawned entity");
    }
}
