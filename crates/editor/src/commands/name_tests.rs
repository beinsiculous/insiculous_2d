//! Tests for the Name commands (#32): the inspector's `SetNameCommand` and
//! the hierarchy's `RenameEntityCommand` (which also covers entities that
//! had no `Name` yet).

use ecs::sprite_components::Name;
use ecs::World;

use super::{EditorCommand, RenameEntityCommand, SetNameCommand};

#[test]
fn test_set_name_command_round_trips_on_undo() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Name::new("Old")).ok();

    let mut cmd = SetNameCommand::new(entity, Name::new("Old"), Name::new("New"), "name");
    cmd.execute(&mut world);
    assert_eq!(world.get::<Name>(entity).unwrap().as_str(), "New");

    cmd.undo(&mut world);
    assert_eq!(world.get::<Name>(entity).unwrap().as_str(), "Old");
}

#[test]
fn test_set_name_command_is_a_noop_without_the_component() {
    // Documented contract (kimi F4): the macro-generated command writes
    // through get_mut and silently no-ops when Name is absent — assigning a
    // first Name is RenameEntityCommand's job.
    let mut world = World::new();
    let entity = world.create_entity();

    let mut cmd = SetNameCommand::new(entity, Name::new("Old"), Name::new("New"), "name");
    cmd.execute(&mut world);
    assert!(world.get::<Name>(entity).is_none(), "no component is created");
    cmd.undo(&mut world);
    assert!(world.get::<Name>(entity).is_none());
}

#[test]
fn test_rename_adds_name_and_undo_removes_the_component() {
    let mut world = World::new();
    let entity = world.create_entity();
    assert!(world.get::<Name>(entity).is_none());

    let mut cmd = RenameEntityCommand::new(&world, entity, Name::new("Fresh"));
    cmd.execute(&mut world);
    assert_eq!(world.get::<Name>(entity).unwrap().as_str(), "Fresh");

    // Undo restores the prior state: no Name component at all, so the
    // entity falls back to its synthesized display name.
    cmd.undo(&mut world);
    assert!(world.get::<Name>(entity).is_none());
}

#[test]
fn test_rename_replaces_and_undo_restores_the_old_name() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Name::new("Old")).ok();

    let mut cmd = RenameEntityCommand::new(&world, entity, Name::new("New"));
    cmd.execute(&mut world);
    assert_eq!(world.get::<Name>(entity).unwrap().as_str(), "New");

    cmd.undo(&mut world);
    assert_eq!(world.get::<Name>(entity).unwrap().as_str(), "Old");
}
