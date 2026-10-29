//! The Songs list's scan, the song pair read beside a MIDI file, and where a part, a
//! binding and the first part live — over the Napolitan fixture with its sidecar built in
//! code, as the author's guide writes it.

use music_midi::midly::num::{u15, u28, u4, u7};
use music_midi::midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use music_midi::{parse_song_file, SongData, SongFile};

use super::*;

const NAPOLITAN: &[u8] = include_bytes!("../../../music_midi/fixtures/dansenapolitaine.mid");

/// The guide's example: `melody` on the track named `flute` (index 1), `drums` on index 2
/// (`tambourin`), `bass_line` on the track named `lowera` (index 4).
const SIDECAR: &str = r#"SongFile(
    version: 1,
    soundfont: "soundfonts/conductor_cast.sf2",
    parts: [
        (name: "melody", track: Name("flute")),
        (name: "drums", track: Index(2)),
        (name: "bass_line", track: Name("lowera")),
    ],
    instruments: [
        (name: "trumpet", source: SoundFont(path: "soundfonts/conductor_cast.sf2", bank: 0, preset: 56)),
        (name: "tuba", source: SoundFont(path: "soundfonts/conductor_cast.sf2", bank: 0, preset: 58)),
    ],
    bindings: [
        (action: "coin", part: "melody", instrument: "trumpet"),
        (action: "bump", part: "drums", instrument: "tuba"),
        (action: "step", part: "bass_line", instrument: "tuba"),
    ],
)"#;

fn napolitan() -> (SongFile, SongData) {
    let (data, _) = SongData::from_smf(NAPOLITAN).expect("the fixture reads");
    (parse_song_file("music/napolitan.mid.ron", SIDECAR).expect("the sidecar parses"), data)
}

fn track(name: Option<&str>, index: usize) -> Location {
    Location::Track { name: name.map(str::to_string), index }
}

#[test]
fn the_songs_list_finds_every_midi_file_in_both_home_folders_by_its_folder_qualified_path() {
    let folder = tempfile::tempdir().expect("an asset folder");
    for file in [
        "music/theme.mid",
        "music/levels/boss.MID",
        "music/intro.midi",
        "ai/music/theme.mid",
        "music/theme.mid.ron",
        "music/intro.midi.ron",
        "music/samples/theme/kick.wav",
        "sounds/beep.mid",
    ] {
        common::vfs::write(&folder.path().join(file), b"x").expect("fixture written");
    }
    assert_eq!(
        scan_songs(folder.path()),
        vec!["ai/music/theme.mid", "music/intro.midi", "music/levels/boss.MID", "music/theme.mid"],
        "a .midi and a .MID list beside a .mid, two same-named songs apart by folder; a \
         sidecar, a sample and a song outside the song folders are not songs here"
    );
    let empty = tempfile::tempdir().expect("an empty asset folder");
    assert!(scan_songs(empty.path()).is_empty(), "a missing folder is an empty list");
}

#[test]
fn an_index_part_opens_at_its_tracks_name_and_index_and_a_name_part_at_the_index_it_sits_at() {
    let (file, data) = napolitan();
    assert_eq!(part_location(&file.parts[1], &data), Ok(track(Some("tambourin"), 2)));
    assert_eq!(part_location(&file.parts[2], &data), Ok(track(Some("lowera"), 4)));
    assert_eq!(
        binding_location(&file, &data, "step"),
        Ok(track(Some("lowera"), 4)),
        "a binding opens where its part's track sits, not at the binding's own position"
    );
    assert_eq!(first_part_location(&file, &data), Ok(Some(track(Some("flute"), 1))));
}

#[test]
fn an_unknown_action_lists_the_actions_the_sidecar_binds() {
    let (file, data) = napolitan();
    let error = binding_location(&file, &data, "jump").expect_err("no binding is called jump");
    assert_eq!(error.to_string(), "binds no action 'jump' — its actions are coin, bump, step");

    let mut bare = file.clone();
    bare.bindings.clear();
    let error = binding_location(&bare, &data, "coin").expect_err("a sidecar binding nothing");
    assert_eq!(error.to_string(), "binds no action 'coin' — it binds none");
}

#[test]
fn a_part_on_a_nameless_track_opens_by_its_index_alone() {
    let event = |delta: u32, kind| TrackEvent { delta: u28::new(delta), kind };
    let channel = u4::new(0);
    let nameless = vec![
        event(0, TrackEventKind::Midi { channel, message: MidiMessage::NoteOn { key: u7::new(60), vel: u7::new(90) } }),
        event(480, TrackEventKind::Midi { channel, message: MidiMessage::NoteOff { key: u7::new(60), vel: u7::new(0) } }),
        event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let timing = vec![event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack))];
    let mut bytes = Vec::new();
    Smf { header: Header::new(Format::Parallel, Timing::Metrical(u15::new(480))), tracks: vec![timing, nameless] }
        .write_std(&mut bytes)
        .expect("the file writes");
    let (data, _) = SongData::from_smf(&bytes).expect("the file reads");
    let file = parse_song_file(
        "music/bare.mid.ron",
        r#"SongFile(version: 1, soundfont: "a.sf2", parts: [(name: "lead", track: Index(1))])"#,
    )
    .expect("the sidecar parses");

    assert_eq!(first_part_location(&file, &data), Ok(Some(track(None, 1))));
    let partless = SongFile { parts: Vec::new(), ..file };
    assert_eq!(first_part_location(&partless, &data), Ok(None), "a sidecar with no parts places nothing");
}

#[test]
fn the_pair_is_read_beside_a_midi_file_by_its_whole_name_and_a_missing_sidecar_is_named() {
    let folder = tempfile::tempdir().expect("an asset folder");
    let song = folder.path().join("music/dance.midi");
    common::vfs::write(&song, NAPOLITAN).expect("the song written");
    let missing = read_song_pair(&song).expect_err("no sidecar yet");
    assert_eq!(missing, SongPairError::SidecarMissing { sidecar: folder.path().join("music/dance.midi.ron") });

    common::vfs::write_string(&folder.path().join("music/dance.midi.ron"), SIDECAR).expect("the sidecar");
    let (file, data) = read_song_pair(&song).expect("the pair reads");
    assert_eq!(binding_location(&file, &data, "coin"), Ok(track(Some("flute"), 1)));
}

const NAPOLITAN_TYPE_ZERO: &[u8] = include_bytes!("../../../music_midi/fixtures/dansenapolitaine-type0.mid");

#[test]
fn a_type_zero_files_channel_tracks_open_at_the_files_one_track_with_no_name() {
    let (data, _) = SongData::from_smf(NAPOLITAN_TYPE_ZERO).expect("the type-0 fixture reads");
    assert!(data.tracks().len() > 1, "read as one track per channel");
    assert!(data.tracks().iter().all(|track| track.source_track() == 0), "all from the file's track 0");
    let sidecar = SIDECAR.replace("track: Name(\"flute\")", "track: Index(1)").replace("track: Name(\"lowera\")", "track: Index(3)");
    let file = parse_song_file("music/napolitan0.mid.ron", &sidecar).expect("the sidecar parses");

    for part in &file.parts {
        assert_eq!(part_location(part, &data), Ok(track(None, 0)), "{}: the tool gets the file's track, nameless", part.name);
    }
    assert_eq!(binding_location(&file, &data, "bump"), Ok(track(None, 0)));
    assert_eq!(first_part_location(&file, &data), Ok(Some(track(None, 0))));
}
