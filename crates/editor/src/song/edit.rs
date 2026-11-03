//! [`SongEdit`]: every change the Conductor panel can make to a song's map, as data. The GUI and
//! the command API's `song edit <json>` both hand one to `SongDocument::apply`, the only
//! path that mutates a song from the editor, so the two cannot drift. The score is never
//! one of them: it is written in MIDI Evil, which exports the `.mid` the engine plays.
//!
//! A part, an instrument and a binding are named by name. Every edit is checked by
//! `SongFile::check_schema` on the file it would produce, so the editor never holds — and
//! never saves — a sidecar the loader refuses.

use music_midi::{Binding, Instrument, Part, PartMode, Provenance, Quantize, SongFileProblem};
use serde::{Deserialize, Serialize};

use super::change::Change;

mod apply;

/// One change to a song. See the module docs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SongEdit {
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
    #[error("no part is named '{0}'")]
    UnknownPart(String),
    #[error("no instrument is named '{0}'")]
    UnknownInstrument(String),
    #[error("no binding has the action '{0}'")]
    UnknownBinding(String),
    #[error("part {part:?} selects the track named {name:?}, which two tracks carry; select one by index")]
    TrackNameAmbiguous { part: String, name: String },
    #[error("an empty batch changes nothing")]
    EmptyBatch,
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
            SongEdit::SetSoundFont { .. } => "SetSoundFont",
            SongEdit::Batch(_) => "Batch",
        }
    }

    /// The one edit a gesture's two frames make: absolute values replace. `None` when the
    /// two are not one gesture — different variants, another part or instrument.
    pub fn try_merge(&self, next: &SongEdit) -> Option<SongEdit> {
        use SongEdit as E;
        match (self, next) {
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
            _ => None,
        }
    }
}
