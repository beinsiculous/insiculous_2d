//! The `song …` verbs: what they parse to, what the two queries answer, `scene`'s song
//! fields, the refusal matrix — every song write refused in Playing and Paused, inside an
//! open scene batch, and with no song open — and the score's verbs refused naming MIDI Evil,
//! which writes the score now.

use ecs::World;
use music_midi::midly::num::{u15, u24, u28, u4, u7};
use music_midi::midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use serde_json::Value;

use super::write::{self, ApiBatch, WriteCtx};
use super::{dispatch_line, parse_line, ApiError, HostedWrite, QueryCtx, Request, WriteCmd};
use crate::commands::CommandHistory;
use crate::play_state::EditorPlayState;
use crate::selection::Selection;
use crate::song::{default_song_file, Gesture, SongDocument};

const SONG_WRITES: [&str; 3] = [
    r#"song edit {"SetLoop": {"looping": false}}"#,
    "song undo",
    "song redo",
];

/// The score edits `song edit` once took, which it now refuses by name.
const SCORE_EDITS: [&str; 10] = [
    "AddNote",
    "DeleteNotes",
    "MoveNotes",
    "ResizeNotes",
    "SetVelocity",
    "SetPitch",
    "AssignPart",
    "SetTempo",
    "SetTimeSignature",
    "AddTrack",
];

/// A type-1 song as MIDI bytes at 480 ticks a quarter: the timing track at 120 BPM, then a
/// track named `melody` on channel 1 holding one eighth, C4 at tick 0.
fn melody_bytes() -> Vec<u8> {
    let event = |delta, kind| TrackEvent {
        delta: u28::new(delta),
        kind,
    };
    let note = |on| TrackEventKind::Midi {
        channel: u4::new(0),
        message: if on {
            MidiMessage::NoteOn { key: u7::new(60), vel: u7::new(100) }
        } else {
            MidiMessage::NoteOff { key: u7::new(60), vel: u7::new(0) }
        },
    };
    let melody = vec![
        event(0, TrackEventKind::Meta(MetaMessage::TrackName(b"melody"))),
        event(0, note(true)),
        event(240, note(false)),
        event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let timing = vec![
        event(0, TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000)))),
        event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let mut bytes = Vec::new();
    Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(480))),
        tracks: vec![timing, melody],
    }
    .write_std(&mut bytes)
    .expect("the melody writes");
    bytes
}

/// [`melody_bytes`] opened as a song never saved, its history empty.
fn melody() -> SongDocument {
    SongDocument::from_smf(&melody_bytes(), default_song_file()).expect("the melody reads")
}

fn query(line: &str, song: Option<&SongDocument>) -> Value {
    let (world, selection) = (World::new(), Selection::new());
    let ctx = QueryCtx {
        world: &world,
        selection: &selection,
        scene_path: None,
        dirty: false,
        play_state: EditorPlayState::Editing,
        song,
        status_bar: None,
    };
    let response = dispatch_line(line, &ctx).expect("a response is owed");
    serde_json::from_str(&response).expect("one line of JSON")
}

fn run(
    line: &str,
    song: &mut Option<SongDocument>,
    play_state: EditorPlayState,
    batch_open: bool,
) -> Result<Value, ApiError> {
    let Request::Write(WriteCmd::Pure(pure)) = parse_line(line)? else {
        panic!("a pure write: {line}");
    };
    let mut batch = batch_open.then(|| ApiBatch {
        name: "level".to_string(),
        commands: Vec::new(),
        selection_before: Vec::new(),
    });
    let (mut world, mut history, mut selection) =
        (World::new(), CommandHistory::new(), Selection::new());
    let mut ctx = WriteCtx {
        world: &mut world,
        history: &mut history,
        selection: &mut selection,
        play_state,
        batch: &mut batch,
        texture_known: &|_| true,
        song,
    };
    write::run(&pure, &mut ctx)
}

#[test]
fn every_song_write_is_refused_while_playing_paused_inside_a_scene_batch_and_without_a_song() {
    for line in SONG_WRITES {
        for (play_state, batch_open, why) in [
            (EditorPlayState::Playing, false, "Playing"),
            (
                EditorPlayState::Paused,
                false,
                "Paused — stricter than scene writes",
            ),
            (EditorPlayState::Editing, true, "an open scene batch"),
        ] {
            let mut song = Some(melody());
            let result = run(line, &mut song, play_state, batch_open);
            assert!(
                matches!(result, Err(ApiError::Refused(_))),
                "{line} under {why}: {result:?}"
            );
            assert_eq!(
                song.map(|song| song.history().undo_count()),
                Some(0),
                "{line}: nothing recorded"
            );
        }
        let result = run(line, &mut None, EditorPlayState::Editing, false);
        assert!(
            matches!(result, Err(ApiError::Refused(message)) if message.contains("no song")),
            "{line}"
        );
    }
}

#[test]
fn song_edit_lands_on_the_songs_history_and_undo_and_redo_name_it() {
    let mut song = Some(melody());
    let landed = run(
        r#"song edit {"SetLoop": {"looping": false}}"#,
        &mut song,
        EditorPlayState::Editing,
        false,
    )
    .expect("lands");
    assert_eq!(landed["edit"], "SetLoop");
    assert_eq!(landed["dirty"], true);
    assert_eq!(song.as_ref().map(|song| song.history().undo_count()), Some(1));

    let undone = run("song undo", &mut song, EditorPlayState::Editing, false).expect("undoes");
    assert_eq!(undone["undid"], "SetLoop");
    assert_eq!(query("song", song.as_ref())["data"]["looping"], true);
    let redone = run("song redo", &mut song, EditorPlayState::Editing, false).expect("redoes");
    assert_eq!(redone["redid"], "SetLoop");
    let nothing = run("song redo", &mut song, EditorPlayState::Editing, false).expect("answers");
    assert_eq!(nothing["redid"], Value::Null);
}

#[test]
fn every_score_edit_is_refused_by_its_name_naming_midi_evil_and_a_misspelt_one_is_serdes_error() {
    let song = melody();
    for name in SCORE_EDITS {
        let answer = query(&format!("song edit {{\"{name}\": {{}}}}"), Some(&song));
        assert_eq!(answer["error"]["kind"], "refused", "{name}");
        let message = answer["error"]["message"].as_str().expect("a message");
        assert!(message.starts_with(&format!("{name} is a score edit")), "{message}");
        assert!(message.contains("MIDI Evil"), "{message}");
    }
    let whole = r#"song edit {"AddNote": {"track": 1, "tick": 0, "key": 60, "velocity": 100, "duration_ticks": 240}}"#;
    assert!(
        matches!(parse_line(whole), Err(ApiError::Refused(message)) if message.contains("MIDI Evil")),
        "a well-formed score edit is refused the same way"
    );

    let misspelt = query(
        r#"song edit {"AddNot": {"track": 1, "tick": 0, "key": 60, "velocity": 100, "duration_ticks": 240}}"#,
        Some(&song),
    );
    assert_eq!(misspelt["error"]["kind"], "parse", "an unknown variant is still serde's error");
    let message = misspelt["error"]["message"].as_str().expect("a message");
    assert!(message.contains("unknown variant") && !message.contains("MIDI Evil"), "{message}");
}

#[test]
fn a_batch_holding_a_score_edit_is_refused_whole_and_lands_nothing() {
    let mut song = Some(melody());
    let before = song.as_ref().map(|song| song.file().clone());
    let line = r#"song edit {"Batch": [{"SetLoop": {"looping": false}}, {"Batch": [{"MoveNotes": {"notes": [1], "tick_delta": 120}}]}]}"#;
    let result = run(line, &mut song, EditorPlayState::Editing, false);
    assert!(
        matches!(&result, Err(ApiError::Refused(message)) if message.starts_with("MoveNotes is a score edit")),
        "a member however deep refuses the batch: {result:?}"
    );
    assert_eq!(song.as_ref().map(|song| song.file().clone()), before, "the loop member did not land");
    assert_eq!(song.as_ref().map(|song| song.history().undo_count()), Some(0));
}

#[test]
fn song_new_is_refused_naming_midi_evil_with_or_without_discard() {
    for line in ["song new", "song new discard"] {
        let result = parse_line(line);
        assert!(
            matches!(&result, Err(ApiError::Refused(message)) if message.contains("MIDI Evil") && message.contains("song init")),
            "{line}: {result:?}"
        );
    }
}

#[test]
fn the_song_query_describes_the_open_song_and_song_notes_filters_by_part() {
    assert_eq!(
        query("song", None)["data"],
        serde_json::json!({ "open": false })
    );

    let mut song = melody();
    let part = r#"{"AddPart": {"part": {"name": "lead", "track": {"Name": "melody"}}}}"#;
    song.apply(
        serde_json::from_str(part).expect("a SongEdit"),
        Gesture::Discrete,
    )
    .expect("a part");
    let state = query("song", Some(&song));
    let data = &state["data"];
    assert_eq!(data["open"], true);
    assert_eq!(data["dirty"], true, "never saved");
    for gone in ["editable", "refused_reason", "selection"] {
        assert!(data.get(gone).is_none(), "{gone} is no longer answered");
    }
    assert_eq!(data["tracks"][1]["name"], "melody");
    assert_eq!(data["tracks"][1]["channel"], 1, "counted from 1");
    assert_eq!(data["parts"][0]["name"], "lead");
    assert_eq!(data["tempo_changes"][0]["beats_per_minute"], 120.0);

    let notes = query("song notes lead", Some(&song));
    assert_eq!(notes["data"]["notes"].as_array().map(Vec::len), Some(1));
    assert_eq!(notes["data"]["notes"][0]["key"], 60);
    let missing = query("song notes bass", Some(&song));
    assert_eq!(missing["error"]["kind"], "not_found");
}

#[test]
fn scene_reports_its_own_dirty_flag_the_songs_and_either() {
    let clean_scene = query("scene", Some(&melody()));
    assert_eq!(
        clean_scene["data"]["dirty"], false,
        "the scene's own flag stays scene-only"
    );
    assert_eq!(clean_scene["data"]["song_dirty"], true);
    assert_eq!(clean_scene["data"]["any_dirty"], true);
    let no_song = query("scene", None);
    assert_eq!(
        (
            no_song["data"]["song_dirty"].clone(),
            no_song["data"]["any_dirty"].clone()
        ),
        (Value::Bool(false), Value::Bool(false))
    );
}

#[test]
fn the_hosted_song_verbs_parse_with_their_discard_word() {
    let hosted = |line: &str| match parse_line(line) {
        Ok(Request::Write(WriteCmd::Hosted(write))) => write,
        other => panic!("{line}: {other:?}"),
    };
    assert_eq!(
        hosted("song open music/theme.mid discard"),
        HostedWrite::SongOpen {
            path: "music/theme.mid".to_string(),
            discard: true
        }
    );
    assert_eq!(
        hosted("song save"),
        HostedWrite::SongSave {
            path: None,
            overwrite: false
        }
    );
    assert_eq!(
        hosted("song save music/theme.mid overwrite"),
        HostedWrite::SongSave {
            path: Some("music/theme.mid".to_string()),
            overwrite: true
        }
    );
    assert_eq!(
        hosted("song close discard"),
        HostedWrite::SongClose { discard: true }
    );
    assert_eq!(
        hosted("song play 4"),
        HostedWrite::SongPlay { from_beat: Some(4) }
    );
    assert_eq!(hosted("song seek 2"), HostedWrite::SongSeek { beat: 2 });
    assert_eq!(hosted("song seek 2.0"), HostedWrite::SongSeek { beat: 2 });
    assert_eq!(
        hosted("song trigger jump"),
        HostedWrite::SongTrigger {
            binding: "jump".to_string()
        }
    );
    assert_eq!(hosted("song stop"), HostedWrite::SongStop);

    for bad in [
        "song edit",
        r#"song edit {"Explode": {}}"#,
        "song select 3 4",
        "song select none",
        "song seek",
        "song seek two",
        "song frobnicate",
        "song save a discard",
        "song save a b",
    ] {
        assert!(matches!(parse_line(bad), Err(ApiError::Parse(_))), "{bad}");
    }
}

#[test]
fn a_fractional_negative_or_non_finite_beat_is_invalid() {
    for bad in [
        "song seek 2.5",
        "song seek -1",
        "song seek inf",
        "song seek NaN",
        "song play 0.5",
        "song play -0.5",
    ] {
        assert!(matches!(parse_line(bad), Err(ApiError::Invalid(_))), "{bad}");
    }
}

#[test]
fn song_edit_sets_the_songs_provenance_and_undo_takes_it_back() {
    let mut song = Some(melody());
    let set = run(
        r#"song edit {"SetProvenance": {"provenance": {"author": "2026 Jesse", "license": "MIT"}}}"#,
        &mut song,
        EditorPlayState::Editing,
        false,
    )
    .expect("lands");
    assert_eq!(set["edit"], "SetProvenance");
    let state = query("song", song.as_ref());
    assert_eq!(state["data"]["provenance"]["author"], "2026 Jesse");
    assert_eq!(state["data"]["provenance"]["license"], "MIT");

    run("song undo", &mut song, EditorPlayState::Editing, false).expect("undoes");
    assert_eq!(query("song", song.as_ref())["data"]["provenance"], Value::Null);
}
