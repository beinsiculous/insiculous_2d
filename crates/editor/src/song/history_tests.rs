//! The song's own save watermark, copying the scene history's contract: undoing back to
//! the saved edit reads clean; a merge into the saved edit reassigns its id so no
//! position reads clean again; an undo or a redo seals the gesture on top, so the next
//! continuing edit never merges into the older entry that surfaces; `break_merge` ends a
//! gesture. And one thing the scene's watermark never says: a song never saved is dirty.

use music_midi::{Part, PartMode, Quantize, TrackSelector};

use super::test_support::{melody, AssetFolder};
use super::{Gesture, SongDocument, SongEdit};

fn part(name: &str) -> Part {
    Part {
        name: name.to_string(),
        track: TrackSelector::Index(1),
        mode: PartMode::Loop,
        quantize: Quantize::Off,
    }
}

/// The melody with two parts, `lead` and `bass`, set as an opened song carries them, so the
/// history starts empty.
fn mapped_melody() -> SongDocument {
    let mut document = melody();
    document.file.parts = vec![part("lead"), part("bass")];
    document
}

fn quantize(part: &str, quantize: Quantize) -> SongEdit {
    SongEdit::SetQuantize {
        part: part.to_string(),
        quantize,
    }
}

fn quantize_of(document: &SongDocument, name: &str) -> Quantize {
    document
        .file()
        .parts
        .iter()
        .find(|part| part.name == name)
        .expect("the part")
        .quantize
}

/// A mapped melody with one edit, saved into an asset folder, so only the watermark decides
/// dirtiness.
fn saved_melody(folder: &AssetFolder) -> SongDocument {
    let mut document = mapped_melody();
    document
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("lands");
    document
        .save(&folder.path("music/theme.mid"), folder.base())
        .expect("saves");
    assert!(!document.is_dirty(), "fixture: a saved song is clean");
    document
}

#[test]
fn a_new_song_is_dirty_until_its_first_save_though_its_history_is_clean() {
    let folder = AssetFolder::new();
    let mut document = melody();
    assert!(
        !document.history().is_dirty(),
        "the watermark alone calls it clean"
    );
    assert!(
        document.is_dirty(),
        "a song that was never saved has nothing on disk"
    );

    document
        .save(&folder.path("music/untitled.mid"), folder.base())
        .expect("saves");
    assert!(!document.is_dirty());
}

#[test]
fn undo_back_to_the_saved_edit_reads_clean_and_past_it_dirty() {
    let folder = AssetFolder::new();
    let mut document = saved_melody(&folder);
    assert_eq!(document.undo(), Some("SetLoop"));
    assert!(document.is_dirty(), "undo past the saved edit is dirty");
    assert_eq!(document.redo(), Some("SetLoop"));
    assert!(!document.is_dirty(), "redo restores the saved edit's id");

    document
        .apply(quantize("lead", Quantize::Beat), Gesture::Discrete)
        .expect("lands");
    assert!(document.is_dirty());
    document.undo();
    assert!(
        !document.is_dirty(),
        "undo back to the saved edit reads clean"
    );

    let mut empty = melody();
    assert_eq!(empty.undo(), None, "nothing to undo");
    assert_eq!(empty.redo(), None, "nothing to redo");
}

#[test]
fn merging_into_the_saved_edit_reassigns_its_id_so_no_position_reads_clean() {
    let folder = AssetFolder::new();
    let mut document = mapped_melody();
    document
        .apply(quantize("lead", Quantize::Beat), Gesture::Continuing)
        .expect("scrubs");
    document
        .save(&folder.path("music/theme.mid"), folder.base())
        .expect("saves");

    document
        .apply(quantize("lead", Quantize::Eighth), Gesture::Continuing)
        .expect("the scrub goes on");
    assert!(
        document.is_dirty(),
        "a merge into the saved edit dirties the song"
    );
    assert_eq!(document.history().undo_count(), 1, "one entry: the merge");
}

/// An undo surfaces an older entry whose gesture ended long ago; a continuing edit after
/// it — a field still focused, a key still held — must not merge into that entry, or one
/// undo would take back both and the undone edit's redo would vanish under a merge.
#[test]
fn an_undo_seals_the_gesture_on_top_so_the_next_continuing_edit_is_its_own_entry() {
    let mut document = mapped_melody();
    document
        .apply(quantize("lead", Quantize::Beat), Gesture::Continuing)
        .expect("quantizes the lead");
    document.break_merge();
    document
        .apply(quantize("bass", Quantize::Beat), Gesture::Continuing)
        .expect("quantizes the bass");
    document.undo();
    assert!(document.history().can_redo());

    document
        .apply(quantize("lead", Quantize::Eighth), Gesture::Continuing)
        .expect("quantizes the lead again");
    assert!(!document.history().can_redo(), "a new entry clears the redo stack");
    assert_eq!(quantize_of(&document, "lead"), Quantize::Eighth);
    document.undo();
    assert_eq!(
        quantize_of(&document, "lead"),
        Quantize::Beat,
        "only the edit after the undo, not the older gesture"
    );
    document.undo();
    assert_eq!(quantize_of(&document, "lead"), Quantize::Off);
}

#[test]
fn break_merge_seals_a_gesture_so_two_scrubs_are_two_entries() {
    let mut unbroken = mapped_melody();
    for value in [Quantize::Beat, Quantize::Eighth, Quantize::Sixteenth] {
        unbroken
            .apply(quantize("lead", value), Gesture::Continuing)
            .expect("scrubs");
    }
    assert_eq!(
        unbroken.history().undo_count(),
        1,
        "unbroken frames collapse to one entry"
    );

    let mut document = mapped_melody();
    document
        .apply(quantize("lead", Quantize::Beat), Gesture::Continuing)
        .expect("scrubs");
    document
        .apply(quantize("lead", Quantize::Eighth), Gesture::Continuing)
        .expect("scrubs");
    document.break_merge();
    document
        .apply(quantize("lead", Quantize::Sixteenth), Gesture::Continuing)
        .expect("scrubs again");
    assert_eq!(
        document.history().undo_count(),
        2,
        "gesture 2 is its own entry"
    );

    let mut discrete = mapped_melody();
    discrete
        .apply(quantize("lead", Quantize::Beat), Gesture::Discrete)
        .expect("an API line");
    discrete
        .apply(quantize("lead", Quantize::Eighth), Gesture::Continuing)
        .expect("then a scrub");
    assert_eq!(
        discrete.history().undo_count(),
        2,
        "a discrete edit never takes a merge"
    );
}

#[test]
fn the_song_history_is_capped_at_a_hundred_entries() {
    let mut document = melody();
    for index in 0..105 {
        document
            .apply(
                SongEdit::SetLoop {
                    looping: index % 2 == 0,
                },
                Gesture::Discrete,
            )
            .expect("lands");
    }
    assert_eq!(document.history().undo_count(), 100);
}
