//! Save writes the map and leaves the score: a map save fingerprints the bytes it loaded, a
//! Save As copies them, both files are compared by content immediately before the save
//! writes, and each final is a temporary file renamed over its target, so a save stopped
//! at the rename leaves the old finals whole.

use std::time::SystemTime;

use super::test_support::{
    melody, melody_bytes, napolitan_song, AssetFolder, RenameFailingStore, RewritingStore,
    NAPOLITAN,
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
fn a_save_whose_rename_fails_leaves_the_old_finals_whole_and_no_temporary_file() {
    let folder = AssetFolder::new();
    let mut document = napolitan_song(&folder, SONG);
    let sidecar_before = folder.read(SIDECAR).expect("the saved sidecar");
    document
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("a map edit");

    for target in [SONG, "music/copy.mid"] {
        let error = document
            .save_to(&mut RenameFailingStore, &folder.path(target), folder.base(), SaveIntent::default())
            .expect_err("the rename fails");
        assert!(matches!(error, SongSaveError::Write { .. }), "{target}: {error}");
        assert_eq!(folder.read(SIDECAR), Some(sidecar_before.clone()), "{target}: the old map is whole");
        assert_eq!(folder.read(SONG).as_deref(), Some(NAPOLITAN), "{target}: and the old score");
        assert_eq!(folder.read("music/copy.mid"), None, "{target}: a Save As target never appears");
        assert_eq!(folder.read("music/copy.mid.ron"), None, "{target}");
        let leftovers: Vec<String> = folder
            .files_under("music")
            .into_iter()
            .filter(|name| name.ends_with(".saving"))
            .collect();
        assert!(leftovers.is_empty(), "{target}: {leftovers:?}");
        assert!(document.is_dirty(), "{target}: nothing was saved");
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
    let other_program = melody_bytes();
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
    // The map save's first write is the home LICENSE; the rewrite lands right after it,
    // where the second look must catch it.
    let mut store = RewritingStore {
        after_operation: 1,
        path: sidecar.clone(),
        bytes: rewritten.clone(),
        landed: 0,
    };

    let refused = document.save_to(&mut store, &folder.path(SONG), folder.base(), SaveIntent::default());
    assert!(matches!(refused, Err(SongSaveError::SidecarChanged { .. })), "{refused:?}");
    assert_eq!(folder.read("music/LICENSE"), Some(license_before), "the LICENSE holds what it held");
    assert_eq!(std::fs::read(&sidecar).ok(), Some(rewritten), "the other program's map is left as it wrote it");
    assert!(document.is_dirty(), "nothing was saved");
}
