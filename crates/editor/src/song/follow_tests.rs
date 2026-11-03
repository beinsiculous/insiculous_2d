//! The document follows its two files: an adopted score keeps the map, its unsaved edits
//! and its history and answers with the track diff's lines; a reloaded map replaces the map
//! and drops its history, whose entries replay a sidecar that is gone.

use music_midi::midly::{MetaMessage, Smf, TrackEventKind};
use music_midi::{Part, PartMode, Quantize, TrackSelector};

use super::test_support::{napolitan_song, AssetFolder, NAPOLITAN};
use super::{midi_fingerprint, Gesture, SongDocument, SongEdit};

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

#[test]
fn an_adopted_score_keeps_the_dirty_map_and_its_history_and_returns_the_diffs_lines() {
    let folder = AssetFolder::new();
    let mut document = mapped_napolitan(&folder);
    document
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("an unsaved map edit");
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

    assert_eq!(document.undo(), Some("SetLoop"), "the map's history is kept");
    assert!(!document.is_dirty(), "back at the saved map");
}

#[test]
fn an_adopted_score_under_a_saved_map_leaves_the_document_clean() {
    let folder = AssetFolder::new();
    let mut document = mapped_napolitan(&folder);
    document.adopt_score(napolitan_renamed(2, "oboe")).expect("adopts");
    assert!(!document.is_dirty(), "the score is the file's and the map is saved");
    assert_eq!(document.undo(), Some("AddPart"), "the saved map's own entries stay");
}

#[test]
fn a_reloaded_map_replaces_the_map_and_drops_its_history() {
    let folder = AssetFolder::new();
    let mut document = mapped_napolitan(&folder);
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
    assert_eq!(document.source_bytes(), NAPOLITAN, "the score is untouched");
    assert_eq!(document.undo(), None, "every entry is gone");
    assert!(!document.is_dirty(), "the reloaded map is the one on disk");

    let error = document.reload_map("SongFile(").expect_err("a broken sidecar");
    assert!(error.to_string().contains("napolitan.mid.ron"), "{error}");
    assert_eq!(document.file().parts[0].name, "drums", "a refused reload changes nothing");
}
