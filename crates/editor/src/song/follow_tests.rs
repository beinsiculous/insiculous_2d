//! The document follows its two files: an adopted score keeps the map and its unsaved
//! edits and answers with the track diff's lines; a reloaded map keeps the score and its
//! unsaved edits; each drops the history entries of the file it replaced; and only a score
//! edit — a batch holding one included — makes the score the engine's.

use music_midi::midly::{MetaMessage, Smf, TrackEventKind};
use music_midi::{Part, PartMode, Quantize, TrackSelector, UnsupportedReport};

use super::test_support::{napolitan_song, note_numbers, AssetFolder, NAPOLITAN};
use super::{midi_fingerprint, EditingPolicy, Gesture, SongDocument, SongEdit, SongEditError};

const SONG: &str = "music/napolitan.mid";

/// The Napolitan file with its track at `index` renamed `name`.
fn napolitan_renamed(index: usize, name: &'static str) -> Vec<u8> {
    let mut smf = Smf::parse(NAPOLITAN).expect("the fixture parses");
    for event in &mut smf.tracks[index] {
        if let TrackEventKind::Meta(MetaMessage::TrackName(_)) = event.kind {
            event.kind = TrackEventKind::Meta(MetaMessage::TrackName(name.as_bytes()));
        }
    }
    let mut bytes = Vec::new();
    smf.write_std(&mut bytes).expect("the copy writes");
    bytes
}

/// The Napolitan song saved with `melody` playing the track named `flute`.
fn mapped_napolitan(folder: &AssetFolder) -> SongDocument {
    let mut document = napolitan_song(folder, SONG);
    let part = Part {
        name: "melody".to_string(),
        track: TrackSelector::Name("flute".to_string()),
        mode: PartMode::Loop,
        quantize: Quantize::Off,
    };
    document
        .apply(SongEdit::AddPart { part }, Gesture::Discrete)
        .expect("the part lands");
    document.save(&folder.path(SONG), folder.base()).expect("the map saves");
    document
}

fn flute_note() -> SongEdit {
    SongEdit::AddNote {
        track: 1,
        tick: 0,
        key: 72,
        velocity: 90,
        duration_ticks: 240,
    }
}

#[test]
fn an_adopted_score_keeps_the_dirty_map_drops_the_score_entries_and_returns_the_diffs_lines() {
    let folder = AssetFolder::new();
    let mut document = mapped_napolitan(&folder);
    document
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("an unsaved map edit");
    document.apply(flute_note(), Gesture::Discrete).expect("a score edit");
    let renamed = napolitan_renamed(1, "Flute (lead)");

    let lines = document.adopt_score(renamed.clone()).expect("adopts");
    assert_eq!(
        lines[0],
        "track `flute` is gone; index 1 is now `Flute (lead)` — renamed in MIDI Evil? Edit the sidecar's part `melody`"
    );
    assert!(!document.file().looping, "the map's unsaved edit is kept");
    assert!(document.is_dirty(), "and still unsaved");
    assert_eq!(document.source_bytes(), renamed.as_slice());
    assert_eq!(document.score_fingerprint(), midi_fingerprint(&renamed));
    assert!(!document.is_score_authored());

    assert_eq!(document.undo().expect("undoes"), Some("SetLoop"), "the note's entry is gone");
    assert!(!document.is_dirty(), "back at the saved map");
}

#[test]
fn an_adopted_score_under_a_saved_map_leaves_the_document_clean() {
    let folder = AssetFolder::new();
    let mut document = mapped_napolitan(&folder);
    document.apply(flute_note(), Gesture::Discrete).expect("a score edit");
    assert!(document.is_dirty());

    document.adopt_score(NAPOLITAN.to_vec()).expect("adopts");
    assert!(!document.is_dirty(), "the score is the file's and the map is saved");
    assert_eq!(document.undo().expect("undoes"), Some("AddPart"), "the saved map's own entries stay");
}

#[test]
fn a_reloaded_map_replaces_the_map_keeps_the_scores_unsaved_edits_and_drops_the_map_entries() {
    let folder = AssetFolder::new();
    let mut document = mapped_napolitan(&folder);
    document.apply(flute_note(), Gesture::Discrete).expect("a score edit");
    document
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("a map edit the reload replaces");
    let text = r#"SongFile(version: 1, soundfont: "soundfonts/cast.sf2",
        parts: [(name: "drums", track: Index(2))])"#;

    document.reload_map(text).expect("reloads");
    assert_eq!(document.file().parts.len(), 1);
    assert_eq!(document.file().parts[0].name, "drums");
    assert!(document.file().looping, "the file's loop, not the edit's");
    assert_eq!(document.map_fingerprint(), Some(midi_fingerprint(text.as_bytes())));
    assert!(document.is_score_authored(), "the score's edit is kept");
    assert_eq!(document.undo().expect("undoes"), Some("AddNote"));
    assert_eq!(document.undo().expect("nothing older"), None, "every map entry is gone");

    let error = document.reload_map("SongFile(").expect_err("a broken sidecar");
    assert!(error.to_string().contains("napolitan.mid.ron"), "{error}");
    assert_eq!(document.file().parts[0].name, "drums", "a refused reload changes nothing");
}

#[test]
fn only_a_score_edit_makes_the_score_the_engines_and_an_undone_one_does_too() {
    let folder = AssetFolder::new();
    let mut document = mapped_napolitan(&folder);
    document
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("a map edit");
    assert!(!document.is_score_authored());
    let nudge = SongEdit::Batch(vec![
        SongEdit::SetLoop { looping: true },
        SongEdit::Batch(vec![flute_note()]),
    ]);
    assert!(nudge.changes_score(), "a batch is a score edit when any member is, however deep");
    document.apply(nudge, Gesture::Discrete).expect("the batch lands");
    assert!(document.is_score_authored());

    document.save(&folder.path(SONG), folder.base()).expect("saves");
    assert!(!document.is_score_authored(), "the save wrote the score");
    document.undo().expect("undoes the batch");
    assert!(
        document.is_score_authored(),
        "the undone note leaves a score the file on disk does not hold"
    );
}

#[test]
fn an_edit_undone_before_any_save_leaves_the_score_the_files_and_a_redo_takes_it_back() {
    let folder = AssetFolder::new();
    let mut document = mapped_napolitan(&folder);
    document.apply(flute_note(), Gesture::Discrete).expect("a note lands");
    assert!(document.is_score_authored());
    document.undo().expect("undoes the note");
    assert!(!document.is_score_authored(), "undone before a save, the score is the file's again");
    document.redo().expect("redoes it");
    assert!(document.is_score_authored(), "and redone, it is the engine's");
}

#[test]
fn a_score_edit_stays_the_engines_after_enough_map_edits_to_evict_it_from_the_history() {
    let folder = AssetFolder::new();
    let mut document = mapped_napolitan(&folder);
    document.apply(flute_note(), Gesture::Discrete).expect("a note lands");
    for turn in 0..120 {
        document
            .apply(SongEdit::SetLoop { looping: turn % 2 == 0 }, Gesture::Discrete)
            .expect("a map edit lands");
    }
    assert!(document.is_score_authored(), "the history forgot the note's entry; the score did not");
    document.save(&folder.path(SONG), folder.base()).expect("saves");
    assert!(!document.is_score_authored(), "the save wrote the edited score");
    let (data, _) = music_midi::SongData::from_smf(&folder.read(SONG).expect("the saved .mid")).expect("reads");
    assert_eq!(data.tracks()[1].note_count(), document.data().tracks()[1].note_count(), "the note is on disk");
}

#[test]
fn an_edit_after_an_undo_past_a_save_is_still_the_engines_score() {
    let folder = AssetFolder::new();
    let mut document = mapped_napolitan(&folder);
    document.apply(flute_note(), Gesture::Discrete).expect("the first note lands");
    document.save(&folder.path(SONG), folder.base()).expect("saves the first");
    document.undo().expect("undoes the first");
    let mut other = flute_note();
    if let SongEdit::AddNote { tick, .. } = &mut other {
        *tick += 960;
    }
    document.apply(other, Gesture::Discrete).expect("another note lands");
    assert!(document.is_score_authored(), "the disk holds the first note and the song the second: no count tells them apart, the bytes do");
}

#[test]
fn a_gesture_the_history_merged_undoes_as_one_and_leaves_the_score_the_files() {
    let folder = AssetFolder::new();
    let mut document = mapped_napolitan(&folder);
    document.apply(flute_note(), Gesture::Discrete).expect("a note to drag");
    document.save(&folder.path(SONG), folder.base()).expect("saves it");
    let notes = vec![*note_numbers(&document).last().expect("the note")];
    for _ in 0..10 {
        document
            .apply(SongEdit::MoveNotes { notes: notes.clone(), tick_delta: 48 }, Gesture::Continuing)
            .expect("a frame of the drag");
    }
    assert!(document.is_score_authored(), "ten frames moved the note");
    document.undo().expect("undoes the whole drag");
    assert!(!document.is_score_authored(), "one undo took the merged gesture back to the saved score");
}

#[test]
fn a_score_that_cannot_be_exported_is_refused_by_the_one_policy_stays_the_files_and_its_map_saves() {
    let editable = UnsupportedReport::default();
    assert_eq!(EditingPolicy::for_score(&editable, true), EditingPolicy::Full);
    let refused = EditingPolicy::for_score(&editable, false);
    assert!(
        matches!(&refused, EditingPolicy::Refused { reason } if reason.contains("written back")),
        "no export: the one policy refuses, so the sidebar and the API say what apply does"
    );
    let folder = AssetFolder::new();
    let mut document = mapped_napolitan(&folder);
    document.baseline_export = None;
    document.editing = refused;
    assert!(!document.is_score_authored(), "no export to compare: the score is the file's");
    assert!(
        matches!(document.apply(flute_note(), Gesture::Discrete), Err(SongEditError::Refused { .. })),
        "a score edit would never be written back"
    );
    document.apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete).expect("the map still edits");
    document.save(&folder.path(SONG), folder.base()).expect("and saves");
    assert_eq!(folder.read(SONG).as_deref(), Some(NAPOLITAN), "the map save left the file's bytes");
}
