//! The `song …` verbs: their parser arm, the two song queries, and the pure song writes,
//! which run through `SongDocument::apply` exactly as the composer's GUI does.
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
use crate::song::{EditingPolicy, Gesture, SongDocument, SongEdit, SongEditError};

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
        let edit: SongEdit = serde_json::from_str(rest).map_err(|error| {
            ApiError::Parse(format!("song edit needs a SongEdit as JSON: {error}"))
        })?;
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
        (Some("select"), [none]) if none == "none" => {
            Ok(Request::Write(WriteCmd::Pure(PureWrite::SongSelect {
                notes: Vec::new(),
            })))
        }
        (Some("select"), numbers) if !numbers.is_empty() => {
            let notes = numbers
                .iter()
                .map(|number| {
                    number
                        .parse::<u64>()
                        .map_err(|_| ApiError::Parse(format!("\"{number}\" is not a note number")))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Request::Write(WriteCmd::Pure(PureWrite::SongSelect {
                notes,
            })))
        }
        (Some("new"), []) => hosted(HostedWrite::SongNew { discard }),
        (Some("open"), [path]) => hosted(HostedWrite::SongOpen {
            path: path.clone(),
            discard,
        }),
        (Some("close"), []) => hosted(HostedWrite::SongClose { discard }),
        (Some("save"), [] | [_]) if !discard => hosted(HostedWrite::SongSave {
            path: arguments.first().cloned(),
        }),
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
             song select <note>…|none | song new [discard] | song open <path> [discard] | \
             song save [path] | song close [discard] | song play [beat] | song stop | \
             song seek <beat> | song trigger <action>"
                .to_string(),
        )),
    }
}

fn parse_beat(word: &str) -> Result<f64, ApiError> {
    word.parse::<f64>()
        .ok()
        .filter(|beat| beat.is_finite() && *beat >= 0.0)
        .ok_or_else(|| ApiError::Parse(format!("\"{word}\" is not a beat (a number from 0)")))
}

/// The `song` query: the open song's file, tracks, timing, state and selection, or
/// `{"open": false}`.
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
    let refused_reason = match song.editing() {
        EditingPolicy::Full => None,
        EditingPolicy::Refused { reason } => Some(reason.clone()),
    };
    json!({
        "open": true,
        "path": song.path().map(|path| path.display().to_string()),
        "imported_from": song.imported_from().map(|path| path.display().to_string()),
        "dirty": song.is_dirty(),
        "editable": refused_reason.is_none(),
        "refused_reason": refused_reason,
        "ticks_per_quarter": data.ticks_per_quarter(),
        "soundfont": file.soundfont,
        "looping": file.looping,
        "tracks": tracks,
        "parts": serde_json::to_value(&file.parts).unwrap_or(Value::Null),
        "instruments": serde_json::to_value(&file.instruments).unwrap_or(Value::Null),
        "bindings": serde_json::to_value(&file.bindings).unwrap_or(Value::Null),
        "tempo_changes": tempo_changes,
        "time_signatures": time_signatures,
        "selection": song.selection().notes().map(|note| note.number()).collect::<Vec<_>>(),
        "pending_samples": song.pending_samples().keys().collect::<Vec<_>>(),
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
            let before: Vec<u64> = note_numbers(song);
            song.apply(edit.clone(), Gesture::Discrete)
                .map_err(edit_refusal)?;
            let added: Vec<u64> = note_numbers(song)
                .into_iter()
                .filter(|number| !before.contains(number))
                .collect();
            Ok(json!({
                "command": "song edit",
                "edit": edit.name(),
                "added_notes": added,
                "dirty": song.is_dirty(),
            }))
        }
        PureWrite::SongUndo => Ok(json!({ "undid": song.undo().map_err(edit_refusal)? })),
        PureWrite::SongRedo => Ok(json!({ "redid": song.redo().map_err(edit_refusal)? })),
        PureWrite::SongSelect { notes } => {
            song.select_notes(notes).map_err(edit_refusal)?;
            let selection: Vec<u64> = song.selection().notes().map(|note| note.number()).collect();
            Ok(json!({ "selection": selection }))
        }
        other => Err(ApiError::Invalid(format!("{other:?} is not a song write"))),
    }
}

fn note_numbers(song: &SongDocument) -> Vec<u64> {
    song.data()
        .tracks()
        .iter()
        .flat_map(|track| track.units())
        .flat_map(|unit| &unit.notes)
        .map(|note| note.id.number())
        .collect()
}

fn edit_refusal(error: SongEditError) -> ApiError {
    match error {
        SongEditError::Refused { .. } => ApiError::Refused(error.to_string()),
        SongEditError::UnknownNote(_) => ApiError::NotFound(error.to_string()),
        other => ApiError::Invalid(other.to_string()),
    }
}

fn no_song() -> ApiError {
    ApiError::Refused("no song is open — `song new` or `song open <path>`".to_string())
}
