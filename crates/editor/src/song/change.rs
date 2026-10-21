//! The exact changes undo and redo replay. An edit is validated once, when it is applied;
//! what the history keeps is what it did — notes put back under their own ids, a tempo or
//! a meter on its tick, a whole sidecar — so replaying never resolves a name or a number
//! again and a redone note keeps the id later entries name it by.

use music_midi::{EditError, NoteId, NoteImage, SongData, SongFile, TempoChange, TimeSignature};

#[derive(Debug, Clone)]
pub(super) enum Change {
    /// Put notes back exactly as their images describe them, re-creating removed ones.
    RestoreNotes(Vec<NoteImage>),
    RemoveNotes(Vec<NoteId>),
    /// The tempo change on `tick`, or none there.
    Tempo {
        tick: u64,
        change: Option<TempoChange>,
    },
    /// The time signature on `tick`, or none there.
    TimeSignature {
        tick: u64,
        signature: Option<TimeSignature>,
    },
    /// The whole sidecar.
    File(Box<SongFile>),
    /// Changes in order.
    Several(Vec<Change>),
}

impl Change {
    pub(super) fn apply(&self, data: &mut SongData, file: &mut SongFile) -> Result<(), EditError> {
        match self {
            Change::RestoreNotes(images) => data.restore_notes(images),
            Change::RemoveNotes(ids) => data.remove_notes(ids).map(drop),
            Change::Tempo { tick, change } => match change {
                Some(change) => data.set_tempo(*change).map(drop),
                None => {
                    data.remove_tempo(*tick);
                    Ok(())
                }
            },
            Change::TimeSignature { tick, signature } => match signature {
                Some(signature) => data.set_time_signature(*signature).map(drop),
                None => {
                    data.remove_time_signature(*tick);
                    Ok(())
                }
            },
            Change::File(image) => {
                *file = SongFile::clone(image);
                Ok(())
            }
            Change::Several(changes) => changes
                .iter()
                .try_for_each(|change| change.apply(data, file)),
        }
    }
}
