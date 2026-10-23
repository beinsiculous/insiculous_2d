//! [`SongEdit`]: every change the composer can make to a song, as data. The GUI and the
//! command API's `song edit <json>` both hand one to `SongDocument::apply`, the only path
//! that mutates a song from the editor, so the two cannot drift.
//!
//! Notes are named by number (`NoteId::number`), the id the API shows; a part, an
//! instrument and a binding by name. An edit to the sidecar is checked by
//! `SongFile::check_schema` on the file it would produce, so the editor never holds — and
//! never saves — a sidecar the loader refuses.

use std::collections::{BTreeSet, HashMap};

use music_midi::{
    Binding, EditError, Instrument, NoteId, Part, PartMode, Provenance, Quantize, SongData,
    SongFileProblem,
};
use serde::{Deserialize, Serialize};

use super::change::Change;

mod apply;

/// A time signature as the composer writes it: `denominator` is the note value (4 for a
/// quarter), not MIDI's exponent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meter {
    pub numerator: u8,
    pub denominator: u8,
}

/// One change to a song. See the module docs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SongEdit {
    AddNote {
        track: usize,
        tick: u64,
        key: u8,
        velocity: u8,
        duration_ticks: u64,
    },
    DeleteNotes {
        notes: Vec<u64>,
    },
    /// Shift notes in time; a negative delta moves them earlier.
    MoveNotes {
        notes: Vec<u64>,
        tick_delta: i64,
    },
    ResizeNotes {
        notes: Vec<u64>,
        duration_ticks: u64,
    },
    SetVelocity {
        notes: Vec<u64>,
        velocity: u8,
    },
    SetPitch {
        notes: Vec<u64>,
        key: u8,
    },
    /// Move notes onto the track the named part plays.
    AssignPart {
        notes: Vec<u64>,
        part: String,
    },
    /// The tempo from `tick` on; `None` removes the change on that tick.
    SetTempo {
        tick: u64,
        microseconds_per_quarter: Option<u32>,
    },
    /// The time signature from `tick` on; `None` removes the one on that tick.
    SetTimeSignature {
        tick: u64,
        meter: Option<Meter>,
    },
    SetLoop {
        looping: bool,
    },
    SetQuantize {
        part: String,
        quantize: Quantize,
    },
    SetPartMode {
        part: String,
        mode: PartMode,
    },
    AddInstrument {
        instrument: Instrument,
    },
    /// Replace the instrument called `name` (a new name renames it).
    SetInstrument {
        name: String,
        instrument: Instrument,
    },
    RemoveInstrument {
        name: String,
    },
    /// Add the binding, or replace the one with its action.
    SetBinding {
        binding: Binding,
    },
    RemoveBinding {
        action: String,
    },
    AddPart {
        part: Part,
    },
    RemovePart {
        name: String,
    },
    /// An empty track at the end of a type-1 song, playing General MIDI `program` (0–127) on
    /// `channel`, counted 1 to 16 as musicians count; `None` takes the next free channel and
    /// never 10, the percussion channel, which a track only gets by naming it.
    AddTrack {
        name: String,
        channel: Option<u8>,
        program: u8,
    },
    /// The SoundFont the background plays on, by its path under the asset folder.
    SetSoundFont {
        path: String,
    },
    /// Who wrote the song and under which license: what Save requires of it, and writes
    /// into the home folder's `LICENSE`.
    SetProvenance {
        provenance: Provenance,
    },
    /// Several edits as one undo entry: every member is checked before any lands, and a
    /// refused member changes nothing.
    Batch(Vec<SongEdit>),
}

/// Why an edit was refused. Nothing was changed.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SongEditError {
    #[error("the composer cannot edit this song: {reason}")]
    Refused { reason: String },
    #[error("note {0} does not exist")]
    UnknownNote(u64),
    #[error("the edit names no notes")]
    NoNotes,
    #[error("no part is named '{0}'")]
    UnknownPart(String),
    #[error("no instrument is named '{0}'")]
    UnknownInstrument(String),
    #[error("no binding has the action '{0}'")]
    UnknownBinding(String),
    #[error("there is no tempo change on tick {0}")]
    NoTempoChange(u64),
    #[error("there is no time signature on tick {0}")]
    NoTimeSignature(u64),
    #[error("channel {0} is outside 1 to 16")]
    ChannelOutOfRange(u8),
    #[error("a part selects the track named {0:?}; a second track of that name would leave it ambiguous")]
    TrackNameSelected(String),
    #[error("part {part:?} selects the track named {name:?}, which two tracks carry; select one by index")]
    TrackNameAmbiguous { part: String, name: String },
    #[error("an empty batch changes nothing")]
    EmptyBatch,
    #[error("{0}")]
    Edit(#[from] EditError),
    #[error("the song file would not load: {0}")]
    Schema(#[from] SongFileProblem),
}

/// What applying an edit did, as the history replays it.
pub(super) struct Applied {
    pub undo: Change,
    pub redo: Change,
}

impl SongEdit {
    /// The edit's variant name, as the API's `undid` and `redid` report it.
    pub fn name(&self) -> &'static str {
        match self {
            SongEdit::AddNote { .. } => "AddNote",
            SongEdit::DeleteNotes { .. } => "DeleteNotes",
            SongEdit::MoveNotes { .. } => "MoveNotes",
            SongEdit::ResizeNotes { .. } => "ResizeNotes",
            SongEdit::SetVelocity { .. } => "SetVelocity",
            SongEdit::SetPitch { .. } => "SetPitch",
            SongEdit::AssignPart { .. } => "AssignPart",
            SongEdit::SetTempo { .. } => "SetTempo",
            SongEdit::SetTimeSignature { .. } => "SetTimeSignature",
            SongEdit::SetLoop { .. } => "SetLoop",
            SongEdit::SetQuantize { .. } => "SetQuantize",
            SongEdit::SetPartMode { .. } => "SetPartMode",
            SongEdit::AddInstrument { .. } => "AddInstrument",
            SongEdit::SetInstrument { .. } => "SetInstrument",
            SongEdit::RemoveInstrument { .. } => "RemoveInstrument",
            SongEdit::SetBinding { .. } => "SetBinding",
            SongEdit::RemoveBinding { .. } => "RemoveBinding",
            SongEdit::AddPart { .. } => "AddPart",
            SongEdit::RemovePart { .. } => "RemovePart",
            SongEdit::SetProvenance { .. } => "SetProvenance",
            SongEdit::AddTrack { .. } => "AddTrack",
            SongEdit::SetSoundFont { .. } => "SetSoundFont",
            SongEdit::Batch(_) => "Batch",
        }
    }

    /// The one edit a gesture's two frames make: deltas sum, absolute values replace.
    /// `None` when the two are not one gesture — different variants, different notes,
    /// another tick, part or instrument.
    pub fn try_merge(&self, next: &SongEdit) -> Option<SongEdit> {
        use SongEdit as E;
        match (self, next) {
            (
                E::MoveNotes { notes, tick_delta },
                E::MoveNotes {
                    notes: next_notes,
                    tick_delta: next_delta,
                },
            ) if same_notes(notes, next_notes) => Some(E::MoveNotes {
                notes: notes.clone(),
                tick_delta: tick_delta.checked_add(*next_delta)?,
            }),
            (
                E::ResizeNotes { notes, .. },
                E::ResizeNotes {
                    notes: next_notes, ..
                },
            )
            | (
                E::SetVelocity { notes, .. },
                E::SetVelocity {
                    notes: next_notes, ..
                },
            )
            | (
                E::SetPitch { notes, .. },
                E::SetPitch {
                    notes: next_notes, ..
                },
            ) if same_notes(notes, next_notes) => Some(next.clone()),
            (
                E::SetTempo { tick, .. },
                E::SetTempo {
                    tick: next_tick, ..
                },
            )
            | (
                E::SetTimeSignature { tick, .. },
                E::SetTimeSignature {
                    tick: next_tick, ..
                },
            ) if tick == next_tick => Some(next.clone()),
            (E::SetLoop { .. }, E::SetLoop { .. })
            | (E::SetSoundFont { .. }, E::SetSoundFont { .. })
            | (E::SetProvenance { .. }, E::SetProvenance { .. }) => Some(next.clone()),
            (
                E::SetQuantize { part, .. },
                E::SetQuantize {
                    part: next_part, ..
                },
            )
            | (
                E::SetPartMode { part, .. },
                E::SetPartMode {
                    part: next_part, ..
                },
            ) if part == next_part => Some(next.clone()),
            (
                E::SetInstrument { name, instrument },
                E::SetInstrument {
                    name: next_name, ..
                },
            ) if name == next_name && instrument.name == *name => Some(next.clone()),
            // A composer nudge is a batch per key press (a move, then a pitch per starting
            // key): a held key's batches merge member by member, or not at all.
            (E::Batch(members), E::Batch(next_members)) if members.len() == next_members.len() => {
                members
                    .iter()
                    .zip(next_members)
                    .map(|(member, next_member)| member.try_merge(next_member))
                    .collect::<Option<Vec<_>>>()
                    .map(E::Batch)
            }
            _ => None,
        }
    }
}

/// Whether two note lists name the same notes, in any order.
fn same_notes(first: &[u64], second: &[u64]) -> bool {
    first.iter().collect::<BTreeSet<_>>() == second.iter().collect::<BTreeSet<_>>()
}

/// The ids of the numbered notes, refusing an unknown number or an empty list.
pub(super) fn note_ids(data: &SongData, numbers: &[u64]) -> Result<Vec<NoteId>, SongEditError> {
    if numbers.is_empty() {
        return Err(SongEditError::NoNotes);
    }
    let by_number: HashMap<u64, NoteId> = data
        .tracks()
        .iter()
        .flat_map(|track| track.units())
        .flat_map(|unit| &unit.notes)
        .map(|note| (note.id.number(), note.id))
        .collect();
    numbers
        .iter()
        .map(|number| {
            by_number
                .get(number)
                .copied()
                .ok_or(SongEditError::UnknownNote(*number))
        })
        .collect()
}
