//! Tests for `CommandHistory` as the dirty-state source of truth
//! (issue #24, audit §1.4): a command was recorded ⇒ the scene changed.
//! The watermark contract: undoing back to the last-saved command reads
//! clean; merging into a post-save command reassigns its id so the scene
//! stays dirty even after undoing past it.

use super::*;
use ecs::EntityId;
use glam::Vec2;

fn setup_entity(world: &mut World) -> EntityId {
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::ZERO)).ok();
    entity
}

/// A transform edit; commands with the same `field_hint` merge.
fn move_cmd(entity: EntityId, to_x: f32) -> Box<dyn EditorCommand> {
    Box::new(SetTransformCommand::new(
        entity,
        common::Transform2D::new(Vec2::ZERO),
        common::Transform2D::new(Vec2::new(to_x, 0.0)),
        "position",
    ))
}

#[test]
fn test_new_history_is_clean() {
    assert!(!CommandHistory::new().is_dirty());
}

#[test]
fn test_execute_marks_dirty() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();

    history.execute(move_cmd(entity, 1.0), &mut world);

    assert!(history.is_dirty());
}

#[test]
fn test_push_already_executed_marks_dirty() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();

    history.push_already_executed(move_cmd(entity, 1.0));

    assert!(history.is_dirty());
}

#[test]
fn test_merge_paths_mark_dirty() {
    // The trap this design guards: both merge paths early-return without
    // reaching execute/push_already_executed, so a merged gizmo drag or
    // slider scrub after a save must still dirty the scene.
    let mut world = World::new();
    let entity = setup_entity(&mut world);

    let mut history = CommandHistory::new();
    history.execute(move_cmd(entity, 1.0), &mut world);
    history.mark_saved();
    assert!(!history.is_dirty());
    history.try_merge_or_execute(move_cmd(entity, 2.0), &mut world);
    assert!(history.is_dirty(), "a merged try_merge_or_execute must dirty the scene");

    let mut history = CommandHistory::new();
    history.push_already_executed(move_cmd(entity, 1.0));
    history.mark_saved();
    assert!(!history.is_dirty());
    history.try_merge_or_push(move_cmd(entity, 2.0));
    assert!(history.is_dirty(), "a merged try_merge_or_push must dirty the scene");
}

#[test]
fn test_undo_back_to_saved_watermark_reads_clean() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();

    // Different field hints so the commands do NOT merge.
    history.execute(
        Box::new(SetTransformCommand::new(
            entity,
            common::Transform2D::new(Vec2::ZERO),
            common::Transform2D::new(Vec2::new(1.0, 0.0)),
            "position",
        )),
        &mut world,
    );
    history.mark_saved();
    history.execute(
        Box::new(SetTransformCommand::new(
            entity,
            common::Transform2D::new(Vec2::new(1.0, 0.0)),
            common::Transform2D::new(Vec2::new(1.0, 5.0)),
            "rotation",
        )),
        &mut world,
    );
    assert!(history.is_dirty());

    assert!(history.undo(&mut world));
    assert!(!history.is_dirty(), "undo back to the saved command reads clean");

    assert!(history.undo(&mut world));
    assert!(history.is_dirty(), "undo PAST the saved command is dirty again");
}

#[test]
fn test_redo_past_watermark_reads_dirty_then_clean() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();

    history.execute(move_cmd(entity, 1.0), &mut world);
    history.mark_saved();
    assert!(history.undo(&mut world));
    assert!(history.is_dirty(), "undone below the watermark is dirty");

    assert!(history.redo(&mut world));
    assert!(!history.is_dirty(), "redo restores the saved command's id — clean again");
}

#[test]
fn test_save_then_merge_then_undo_stays_dirty() {
    // A merge mutates the saved command in place, so its pre-merge saved
    // state no longer exists anywhere: no history position may read clean.
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();

    history.execute(move_cmd(entity, 1.0), &mut world);
    history.mark_saved();
    history.try_merge_or_execute(move_cmd(entity, 2.0), &mut world);
    assert!(history.is_dirty());

    assert!(history.undo(&mut world));
    assert!(history.is_dirty(), "the merged command's original saved state is gone — still dirty");
    assert!(history.redo(&mut world));
    assert!(history.is_dirty());
}

#[test]
fn test_merge_clears_redo_history() {
    // Kimi review F1: save, record A then B, undo B, merge into A, redo.
    // Without redo invalidation the redo would restore B's (saved) id on
    // top of a CHANGED world — falsely clean. A merge is a new mutation:
    // it must clear the redo stack like execute/push do.
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();

    history.execute(move_cmd(entity, 1.0), &mut world); // A ("position")
    history.execute(
        Box::new(SetTransformCommand::new(
            entity,
            common::Transform2D::new(Vec2::new(1.0, 0.0)),
            common::Transform2D::new(Vec2::new(1.0, 7.0)),
            "rotation",
        )),
        &mut world,
    ); // B
    history.mark_saved();

    assert!(history.undo(&mut world)); // undo B; redo holds B
    history.try_merge_or_execute(move_cmd(entity, 2.0), &mut world); // merges into A

    assert!(!history.can_redo(), "a merge invalidates redo history");
    assert!(history.is_dirty(), "the merged world differs from the saved one");
}

#[test]
fn test_clear_resets_to_clean() {
    let mut world = World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();
    history.execute(move_cmd(entity, 1.0), &mut world);

    history.clear();

    assert!(!history.is_dirty(), "clear() = fresh world that IS the on-disk state");
}

#[test]
fn test_noop_undo_redo_on_empty_history_stay_clean() {
    let mut world = World::new();
    let mut history = CommandHistory::new();

    assert!(!history.undo(&mut world));
    assert!(!history.redo(&mut world));

    assert!(!history.is_dirty(), "no-op undo/redo must not dirty a clean scene");
}
