//! Open settles what an interrupted save left: a save stopped at each of its writes and
//! removals reopens as the old song or the new one, never neither; a pair that matches
//! itself is recovered over the finals; a matching leftover is purged; a differing lone
//! `.mid.new` is opened as an import; a lone `.mid.ron.new` is removed; mismatched finals
//! with no pair are an error naming both files.

use music_midi::InstrumentSource;

use super::io::open::open_from;
use super::io::VfsStore;
use super::test_support::{
    add_note, exported, melody, sample_instrument, AssetFolder, InterruptedStore,
};
use super::{Gesture, SongDocument, SongEdit, SongOpenError};

const SONG: &str = "music/theme.mid";

/// A melody saved at [`SONG`] (the old song), and the same melody with a fourth note
/// (the new one, unsaved).
fn old_and_new(folder: &AssetFolder) -> (Vec<u8>, SongDocument) {
    let mut document = melody();
    document
        .save(&folder.path(SONG), folder.base())
        .expect("saves the old song");
    let old = exported(&document);
    document
        .apply(add_note(1440, 72), Gesture::Discrete)
        .expect("edits");
    (old, document)
}

fn reopen(folder: &AssetFolder) -> Result<super::OpenedSong, SongOpenError> {
    open_from(&mut VfsStore, &folder.path(SONG), folder.base())
}

#[test]
fn a_save_interrupted_at_any_step_reopens_as_the_old_song_or_the_new_one() {
    // The save of a song without samples makes seven writes and removals: the pair (sidecar,
    // then MIDI), the empty sample folder's removal, the finals (MIDI, then sidecar), and the
    // pair's two removals — the home folder's LICENSE already names the song, so it is not
    // written again. The pair is the commit point, so from the second on it is the new song.
    for operations in 0..=7 {
        let folder = AssetFolder::new();
        let (old, mut document) = old_and_new(&folder);
        let new = exported(&document);
        let mut store = InterruptedStore {
            operations_left: operations,
        };
        let saved = document.save_to(&mut store, &folder.path(SONG), folder.base());
        assert_eq!(
            saved.is_ok(),
            operations == 7,
            "after {operations} operations"
        );
        assert!(
            document.is_dirty() || operations == 7,
            "an interrupted save keeps the song dirty"
        );

        let opened = reopen(&folder).unwrap_or_else(|error| panic!("after {operations}: {error}"));
        let reopened = exported(&opened.document);
        let expected = if operations >= 2 { &new } else { &old };
        assert_eq!(
            &reopened, expected,
            "after {operations} operations: {:?}",
            opened.notes
        );
        assert!(
            !opened.document.is_dirty(),
            "after {operations}: the finals stand or are recovered"
        );
        let left: Vec<String> = folder.files_under("music");
        assert_eq!(
            left,
            ["LICENSE", "theme.mid", "theme.mid.ron"],
            "after {operations}: settled to the finals"
        );
    }
}

#[test]
fn a_pair_landed_with_the_finals_untouched_is_recovered() {
    let folder = AssetFolder::new();
    let (_, mut document) = old_and_new(&folder);
    let new = exported(&document);
    let mut store = InterruptedStore { operations_left: 3 };
    document
        .save_to(&mut store, &folder.path(SONG), folder.base())
        .expect_err("stopped before the finals");
    assert_eq!(
        folder.files_under("music").len(),
        5,
        "the pair beside the old finals and the LICENSE"
    );

    let opened = reopen(&folder).expect("recovers");
    assert_eq!(exported(&opened.document), new);
    assert!(
        opened.notes[0].contains("finished the save"),
        "{:?}",
        opened.notes
    );
}

#[test]
fn the_final_midi_written_alone_is_recovered_from_the_pair() {
    let folder = AssetFolder::new();
    let (_, mut document) = old_and_new(&folder);
    let new = exported(&document);
    let mut store = InterruptedStore { operations_left: 4 };
    document
        .save_to(&mut store, &folder.path(SONG), folder.base())
        .expect_err("stopped between the finals");

    let opened = reopen(&folder).expect("the pair is the truth over mismatched finals");
    assert_eq!(exported(&opened.document), new);
}

#[test]
fn a_leftover_matching_the_finals_is_purged() {
    let folder = AssetFolder::new();
    let (old, _) = old_and_new(&folder);
    folder.write("music/theme.mid.new", &old);

    let opened = reopen(&folder).expect("opens");
    assert_eq!(exported(&opened.document), old);
    assert_eq!(
        folder.files_under("music"),
        ["LICENSE", "theme.mid", "theme.mid.ron"]
    );
    assert_eq!(opened.notes.len(), 1, "{:?}", opened.notes);
}

#[test]
fn a_lone_midi_new_that_differs_opens_as_an_import_and_the_finals_stay() {
    let folder = AssetFolder::new();
    let (old, document) = old_and_new(&folder);
    let new = exported(&document);
    folder.write("music/theme.mid.new", &new);

    let opened = reopen(&folder).expect("opens");
    assert_eq!(
        exported(&opened.document),
        new,
        "the unsaved work is what opens"
    );
    assert_eq!(opened.document.path(), None, "as an import");
    assert!(opened.document.is_dirty());
    assert_eq!(
        folder.read(SONG),
        Some(old),
        "the final is never written over"
    );
}

#[test]
fn a_lone_sidecar_new_is_removed_and_the_finals_open() {
    let folder = AssetFolder::new();
    let (old, _) = old_and_new(&folder);
    let sidecar = folder.read("music/theme.mid.ron").expect("the sidecar");
    folder.write("music/theme.mid.ron.new", &sidecar);

    let opened = reopen(&folder).expect("opens");
    assert_eq!(exported(&opened.document), old);
    assert_eq!(
        folder.files_under("music"),
        ["LICENSE", "theme.mid", "theme.mid.ron"]
    );
}

#[test]
fn mismatched_finals_with_no_pair_are_an_error_naming_both_files() {
    let folder = AssetFolder::new();
    let (_, document) = old_and_new(&folder);
    folder.write(SONG, &exported(&document));

    let error = reopen(&folder).expect_err("never a silent load");
    let message = error.to_string();
    assert!(matches!(error, SongOpenError::MismatchedFinals { .. }));
    assert!(
        message.contains("theme.mid ") && message.contains("theme.mid.ron"),
        "{message}"
    );
}

#[test]
fn a_midi_file_without_a_sidecar_opens_as_an_import_on_the_default_bank() {
    let folder = AssetFolder::new();
    let (old, _) = old_and_new(&folder);
    let lonely = folder.write("music/lonely.mid", &old);

    let opened = SongDocument::open(&lonely, folder.base()).expect("opens");
    assert_eq!(opened.document.path(), None);
    assert_eq!(
        opened.document.file().soundfont,
        super::DEFAULT_SOUND_FONT_PATH
    );
    let missing = SongDocument::open(&folder.path("music/absent.mid"), folder.base());
    assert!(matches!(missing, Err(SongOpenError::Missing { .. })));
}

#[test]
fn every_sample_the_reopened_song_names_is_on_disk_wherever_its_save_was_interrupted() {
    // The old song plays two samples; the new one has dropped one of them, so its save
    // prunes that file. Eight writes and removals: the pair, the dropped file, the
    // `LICENSE`, the finals and the pair's removals.
    for operations in 0..=8 {
        let folder = AssetFolder::new();
        folder.write("sounds/blip.wav", b"RIFF blip");
        folder.write("sounds/boop.wav", b"RIFF boop");
        let mut document = melody();
        for (name, path) in [("blip", "sounds/blip.wav"), ("boop", "sounds/boop.wav")] {
            let instrument = sample_instrument(name, path);
            document
                .apply(SongEdit::AddInstrument { instrument }, Gesture::Discrete)
                .expect("adds the sample");
        }
        document
            .save(&folder.path(SONG), folder.base())
            .expect("saves the old song");
        document
            .apply(
                SongEdit::RemoveInstrument {
                    name: "boop".to_string(),
                },
                Gesture::Discrete,
            )
            .expect("drops one");
        let mut store = InterruptedStore {
            operations_left: operations,
        };
        let saved = document.save_to(&mut store, &folder.path(SONG), folder.base());
        assert_eq!(
            saved.is_ok(),
            operations == 8,
            "after {operations} operations"
        );

        let opened = reopen(&folder).unwrap_or_else(|error| panic!("after {operations}: {error}"));
        for instrument in &opened.document.file().instruments {
            if let InstrumentSource::Sample { path, .. } = &instrument.source {
                assert!(
                    folder.read(path).is_some(),
                    "after {operations} operations: '{}' names {path}, which is gone",
                    instrument.name
                );
            }
        }
    }
}
