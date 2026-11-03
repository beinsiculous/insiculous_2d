//! Open reads a song's two files as they are and writes nothing: an unmapped `.mid` opens
//! in place on a scaffold sidecar, a `.mid` its sidecar fingerprints differently opens on
//! the disk's score, and a save interrupted at any write leaves finals that open with every
//! sample they name.

use super::io::open::open_from;
use super::io::VfsStore;
use super::test_support::{melody, melody_bytes, AssetFolder, InterruptedStore, NAPOLITAN};
use super::{Gesture, SaveIntent, SongDocument, SongEdit, SongOpenError};

const SONG: &str = "music/theme.mid";

fn reopen(folder: &AssetFolder) -> Result<super::OpenedSong, SongOpenError> {
    open_from(&VfsStore, &folder.path(SONG), folder.base())
}

#[test]
fn a_midi_file_without_a_sidecar_opens_in_place_on_the_projects_first_bank_with_its_map_unsaved() {
    let folder = AssetFolder::new();
    folder.write("soundfonts/strings.sf2", b"a bank");
    let lonely = folder.write("music/lonely.mid", &melody_bytes());

    let opened = SongDocument::open(&lonely, folder.base()).expect("opens");
    assert_eq!(opened.document.path(), Some(lonely.as_path()), "in place, not as an import");
    assert!(opened.document.is_dirty(), "the sidecar is not on disk yet");
    let file = opened.document.file();
    assert_eq!(file.soundfont, "soundfonts/strings.sf2");
    assert!(file.parts.is_empty() && file.bindings.is_empty() && file.looping);
    assert!(opened.notes[0].contains("no sidecar"), "{:?}", opened.notes);
    assert_eq!(folder.read("music/lonely.mid.ron"), None, "an open writes nothing");

    let missing = SongDocument::open(&folder.path("music/absent.mid"), folder.base());
    assert!(matches!(missing, Err(SongOpenError::Missing { .. })));
}

#[test]
fn a_midi_its_sidecar_fingerprints_differently_opens_on_the_disk_score_and_the_next_save_records_it() {
    let folder = AssetFolder::new();
    melody().save(&folder.path(SONG), folder.base()).expect("saves the melody");
    folder.write(SONG, NAPOLITAN);

    let mut opened = reopen(&folder).expect("opens");
    assert!(opened.notes[0].contains("changed outside the editor"), "{:?}", opened.notes);
    assert_eq!(opened.document.source_bytes(), NAPOLITAN);
    opened.document.save(&folder.path(SONG), folder.base()).expect("a map save");
    assert_eq!(folder.read(SONG).as_deref(), Some(NAPOLITAN), "the map save leaves the score");
    let reopened = reopen(&folder).expect("reopens");
    assert!(reopened.notes.is_empty(), "the fingerprint is the disk's now: {:?}", reopened.notes);
}

#[test]
fn a_midi_song_saves_and_reopens_under_its_whole_name() {
    let folder = AssetFolder::new();
    let target = folder.path("music/theme.midi");
    melody().save(&target, folder.base()).expect("saves");
    assert_eq!(folder.files_under("music"), ["LICENSE", "theme.midi", "theme.midi.ron"]);

    let opened = open_from(&VfsStore, &target, folder.base()).expect("opens");
    assert!(opened.notes.is_empty(), "{:?}", opened.notes);
    assert!(!opened.document.is_dirty());
}

/// Every sample each sidecar under `music/` names, by the sidecar it is named in.
fn named_samples(folder: &AssetFolder) -> Vec<(String, String)> {
    folder
        .files_under("music")
        .into_iter()
        .filter(|name| name.ends_with(".mid.ron"))
        .flat_map(|name| {
            let text = String::from_utf8(folder.read(&format!("music/{name}")).expect("read")).expect("text");
            let file = music_midi::parse_song_file(&name, &text).expect("parses");
            file.instruments
                .into_iter()
                .filter_map(move |instrument| match instrument.source {
                    music_midi::InstrumentSource::Sample { path, .. } => Some((name.clone(), path)),
                    music_midi::InstrumentSource::SoundFont { .. } => None,
                })
        })
        .collect()
}

fn with_samples(folder: &AssetFolder, names: &[&str]) -> SongDocument {
    let mut document = melody();
    for name in names {
        let path = format!("sounds/{name}.wav");
        folder.write(&path, format!("RIFF {name}").as_bytes());
        let instrument = super::test_support::sample_instrument(name, &path);
        document
            .apply(SongEdit::AddInstrument { instrument }, Gesture::Discrete)
            .expect("adds the sample");
    }
    document
}


#[test]
fn every_sample_the_finals_name_is_on_disk_wherever_a_save_was_interrupted() {
    // The old song plays two samples; the new map drops one, so its save prunes that file.
    // A map save makes four writes, renames and removals: the sidecar's temporary file, its
    // rename over the final, then the prune — the dropped file and the folder's LICENSE.
    for operations in 0..=4 {
        let folder = AssetFolder::new();
        let mut document = with_samples(&folder, &["blip", "boop"]);
        document.save(&folder.path(SONG), folder.base()).expect("saves the old song");
        document
            .apply(SongEdit::RemoveInstrument { name: "boop".to_string() }, Gesture::Discrete)
            .expect("drops one");
        let mut store = InterruptedStore {
            operations_left: operations,
        };
        let saved = document.save_to(&mut store, &folder.path(SONG), folder.base(), SaveIntent::default());
        assert_eq!(saved.is_ok(), operations == 4, "after {operations} operations");

        reopen(&folder).unwrap_or_else(|error| panic!("after {operations}: {error}"));
        for (sidecar, sample) in named_samples(&folder) {
            assert!(
                folder.read(&sample).is_some(),
                "after {operations} operations: {sidecar} names {sample}, which is gone"
            );
        }
    }
}

#[test]
fn song_init_writes_the_scaffold_beside_an_unmapped_midi_and_refuses_a_mapped_one() {
    let folder = AssetFolder::new();
    folder.write("soundfonts/strings.sf2", b"a bank");
    let score = melody_bytes();
    let lonely = folder.write("music/lonely.mid", &score);

    let sidecar = super::init_song(&lonely, folder.base()).expect("maps it");
    assert_eq!(sidecar, folder.path("music/lonely.mid.ron"));
    let opened = SongDocument::open(&lonely, folder.base()).expect("opens");
    assert!(opened.notes.is_empty(), "the scaffold fingerprints the score: {:?}", opened.notes);
    assert!(!opened.document.is_dirty(), "the sidecar is on disk");
    let file = opened.document.file();
    assert_eq!(file.soundfont, "soundfonts/strings.sf2");
    assert!(file.parts.is_empty() && file.instruments.is_empty() && file.bindings.is_empty() && file.looping);
    assert_eq!(folder.read("music/lonely.mid"), Some(score.clone()), "the score is never written");

    let mapped = super::init_song(&lonely, folder.base()).expect_err("it has a sidecar now");
    assert!(matches!(mapped, super::SongInitError::Mapped { .. }), "{mapped}");
    let outside = folder.write("downloads/tune.mid", &score);
    assert!(matches!(
        super::init_song(&outside, folder.base()),
        Err(super::SongInitError::OutsideHome { .. })
    ));
    assert!(folder.read("downloads/tune.mid.ron").is_none());
}
