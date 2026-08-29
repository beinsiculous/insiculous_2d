//! Selection restore on undo/redo (#59): undoing a destructive command
//! brings back the selection that existed before it — platform convention —
//! and redo restores what was selected at undo time, generally (the
//! mechanism lives in `CommandHistory`, not in any one command).

use ecs::World;
use glam::Vec2;

use super::*;
use crate::selection::Selection;

fn spawn(world: &mut World) -> ecs::EntityId {
    let e = world.create_entity();
    world
        .add_component(&e, common::Transform2D::new(Vec2::ZERO))
        .ok();
    e
}

/// The host contract in miniature: note before handlers run, apply the
/// restore after undo/redo.
fn apply_restore(history: &mut CommandHistory, selection: &mut Selection) {
    if let Some(ids) = history.take_selection_restore() {
        selection.clear();
        selection.select_multiple(ids);
    }
}

#[test]
fn test_undo_delete_restores_the_selection() {
    let mut world = World::new();
    let mut history = CommandHistory::new();
    let mut selection = Selection::new();
    let a = spawn(&mut world);
    let b = spawn(&mut world);

    selection.select(a);
    selection.add(b);
    history.note_selection(&selection); // frame start

    // The delete handler clears the selection, then records the command —
    // exactly the order the editor uses.
    selection.clear();
    history.execute(Box::new(DeleteEntityCommand::new(a)), &mut world);

    assert!(history.undo(&mut world));
    apply_restore(&mut history, &mut selection);
    assert!(selection.contains(a), "the deleted entity is selected again");
    assert!(selection.contains(b), "co-selected survivors come back too");
    assert_eq!(selection.primary(), Some(a), "insertion order restores the primary");
}

#[test]
fn test_redo_delete_reclears_and_redo_create_reselects() {
    let mut world = World::new();
    let mut history = CommandHistory::new();
    let mut selection = Selection::new();
    let a = spawn(&mut world);

    selection.select(a);
    history.note_selection(&selection);
    selection.clear();
    history.execute(Box::new(DeleteEntityCommand::new(a)), &mut world);

    // undo → selection back
    history.undo(&mut world);
    apply_restore(&mut history, &mut selection);
    assert!(selection.contains(a));

    // redo → the delete re-applies; the after-image (captured at undo time
    // = {a}) prunes to nothing because a is gone again.
    history.note_selection(&selection);
    assert!(history.redo(&mut world));
    apply_restore(&mut history, &mut selection);
    assert!(
        selection.is_empty(),
        "redo of a delete leaves nothing selected (pruned)"
    );
}

#[test]
fn test_redo_of_create_reselects_the_created_entity() {
    // kimi plan R1-F5: redo executes FIRST, then restores — so the recreated
    // entity exists before pruning and stays selected.
    let mut world = World::new();
    let mut history = CommandHistory::new();
    let mut selection = Selection::new();

    history.note_selection(&selection); // empty before-create
    let created = spawn(&mut world);
    history.push_already_executed(Box::new(CreateEntityCommand::already_created(
        &world, created,
    )));
    selection.select(created); // the editor selects what it creates

    // Undo: the create reverts; before-image (empty) restores.
    history.note_selection(&selection);
    history.undo(&mut world);
    apply_restore(&mut history, &mut selection);
    assert!(selection.is_empty());

    // Redo: the entity exists again (id-exact) and is selected again.
    assert!(history.redo(&mut world));
    apply_restore(&mut history, &mut selection);
    assert!(selection.contains(created), "redo re-selects the recreated entity");
}

#[test]
fn test_merged_entries_keep_the_first_before_image() {
    let mut world = World::new();
    let mut history = CommandHistory::new();
    let mut selection = Selection::new();
    let a = spawn(&mut world);

    selection.select(a);
    history.note_selection(&selection);
    // Two merging nudges in one gesture window.
    history.try_merge_or_push(Box::new(NudgeCommand::new(
        vec![(a, Vec2::ZERO, Vec2::new(1.0, 0.0))],
    )));
    // Selection changes mid-gesture (say the user also clicked elsewhere
    // in a way that didn't record a command)...
    selection.clear();
    history.note_selection(&selection);
    history.try_merge_or_push(Box::new(NudgeCommand::new(
        vec![(a, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0))],
    )));

    // ...but the merged entry's before-image is the FIRST one: {a}.
    history.undo(&mut world);
    apply_restore(&mut history, &mut selection);
    assert!(selection.contains(a), "a merged gesture restores its first before-image");
}

#[test]
fn test_stale_ids_are_pruned_from_the_restore() {
    let mut world = World::new();
    let mut history = CommandHistory::new();
    let mut selection = Selection::new();
    let a = spawn(&mut world);
    let doomed = spawn(&mut world);

    selection.select(a);
    selection.add(doomed);
    history.note_selection(&selection);
    selection.clear();
    history.execute(Box::new(DeleteEntityCommand::new(a)), &mut world);

    // `doomed` dies OUTSIDE the history (a game-side removal).
    world.remove_entity(&doomed).ok();

    history.undo(&mut world);
    apply_restore(&mut history, &mut selection);
    assert!(selection.contains(a));
    assert!(
        !selection.contains(doomed),
        "an id that no longer exists is pruned, never restored dangling"
    );
}
