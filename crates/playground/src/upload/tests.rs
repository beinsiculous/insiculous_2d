//! An imported file: each kind lands in its folder; a name that would break the quoted
//! `song open`, the provenance gate or the song path rule is refused; every file is refused
//! by its size before a decode and by its loader before a write; a taken name is replaced
//! only when asked, a song's sidecar kept for its document to follow the new score with —
//! the open song's import said so, for the host to hand on; a `.midi` is a song; and a sample imported, assigned and saved in
//! one session lands in the song's folder.

use std::path::{Path, PathBuf};

use editor::{Gesture, SongDocument, SongEdit};
use music_midi::{Instrument, InstrumentSource, Provenance, SamplePlayback};

use super::*;

const SINE_WAV: &[u8] = include_bytes!("../../../music_midi/fixtures/sine_48000_mono.wav");
const SINE_FONT: &[u8] = include_bytes!("../../../music_midi/fixtures/sine.sf2");

fn midi() -> Vec<u8> {
    SongDocument::new_song()
        .expect("a blank song")
        .data()
        .write_smf()
        .expect("exports")
}

fn project() -> tempfile::TempDir {
    tempfile::tempdir().expect("a temporary project root")
}

fn exists(root: &Path, relative: &str) -> bool {
    common::vfs::file_size(&root.join(relative)).is_ok()
}

fn provenance() -> Provenance {
    Provenance {
        author: "2026 Jesse".to_string(),
        license: "MIT".to_string(),
    }
}

#[test]
fn each_kind_lands_in_its_own_folder() {
    let root = project();
    let song = import_file(root.path(), "theme.mid", &midi(), None, false).expect("a song");
    let sample = import_file(root.path(), "blip.wav", SINE_WAV, None, false).expect("a sample");
    let bank = import_file(root.path(), "cast.sf2", SINE_FONT, None, false).expect("a bank");
    for (outcome, destination, kind) in [
        (song, "assets/music/theme.mid", UploadKind::Song),
        (sample, "assets/music/imports/blip.wav", UploadKind::Sample),
        (bank, "assets/soundfonts/cast.sf2", UploadKind::SoundFont),
    ] {
        assert!(outcome.written, "{destination}");
        assert_eq!((outcome.destination.as_str(), outcome.kind), (destination, kind));
        assert!(exists(root.path(), destination));
    }
}

#[test]
fn a_name_that_would_break_the_quoted_open_the_gate_or_the_path_rule_is_refused_and_a_samples_space_becomes_an_underscore() {
    let root = project();
    for refused in ["the \"best\" song.mid", "my song.mid", "café.mid", "a__b.mid", "_theme.mid", "blip\".wav", "plätt.wav"] {
        let bytes = if refused.ends_with(".mid") { midi() } else { SINE_WAV.to_vec() };
        let error = import_file(root.path(), refused, &bytes, None, false).expect_err(refused);
        assert!(!error.is_empty(), "{refused}");
    }
    assert!(common::vfs::list_files(&root.path().join("assets")).unwrap_or_default().is_empty(), "nothing written");

    let spaced = import_file(root.path(), "big blip.wav", SINE_WAV, None, false).expect("a spaced sample");
    assert_eq!(spaced.destination, "assets/music/imports/big_blip.wav");
    let unknown = plan_upload("theme.mp3", b"ID3").expect_err("not a kind");
    assert!(unknown.contains(".mid") && unknown.contains(".wav") && unknown.contains(".sf2"), "{unknown}");
}

#[test]
fn an_empty_a_truncated_and_a_header_only_file_of_each_kind_is_refused() {
    let midi_header_only: &[u8] = b"MThd\0\0\0\x06\0\x01\0\x01\x01\xe0";
    let mut wav_header_only = SINE_WAV[..44].to_vec();
    wav_header_only[40..44].copy_from_slice(&0u32.to_le_bytes());
    let sound_font_header_only: &[u8] = b"RIFF\x04\0\0\0sfbk";
    for (name, header_only) in [
        ("theme.mid", midi_header_only.to_vec()),
        ("blip.wav", wav_header_only),
        ("cast.sf2", sound_font_header_only.to_vec()),
    ] {
        let whole = match name {
            "theme.mid" => midi(),
            "blip.wav" => SINE_WAV.to_vec(),
            _ => SINE_FONT.to_vec(),
        };
        for (shape, bytes) in [("empty", Vec::new()), ("three bytes", whole[..3].to_vec()), ("header only", header_only)] {
            let error = plan_upload(name, &bytes).expect_err(&format!("{name}, {shape}"));
            assert!(error.contains("cannot be imported"), "{name}, {shape}: {error}");
        }
    }
}

#[test]
fn every_kind_is_refused_by_its_size_before_any_decode() {
    for (name, limit) in [
        ("theme.mid", music_midi::MIDI_BYTE_LIMIT),
        ("blip.wav", audio::SAMPLE_BYTE_LIMIT),
        ("cast.sf2", audio::SOUND_FONT_BYTE_LIMIT),
    ] {
        let error = plan_upload(name, &vec![0; limit + 1]).expect_err(name);
        assert!(error.contains(&format!("{} KiB", limit / 1024)), "named by its limit, not a decode: {error}");
    }
}

/// A song at `music/theme.mid` whose sidecar, written by hand, names the imported
/// `music/imports/blip.wav` and the background bank.
fn song_naming_the_sample(root: &Path) -> PathBuf {
    import_file(root, "blip.wav", SINE_WAV, None, false).expect("the sample");
    let target = root.join("assets/music/theme.mid");
    common::vfs::write(&target, &midi()).expect("the MIDI file");
    let sidecar = r#"SongFile(version: 1, soundfont: "soundfonts/cast.sf2",
        instruments: [(name: "blip", source: Sample(path: "music/imports/blip.wav", root_note: 60))])"#;
    common::vfs::write_string(&root.join("assets/music/theme.mid.ron"), sidecar).expect("the sidecar");
    target
}

#[test]
fn a_taken_name_is_replaced_only_when_asked_and_the_answer_names_every_song_using_it() {
    let root = project();
    song_naming_the_sample(root.path());
    let other = SINE_WAV.to_vec();

    let asked = import_file(root.path(), "blip.wav", &other, None, false).expect("asks");
    assert!(!asked.written);
    assert_eq!(asked.referenced_by, ["music/theme.mid"]);
    assert!(!asked.sidecar_lost, "a sample's replace loses no sidecar");

    let replaced = import_file(root.path(), "blip.wav", &other, None, true).expect("replaces");
    assert!(replaced.written);
    assert_eq!(songs_referencing(root.path(), "assets/soundfonts/cast.sf2"), ["music/theme.mid"], "the background bank counts too");
}

#[test]
fn a_replaced_song_keeps_its_sidecar_and_any_journal_and_says_when_it_is_the_open_one() {
    let root = project();
    let target = song_naming_the_sample(root.path());
    let kept = ["assets/music/theme.mid.ron", "assets/music/theme.mid.new", "assets/music/theme.mid.ron.new"];
    for leftover in &kept[1..] {
        common::vfs::write(&root.path().join(leftover), b"journal").expect("written");
    }

    let asked = import_file(root.path(), "theme.mid", &midi(), None, false).expect("asks");
    assert!(!asked.written && asked.sidecar_lost, "the score would change and the map be kept: {asked:?}");
    assert!(!asked.open_song_replaced, "nothing was written");
    let open = import_file(root.path(), "theme.mid", &midi(), Some(&target), true).expect("the open song is replaced");
    assert!(open.written && open.open_song_replaced, "the composer's document follows it: {open:?}");

    let replaced = import_file(root.path(), "theme.mid", &midi(), None, true).expect("replaces");
    assert!(replaced.written && !replaced.open_song_replaced);
    assert_eq!(common::vfs::read(&target).ok(), Some(midi()));
    for kept in kept {
        assert!(exists(root.path(), kept), "{kept} is kept: the open settles a journal, never the import");
    }
}

#[test]
fn a_reimported_score_keeps_the_saved_map_and_the_next_open_follows_it_with_the_diffs_lines() {
    let root = project();
    let asset_base = root.path().join("assets");
    let napolitan: &[u8] = include_bytes!("../../../music_midi/fixtures/dansenapolitaine.mid");
    let first = import_file(root.path(), "napolitan.mid", napolitan, None, false).expect("imported");
    let target = asset_base.join(first.destination.trim_start_matches("assets/"));
    let mut song = SongDocument::open(&target, &asset_base).expect("opens in place").document;
    let melody = music_midi::Part {
        name: "melody".to_string(),
        track: music_midi::TrackSelector::Name("flute".to_string()),
        mode: music_midi::PartMode::Loop,
        quantize: music_midi::Quantize::Off,
    };
    for edit in [SongEdit::AddPart { part: melody }, SongEdit::SetProvenance { provenance: provenance() }] {
        song.apply(edit, Gesture::Discrete).expect("a map edit");
    }
    song.save(&target, &asset_base).expect("the map saves");
    let notes_before = song.data().tracks().iter().map(|track| track.note_count()).sum::<usize>();
    drop(song);

    let replaced = import_file(root.path(), "napolitan.mid", &midi(), None, true).expect("re-imported");
    assert!(replaced.written && replaced.sidecar_lost, "{replaced:?}");
    let opened = SongDocument::open(&target, &asset_base).expect("the composer's next open");
    let notes_after = opened.document.data().tracks().iter().map(|track| track.note_count()).sum::<usize>();
    assert_ne!(notes_after, notes_before, "the notes are the new score's");
    assert_eq!(opened.document.file().parts.len(), 1, "the map survives");
    assert_eq!(opened.document.file().parts[0].name, "melody");
    assert!(opened.notes.iter().any(|note| note.contains("changed outside the editor")), "{:?}", opened.notes);
    assert!(
        opened.notes.iter().any(|note| note.contains("track `flute` is gone")),
        "the diff's lines: {:?}",
        opened.notes
    );
}

#[test]
fn a_midi_song_is_imported_and_listed_by_the_songs_naming_a_file() {
    let root = project();
    let imported = import_file(root.path(), "theme.midi", &midi(), None, false).expect("a .midi is a song");
    assert_eq!((imported.kind, imported.destination.as_str()), (UploadKind::Song, "assets/music/theme.midi"));
    import_file(root.path(), "blip.wav", SINE_WAV, None, false).expect("the sample");
    let sidecar = r#"SongFile(version: 1, soundfont: "soundfonts/cast.sf2",
        instruments: [(name: "blip", source: Sample(path: "music/imports/blip.wav", root_note: 60))])"#;
    common::vfs::write_string(&root.path().join("assets/music/theme.midi.ron"), sidecar).expect("its sidecar");

    assert_eq!(songs_referencing(root.path(), "assets/music/imports/blip.wav"), ["music/theme.midi"]);
    let asked = import_file(root.path(), "theme.midi", &midi(), None, false).expect("asks");
    assert!(asked.sidecar_lost, "the .midi's own sidecar, by the whole name");
    let open = root.path().join("assets/music/theme.midi");
    let replaced = import_file(root.path(), "theme.midi", &midi(), Some(&open), true).expect("the open song");
    assert!(replaced.open_song_replaced, "the .midi's open song by its whole name");
}

#[test]
fn a_sample_imported_assigned_and_saved_in_one_session_lands_in_the_songs_folder() {
    let root = project();
    let asset_base = root.path().join("assets");
    let imported = import_file(root.path(), "kick drum.wav", SINE_WAV, None, false).expect("imported");
    let sample_path = imported.destination.trim_start_matches("assets/").to_string();

    let mut song = SongDocument::new_song().expect("a blank song");
    let instrument = Instrument {
        name: "kick".to_string(),
        source: InstrumentSource::Sample {
            path: sample_path,
            root_note: 60,
            gain: 1.0,
            playback: SamplePlayback::OneShot,
        },
        provenance: Some(provenance()),
    };
    for edit in [SongEdit::AddInstrument { instrument }, SongEdit::SetProvenance { provenance: provenance() }] {
        song.apply(edit, Gesture::Discrete).expect("lands");
    }
    song.save(&asset_base.join("music/beat.mid"), &asset_base).expect("saves");

    assert!(exists(root.path(), "assets/music/samples/beat/kick_drum.wav"));
    let license = common::vfs::read_to_string(&asset_base.join("music/samples/beat/LICENSE")).expect("written");
    assert!(license.contains("Files: kick_drum.wav"), "{license}");
}
