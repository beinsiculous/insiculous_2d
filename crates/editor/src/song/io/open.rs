//! Opening a song, its `.mid` and its sidecar read as they are.
//!
//! - **A `.mid` its sidecar fingerprints differently** is the score changed outside: it
//!   opens on the disk's bytes with its sidecar, a note saying so, and the track diff's
//!   lines against no old score; the next save rewrites the fingerprint.
//! - **A `.mid` with no sidecar** opens in place on a scaffold sidecar
//!   ([`scaffold_song_file`]), the map unsaved, so Save writes it.
//! - **A sidecar with no `.mid`** is no song to open: `Missing`, nothing written.
//!
//! A `.mid` outside the song folders opens as an import and none of its neighbours is
//! touched. Nothing an open does writes a file.

use std::io;
use std::path::{Path, PathBuf};

use music_midi::{parse_song_file, SongError, SongFile, SongFileError};

use super::{is_in_home_directory, midi_fingerprint, SongStore, VfsStore};
use crate::external_tools::{diff_tracks, midi_sidecar_path, TrackListing};
use crate::song::{default_song_file, scaffold_song_file, SongDocument};

/// A song opened, and what opening it settled, for the status bar.
#[derive(Debug)]
pub struct OpenedSong {
    pub document: SongDocument,
    pub notes: Vec<String>,
}

/// Why a song could not be opened. Nothing was written.
#[derive(Debug, thiserror::Error)]
pub enum SongOpenError {
    #[error("{} does not exist", path.display())]
    Missing { path: PathBuf },
    #[error("{} cannot be read: {reason}", path.display())]
    Unreadable { path: PathBuf, reason: String },
    #[error("{}: {error}", path.display())]
    Midi { path: PathBuf, error: SongError },
    #[error("{0}")]
    Sidecar(#[from] SongFileError),
}

impl SongDocument {
    /// Open the song at `path` (its `.mid`) through `common::vfs`, as the module docs lay
    /// out. `asset_base` places the song folders and the bank a scaffold sidecar plays.
    pub fn open(path: &Path, asset_base: &Path) -> Result<OpenedSong, SongOpenError> {
        open_from(&VfsStore, path, asset_base)
    }

    /// Open the `.mid` at `path`, and its sidecar if it has one, as an import: dirty, with
    /// no path, so its first save is a Save As and the file is never written.
    pub fn import(path: &Path) -> Result<SongDocument, SongOpenError> {
        import_from(&VfsStore, path)
    }
}

pub(crate) fn open_from(
    store: &impl SongStore,
    path: &Path,
    asset_base: &Path,
) -> Result<OpenedSong, SongOpenError> {
    if !is_in_home_directory(path, asset_base) {
        return Ok(OpenedSong {
            document: import_from(store, path)?,
            notes: vec![format!(
                "{} is outside the song folders: opened as an import, Save As to keep it",
                path.display()
            )],
        });
    }
    let sidecar_path = midi_sidecar_path(path);
    let midi = read_optional(store, path)?;
    let sidecar = read_optional(store, &sidecar_path)?;
    let Some(midi) = midi else {
        return Err(SongOpenError::Missing {
            path: path.to_path_buf(),
        });
    };
    let mut notes = Vec::new();
    let mut document = match &sidecar {
        Some(sidecar_bytes) => {
            let sidecar = parse_sidecar(&sidecar_path, sidecar_bytes)?;
            let changed_outside = sidecar
                .midi_fingerprint
                .is_some_and(|fingerprint| fingerprint != midi_fingerprint(&midi));
            let document = document_from(path, &midi, Some(sidecar), Some(sidecar_bytes))?;
            if changed_outside {
                notes.push(format!(
                    "{} changed outside the editor since its sidecar was saved: opened on the \
                     new score with its map — Save to record it",
                    path.display()
                ));
                let listing = TrackListing::of(document.file(), document.data());
                notes.extend(diff_tracks(&TrackListing::empty(), &listing).texts());
            }
            document
        }
        None => {
            notes.push(format!(
                "{} has no sidecar: opened with a new one, unsaved — Save writes it",
                path.display()
            ));
            document_from(path, &midi, Some(scaffold_song_file(asset_base)), None)?
        }
    };
    document.path = Some(path.to_path_buf());
    Ok(OpenedSong { document, notes })
}

fn import_from(store: &impl SongStore, path: &Path) -> Result<SongDocument, SongOpenError> {
    let midi = read_optional(store, path)?.ok_or_else(|| SongOpenError::Missing {
        path: path.to_path_buf(),
    })?;
    let sidecar_path = midi_sidecar_path(path);
    let sidecar_bytes = read_optional(store, &sidecar_path)?;
    let sidecar = match &sidecar_bytes {
        Some(bytes) => Some(parse_sidecar(&sidecar_path, bytes)?),
        None => None,
    };
    let mut document = document_from(path, &midi, sidecar, sidecar_bytes.as_deref())?;
    document.imported_from = Some(path.to_path_buf());
    Ok(document)
}

/// A document over these bytes with no path yet, its sidecar's fingerprint taken from
/// `sidecar_bytes` — the file as read, `None` when there is none on disk. The sidecar's own
/// `midi_fingerprint` is dropped, since it describes a file this document has not written.
fn document_from(
    path: &Path,
    midi: &[u8],
    sidecar: Option<SongFile>,
    sidecar_bytes: Option<&[u8]>,
) -> Result<SongDocument, SongOpenError> {
    let mut file = sidecar.unwrap_or_else(default_song_file);
    file.midi_fingerprint = None;
    let mut document = SongDocument::from_smf(midi, file).map_err(|error| SongOpenError::Midi {
        path: path.to_path_buf(),
        error,
    })?;
    document.map_fingerprint = sidecar_bytes.map(midi_fingerprint);
    Ok(document)
}

fn parse_sidecar(path: &Path, bytes: &[u8]) -> Result<SongFile, SongOpenError> {
    let label = path.display().to_string();
    let text = std::str::from_utf8(bytes).map_err(|error| SongOpenError::Unreadable {
        path: path.to_path_buf(),
        reason: error.to_string(),
    })?;
    Ok(parse_song_file(&label, text)?)
}

fn read_optional(store: &impl SongStore, path: &Path) -> Result<Option<Vec<u8>>, SongOpenError> {
    match store.read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(SongOpenError::Unreadable {
            path: path.to_path_buf(),
            reason: error.to_string(),
        }),
    }
}
