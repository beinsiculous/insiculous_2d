//! Entity CRUD operations for the editor.
//!
//! Pure functions operating on `&mut World` + `&mut Selection` with no UI
//! dependency — fully testable headlessly.

use ecs::sprite_components::{Name, Sprite};
use ecs::hierarchy::GlobalTransform2D;
use ecs::ui_components::{UiButton, UiLabel, UiPanel};
use ecs::{EntityId, World, WorldHierarchyExt};
use editor::{capture_all_components, restore_components, Selection};
use glam::Vec2;
use physics::components::{Collider, RigidBody, RigidBodyType};

use crate::constants::DUPLICATE_OFFSET;

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

/// Delete all selected entities, reparenting their children.
///
/// For each deleted entity:
/// - Children are reparented to the deleted entity's parent (or made roots).
/// - The entity and all its components are removed.
/// - Selection is cleared afterward.
///
/// Used in tests; production code uses command system (`DeleteEntityCommand`).
#[cfg(test)]
pub fn delete_selected_entities(world: &mut World, selection: &mut Selection) {
    let selected: Vec<EntityId> = selection.selected().collect();
    if selected.is_empty() {
        return;
    }

    for &entity in &selected {
        // Get this entity's parent (before removing)
        let parent_id = world.get_parent(entity);

        // Reparent children to grandparent (or make roots)
        if let Some(children) = world.get_children(entity) {
            let child_ids: Vec<EntityId> = children.to_vec();
            for child in child_ids {
                if let Some(new_parent) = parent_id {
                    world.set_parent(child, new_parent).ok();
                } else {
                    world.remove_parent(child).ok();
                }
            }
        }

        // Remove hierarchy links then entity
        world.remove_parent(entity).ok();
        world.remove_entity(&entity).ok();
    }

    selection.clear();
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

/// Duplicate the primary selected entity (and its descendants).
///
/// The duplicate is offset by `(20, -20)` and gets " (Copy)" appended to its name.
/// Children are recursively duplicated with hierarchy preserved.
/// The new top-level entity is selected afterward.
pub fn duplicate_selected_entities(
    world: &mut World,
    selection: &mut Selection,
    counter: &mut u32,
) {
    let primary = match selection.primary() {
        Some(id) => id,
        None => return,
    };

    let parent_id = world.get_parent(primary);
    let new_entity = duplicate_entity_recursive(
        world,
        primary,
        parent_id,
        DUPLICATE_OFFSET,
        counter,
        true,
    );

    if let Some(entity) = new_entity {
        selection.select(entity);
    }
}

/// Recursively duplicate an entity and its children.
///
/// `offset` is applied only to the top-level entity (is_root=true).
fn duplicate_entity_recursive(
    world: &mut World,
    source: EntityId,
    new_parent: Option<EntityId>,
    offset: Vec2,
    counter: &mut u32,
    is_root: bool,
) -> Option<EntityId> {
    *counter += 1;
    let new_entity = world.create_entity();

    // Clone all registry-known component types (hierarchy links excluded —
    // hierarchy is rebuilt explicitly below).
    let stored = capture_all_components(world, source);
    restore_components(world, new_entity, &stored);

    // Offset top-level duplicate's position
    if is_root {
        if let Some(t) = world.get_mut::<common::Transform2D>(new_entity) {
            t.position += offset;
        }
    }

    // Append " (Copy)" to name
    if let Some(name) = world.get::<Name>(new_entity) {
        let new_name = format!("{} (Copy)", name.as_str());
        world.add_component(&new_entity, Name::new(new_name)).ok();
    }

    // Set parent for this duplicate
    if let Some(parent) = new_parent {
        world.set_parent(new_entity, parent).ok();
    }

    // Recurse for children
    if let Some(children) = world.get_children(source) {
        let child_ids: Vec<EntityId> = children.to_vec();
        for child in child_ids {
            duplicate_entity_recursive(
                world,
                child,
                Some(new_entity),
                Vec2::ZERO,
                counter,
                false,
            );
        }
    }

    Some(new_entity)
}

#[cfg(test)]
#[path = "entity_ops_tests.rs"]
mod tests_file;
