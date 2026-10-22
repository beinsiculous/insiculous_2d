//! The `song …` verbs: what they parse to, what the two queries answer, `scene`'s song
//! fields, and the refusal matrix — every song write refused in Playing and Paused, inside
//! an open scene batch, and with no song open.

use ecs::World;
use serde_json::Value;

use super::write::{self, ApiBatch, WriteCtx};
use super::{
    dispatch_line, parse_line, ApiError, HostedWrite, PureWrite, QueryCtx, Request, WriteCmd,
};
use crate::commands::CommandHistory;
use crate::play_state::EditorPlayState;
use crate::selection::Selection;
use crate::song::{Gesture, SongDocument, SongEdit};

const SONG_WRITES: [&str; 4] = [
    r#"song edit {"SetLoop": {"looping": false}}"#,
    "song undo",
    "song redo",
    "song select none",
];

fn melody() -> SongDocument {
    let mut song = SongDocument::new_song().expect("a blank song");
    let note = SongEdit::AddNote {
        track: 1,
        tick: 0,
        key: 60,
        velocity: 100,
        duration_ticks: 240,
    };
    song.apply(note, Gesture::Discrete).expect("a note");
    song
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
                Some(1),
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
fn song_edit_lands_on_the_songs_history_and_names_the_notes_it_added() {
    let mut song = Some(melody());
    let added = run(
        r#"song edit {"AddNote": {"track": 1, "tick": 480, "key": 64, "velocity": 90, "duration_ticks": 240}}"#,
        &mut song,
        EditorPlayState::Editing,
        false,
    )
    .expect("lands");
    assert_eq!(added["edit"], "AddNote");
    let new_note = added["added_notes"][0].as_u64().expect("the new note's id");

    let selected = run(
        &format!("song select {new_note}"),
        &mut song,
        EditorPlayState::Editing,
        false,
    )
    .expect("selects");
    assert_eq!(selected["selection"][0], new_note);
    let unknown = run(
        "song select 9999",
        &mut song,
        EditorPlayState::Editing,
        false,
    );
    assert!(matches!(unknown, Err(ApiError::NotFound(_))), "{unknown:?}");

    let undone = run("song undo", &mut song, EditorPlayState::Editing, false).expect("undoes");
    assert_eq!(undone["undid"], "AddNote");
    let state = query("song", song.as_ref());
    assert_eq!(
        state["data"]["selection"],
        serde_json::json!([]),
        "an undone note leaves the selection"
    );
    let redone = run("song redo", &mut song, EditorPlayState::Editing, false).expect("redoes");
    assert_eq!(redone["redid"], "AddNote");
    let nothing = run("song redo", &mut song, EditorPlayState::Editing, false).expect("answers");
    assert_eq!(nothing["redid"], Value::Null);
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
    assert_eq!(data["editable"], true);
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
    assert_eq!(hosted("song new"), HostedWrite::SongNew { discard: false });
    assert_eq!(
        hosted("song new discard"),
        HostedWrite::SongNew { discard: true }
    );
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

    assert!(
        matches!(parse_line("song select 3 4"), Ok(Request::Write(WriteCmd::Pure(PureWrite::SongSelect { notes }))) if notes == [3, 4])
    );
    for bad in [
        "song edit",
        r#"song edit {"Explode": {}}"#,
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
