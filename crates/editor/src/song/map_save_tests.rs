//! Save writes the map and leaves the score: a map save fingerprints the bytes it loaded,
//! a Save As copies them, an edited score is written only by the transition's rule — kept
//! as a recovery when another program rewrote the `.mid` under it — and both files are
//! compared by content immediately before the save writes.

use std::time::SystemTime;

use super::test_support::{
    add_note, exported, melody, napolitan_song, AssetFolder, RewritingStore, NAPOLITAN,
};
use super::{midi_fingerprint, Gesture, SaveIntent, SongEdit, SongSaveError};

const SONG: &str = "music/napolitan.mid";
const SIDECAR: &str = "music/napolitan.mid.ron";

fn sidecar_fingerprint(folder: &AssetFolder, relative: &str) -> Option<u64> {
    let text = String::from_utf8(folder.read(relative).expect("the sidecar")).expect("text");
    music_midi::parse_song_file(relative, &text)
        .expect("parses")
        .midi_fingerprint
}

fn stamp(folder: &AssetFolder, relative: &str) -> common::vfs::ChangeStamp {
    common::vfs::change_stamp(&folder.path(relative))
        .expect("stamped")
        .expect("a native file keeps its modified time")
}

#[test]
fn a_map_save_leaves_the_midis_bytes_and_stamp_and_fingerprints_the_loaded_bytes() {
    let (data, _) = music_midi::SongData::from_smf(NAPOLITAN).expect("reads");
    assert_ne!(
        data.write_smf().expect("exports"),
        NAPOLITAN,
        "the fixture's export differs from its bytes, or this test proves nothing"
    );
    let folder = AssetFolder::new();
    let mut document = napolitan_song(&folder, SONG);
    let before = stamp(&folder, SONG);
    document
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("a map edit");

    document.save(&folder.path(SONG), folder.base()).expect("saves");
    assert_eq!(folder.read(SONG).as_deref(), Some(NAPOLITAN));
    assert_eq!(stamp(&folder, SONG), before, "the .mid is never written");
    assert_eq!(sidecar_fingerprint(&folder, SIDECAR), Some(midi_fingerprint(NAPOLITAN)));
    assert!(!document.is_dirty());
}

#[test]
fn save_as_to_a_new_and_to_an_overwritten_target_writes_the_loaded_bytes() {
    let folder = AssetFolder::new();
    let mut document = napolitan_song(&folder, SONG);
    document.save(&folder.path("music/copy.mid"), folder.base()).expect("a new target");
    assert_eq!(folder.read("music/copy.mid").as_deref(), Some(NAPOLITAN));
    assert_eq!(sidecar_fingerprint(&folder, "music/copy.mid.ron"), Some(midi_fingerprint(NAPOLITAN)));

    let mut other = melody();
    other.save(&folder.path("music/other.mid"), folder.base()).expect("another song");
    document.save(&folder.path("music/other.mid"), folder.base()).expect("over it");
    assert_eq!(folder.read("music/other.mid").as_deref(), Some(NAPOLITAN));
    assert_eq!(document.path(), Some(folder.path("music/other.mid").as_path()));
}

#[test]
fn save_as_of_an_edited_score_writes_the_edited_score_and_the_document_is_clean_against_it() {
    let folder = AssetFolder::new();
    let mut document = napolitan_song(&folder, SONG);
    document.apply(add_note(0, 72), Gesture::Discrete).expect("a score edit");
    let edited = exported(&document);

    document.save(&folder.path("music/copy.mid"), folder.base()).expect("saves as");
    assert_eq!(folder.read("music/copy.mid"), Some(edited.clone()));
    assert_eq!(document.source_bytes(), edited.as_slice());
    assert_eq!(document.score_fingerprint(), midi_fingerprint(&edited));
    assert!(!document.is_score_authored() && !document.is_dirty());
    assert_eq!(folder.read(SONG).as_deref(), Some(NAPOLITAN), "the song it came from is untouched");
}

#[test]
fn an_edited_score_saved_over_a_midi_another_program_rewrote_lands_as_a_recovery() {
    let other_program = exported(&melody());
    // Rewritten before the save starts, and while it is between its journal and its finals.
    for after_operation in [None, Some(2)] {
        let folder = AssetFolder::new();
        let mut document = napolitan_song(&folder, SONG);
        document.apply(add_note(0, 72), Gesture::Discrete).expect("a score edit");
        let edited = exported(&document);
        let mut store = RewritingStore {
            after_operation: after_operation.unwrap_or(0),
            path: folder.path(SONG),
            bytes: other_program.clone(),
            landed: 0,
        };
        if after_operation.is_none() {
            folder.write(SONG, &other_program);
        }

        let notes = document
            .save_to(&mut store, &folder.path(SONG), folder.base(), SaveIntent::default())
            .expect("the save lands");
        let kept = format!("napolitan.recovered-{:016x}.mid", midi_fingerprint(&edited));
        assert_eq!(folder.read(SONG), Some(other_program.clone()), "{after_operation:?}: the disk's file is untouched");
        assert_eq!(folder.read(&format!("music/{kept}")), Some(edited), "{after_operation:?}");
        assert!(notes[0].contains(&kept), "{after_operation:?}: {notes:?}");
        assert_eq!(sidecar_fingerprint(&folder, SIDECAR), Some(midi_fingerprint(&other_program)));
        assert_eq!(document.score_fingerprint(), midi_fingerprint(&other_program), "it follows the disk");
        assert!(!document.is_score_authored() && !document.is_dirty());
    }
}

#[test]
fn a_sidecar_rewritten_at_the_same_length_and_stamp_refuses_the_save_and_overwrite_lands() {
    let folder = AssetFolder::new();
    let mut document = napolitan_song(&folder, SONG);
    document
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("a map edit");
    let before = stamp(&folder, SIDECAR);
    let modified = std::fs::metadata(folder.path(SIDECAR))
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let text = String::from_utf8(folder.read(SIDECAR).expect("the sidecar")).expect("text");
    let rewritten = text.replace("cast.sf2", "fast.sf2");
    assert_eq!(rewritten.len(), text.len());
    folder.write(SIDECAR, rewritten.as_bytes());
    std::fs::File::options()
        .write(true)
        .open(folder.path(SIDECAR))
        .and_then(|file| file.set_modified(modified))
        .expect("the stamp put back");
    assert_eq!(stamp(&folder, SIDECAR), before, "a rewrite the watcher's stamp cannot see");

    let error = document
        .save(&folder.path(SONG), folder.base())
        .expect_err("the compare is by content");
    let SongSaveError::SidecarChanged { loaded, on_disk, .. } = &error else {
        panic!("{error}");
    };
    assert_eq!(*on_disk, Some(midi_fingerprint(rewritten.as_bytes())));
    assert_ne!(*loaded, *on_disk);
    assert!(error.to_string().contains("the map changed on disk since you loaded it"), "{error}");
    assert_eq!(folder.read(SIDECAR), Some(rewritten.into_bytes()), "nothing was written");
    assert!(document.is_dirty());

    document
        .save_with(&folder.path(SONG), folder.base(), SaveIntent::OVERWRITE)
        .expect("Overwrite passes the compare it asked to pass");
    let saved = String::from_utf8(folder.read(SIDECAR).expect("the sidecar")).expect("text");
    assert!(saved.contains("cast.sf2") && saved.contains("looping: false"), "{saved}");
    document.save(&folder.path(SONG), folder.base()).expect("and the next save compares against it");
}

#[test]
fn a_midi_rewritten_between_the_load_and_a_map_save_is_adopted_and_the_save_fingerprints_it() {
    let folder = AssetFolder::new();
    let mut document = napolitan_song(&folder, SONG);
    document
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("a map edit");
    let other_program = exported(&melody());
    folder.write(SONG, &other_program);

    let notes = document.save(&folder.path(SONG), folder.base()).expect("saves");
    assert!(notes[0].contains("changed on disk: the song follows it"), "{notes:?}");
    assert_eq!(folder.read(SONG), Some(other_program.clone()), "a map save never writes the score");
    assert_eq!(sidecar_fingerprint(&folder, SIDECAR), Some(midi_fingerprint(&other_program)));
    assert_eq!(document.score_fingerprint(), midi_fingerprint(&other_program));
    assert!(!document.file().looping, "the map's edit was saved");
    assert!(!document.is_dirty());
}

#[test]
fn a_sidecar_rewritten_while_the_home_license_was_being_written_refuses_and_puts_the_license_back() {
    let folder = AssetFolder::new();
    let mut document = napolitan_song(&folder, SONG);
    let license_before = folder.read("music/LICENSE").expect("the first save wrote the home LICENSE");
    document
        .apply(
            SongEdit::SetProvenance {
                provenance: music_midi::Provenance {
                    author: "Someone Else".to_string(),
                    license: "CC0-1.0".to_string(),
                },
            },
            Gesture::Discrete,
        )
        .expect("a provenance edit, which changes the home LICENSE's stanza");
    let sidecar = crate::external_tools::midi_sidecar_path(&folder.path(SONG));
    let mut rewritten = std::fs::read(&sidecar).expect("the sidecar");
    rewritten.extend_from_slice(b"\n// rewritten by another program\n");
    // The map save's writes: the sidecar's journal, then the home LICENSE; the rewrite
    // lands right after the LICENSE, where the second look must catch it.
    let mut store = RewritingStore {
        after_operation: 2,
        path: sidecar.clone(),
        bytes: rewritten,
        landed: 0,
    };

    let refused = document.save_to(&mut store, &folder.path(SONG), folder.base(), SaveIntent::default());
    assert!(matches!(refused, Err(super::SongSaveError::SidecarChanged { .. })), "{refused:?}");
    assert_eq!(folder.read("music/LICENSE"), Some(license_before), "the LICENSE holds what it held");
    assert_eq!(folder.read("music/napolitan.mid.ron.new"), None, "this save's journal is gone");
    assert!(document.is_dirty(), "nothing was saved");
}
