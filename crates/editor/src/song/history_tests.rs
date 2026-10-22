//! The song's own save watermark, copying the scene history's contract: undoing back to
//! the saved edit reads clean; a merge into the saved edit reassigns its id so no
//! position reads clean again; an undo or a redo seals the gesture on top, so the next
//! continuing edit never merges into the older entry that surfaces; `break_merge` ends a
//! gesture. And one thing the scene's watermark never says: a song never saved is dirty.

use super::test_support::{add_note, melody, note_numbers, AssetFolder};
use super::{Gesture, SongDocument, SongEdit};

fn move_notes(notes: &[u64], tick_delta: i64) -> SongEdit {
    SongEdit::MoveNotes {
        notes: notes.to_vec(),
        tick_delta,
    }
}

/// A melody saved into an asset folder, so only the watermark decides dirtiness.
fn saved_melody(folder: &AssetFolder) -> SongDocument {
    let mut document = melody();
    document
        .save(&folder.path("music/theme.mid"), folder.base())
        .expect("saves");
    assert!(!document.is_dirty(), "fixture: a saved song is clean");
    document
}

#[test]
fn a_new_song_is_dirty_until_its_first_save_though_its_history_is_clean() {
    let folder = AssetFolder::new();
    let mut document = SongDocument::new_song().expect("a blank song");
    assert!(
        !document.history().is_dirty(),
        "the watermark alone calls it clean"
    );
    assert!(
        document.is_dirty(),
        "a song that was never saved has nothing on disk"
    );

    document.file.provenance = Some(super::test_support::song_provenance());
    document
        .save(&folder.path("music/untitled.mid"), folder.base())
        .expect("saves");
    assert!(!document.is_dirty());
}

#[test]
fn undo_back_to_the_saved_edit_reads_clean_and_past_it_dirty() {
    let folder = AssetFolder::new();
    let mut document = saved_melody(&folder);
    assert_eq!(document.undo().expect("undoes"), Some("AddNote"));
    assert!(document.is_dirty(), "undo past the saved edit is dirty");
    assert_eq!(document.redo().expect("redoes"), Some("AddNote"));
    assert!(!document.is_dirty(), "redo restores the saved edit's id");

    document
        .apply(add_note(1440, 72), Gesture::Discrete)
        .expect("lands");
    assert!(document.is_dirty());
    document.undo().expect("undoes");
    assert!(
        !document.is_dirty(),
        "undo back to the saved edit reads clean"
    );

    let mut empty = SongDocument::new_song().expect("a blank song");
    assert_eq!(empty.undo().expect("nothing to undo"), None);
    assert_eq!(empty.redo().expect("nothing to redo"), None);
}

#[test]
fn merging_into_the_saved_edit_reassigns_its_id_so_no_position_reads_clean() {
    let folder = AssetFolder::new();
    let mut document = melody();
    let first = note_numbers(&document)[0];
    document
        .apply(move_notes(&[first], 10), Gesture::Continuing)
        .expect("moves");
    document
        .save(&folder.path("music/theme.mid"), folder.base())
        .expect("saves");

    document
        .apply(move_notes(&[first], 10), Gesture::Continuing)
        .expect("the drag goes on");
    assert!(
        document.is_dirty(),
        "a merge into the saved edit dirties the song"
    );
    assert_eq!(document.history().undo_count(), 4, "one entry: the merge");
}

/// An undo surfaces an older entry whose gesture ended long ago; a continuing edit after
/// it — a field still focused, a key still held — must not merge into that entry, or one
/// undo would take back both and the undone edit's redo would vanish under a merge.
#[test]
fn an_undo_seals_the_gesture_on_top_so_the_next_continuing_edit_is_its_own_entry() {
    let mut document = melody();
    let [first, second, _] = note_numbers(&document)[..] else {
        panic!("three notes");
    };
    document
        .apply(move_notes(&[first], 10), Gesture::Continuing)
        .expect("moves the first");
    document.break_merge();
    document
        .apply(move_notes(&[second], 10), Gesture::Continuing)
        .expect("moves the second");
    document.undo().expect("undoes the second");
    assert!(document.history().can_redo());

    document
        .apply(move_notes(&[first], 5), Gesture::Continuing)
        .expect("moves the first again");
    assert!(!document.history().can_redo(), "a new entry clears the redo stack");
    let tick = |document: &SongDocument, number| {
        let id = document.note_id(number).expect("the note");
        document.data().note_image(id).map(|image| image.tick)
    };
    assert_eq!(tick(&document, first), Some(15));
    document.undo().expect("undoes the move after the undo");
    assert_eq!(tick(&document, first), Some(10), "only the edit after the undo, not the older gesture");
    document.undo().expect("undoes the older gesture");
    assert_eq!(tick(&document, first), Some(0));
}

#[test]
fn break_merge_seals_a_gesture_so_two_drags_are_two_entries() {
    let mut unbroken = melody();
    let first = note_numbers(&unbroken)[0];
    for _ in 0..3 {
        unbroken
            .apply(move_notes(&[first], 10), Gesture::Continuing)
            .expect("drags");
    }
    assert_eq!(
        unbroken.history().undo_count(),
        4,
        "unbroken frames collapse to one entry"
    );

    let mut document = melody();
    document
        .apply(move_notes(&[first], 10), Gesture::Continuing)
        .expect("drags");
    document
        .apply(move_notes(&[first], 10), Gesture::Continuing)
        .expect("drags");
    document.break_merge();
    document
        .apply(move_notes(&[first], 10), Gesture::Continuing)
        .expect("drags again");
    assert_eq!(
        document.history().undo_count(),
        5,
        "gesture 2 is its own entry"
    );

    let mut discrete = melody();
    discrete
        .apply(move_notes(&[first], 10), Gesture::Discrete)
        .expect("an API line");
    discrete
        .apply(move_notes(&[first], 10), Gesture::Continuing)
        .expect("then a drag");
    assert_eq!(
        discrete.history().undo_count(),
        5,
        "a discrete edit never takes a merge"
    );
}

#[test]
fn the_song_history_is_capped_at_a_hundred_entries() {
    let mut document = SongDocument::new_song().expect("a blank song");
    for index in 0..105 {
        document
            .apply(add_note(index * 10, 60), Gesture::Discrete)
            .expect("lands");
    }
    assert_eq!(document.history().undo_count(), 100);
}
