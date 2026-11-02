//! The diff over the Napolitan fixture (`control track`, `flute`, `tambourin`, `uppera`,
//! `lowera`) and copies of it built here with midly: a track renamed, two tracks swapped,
//! a track added, a name duplicated — the shapes MIDI Evil's saves take.

use music_midi::midly::num::{u28, u4, u7};
use music_midi::midly::{MetaMessage, MidiMessage, Smf, TrackEvent, TrackEventKind};
use music_midi::{parse_song_file, SongData, SongFile};

use super::*;

const NAPOLITAN: &[u8] = include_bytes!("../../../music_midi/fixtures/dansenapolitaine.mid");

/// `melody` by the name `flute`, `drums` by index 2 (`tambourin`), `bass_line` by the name
/// `lowera`.
const SIDECAR: &str = r#"SongFile(
    version: 1,
    soundfont: "soundfonts/cast.sf2",
    parts: [
        (name: "melody", track: Name("flute")),
        (name: "drums", track: Index(2)),
        (name: "bass_line", track: Name("lowera")),
    ],
)"#;

fn sidecar() -> SongFile {
    parse_song_file("music/napolitan.mid.ron", SIDECAR).expect("the sidecar parses")
}

fn napolitan() -> SongData {
    SongData::from_smf(NAPOLITAN).expect("the fixture reads").0
}

/// The Napolitan file changed by `change`, written back and read again.
fn edited(change: impl FnOnce(&mut Vec<Vec<TrackEvent<'static>>>)) -> SongData {
    let smf = Smf::parse(NAPOLITAN).expect("the fixture parses");
    let mut tracks = smf.tracks;
    change(&mut tracks);
    let mut bytes = Vec::new();
    Smf {
        header: smf.header,
        tracks,
    }
    .write_std(&mut bytes)
    .expect("the copy writes");
    SongData::from_smf(&bytes).expect("the copy reads").0
}

/// Rename the track at `index` by its name event.
fn rename(tracks: &mut [Vec<TrackEvent<'static>>], index: usize, name: &'static str) {
    for event in &mut tracks[index] {
        if let TrackEventKind::Meta(MetaMessage::TrackName(_)) = event.kind {
            event.kind = TrackEventKind::Meta(MetaMessage::TrackName(name.as_bytes()));
        }
    }
}

fn diff(file: &SongFile, old: &SongData, new: &SongData) -> TrackDiff {
    diff_tracks(&TrackListing::of(file, old), &TrackListing::of(file, new))
}

#[test]
fn the_same_score_diffs_to_nothing() {
    let data = napolitan();
    assert_eq!(diff(&sidecar(), &data, &data), TrackDiff::default());
}

#[test]
fn a_named_track_that_moved_keeps_its_part_while_the_index_it_left_is_refused_naming_both() {
    let swapped = edited(|tracks| tracks.swap(1, 2));
    let result = diff(&sidecar(), &napolitan(), &swapped);
    assert!(result.refuses());
    assert_eq!(
        result.texts(),
        ["part `drums` selects index 2, which was `tambourin` and is now `flute`"],
        "melody still finds `flute` by its name wherever it moved"
    );
}

#[test]
fn a_renamed_track_a_name_selects_is_gone_with_the_rename_hint_at_its_index() {
    let renamed = edited(|tracks| rename(tracks, 1, "Flute (lead)"));
    let result = diff(&sidecar(), &napolitan(), &renamed);
    assert!(result.refuses());
    assert_eq!(
        result.lines[0].text,
        "track `flute` is gone; index 1 is now `Flute (lead)` — renamed in MIDI Evil? Edit the sidecar's part `melody`"
    );
    assert_eq!(result.lines[0].severity, Severity::Error);
    assert_eq!(
        result.lines[1],
        DiffLine {
            text: "track `Flute (lead)` is new and no part selects it; it plays as background".to_string(),
            severity: Severity::Info,
        },
        "the refusal first, for the status bar"
    );
}

#[test]
fn an_index_whose_name_changed_is_refused_naming_both() {
    let renamed = edited(|tracks| rename(tracks, 2, "Tambourine"));
    let result = diff(&sidecar(), &napolitan(), &renamed);
    assert_eq!(
        result.lines[0].text,
        "part `drums` selects index 2, which was `tambourin` and is now `Tambourine`"
    );
    assert!(result.refuses());
}

#[test]
fn a_track_new_in_the_file_that_no_part_selects_plays_as_background() {
    let added = edited(|tracks| {
        let event = |kind| TrackEvent {
            delta: u28::new(0),
            kind,
        };
        tracks.push(vec![
            event(TrackEventKind::Meta(MetaMessage::TrackName(b"celesta"))),
            event(TrackEventKind::Midi {
                channel: u4::new(6),
                message: MidiMessage::ProgramChange { program: u7::new(8) },
            }),
            event(TrackEventKind::Meta(MetaMessage::EndOfTrack)),
        ]);
    });
    let result = diff(&sidecar(), &napolitan(), &added);
    assert!(!result.refuses(), "a new track refuses nothing");
    assert_eq!(
        result.texts(),
        ["track `celesta` is new and no part selects it; it plays as background"]
    );
}

#[test]
fn a_name_two_tracks_now_carry_is_refused_with_the_index_hint() {
    let duplicated = edited(|tracks| rename(tracks, 3, "lowera"));
    let result = diff(&sidecar(), &napolitan(), &duplicated);
    assert!(result.refuses());
    assert_eq!(result.texts(), ["`lowera` names two tracks; select one by `Index`"]);
}

#[test]
fn a_selector_the_author_changed_is_accepted_and_reported() {
    let renamed = edited(|tracks| rename(tracks, 1, "Flute (lead)"));
    let mut repaired = sidecar();
    repaired.parts[0].track = music_midi::TrackSelector::Name("Flute (lead)".to_string());
    let result = diff_tracks(
        &TrackListing::of(&sidecar(), &napolitan()),
        &TrackListing::of(&repaired, &renamed),
    );
    assert!(!result.refuses(), "{:?}", result.lines);
    assert_eq!(result.texts(), ["part `melody` now selects `Flute (lead)`"]);
}

#[test]
fn against_no_old_score_only_what_cannot_play_is_named_and_no_track_is_called_new() {
    let renamed = edited(|tracks| rename(tracks, 1, "Flute (lead)"));
    let result = diff_tracks(&TrackListing::empty(), &TrackListing::of(&sidecar(), &renamed));
    assert_eq!(
        result.lines[0].text,
        "track `flute` is gone; part `melody` selects nothing",
        "no old score, so no rename hint"
    );
    assert!(result.refuses());
    assert_eq!(result.lines.len(), 1, "with no old score every background track would be 'new', which says nothing");
}
