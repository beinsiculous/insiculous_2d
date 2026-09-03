//! Selection restore on undo/redo (#59): undoing a destructive command
//! brings back the selection that existed before it — platform convention —
//! and redo restores what was selected at undo time. The mechanism lives in
//! `CommandHistory`, not in any one command.

use glam::Vec2;

use super::*;
use crate::selection::Selection;
use crate::test_support::setup_entity;

/// The host contract in miniature: note before handlers run, apply the
/// restore after undo/redo.
fn apply_restore(history: &mut CommandHistory, selection: &mut Selection) {
    if let Some(ids) = history.take_selection_restore() {
        selection.clear();
        selection.select_multiple(ids);
    }
}

#[test]
fn test_undo_of_a_delete_restores_the_selection_and_redo_reclears_it() {
    let mut world = World::new();
    let mut history = CommandHistory::new();
    let mut selection = Selection::new();
    let deleted = setup_entity(&mut world);
    let survivor = setup_entity(&mut world);
    selection.select(deleted);
    selection.add(survivor);
    history.note_selection(&selection); // frame start

    // The delete handler clears the selection, then records the command —
    // exactly the order the editor uses.
    selection.clear();
    history.execute(Box::new(DeleteEntityCommand::new(deleted)), &mut world);

    assert!(history.undo(&mut world));
    apply_restore(&mut history, &mut selection);
    assert_eq!(
        selection.selected().collect::<Vec<_>>(),
        vec![deleted, survivor],
        "the deleted entity and its co-selected survivor come back in order"
    );
    assert_eq!(selection.primary(), Some(deleted), "insertion order restores the primary");

    // Redo re-applies the delete; the after-image (captured at undo time)
    // prunes to the survivor because the deleted entity is gone again.
    history.note_selection(&selection);
    assert!(history.redo(&mut world));
    apply_restore(&mut history, &mut selection);
    assert_eq!(selection.selected().collect::<Vec<_>>(), vec![survivor]);
}

#[test]
fn test_redo_re_executes_before_restoring_so_a_recreated_entity_stays_selected() {
    let mut world = World::new();
    let mut history = CommandHistory::new();
    let mut selection = Selection::new();
    history.note_selection(&selection); // empty before-create

    let created = setup_entity(&mut world);
    history.push_already_executed(Box::new(CreateEntityCommand::already_created(&world, created)));
    selection.select(created); // the editor selects what it creates

    history.note_selection(&selection);
    assert!(history.undo(&mut world));
    apply_restore(&mut history, &mut selection);
    assert!(selection.is_empty(), "undo of create restores the empty before-image");

    assert!(history.redo(&mut world));
    apply_restore(&mut history, &mut selection);
    assert!(
        selection.contains(created),
        "redo executes FIRST, so the recreated entity exists before pruning and is selected again"
    );
}

#[test]
fn test_a_merged_gesture_restores_its_first_before_image() {
    let mut world = World::new();
    let mut history = CommandHistory::new();
    let mut selection = Selection::new();
    let nudged = setup_entity(&mut world);
    selection.select(nudged);
    history.note_selection(&selection);

    history.try_merge_or_push(Box::new(NudgeCommand::new(vec![(nudged, Vec2::ZERO, Vec2::new(1.0, 0.0))])));
    // The selection changes mid-gesture without recording a command...
    selection.clear();
    history.note_selection(&selection);
    history.try_merge_or_push(Box::new(NudgeCommand::new(vec![(nudged, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0))])));

    assert!(history.undo(&mut world));
    apply_restore(&mut history, &mut selection);
    assert_eq!(
        selection.selected().collect::<Vec<_>>(),
        vec![nudged],
        "...but the merged entry's before-image is the FIRST one"
    );
    assert!(!history.can_undo(), "the two nudges were one entry");
}

#[test]
fn test_ids_that_no_longer_exist_are_pruned_from_the_restore() {
    let mut world = World::new();
    let mut history = CommandHistory::new();
    let mut selection = Selection::new();
    let deleted = setup_entity(&mut world);
    let doomed = setup_entity(&mut world);
    selection.select(deleted);
    selection.add(doomed);
    history.note_selection(&selection);
    selection.clear();
    history.execute(Box::new(DeleteEntityCommand::new(deleted)), &mut world);

    // `doomed` dies OUTSIDE the history (a game-side removal).
    world.remove_entity(&doomed).ok();

    assert!(history.undo(&mut world));
    apply_restore(&mut history, &mut selection);
    assert_eq!(
        selection.selected().collect::<Vec<_>>(),
        vec![deleted],
        "an id that no longer exists is never restored dangling"
    );
}
