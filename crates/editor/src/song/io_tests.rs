//! Save: the refusals that write nothing, the samples and their `LICENSE`, the round trip
//! through a real folder, and the song folders a save may write into.

use std::path::Path;

use super::test_support::{exported, melody, sample_instrument, AssetFolder};
use super::{midi_fingerprint, Gesture, SongDocument, SongEdit, SongSaveError};

const BLIP: &[u8] = b"RIFF blip recording";
const OTHER_BLIP: &[u8] = b"RIFF another recording entirely";

fn with_instrument(document: &mut SongDocument, instrument: music_midi::Instrument) {
    document
        .apply(SongEdit::AddInstrument { instrument }, Gesture::Discrete)
        .expect("the instrument lands");
}

/// The `Files:` tokens of a `LICENSE`, as the provenance gate reads them.
fn license_files(license: &str) -> Vec<String> {
    license
        .lines()
        .filter_map(|line| line.strip_prefix("Files:"))
        .flat_map(|files| files.split_whitespace().map(str::to_string))
        .collect()
}

#[test]
fn fnv_one_a_is_the_fingerprint_the_sidecar_carries() {
    assert_eq!(midi_fingerprint(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(midi_fingerprint(b"a"), 0xaf63_dc4c_8601_ec8c);
}

#[test]
fn a_saved_song_reopens_with_every_note_and_its_sidecar_and_leaves_no_journal() {
    let folder = AssetFolder::new();
    let mut document = melody();
    document
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("edits");
    let target = folder.path("music/theme.mid");
    document.save(&target, folder.base()).expect("saves");

    assert_eq!(document.path(), Some(target.as_path()));
    assert_eq!(
        folder.files_under("music"),
        ["theme.mid", "theme.mid.ron"],
        "no .new is left"
    );
    let midi = folder.read("music/theme.mid").expect("the MIDI file");
    assert_eq!(
        document.file().midi_fingerprint,
        Some(midi_fingerprint(&midi))
    );

    let opened = SongDocument::open(&target, folder.base()).expect("reopens");
    assert!(
        opened.notes.is_empty(),
        "nothing to settle: {:?}",
        opened.notes
    );
    assert!(!opened.document.is_dirty());
    assert_eq!(exported(&opened.document), exported(&document));
    assert!(!opened.document.file().looping);
}

#[test]
fn a_sample_without_provenance_refuses_the_save_naming_the_instrument_and_writes_nothing() {
    let folder = AssetFolder::new();
    folder.write("sounds/blip.wav", BLIP);
    let mut document = melody();
    let mut instrument = sample_instrument("blip", "sounds/blip.wav");
    instrument.provenance = None;
    with_instrument(&mut document, instrument);

    let error = document
        .save(&folder.path("music/theme.mid"), folder.base())
        .expect_err("refused");
    assert!(
        matches!(&error, SongSaveError::MissingProvenance { instrument, field: "provenance" } if instrument == "blip")
    );
    assert!(error.to_string().contains("instruments['blip']"), "{error}");
    assert!(
        folder.files_under("music").is_empty(),
        "nothing was written"
    );
    assert!(document.is_dirty());
}

#[test]
fn the_ninth_instrument_is_refused_at_save_too_before_any_write() {
    let folder = AssetFolder::new();
    let mut document = melody();
    for index in 0..9 {
        let mut instrument = sample_instrument(&format!("sample {index}"), "sounds/blip.wav");
        instrument.source = music_midi::InstrumentSource::SoundFont {
            path: "soundfonts/cast.sf2".to_string(),
            bank: 0,
            preset: 0,
        };
        document.file.instruments.push(instrument);
    }
    let error = document
        .save(&folder.path("music/theme.mid"), folder.base())
        .expect_err("a sidecar the loader refuses is never written");
    assert!(matches!(
        error,
        SongSaveError::Schema(music_midi::SongFileProblem::TooManyInstruments { .. })
    ));
    assert!(folder.files_under("music").is_empty());
}

#[test]
fn a_saved_sample_lands_in_the_songs_folder_named_in_its_license_and_passes_the_gate_grammar() {
    let folder = AssetFolder::new();
    folder.write("sounds/blip.wav", BLIP);
    let mut document = melody();
    with_instrument(&mut document, sample_instrument("blip", "sounds/blip.wav"));
    document
        .save(&folder.path("music/theme.mid"), folder.base())
        .expect("saves");

    assert_eq!(
        folder.files_under("music/samples/theme"),
        ["LICENSE", "blip.wav"]
    );
    assert_eq!(
        folder.read("music/samples/theme/blip.wav").as_deref(),
        Some(BLIP)
    );
    let license = String::from_utf8(folder.read("music/samples/theme/LICENSE").expect("written"))
        .expect("text");
    assert_eq!(license_files(&license), ["blip.wav"]);
    assert!(
        license.contains("Copyright: 2026 Jesse\nLicense: MIT"),
        "{license}"
    );
    let source = &document.file().instruments[0].source;
    assert!(
        matches!(source, music_midi::InstrumentSource::Sample { path, .. } if path == "music/samples/theme/blip.wav"),
        "the instrument follows its sample: {source:?}"
    );
    assert_eq!(
        folder.read("sounds/blip.wav").as_deref(),
        Some(BLIP),
        "the source file is untouched"
    );
}

#[test]
fn a_removed_instruments_sample_is_pruned_and_its_stanza_gone() {
    let folder = AssetFolder::new();
    folder.write("sounds/blip.wav", BLIP);
    folder.write("sounds/boop.wav", OTHER_BLIP);
    let mut document = melody();
    with_instrument(&mut document, sample_instrument("blip", "sounds/blip.wav"));
    with_instrument(&mut document, sample_instrument("boop", "sounds/boop.wav"));
    let target = folder.path("music/theme.mid");
    document.save(&target, folder.base()).expect("saves both");
    assert_eq!(
        folder.files_under("music/samples/theme"),
        ["LICENSE", "blip.wav", "boop.wav"]
    );

    document
        .apply(
            SongEdit::RemoveInstrument {
                name: "boop".to_string(),
            },
            Gesture::Discrete,
        )
        .expect("removes one");
    document.save(&target, folder.base()).expect("saves again");
    assert_eq!(
        folder.files_under("music/samples/theme"),
        ["LICENSE", "blip.wav"]
    );
    let license = String::from_utf8(folder.read("music/samples/theme/LICENSE").expect("written"))
        .expect("text");
    assert_eq!(license_files(&license), ["blip.wav"]);

    document
        .apply(
            SongEdit::RemoveInstrument {
                name: "blip".to_string(),
            },
            Gesture::Discrete,
        )
        .expect("removes the other");
    document
        .save(&target, folder.base())
        .expect("saves without samples");
    assert!(
        folder.files_under("music/samples/theme").is_empty(),
        "the folder goes with its last sample"
    );
}

#[test]
fn a_same_named_sample_with_other_bytes_gets_a_suffix_and_its_instrument_follows() {
    let folder = AssetFolder::new();
    folder.write("sounds/blip.wav", BLIP);
    folder.write("other/blip.wav", OTHER_BLIP);
    let mut document = melody();
    with_instrument(&mut document, sample_instrument("first", "sounds/blip.wav"));
    with_instrument(&mut document, sample_instrument("second", "other/blip.wav"));
    document
        .save(&folder.path("music/theme.mid"), folder.base())
        .expect("saves");

    let paths: Vec<String> = document
        .file()
        .instruments
        .iter()
        .filter_map(|instrument| match &instrument.source {
            music_midi::InstrumentSource::Sample { path, .. } => Some(path.clone()),
            music_midi::InstrumentSource::SoundFont { .. } => None,
        })
        .collect();
    let suffixed = format!("blip-{:08x}.wav", midi_fingerprint(OTHER_BLIP) as u32);
    assert_eq!(
        paths,
        [
            "music/samples/theme/blip.wav".to_string(),
            format!("music/samples/theme/{suffixed}"),
        ]
    );
    assert_eq!(
        folder
            .read(&format!("music/samples/theme/{suffixed}"))
            .as_deref(),
        Some(OTHER_BLIP)
    );
    let license = String::from_utf8(folder.read("music/samples/theme/LICENSE").expect("written"))
        .expect("text");
    assert_eq!(
        license_files(&license),
        [suffixed, "blip.wav".to_string()],
        "one stanza each, by name"
    );
}

#[test]
fn a_never_saved_songs_held_samples_reach_its_folder_on_the_first_save() {
    let folder = AssetFolder::new();
    let mut document = SongDocument::new_song().expect("a blank song");
    document.stage_sample("upload/blip.wav", BLIP.to_vec());
    with_instrument(&mut document, sample_instrument("blip", "upload/blip.wav"));
    assert!(
        folder.files_under("").is_empty(),
        "nothing is staged on disk"
    );

    document
        .save(&folder.path("music/first.mid"), folder.base())
        .expect("saves");
    assert_eq!(
        folder.read("music/samples/first/blip.wav").as_deref(),
        Some(BLIP)
    );
    assert!(document.pending_samples().is_empty());
}

#[test]
fn an_agents_song_saves_under_ai_music_reopens_and_saves_in_place() {
    let folder = AssetFolder::new();
    let target = folder.path("ai/music/ai_waltz.mid");
    let mut document = melody();
    document
        .save(&target, folder.base())
        .expect("saves under ai/music");

    let mut reopened = SongDocument::open(&target, folder.base())
        .expect("reopens")
        .document;
    assert_eq!(reopened.path(), Some(target.as_path()));
    reopened
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("edits");
    reopened
        .save(&target, folder.base())
        .expect("saves in place");
    assert_eq!(
        folder.files_under("ai/music"),
        ["ai_waltz.mid", "ai_waltz.mid.ron"]
    );
}

#[test]
fn a_save_outside_the_song_folders_is_refused_and_an_imported_original_never_written() {
    let folder = AssetFolder::new();
    let mut source = melody();
    source
        .save(&folder.path("music/source.mid"), folder.base())
        .expect("saves the source");
    let original = folder.read("music/source.mid").expect("the source file");
    let outside = folder.write("downloads/tune.mid", &original);

    let opened = SongDocument::open(&outside, folder.base()).expect("opens");
    assert_eq!(
        opened.document.path(),
        None,
        "outside the song folders it is an import"
    );
    assert_eq!(opened.document.imported_from(), Some(outside.as_path()));
    assert!(opened.document.is_dirty());

    let mut document = opened.document;
    let error = document
        .save(&outside, folder.base())
        .expect_err("never written");
    assert!(matches!(error, SongSaveError::OutsideHome { .. }));
    let relative = document
        .save(Path::new("music/tune.mid"), folder.base())
        .expect_err("a target not under the asset base is outside, however it reads");
    assert!(matches!(relative, SongSaveError::OutsideHome { .. }));
    document
        .save(&folder.path("music/tune.mid"), folder.base())
        .expect("saves as");
    assert_eq!(
        folder.read("downloads/tune.mid"),
        Some(original),
        "the original is untouched"
    );
    assert_eq!(document.imported_from(), None);
}
