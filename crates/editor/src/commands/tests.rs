//! `CommandHistory` and the entity/component commands: every kept test
//! locks a contract an editor user would notice — undo restores what was
//! there, redo brings back the SAME entity ids, continuous edits collapse
//! into one entry and unrelated edits never do.

use super::*;
use common::Transform2D;
use ecs::sprite_components::{Name, Sprite};
use glam::Vec2;
use physics::components::{Collider, RigidBody};

use crate::test_support::setup_entity;

fn position(world: &World, entity: EntityId) -> Vec2 {
    world.get::<Transform2D>(entity).map(|t| t.position).expect("entity has a Transform2D")
}

fn move_to(entity: EntityId, from: Vec2, to: Vec2, hint: &'static str) -> Box<dyn EditorCommand> {
    Box::new(SetTransformCommand::new(entity, Transform2D::new(from), Transform2D::new(to), hint))
}

fn undo_count(history: &mut CommandHistory, world: &mut World) -> usize {
    let mut count = 0;
    while history.undo(world) {
        count += 1;
    }
    count
}

// ---------------------------------------------------------------------------
// History state machine
// ---------------------------------------------------------------------------

#[test]
fn test_new_command_invalidates_redo_and_drives_the_undo_redo_state() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();
    assert_eq!((history.can_undo(), history.can_redo()), (false, false));
    assert_eq!((history.undo_name(), history.redo_name()), (None, None));

    history.execute(move_to(entity, Vec2::ZERO, Vec2::new(10.0, 20.0), "position"), &mut world);
    assert_eq!(position(&world, entity), Vec2::new(10.0, 20.0));
    assert_eq!((history.can_undo(), history.can_redo()), (true, false));
    assert_eq!(history.undo_name(), Some("Set Transform"));

    assert!(history.undo(&mut world));
    assert_eq!(position(&world, entity), Vec2::ZERO, "undo restores the old value");
    assert_eq!((history.can_undo(), history.can_redo()), (false, true));
    assert_eq!(history.redo_name(), Some("Set Transform"));

    assert!(history.redo(&mut world));
    assert_eq!(position(&world, entity), Vec2::new(10.0, 20.0), "redo re-applies the new value");
    assert!(history.undo(&mut world));

    history.execute(move_to(entity, Vec2::ZERO, Vec2::new(2.0, 0.0), "position"), &mut world);
    assert!(!history.can_redo(), "a new command after undo invalidates the redo branch");
    assert!(!history.redo(&mut world), "nothing to redo");
    assert_eq!(position(&world, entity), Vec2::new(2.0, 0.0));
}

#[test]
fn test_history_cap_drops_the_oldest_entry_and_undo_runs_newest_first() {
    // Command i (1..=102) moves position from i-1 to i. With the cap at
    // 100, commands 1 and 2 are evicted from the FRONT: undoing the whole
    // stack lands on the oldest survivor's `old` (2), never on 0.
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();
    for i in 1..=102 {
        let from = Vec2::new((i - 1) as f32, 0.0);
        let to = Vec2::new(i as f32, 0.0);
        history.execute(move_to(entity, from, to, "position"), &mut world);
    }
    assert_eq!(position(&world, entity), Vec2::new(102.0, 0.0));

    assert!(history.undo(&mut world));
    assert_eq!(position(&world, entity), Vec2::new(101.0, 0.0), "undo is LIFO");

    let remaining = undo_count(&mut history, &mut world);
    assert_eq!(remaining + 1, 100, "at most 100 entries survive");
    assert_eq!(position(&world, entity), Vec2::new(2.0, 0.0), "the oldest two were dropped");
}

// ---------------------------------------------------------------------------
// Entity commands — ids stay stable across undo/redo
// ---------------------------------------------------------------------------

#[test]
fn test_delete_undo_restores_every_captured_component_under_the_same_id() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    world.add_component(&entity, Sprite::new(7)).ok();
    world.add_component(&entity, RigidBody::default()).ok();
    world.add_component(&entity, Collider::default()).ok();
    let mut history = CommandHistory::new();

    history.execute(Box::new(DeleteEntityCommand::new(entity)), &mut world);
    assert_eq!(world.entity_count(), 0);

    assert!(history.undo(&mut world));
    assert_eq!(world.entity_count(), 1);
    assert_eq!(
        world.get::<Name>(entity).map(Name::as_str),
        Some("Test"),
        "undo resurrects the entity under its ORIGINAL id, so a Selection holding it stays valid"
    );
    assert_eq!(world.get::<Sprite>(entity).map(|s| s.texture_handle), Some(7));
    assert!(world.get::<Transform2D>(entity).is_some());
    assert!(world.get::<RigidBody>(entity).is_some());
    assert!(world.get::<Collider>(entity).is_some());

    assert!(history.redo(&mut world));
    assert_eq!(world.entity_count(), 0, "redo deletes again");
}

#[test]
fn test_create_undo_removes_and_redo_recreates_the_same_id_with_its_data() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let placed = Vec2::new(42.0, 99.0);
    world.get_mut::<Transform2D>(entity).expect("transform").position = placed;
    let mut history = CommandHistory::new();
    history.push_already_executed(Box::new(CreateEntityCommand::already_created(&world, entity)));

    assert!(history.undo(&mut world));
    assert_eq!(world.entity_count(), 0, "undo of create removes the entity");

    assert!(history.redo(&mut world));
    assert_eq!(world.entity_count(), 1);
    assert_eq!(position(&world, entity), placed, "redo recreates it under the same id with its data");
}

#[test]
fn test_set_command_survives_delete_undo_cycle() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();
    history.execute(move_to(entity, Vec2::ZERO, Vec2::new(50.0, 0.0), "position"), &mut world);
    history.execute(Box::new(DeleteEntityCommand::new(entity)), &mut world);

    assert!(history.undo(&mut world), "un-delete (same id)");
    assert!(history.undo(&mut world), "un-edit");
    assert_eq!(
        position(&world, entity),
        Vec2::ZERO,
        "the earlier Set command must still resolve after a delete/undo cycle"
    );

    assert!(history.redo(&mut world));
    assert_eq!(position(&world, entity), Vec2::new(50.0, 0.0));
}

#[test]
fn test_macro_command_undoes_every_member_as_one_entry() {
    let mut world = World::new();
    let first = setup_entity(&mut world);
    let second = setup_entity(&mut world);
    let mut history = CommandHistory::new();

    history.execute(
        Box::new(MacroCommand::new(
            "Move Two",
            vec![
                move_to(first, Vec2::ZERO, Vec2::new(10.0, 0.0), "position"),
                move_to(second, Vec2::ZERO, Vec2::new(0.0, 10.0), "position"),
            ],
        )),
        &mut world,
    );
    assert_eq!(position(&world, first), Vec2::new(10.0, 0.0));
    assert_eq!(position(&world, second), Vec2::new(0.0, 10.0));

    assert_eq!(undo_count(&mut history, &mut world), 1, "a macro is one undo entry");
    assert_eq!(position(&world, first), Vec2::ZERO);
    assert_eq!(position(&world, second), Vec2::ZERO);
}

// ---------------------------------------------------------------------------
// Component commands
// ---------------------------------------------------------------------------

#[test]
fn test_add_and_remove_component_undo_restore_the_previous_value() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();

    history.execute(Box::new(AddComponentCommand::new(entity, ComponentKind::Sprite)), &mut world);
    assert!(world.get::<Sprite>(entity).is_some(), "add attaches a default Sprite");
    assert!(history.undo(&mut world));
    assert!(world.get::<Sprite>(entity).is_none(), "undo of add removes it again");

    world.add_component(&entity, Sprite::new(3)).ok();
    history.execute(Box::new(RemoveComponentCommand::new(entity, ComponentKind::Sprite)), &mut world);
    assert!(world.get::<Sprite>(entity).is_none());
    assert!(history.undo(&mut world));
    assert_eq!(
        world.get::<Sprite>(entity).map(|s| s.texture_handle),
        Some(3),
        "undo of remove restores the component's VALUE, not a default"
    );
}

#[test]
fn test_removing_a_rigid_body_cascades_to_its_collider_but_not_the_reverse() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    world.add_component(&entity, RigidBody::default()).ok();
    world.add_component(&entity, Collider::default()).ok();
    let mut history = CommandHistory::new();

    history.execute(Box::new(RemoveComponentCommand::new(entity, ComponentKind::RigidBody)), &mut world);
    assert!(world.get::<RigidBody>(entity).is_none());
    assert!(world.get::<Collider>(entity).is_none(), "a collider cannot outlive its body");

    assert!(history.undo(&mut world));
    assert!(world.get::<RigidBody>(entity).is_some());
    assert!(world.get::<Collider>(entity).is_some(), "undo restores the cascaded collider too");

    history.execute(Box::new(RemoveComponentCommand::new(entity, ComponentKind::Collider)), &mut world);
    assert!(world.get::<Collider>(entity).is_none());
    assert!(world.get::<RigidBody>(entity).is_some(), "removing the collider keeps the body");
}

// ---------------------------------------------------------------------------
// Merging: one gesture = one entry, unrelated edits never collapse
// ---------------------------------------------------------------------------

/// The inspector-writeback path: the value is already on the world when the
/// command is recorded (`apply_component_edit`), so the history only merges
/// or pushes — it never executes.
fn write_position(world: &mut World, entity: EntityId, to: Vec2) {
    world.get_mut::<Transform2D>(entity).expect("transform").position = to;
}

#[test]
fn test_same_field_edits_and_gizmo_drags_merge_into_one_undo_entry() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();

    write_position(&mut world, entity, Vec2::new(1.0, 0.0));
    history.try_merge_or_push(move_to(entity, Vec2::ZERO, Vec2::new(1.0, 0.0), "position"));
    write_position(&mut world, entity, Vec2::new(2.0, 0.0));
    history.try_merge_or_push(move_to(entity, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0), "position"));
    assert_eq!(undo_count(&mut history, &mut world), 1, "same-field frames are ONE entry");
    assert_eq!(position(&world, entity), Vec2::ZERO, "undo returns to the FIRST before-image");
    assert!(history.redo(&mut world));
    assert_eq!(position(&world, entity), Vec2::new(2.0, 0.0), "redo re-applies the LAST frame's value");
    assert!(history.undo(&mut world));

    let start = Transform2D::new(Vec2::ZERO);
    let mid = Transform2D::new(Vec2::new(50.0, 50.0));
    let end = Transform2D::new(Vec2::new(100.0, 100.0));
    write_position(&mut world, entity, mid.position);
    history.try_merge_or_push(Box::new(TransformGizmoCommand::new(entity, start, mid)));
    write_position(&mut world, entity, end.position);
    history.try_merge_or_push(Box::new(TransformGizmoCommand::new(entity, mid, end)));
    assert_eq!(undo_count(&mut history, &mut world), 1, "gizmo drag frames on one entity are ONE entry");
    assert_eq!(position(&world, entity), Vec2::ZERO);
}

#[test]
fn test_edits_on_different_entities_never_merge_even_with_the_same_field_hint() {
    let mut world = World::new();
    let first = setup_entity(&mut world);
    let second = setup_entity(&mut world);
    let mut history = CommandHistory::new();

    history.try_merge_or_push(move_to(first, Vec2::ZERO, Vec2::new(1.0, 0.0), "position"));
    history.try_merge_or_push(move_to(second, Vec2::ZERO, Vec2::new(1.0, 0.0), "position"));

    assert_eq!(
        undo_count(&mut history, &mut world),
        2,
        "an edit to entity B must not fold into the pending edit on entity A"
    );
}

#[test]
fn test_a_sprite_edit_never_merges_into_a_transform_edit_on_the_same_entity() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    world.add_component(&entity, Sprite::new(1)).ok();
    let mut history = CommandHistory::new();

    history.try_merge_or_push(move_to(entity, Vec2::ZERO, Vec2::new(1.0, 0.0), "position"));
    history.try_merge_or_push(Box::new(SetSpriteCommand::new(entity, Sprite::new(1), Sprite::new(2), "position")));

    assert_eq!(
        undo_count(&mut history, &mut world),
        2,
        "commands of different component types never merge, whatever the hint"
    );
}

#[test]
fn test_mismatched_field_hints_yield_one_entry_per_gesture_frame() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();

    history.try_merge_or_push(move_to(entity, Vec2::ZERO, Vec2::new(1.0, 0.0), "position"));
    history.try_merge_or_push(move_to(entity, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0), "rotation"));
    history.try_merge_or_push(move_to(entity, Vec2::new(2.0, 0.0), Vec2::new(3.0, 0.0), "scale"));

    assert_eq!(
        undo_count(&mut history, &mut world),
        3,
        "a wrong field_hint floods the history with one entry per frame — the symptom to look for"
    );
}

// ---------------------------------------------------------------------------
// Name commands
// ---------------------------------------------------------------------------

#[test]
fn test_rename_adds_a_name_and_undo_removes_the_component() {
    let mut world = World::new();
    let entity = world.create_entity();

    let mut rename = RenameEntityCommand::new(&world, entity, Name::new("Fresh"));
    rename.execute(&mut world);
    assert_eq!(world.get::<Name>(entity).map(Name::as_str), Some("Fresh"));

    rename.undo(&mut world);
    assert!(
        world.get::<Name>(entity).is_none(),
        "undo restores NO Name at all, so the hierarchy falls back to the synthesized display name"
    );

    // Set Name is the inspector's edit of an EXISTING Name: it round-trips
    // the old value on undo but never creates the component.
    let mut set_on_nameless = SetNameCommand::new(entity, Name::new("Old"), Name::new("New"), "name");
    set_on_nameless.execute(&mut world);
    assert!(world.get::<Name>(entity).is_none(), "SetName on a nameless entity creates nothing");

    world.add_component(&entity, Name::new("Old")).ok();
    let mut set = SetNameCommand::new(entity, Name::new("Old"), Name::new("New"), "name");
    set.execute(&mut world);
    assert_eq!(world.get::<Name>(entity).map(Name::as_str), Some("New"));
    set.undo(&mut world);
    assert_eq!(world.get::<Name>(entity).map(Name::as_str), Some("Old"));
}
