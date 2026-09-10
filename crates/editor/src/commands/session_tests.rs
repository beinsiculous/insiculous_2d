//! The Play boundary: what a play session's history may undo, what Stop's
//! Discard removes, and what Keep replays onto the restored world.

use common::Transform2D;
use ecs::sprite_components::Name;
use ecs::{EntityId, World, WorldHierarchyExt};
use glam::Vec2;

use crate::clipboard::DeleteTreeCommand;

use super::{
    CommandHistory, CreateEntityCommand, EditorCommand, MacroCommand, NudgeCommand,
    SetNameCommand, SetTransformCommand,
};

/// An entity with a `Transform2D` at `position`.
fn spawn_at(world: &mut World, position: Vec2) -> EntityId {
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(position)).ok();
    entity
}

fn transform(world: &World, entity: EntityId) -> Transform2D {
    world.get::<Transform2D>(entity).cloned().expect("entity has a transform")
}

/// A whole-component write, the shape the inspector records.
fn set_transform(entity: EntityId, old: Transform2D, new: Transform2D) -> Box<dyn EditorCommand> {
    Box::new(SetTransformCommand::new(entity, old, new, "position"))
}

#[test]
fn test_session_floor_blocks_undo_below_the_play_boundary_and_redo_of_pre_session_entries() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut history = CommandHistory::new();

    history.execute(
        set_transform(entity, Transform2D::new(Vec2::ZERO), Transform2D::new(Vec2::new(10.0, 0.0))),
        &mut world,
    );
    history.undo(&mut world);
    assert!(history.can_redo(), "the pre-session entry is redoable before Play");
    history.redo(&mut world);

    history.begin_session();
    assert!(!history.can_undo(), "the authored history is behind the boundary");
    assert!(!history.can_redo(), "and so is anything undone before it");
    assert_eq!(history.undo_name(), None);
    assert!(!history.undo(&mut world), "undo refuses at the boundary");

    history.execute(
        set_transform(
            entity,
            Transform2D::new(Vec2::new(10.0, 0.0)),
            Transform2D::new(Vec2::new(20.0, 0.0)),
        ),
        &mut world,
    );
    assert!(history.can_undo(), "the session's own entry is undoable");
    assert!(history.undo(&mut world));
    assert!(!history.can_undo(), "back at the boundary");
    assert_eq!(transform(&world, entity).position, Vec2::new(10.0, 0.0));
}

#[test]
fn test_a_merge_right_after_the_boundary_starts_a_fresh_entry() {
    // Merging into the pre-session top would reassign that entry's id and
    // drag an authored entry above the session floor.
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut history = CommandHistory::new();

    history.execute(
        set_transform(entity, Transform2D::new(Vec2::ZERO), Transform2D::new(Vec2::new(1.0, 0.0))),
        &mut world,
    );
    history.begin_session();
    history.try_merge_or_push(set_transform(
        entity,
        Transform2D::new(Vec2::new(1.0, 0.0)),
        Transform2D::new(Vec2::new(2.0, 0.0)),
    ));
    assert_eq!(history.session_entry_count(), 1, "the paused edit is its own entry");
}

#[test]
fn test_drop_session_entries_removes_only_the_entries_recorded_after_the_floor() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut history = CommandHistory::new();

    history.execute(
        set_transform(entity, Transform2D::new(Vec2::ZERO), Transform2D::new(Vec2::new(5.0, 0.0))),
        &mut world,
    );
    let dirty_before = history.is_dirty();
    history.begin_session();
    for step in 1..=3 {
        let from = Vec2::new(5.0 * step as f32, 0.0);
        let to = Vec2::new(5.0 * (step + 1) as f32, 0.0);
        history.execute(set_transform(entity, Transform2D::new(from), Transform2D::new(to)), &mut world);
    }
    // One session entry parked on the REDO stack must go too.
    history.undo(&mut world);

    assert_eq!(history.drop_session_entries(), 2);
    assert_eq!(history.session_entry_count(), 0, "nothing recorded since the boundary remains");
    history.end_session();
    assert!(!history.can_redo(), "no session entry survives on the redo stack");
    assert!(history.can_undo(), "the authored entry is reachable again");
    assert_eq!(history.is_dirty(), dirty_before, "the dirty watermark reads as before the session");
}

#[test]
fn test_rebase_session_entries_replays_creates_and_edits_onto_the_replaced_world_with_authored_before_images(
) {
    let mut world = World::new();
    let authored = spawn_at(&mut world, Vec2::ZERO);
    let mut history = CommandHistory::new();
    history.execute(
        set_transform(authored, Transform2D::new(Vec2::ZERO), Transform2D::new(Vec2::new(3.0, 0.0))),
        &mut world,
    );

    history.begin_session();
    // The simulation moved the authored entity and the user created one.
    world.get_mut::<Transform2D>(authored).expect("alive").position = Vec2::new(99.0, 0.0);
    let created = spawn_at(&mut world, Vec2::new(7.0, 7.0));
    history.push_already_executed(Box::new(CreateEntityCommand::already_created(&world, created)));
    history.execute(
        set_transform(
            authored,
            Transform2D::new(Vec2::new(99.0, 0.0)),
            Transform2D::new(Vec2::new(99.0, 4.0)),
        ),
        &mut world,
    );

    let dropped = history.rebase_session_entries(&mut world, |restored| {
        restored.get_mut::<Transform2D>(authored).expect("alive").position = Vec2::new(3.0, 0.0);
        restored.remove_entity(&created).ok();
    });
    history.end_session();

    assert_eq!(dropped, 0);
    assert!(world.entities().contains(&created), "the paused creation is back under its id");
    assert_eq!(
        transform(&world, authored).position,
        Vec2::new(3.0, 4.0),
        "the edited field survives over the authored value"
    );
    assert!(history.undo(&mut world));
    assert_eq!(
        transform(&world, authored).position,
        Vec2::new(3.0, 0.0),
        "undo returns the AUTHORED value, not the simulated one"
    );
    assert!(history.undo(&mut world));
    assert!(!world.entities().contains(&created));
}

#[test]
fn test_rebase_replays_only_the_changed_fields_over_the_authored_component() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut history = CommandHistory::new();
    history.begin_session();

    // Simulated pose (100, 50); the user edits rotation only.
    let simulated = Transform2D { position: Vec2::new(100.0, 50.0), rotation: 0.0, ..Default::default() };
    let edited = Transform2D { rotation: 90.0, ..simulated };
    world.get_mut::<Transform2D>(entity).expect("alive").clone_from(&edited);
    history.push_already_executed(Box::new(SetTransformCommand::new(
        entity,
        simulated,
        edited,
        "rotation",
    )));

    let dropped = history.rebase_session_entries(&mut world, |restored| {
        restored.get_mut::<Transform2D>(entity).expect("alive").clone_from(&Transform2D::new(Vec2::ZERO));
    });
    history.end_session();

    assert_eq!(dropped, 0);
    assert_eq!(transform(&world, entity).position, Vec2::ZERO, "the simulated position is not kept");
    assert_eq!(transform(&world, entity).rotation, 90.0, "the edited field is");
    assert!(history.undo(&mut world));
    assert_eq!(transform(&world, entity).rotation, 0.0, "undo returns the authored rotation");
}

#[test]
fn test_rebase_replays_a_nudge_as_a_delta_from_the_authored_position() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut history = CommandHistory::new();
    history.begin_session();

    world.get_mut::<Transform2D>(entity).expect("alive").position = Vec2::new(100.0, 50.0);
    history.execute(
        Box::new(NudgeCommand::new(vec![(
            entity,
            Vec2::new(100.0, 50.0),
            Vec2::new(101.0, 50.0),
        )])),
        &mut world,
    );

    history.rebase_session_entries(&mut world, |restored| {
        restored.get_mut::<Transform2D>(entity).expect("alive").position = Vec2::new(8.0, 8.0);
    });
    history.end_session();
    assert_eq!(transform(&world, entity).position, Vec2::new(9.0, 8.0), "the delta lands on the authored pose");
}

#[test]
fn test_rebase_purges_undone_session_entries_instead_of_replaying_them() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut history = CommandHistory::new();
    history.begin_session();

    history.execute(
        set_transform(entity, Transform2D::new(Vec2::ZERO), Transform2D::new(Vec2::new(6.0, 0.0))),
        &mut world,
    );
    history.undo(&mut world);

    let dropped = history.rebase_session_entries(&mut world, |restored| {
        restored.get_mut::<Transform2D>(entity).expect("alive").position = Vec2::ZERO;
    });
    history.end_session();

    assert_eq!(dropped, 0, "an undone entry is purged, not counted as unapplied");
    assert_eq!(transform(&world, entity).position, Vec2::ZERO, "the undone edit stays undone");
    assert!(!history.can_redo(), "and leaves nothing on the redo stack");
    assert!(!history.can_undo());
}

#[test]
fn test_undo_and_redo_seal_merging_so_a_session_command_cannot_merge_into_a_pre_session_entry() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Name::new("start")).ok();
    let mut history = CommandHistory::new();

    history.try_merge_or_push(Box::new(SetNameCommand::new(
        entity,
        Name::new("start"),
        Name::new("first"),
        "name",
    )));
    history.undo(&mut world);
    history.redo(&mut world);
    history.try_merge_or_push(Box::new(SetNameCommand::new(
        entity,
        Name::new("first"),
        Name::new("second"),
        "name",
    )));
    assert!(history.undo(&mut world), "the post-redo edit is its own entry");
    assert!(history.can_undo(), "and the redone entry is still under it");
}

#[test]
fn test_eviction_spares_the_session_entries() {
    let mut world = World::new();
    let mut history = CommandHistory::new();
    for _ in 0..80 {
        let filler = world.create_entity();
        history.push_already_executed(Box::new(CreateEntityCommand::already_created(&world, filler)));
    }

    history.begin_session();
    let created = spawn_at(&mut world, Vec2::ZERO);
    history.push_already_executed(Box::new(CreateEntityCommand::already_created(&world, created)));
    for step in 0..100 {
        let from = Vec2::new(step as f32, 0.0);
        let to = Vec2::new(step as f32 + 1.0, 0.0);
        history.execute(set_transform(created, Transform2D::new(from), Transform2D::new(to)), &mut world);
    }

    assert_eq!(
        history.session_entry_count(),
        101,
        "a hundred-and-first paused edit must not evict the paused create it depends on"
    );
    history.end_session();
    let filler = world.create_entity();
    history.push_already_executed(Box::new(CreateEntityCommand::already_created(&world, filler)));
    assert!(history.undo(&mut world), "the limit applies again once the session closes");
}

#[test]
fn test_rebase_keeps_a_cut_subtree_as_authored_so_undo_restores_no_simulated_state() {
    // Cut captures the subtree it removes; recorded against the paused
    // world, that capture is the simulated pose, and an undo after Keep
    // would put the simulated entity back into the authored scene.
    let mut world = World::new();
    let parent = spawn_at(&mut world, Vec2::ZERO);
    let child = spawn_at(&mut world, Vec2::new(1.0, 1.0));
    world.set_parent(child, parent).expect("child under parent");
    let mut history = CommandHistory::new();

    history.begin_session();
    world.get_mut::<Transform2D>(parent).expect("alive").position = Vec2::new(100.0, 50.0);
    history.execute(Box::new(DeleteTreeCommand::new(&world, parent)), &mut world);
    assert!(!world.entities().contains(&parent), "the cut removed the subtree");

    let unapplied = history.rebase_session_entries(&mut world, |restored| {
        // What `WorldSnapshot::restore` does: clear, then rebuild under the
        // original ids (the undo pass has just resurrected the subtree).
        restored.clear();
        restored.create_entity_with_id(parent);
        restored.add_component(&parent, Transform2D::new(Vec2::ZERO)).ok();
        restored.create_entity_with_id(child);
        restored.add_component(&child, Transform2D::new(Vec2::new(1.0, 1.0))).ok();
        restored.set_parent(child, parent).expect("child under parent");
    });
    assert_eq!(unapplied, 0);
    assert!(!world.entities().contains(&parent), "the cut replayed onto the authored world");

    assert!(history.undo(&mut world));
    assert_eq!(transform(&world, parent).position, Vec2::ZERO, "authored, not simulated");
    assert_eq!(world.get_children(parent).map(|children| children.len()), Some(1));
}

#[test]
fn test_rebase_replays_a_kept_creation_at_its_creation_state() {
    // The user asked for an entity at the origin; what the simulation did
    // to it after a Resume is not their edit.
    let mut world = World::new();
    let mut history = CommandHistory::new();
    history.begin_session();
    let created = spawn_at(&mut world, Vec2::ZERO);
    history.execute(Box::new(CreateEntityCommand::already_created(&world, created)), &mut world);
    world.get_mut::<Transform2D>(created).expect("alive").position = Vec2::new(100.0, 50.0);

    let unapplied = history.rebase_session_entries(&mut world, |restored| {
        restored.remove_entity(&created).ok();
    });
    assert_eq!(unapplied, 0);
    assert_eq!(transform(&world, created).position, Vec2::ZERO);
}

#[test]
fn test_rebase_reports_the_children_a_macro_could_not_apply() {
    // A batch that renamed an authored entity and edited a runtime-only
    // one is kept for the rename, and the lost edit is counted, not hidden.
    let mut world = World::new();
    let authored = spawn_at(&mut world, Vec2::ZERO);
    world.add_component(&authored, Name::new("Before")).ok();
    let runtime_only = spawn_at(&mut world, Vec2::ZERO);
    let mut history = CommandHistory::new();
    history.begin_session();
    history.execute(
        Box::new(MacroCommand::new(
            "batch",
            vec![
                Box::new(SetNameCommand::new(authored, Name::new("Before"), Name::new("After"), "name")),
                set_transform(
                    runtime_only,
                    Transform2D::new(Vec2::ZERO),
                    Transform2D::new(Vec2::new(5.0, 0.0)),
                ),
            ],
        )),
        &mut world,
    );

    let unapplied = history.rebase_session_entries(&mut world, |restored| {
        restored.remove_entity(&runtime_only).ok();
    });
    assert_eq!(unapplied, 1, "the runtime-only edit is counted");
    assert!(history.can_undo(), "the macro itself is kept for the rename");
    assert_eq!(
        world.get::<Name>(authored).map(|name| name.as_str().to_string()),
        Some("After".to_string())
    );
}

#[test]
fn test_rebase_counts_every_child_of_a_macro_that_lost_all_of_them() {
    let mut world = World::new();
    let runtime_only = spawn_at(&mut world, Vec2::ZERO);
    let mut history = CommandHistory::new();
    history.begin_session();
    let step = |from: f32, to: f32| {
        set_transform(
            runtime_only,
            Transform2D::new(Vec2::new(from, 0.0)),
            Transform2D::new(Vec2::new(to, 0.0)),
        )
    };
    history.execute(
        Box::new(MacroCommand::new("batch", vec![step(0.0, 1.0), step(1.0, 2.0), step(2.0, 3.0)])),
        &mut world,
    );

    let unapplied = history.rebase_session_entries(&mut world, |restored| {
        restored.remove_entity(&runtime_only).ok();
    });
    assert_eq!(unapplied, 3, "three edits were lost, not one entry");
    assert!(!history.can_undo(), "the empty macro is not retained");
}
