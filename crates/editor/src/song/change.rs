//! The exact changes undo and redo replay. An edit is validated once, when it is applied;
//! what the history keeps is what it did — a whole sidecar — so replaying never resolves a
//! name again.

use music_midi::SongFile;

#[derive(Debug, Clone)]
pub(super) enum Change {
    /// The whole sidecar.
    File(Box<SongFile>),
    /// Changes in order.
    Several(Vec<Change>),
}

impl Change {
    pub(super) fn apply(&self, file: &mut SongFile) {
        match self {
            Change::File(image) => *file = SongFile::clone(image),
            Change::Several(changes) => changes.iter().for_each(|change| change.apply(file)),
        }
    }
}
