//! `CommandHistory` as the dirty-state source of truth (issue #24): a
//! command was recorded ⇒ the scene changed. The watermark contract:
//! undoing back to the last-saved command reads clean; merging into a
//! post-save command reassigns its id so no history position reads clean
//! again; a merge is a mutation, so it invalidates redo and a gesture
//! boundary (`break_merge`) is the only thing that stops it.

use super::*;
use common::Transform2D;
use glam::Vec2;

use crate::test_support::setup_entity;

/// A transform edit under `hint`; commands with the same hint merge.
fn move_cmd(entity: EntityId, to: Vec2, hint: &'static str) -> Box<dyn EditorCommand> {
    Box::new(SetTransformCommand::new(entity, Transform2D::new(Vec2::ZERO), Transform2D::new(to), hint))
}

#[test]
fn test_undo_back_to_the_saved_watermark_reads_clean_and_past_it_dirty() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();
    assert!(!history.is_dirty(), "a fresh history is the on-disk state");
    assert!(!history.undo(&mut world));
    assert!(!history.redo(&mut world));
    assert!(!history.is_dirty(), "no-op undo/redo must not dirty a clean scene");

    history.execute(move_cmd(entity, Vec2::new(1.0, 0.0), "position"), &mut world);
    assert!(history.is_dirty(), "an executed command dirties the scene");
    history.mark_saved();
    assert!(!history.is_dirty(), "saving at the top reads clean");

    history.push_already_executed(move_cmd(entity, Vec2::new(1.0, 5.0), "rotation"));
    assert!(history.is_dirty(), "a pushed (pre-applied) command dirties the scene too");

    assert!(history.undo(&mut world));
    assert!(!history.is_dirty(), "undo back to the saved command reads clean");
    assert!(history.undo(&mut world));
    assert!(history.is_dirty(), "undo PAST the saved command is dirty again");
    assert!(history.redo(&mut world));
    assert!(!history.is_dirty(), "redo restores the saved command's id — clean again");

    history.clear();
    assert!(!history.is_dirty(), "clear() = a fresh world that IS the on-disk state");
}

#[test]
fn test_merging_into_a_saved_command_reassigns_its_id_so_no_position_reads_clean() {
    // A merge mutates the saved command in place, so its pre-merge saved
    // state no longer exists anywhere. Both merge paths early-return
    // before execute/push, which is exactly where a naive dirty flag
    // would be set.
    let mut world = World::new();
    let entity = setup_entity(&mut world);

    let mut pushed = CommandHistory::new();
    pushed.push_already_executed(move_cmd(entity, Vec2::new(1.0, 0.0), "position"));
    pushed.mark_saved();
    pushed.try_merge_or_push(move_cmd(entity, Vec2::new(2.0, 0.0), "position"));
    assert!(pushed.is_dirty(), "a merged try_merge_or_push must dirty the scene");
}

#[test]
fn test_a_merge_invalidates_redo() {
    // Save, record A then B, undo B, merge into A, redo. Without redo
    // invalidation the redo would restore B's (saved) id on top of a
    // CHANGED world — falsely clean.
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();
    history.execute(move_cmd(entity, Vec2::new(1.0, 0.0), "position"), &mut world);
    history.execute(move_cmd(entity, Vec2::new(1.0, 7.0), "rotation"), &mut world);
    history.mark_saved();
    assert!(history.undo(&mut world), "undo B; redo holds B");

    history.try_merge_or_push(move_cmd(entity, Vec2::new(2.0, 0.0), "position"));

    assert!(!history.can_redo(), "a merge is a new mutation and clears the redo stack");
    assert!(history.is_dirty(), "the merged world differs from the saved one");
}

#[test]
fn test_break_merge_seals_the_gesture_so_two_scrubs_are_two_entries() {
    // Without the seal, field_hint merging is unbounded in time: every
    // scrub on the same field for the rest of the session would be one
    // undo entry.
    let mut world = World::new();
    let entity = setup_entity(&mut world);

    let mut unbroken = CommandHistory::new();
    unbroken.try_merge_or_push(move_cmd(entity, Vec2::new(10.0, 0.0), "position"));
    unbroken.try_merge_or_push(move_cmd(entity, Vec2::new(20.0, 0.0), "position"));
    unbroken.try_merge_or_push(move_cmd(entity, Vec2::new(30.0, 0.0), "position"));
    assert!(unbroken.undo(&mut world));
    assert!(!unbroken.can_undo(), "control: unbroken frames collapse to one entry");

    let mut history = CommandHistory::new();
    history.try_merge_or_push(move_cmd(entity, Vec2::new(10.0, 0.0), "position"));
    history.try_merge_or_push(move_cmd(entity, Vec2::new(20.0, 0.0), "position"));
    history.break_merge();
    history.try_merge_or_push(move_cmd(entity, Vec2::new(30.0, 0.0), "position"));

    assert!(history.undo(&mut world), "undo gesture 2");
    assert!(history.can_undo(), "gesture 1 is its own entry");
    assert!(history.undo(&mut world), "undo gesture 1");
    assert!(!history.can_undo());
}
