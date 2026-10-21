//! The composer's song: a second document beside the scene, with its own history, save
//! watermark and selection.
//!
//! A [`SongDocument`] holds the sidecar ([`SongFile`]) and the MIDI data ([`SongData`]) it
//! saves as `<stem>.mid.ron` and `<stem>.mid`. Every change goes through
//! [`SongDocument::apply`] — the GUI's and the command API's alike — and lands on the
//! song's [`SongHistory`], never the scene's. A song is dirty when its history moved past
//! the save watermark or when it has never been saved at all. Saving and opening are the
//! journal in [`io`].

mod change;
mod edit;
mod history;
mod io;
mod selection;

#[cfg(test)]
mod edit_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod io_tests;
#[cfg(test)]
mod open_tests;
#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod test_support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use music_midi::midly::num::{u15, u24, u28, u4, u7};
use music_midi::midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
};
use music_midi::{
    NoteId, SongData, SongError, SongFile, UnsupportedReport, DEFAULT_MICROSECONDS_PER_QUARTER,
    SONG_FILE_VERSION,
};

pub use edit::{Meter, SongEdit, SongEditError};
pub use history::{Gesture, SongHistory};
pub use io::{
    midi_fingerprint, OpenedSong, SongOpenError, SongSaveError, HOME_DIRECTORIES, SAMPLE_DIRECTORY,
};
pub use selection::SongSelection;

/// The SoundFont a new or sidecar-less song plays its background on: the bank every game
/// copies in, by its path under the asset folder.
pub const DEFAULT_SOUND_FONT_PATH: &str = "soundfonts/cast.sf2";

/// The division of a new song, in ticks per quarter note.
const NEW_SONG_TICKS_PER_QUARTER: u16 = 480;

/// Whether the composer may edit a song.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditingPolicy {
    Full,
    /// The MIDI file holds something the editing model cannot represent; every edit is
    /// refused with this reason, and the song still opens, plays and saves as it was.
    Refused {
        reason: String,
    },
}

impl EditingPolicy {
    /// The policy `SongData::from_smf`'s report calls for: refused, with its features
    /// joined, when any of them makes the file uneditable.
    pub fn from_report(report: &UnsupportedReport) -> Self {
        if report.is_editable() {
            return EditingPolicy::Full;
        }
        let reasons: Vec<String> = report
            .unsupported
            .iter()
            .map(|feature| feature.to_string())
            .collect();
        EditingPolicy::Refused {
            reason: reasons.join("; "),
        }
    }
}

/// See the module docs.
#[derive(Debug, Clone)]
pub struct SongDocument {
    file: SongFile,
    data: SongData,
    history: SongHistory,
    selection: SongSelection,
    /// Where the song saves; `None` until its first save into a song folder.
    path: Option<PathBuf>,
    /// The file it was imported from, which no save ever writes.
    imported_from: Option<PathBuf>,
    editing: EditingPolicy,
    /// Sample bytes the song holds until its first save puts them beside it, by the path
    /// its instrument names them by.
    pending_samples: BTreeMap<String, Vec<u8>>,
}

impl SongDocument {
    /// An empty song: one tempo track and one melody track on channel 1, 120 BPM in 4/4,
    /// never saved.
    pub fn new_song() -> Result<Self, SongError> {
        Self::from_smf(&blank_song_bytes()?, default_song_file())
    }

    /// A song read from MIDI bytes with `file` as its sidecar, never saved: the report's
    /// unsupported features decide the [`EditingPolicy`].
    pub fn from_smf(bytes: &[u8], file: SongFile) -> Result<Self, SongError> {
        let (data, report) = SongData::from_smf(bytes)?;
        Ok(Self {
            file,
            data,
            history: SongHistory::new(),
            selection: SongSelection::new(),
            path: None,
            imported_from: None,
            editing: EditingPolicy::from_report(&report),
            pending_samples: BTreeMap::new(),
        })
    }

    pub fn file(&self) -> &SongFile {
        &self.file
    }

    pub fn data(&self) -> &SongData {
        &self.data
    }

    pub fn history(&self) -> &SongHistory {
        &self.history
    }

    pub fn selection(&self) -> &SongSelection {
        &self.selection
    }

    /// The `.mid` the song saves to, once it has one.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The file the song was imported from; Save never writes it.
    pub fn imported_from(&self) -> Option<&Path> {
        self.imported_from.as_deref()
    }

    pub fn editing(&self) -> &EditingPolicy {
        &self.editing
    }

    /// The samples held until the song's first save, by the path their instrument names.
    pub fn pending_samples(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.pending_samples
    }

    /// Whether the song has edits no save holds: its history past the watermark, or no
    /// save at all.
    pub fn is_dirty(&self) -> bool {
        self.history.is_dirty() || self.path.is_none()
    }

    /// Apply one edit and record it — the one way the editor changes a song. A refused
    /// edit changes nothing and records nothing.
    pub fn apply(&mut self, edit: SongEdit, gesture: Gesture) -> Result<(), SongEditError> {
        if let EditingPolicy::Refused { reason } = &self.editing {
            return Err(SongEditError::Refused {
                reason: reason.clone(),
            });
        }
        let applied = edit.apply_to(&mut self.data, &mut self.file)?;
        self.history
            .record(edit, applied.undo, applied.redo, gesture);
        self.selection.retain_existing(&self.data);
        Ok(())
    }

    /// Undo the edit on top; `Ok(None)` when there is none, else the undone edit's name.
    pub fn undo(&mut self) -> Result<Option<&'static str>, SongEditError> {
        let Some(entry) = self.history.pop_undo() else {
            return Ok(None);
        };
        match entry.undo.apply(&mut self.data, &mut self.file) {
            Ok(()) => {
                let name = entry.edit.name();
                self.history.push_undone(entry);
                self.selection.retain_existing(&self.data);
                Ok(Some(name))
            }
            Err(error) => {
                self.history.restore_undo(entry);
                Err(error.into())
            }
        }
    }

    /// Redo the last undone edit; `Ok(None)` when there is none, else its name.
    pub fn redo(&mut self) -> Result<Option<&'static str>, SongEditError> {
        let Some(entry) = self.history.pop_redo() else {
            return Ok(None);
        };
        match entry.redo.apply(&mut self.data, &mut self.file) {
            Ok(()) => {
                let name = entry.edit.name();
                self.history.push_redone(entry);
                self.selection.retain_existing(&self.data);
                Ok(Some(name))
            }
            Err(error) => {
                self.history.restore_redo(entry);
                Err(error.into())
            }
        }
    }

    /// End the gesture on top: the next continuing edit is an undo entry of its own.
    pub fn break_merge(&mut self) {
        self.history.break_merge();
    }

    /// Select exactly the numbered notes; an empty list clears the selection.
    pub fn select_notes(&mut self, numbers: &[u64]) -> Result<(), SongEditError> {
        let notes = if numbers.is_empty() {
            Vec::new()
        } else {
            edit::note_ids(&self.data, numbers)?
        };
        self.selection.replace(notes);
        Ok(())
    }

    /// The note with this number, if the song holds it.
    pub fn note_id(&self, number: u64) -> Option<NoteId> {
        edit::note_ids(&self.data, &[number])
            .ok()
            .and_then(|ids| ids.first().copied())
    }

    /// Hold a sample's bytes until the next save writes them beside the song, under the
    /// path a `Sample` instrument names (an imported file's own path, or any name). Not an
    /// edit: the instrument that names it is.
    pub fn stage_sample(&mut self, path: impl Into<String>, bytes: Vec<u8>) {
        self.pending_samples.insert(path.into(), bytes);
    }
}

/// A sidecar with no parts, instruments or bindings, playing its background on
/// [`DEFAULT_SOUND_FONT_PATH`].
pub fn default_song_file() -> SongFile {
    SongFile {
        version: SONG_FILE_VERSION,
        soundfont: DEFAULT_SOUND_FONT_PATH.to_string(),
        looping: true,
        parts: Vec::new(),
        instruments: Vec::new(),
        bindings: Vec::new(),
        midi_fingerprint: None,
        resources: Default::default(),
    }
}

/// A type-1 file: a tempo track at 120 BPM in 4/4, and a track named `melody` whose
/// program change gives it channel 1 to write notes on.
fn blank_song_bytes() -> Result<Vec<u8>, SongError> {
    let event = |kind| TrackEvent {
        delta: u28::new(0),
        kind,
    };
    let tempo_track = vec![
        event(TrackEventKind::Meta(MetaMessage::Tempo(u24::new(
            DEFAULT_MICROSECONDS_PER_QUARTER,
        )))),
        event(TrackEventKind::Meta(MetaMessage::TimeSignature(
            4, 2, 24, 8,
        ))),
        event(TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let melody_track = vec![
        event(TrackEventKind::Meta(MetaMessage::TrackName(b"melody"))),
        event(TrackEventKind::Midi {
            channel: u4::new(0),
            message: MidiMessage::ProgramChange {
                program: u7::new(0),
            },
        }),
        event(TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let header = Header::new(
        Format::Parallel,
        Timing::Metrical(u15::new(NEW_SONG_TICKS_PER_QUARTER)),
    );
    let mut bytes = Vec::new();
    Smf {
        header,
        tracks: vec![tempo_track, melody_track],
    }
    .write_std(&mut bytes)
    .map_err(|error| SongError::WriteFailed {
        reason: error.to_string(),
    })?;
    Ok(bytes)
}
