//! The `song …` verbs: their parser arm, the two song queries, and the pure song writes,
//! which run through `SongDocument::apply` exactly as the Conductor panel's GUI does. The score is
//! not the engine's to write: a score edit, and `song new`, are refused naming MIDI Evil.
//!
//! Song writes are stricter than scene writes: refused in Playing *and* Paused (a running
//! game owns the music), and refused while a scene batch is open — that batch is one
//! `MacroCommand` over world commands, which a song edit has no place in. `song edit
//! {"Batch": […]}` is the song's own composite.

use serde_json::{json, Value};

use super::parse::{split_fixed_tokens, tokens};
use super::write::WriteCtx;
use super::{ApiError, HostedWrite, PureWrite, Query, QueryCtx, Request, WriteCmd};
use crate::play_state::EditorPlayState;
use crate::song::{Gesture, SongEdit, SongEditError};

/// The score edits the engine once made, by the name `song edit` took them under: the score
/// is MIDI Evil's now, and each is refused naming it rather than read as an unknown edit.
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

/// Why a song write cannot run now, if it cannot: a play session, or an open scene batch.
/// Hosted song writes ask it too.
pub fn song_write_refusal(play_state: EditorPlayState, scene_batch_open: bool) -> Option<ApiError> {
    if play_state != EditorPlayState::Editing {
        return Some(ApiError::Refused(
            "song writes are refused during a play session (Playing or Paused) — stop first"
                .to_string(),
        ));
    }
    if scene_batch_open {
        return Some(ApiError::Refused(
            "a scene batch is open and a song write has no place in it — `batch end` or \
             `batch abort` first (`song edit {\"Batch\": [...]}` groups song edits)"
                .to_string(),
        ));
    }
    None
}

/// Parse a `song …` line. `song edit` takes the rest of the line as JSON, so it never
/// meets the tokenizer.
pub(super) fn parse(line: &str) -> Result<Request, ApiError> {
    let (fixed, rest) = split_fixed_tokens(line, 2)?;
    if fixed.get(1).map(String::as_str) == Some("edit") {
        if rest.is_empty() {
            return Err(ApiError::Parse("usage: song edit <json>".to_string()));
        }
        let parse_error =
            |error: serde_json::Error| ApiError::Parse(format!("song edit needs a SongEdit as JSON: {error}"));
        let value: Value = serde_json::from_str(rest).map_err(parse_error)?;
        if let Some(name) = score_edit_named(&value) {
            return Err(written_in_midi_evil(&format!("{name} is a score edit")));
        }
        let edit: SongEdit = serde_json::from_value(value).map_err(parse_error)?;
        return Ok(Request::Write(WriteCmd::Pure(PureWrite::SongEdit(edit))));
    }
    let mut words = tokens(line)?.into_iter().skip(1);
    let subcommand = words.next();
    let mut arguments: Vec<String> = words.collect();
    let discard = arguments.last().is_some_and(|word| word == "discard");
    if discard {
        arguments.pop();
    }
    let hosted = |write| Ok(Request::Write(WriteCmd::Hosted(write)));
    match (subcommand.as_deref(), arguments.as_slice()) {
        (None, []) => Ok(Request::Query(Query::Song)),
        (Some("notes"), [] | [_]) => Ok(Request::Query(Query::SongNotes {
            part: arguments.first().cloned(),
        })),
        (Some("undo"), []) => Ok(Request::Write(WriteCmd::Pure(PureWrite::SongUndo))),
        (Some("redo"), []) => Ok(Request::Write(WriteCmd::Pure(PureWrite::SongRedo))),
        (Some("new"), []) => Err(written_in_midi_evil(
            "song new makes a score — `song init <path.mid>` maps one MIDI Evil exported",
        )),
        (Some("open"), [path]) => hosted(HostedWrite::SongOpen {
            path: path.clone(),
            discard,
        }),
        (Some("init"), [path]) if !discard => hosted(HostedWrite::SongInit { path: path.clone() }),
        (Some("close"), []) => hosted(HostedWrite::SongClose { discard }),
        (Some("save"), [] | [_]) if !discard => hosted(HostedWrite::SongSave {
            path: arguments.first().cloned(),
            overwrite: false,
        }),
        // Its own word, not `discard`: it replaces another song's files rather than
        // dropping this one's edits.
        (Some("save"), [path, word]) if !discard && word == "overwrite" => {
            hosted(HostedWrite::SongSave {
                path: Some(path.clone()),
                overwrite: true,
            })
        }
        (Some("play"), [] | [_]) if !discard => hosted(HostedWrite::SongPlay {
            from_beat: arguments.first().map(|beat| parse_beat(beat)).transpose()?,
        }),
        (Some("stop"), []) if !discard => hosted(HostedWrite::SongStop),
        (Some("seek"), [beat]) if !discard => hosted(HostedWrite::SongSeek {
            beat: parse_beat(beat)?,
        }),
        (Some("trigger"), [binding]) if !discard => hosted(HostedWrite::SongTrigger {
            binding: binding.clone(),
        }),
        _ => Err(ApiError::Parse(
            "usage: song | song notes [part] | song edit <json> | song undo | song redo | \
             song open <path> [discard] | \
             song init <path> | song save [path [overwrite]] | song close [discard] | \
             song play [beat] | song stop | \
             song seek <beat> | song trigger <action>"
                .to_string(),
        )),
    }
}

/// The first score edit `edit` names — a `Batch`'s members searched however deep — when
/// it names one.
fn score_edit_named(edit: &Value) -> Option<&'static str> {
    let Value::Object(fields) = edit else {
        return None;
    };
    fields.iter().find_map(|(name, body)| match name.as_str() {
        "Batch" => body.as_array()?.iter().find_map(score_edit_named),
        name => SCORE_EDITS.iter().copied().find(|score_edit| *score_edit == name),
    })
}

/// The refusal of what writes a score: `what` names it, the rest says where scores are
/// written now.
fn written_in_midi_evil(what: &str) -> ApiError {
    ApiError::Refused(format!(
        "{what}; the score is written in MIDI Evil, which exports the .mid the engine plays"
    ))
}

/// A whole beat counted from zero: the preview moves to the start of a beat, never inside one.
fn parse_beat(word: &str) -> Result<u64, ApiError> {
    let beat = word.parse::<f64>().map_err(|_| {
        ApiError::Parse(format!("\"{word}\" is not a beat (a whole number from 0)"))
    })?;
    if !beat.is_finite() || beat < 0.0 || beat.fract() != 0.0 {
        return Err(ApiError::Invalid(format!(
            "\"{word}\" is not a beat: beats are whole numbers counted from zero"
        )));
    }
    Ok(beat as u64)
}

/// The `song` query: the open song's file, tracks, timing and state, or `{"open": false}`.
pub(super) fn answer_song(ctx: &QueryCtx<'_>) -> Value {
    let Some(song) = ctx.song else {
        return json!({ "open": false });
    };
    let data = song.data();
    let file = song.file();
    let tracks: Vec<Value> = data
        .tracks()
        .iter()
        .enumerate()
        .map(|(index, track)| {
            json!({
                "index": index,
                "name": track.name(),
                "channel": track.channel().map(|channel| channel + 1),
                "note_count": track.note_count(),
            })
        })
        .collect();
    let tempo_changes: Vec<Value> = data
        .tempo_changes()
        .iter()
        .map(|change| {
            json!({
                "tick": change.tick,
                "microseconds_per_quarter": change.microseconds_per_quarter,
                "beats_per_minute": change.beats_per_minute(),
            })
        })
        .collect();
    let time_signatures: Vec<Value> = data
        .time_signatures()
        .iter()
        .map(|signature| {
            json!({
                "tick": signature.tick,
                "numerator": signature.numerator,
                "denominator": signature.denominator,
            })
        })
        .collect();
    json!({
        "open": true,
        "path": song.path().map(|path| path.display().to_string()),
        "imported_from": song.imported_from().map(|path| path.display().to_string()),
        "dirty": song.is_dirty(),
        "ticks_per_quarter": data.ticks_per_quarter(),
        "soundfont": file.soundfont,
        "looping": file.looping,
        "tracks": tracks,
        "parts": serde_json::to_value(&file.parts).unwrap_or(Value::Null),
        "instruments": serde_json::to_value(&file.instruments).unwrap_or(Value::Null),
        "bindings": serde_json::to_value(&file.bindings).unwrap_or(Value::Null),
        "tempo_changes": tempo_changes,
        "time_signatures": time_signatures,
        "provenance": serde_json::to_value(&file.provenance).unwrap_or(Value::Null),
        "can_undo": song.history().can_undo(),
        "can_redo": song.history().can_redo(),
    })
}

/// The `song notes [part]` query: every note, or the notes of the track a part plays.
pub(super) fn answer_song_notes(ctx: &QueryCtx<'_>, part: Option<&str>) -> Result<Value, ApiError> {
    let song = ctx.song.ok_or_else(no_song)?;
    let data = song.data();
    let only_track = match part {
        None => None,
        Some(name) => {
            let part = song
                .file()
                .parts
                .iter()
                .find(|candidate| candidate.name == name)
                .ok_or_else(|| ApiError::NotFound(format!("no part is named '{name}'")))?;
            Some(
                part.resolve_track(data)
                    .map_err(|problem| ApiError::Invalid(problem.to_string()))?,
            )
        }
    };
    let notes: Vec<Value> = data
        .tracks()
        .iter()
        .enumerate()
        .filter(|(index, _)| only_track.is_none_or(|track| track == *index))
        .flat_map(|(index, track)| {
            track.units().iter().flat_map(move |unit| {
                unit.notes.iter().map(move |note| {
                    json!({
                        "id": note.id.number(),
                        "track": index,
                        "tick": unit.tick,
                        "key": note.key,
                        "velocity": note.velocity,
                        "duration_ticks": note.duration_ticks,
                    })
                })
            })
        })
        .collect();
    Ok(json!({ "notes": notes }))
}

/// Run a pure song write through the open song's one mutation path.
pub(super) fn run_write(write: &PureWrite, ctx: &mut WriteCtx<'_>) -> Result<Value, ApiError> {
    if let Some(refusal) = song_write_refusal(ctx.play_state, ctx.batch.is_some()) {
        return Err(refusal);
    }
    let song = ctx.song.as_mut().ok_or_else(no_song)?;
    match write {
        PureWrite::SongEdit(edit) => {
            song.apply(edit.clone(), Gesture::Discrete)
                .map_err(edit_refusal)?;
            Ok(json!({
                "command": "song edit",
                "edit": edit.name(),
                "dirty": song.is_dirty(),
            }))
        }
        PureWrite::SongUndo => Ok(json!({ "undid": song.undo() })),
        PureWrite::SongRedo => Ok(json!({ "redid": song.redo() })),
        other => Err(ApiError::Invalid(format!("{other:?} is not a song write"))),
    }
}

fn edit_refusal(error: SongEditError) -> ApiError {
    ApiError::Invalid(error.to_string())
}

fn no_song() -> ApiError {
    ApiError::Refused("no song is open — `song open <path>` or `song init <path>`".to_string())
}
