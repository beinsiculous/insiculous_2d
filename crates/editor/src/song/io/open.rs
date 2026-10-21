//! Opening a song, and settling whatever an interrupted save left beside it.
//!
//! In order:
//! - **A `.new` pair whose `.mid.new` matches its `.mid.ron.new` fingerprint is the
//!   truth.** It is recovered over the finals whatever state they are in — a save stopped
//!   between its two final writes leaves them mismatched, and the pair, removed last, is
//!   still there — by finishing that save: the finals written from it, the pair removed.
//! - **Otherwise the finals stand when they match each other** (the `.mid` against the
//!   `.mid.ron`'s fingerprint; a hand-written sidecar has none and always matches). A
//!   leftover is purged when it matches them; a `.mid.new` that differs is opened instead,
//!   as an import (dirty, no path, the finals untouched), so its work is never lost; a lone
//!   `.mid.ron.new` is removed.
//! - **Finals that do not match, with no valid pair, are an error naming both files.**
//!
//! The finals' fingerprint decides a purge; the pair's own fingerprint decides a recovery —
//! never the other way. A `.mid` outside the song folders, or without a sidecar, opens as
//! an import and none of its neighbours is touched. On the web a removed `.new` lingers in
//! the project store until the store persists deletions; these rules keep that safe.

use std::io;
use std::path::{Path, PathBuf};

use music_midi::{parse_song_file, SongError, SongFile, SongFileError};

use super::{is_in_home_directory, midi_fingerprint, with_suffix, SongStore, VfsStore};
use crate::song::{default_song_file, SongDocument};

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
    #[error(
        "{} and {} do not match: the sidecar was saved with another MIDI file, and no \
         interrupted save is left to restore the pair",
        midi.display(),
        sidecar.display()
    )]
    MismatchedFinals { midi: PathBuf, sidecar: PathBuf },
    #[error("finishing the save interrupted at {}: {reason}", path.display())]
    Recovery { path: PathBuf, reason: String },
}

/// The four files a song's journal may leave, read.
struct SongFiles {
    midi: Option<Vec<u8>>,
    sidecar: Option<Vec<u8>>,
    journal_midi: Option<Vec<u8>>,
    journal_sidecar: Option<Vec<u8>>,
}

/// The four paths of a song at `midi`.
struct SongPaths {
    midi: PathBuf,
    sidecar: PathBuf,
    journal_midi: PathBuf,
    journal_sidecar: PathBuf,
}

impl SongPaths {
    fn of(midi: &Path) -> Self {
        let sidecar = with_suffix(midi, ".ron");
        Self {
            journal_midi: with_suffix(midi, ".new"),
            journal_sidecar: with_suffix(&sidecar, ".new"),
            midi: midi.to_path_buf(),
            sidecar,
        }
    }
}

impl SongDocument {
    /// Open the song at `path` (its `.mid`) through `common::vfs`, settling an
    /// interrupted save as the module docs lay out.
    pub fn open(path: &Path, asset_base: &Path) -> Result<OpenedSong, SongOpenError> {
        open_from(&mut VfsStore, path, asset_base)
    }

    /// Open the `.mid` at `path`, and its sidecar if it has one, as an import: dirty, with
    /// no path, so its first save is a Save As and the file is never written.
    pub fn import(path: &Path) -> Result<SongDocument, SongOpenError> {
        import_from(&VfsStore, path)
    }
}

pub(crate) fn open_from(
    store: &mut impl SongStore,
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
    let paths = SongPaths::of(path);
    let files = SongFiles {
        midi: read_optional(store, &paths.midi)?,
        sidecar: read_optional(store, &paths.sidecar)?,
        journal_midi: read_optional(store, &paths.journal_midi)?,
        journal_sidecar: read_optional(store, &paths.journal_sidecar)?,
    };
    if let Some(recovered) = recover_pair(store, &paths, &files)? {
        return Ok(recovered);
    }
    let mut notes = Vec::new();
    let (Some(midi), Some(sidecar_bytes)) = (&files.midi, &files.sidecar) else {
        return open_without_finals(store, &paths, &files);
    };
    let sidecar = parse_sidecar(&paths.sidecar, sidecar_bytes)?;
    if sidecar
        .midi_fingerprint
        .is_some_and(|fingerprint| fingerprint != midi_fingerprint(midi))
    {
        return Err(SongOpenError::MismatchedFinals {
            midi: paths.midi.clone(),
            sidecar: paths.sidecar.clone(),
        });
    }
    if let Some(journal_midi) = &files.journal_midi {
        if journal_midi != midi {
            let mut document = document_from(&paths.journal_midi, journal_midi, Some(sidecar))?;
            document.imported_from = Some(paths.journal_midi.clone());
            remove_noting(store, &paths.journal_sidecar, &mut notes);
            notes.push(format!(
                "{} holds edits the saved song lacks: opened as an import, the saved files \
                 untouched — Save to keep it",
                paths.journal_midi.display()
            ));
            return Ok(OpenedSong { document, notes });
        }
        remove_noting(store, &paths.journal_midi, &mut notes);
    }
    if files.journal_sidecar.is_some() {
        remove_noting(store, &paths.journal_sidecar, &mut notes);
    }
    let mut document = document_from(&paths.midi, midi, Some(sidecar))?;
    document.path = Some(paths.midi.clone());
    Ok(OpenedSong { document, notes })
}

/// A `.new` pair that matches itself, made the finals; `None` when there is no such pair.
fn recover_pair(
    store: &mut impl SongStore,
    paths: &SongPaths,
    files: &SongFiles,
) -> Result<Option<OpenedSong>, SongOpenError> {
    let (Some(journal_midi), Some(journal_sidecar)) = (&files.journal_midi, &files.journal_sidecar)
    else {
        return Ok(None);
    };
    let Ok(sidecar) = parse_sidecar(&paths.journal_sidecar, journal_sidecar) else {
        return Ok(None);
    };
    if sidecar.midi_fingerprint != Some(midi_fingerprint(journal_midi)) {
        return Ok(None);
    }
    let recovery_failed = |path: &Path, error: io::Error| SongOpenError::Recovery {
        path: path.to_path_buf(),
        reason: error.to_string(),
    };
    store
        .write(&paths.midi, journal_midi)
        .map_err(|error| recovery_failed(&paths.midi, error))?;
    store
        .write(&paths.sidecar, journal_sidecar)
        .map_err(|error| recovery_failed(&paths.sidecar, error))?;
    for journal in [&paths.journal_midi, &paths.journal_sidecar] {
        store
            .remove(journal)
            .map_err(|error| recovery_failed(journal, error))?;
    }
    let mut document = document_from(&paths.midi, journal_midi, Some(sidecar))?;
    document.path = Some(paths.midi.clone());
    Ok(Some(OpenedSong {
        document,
        notes: vec![format!(
            "finished the save interrupted at {}",
            paths.midi.display()
        )],
    }))
}

/// No pair to recover and the finals incomplete: a `.mid` without a sidecar, or only what
/// an interrupted first save left.
fn open_without_finals(
    store: &mut impl SongStore,
    paths: &SongPaths,
    files: &SongFiles,
) -> Result<OpenedSong, SongOpenError> {
    let mut notes = Vec::new();
    if let Some(midi) = &files.midi {
        if files.journal_sidecar.is_some() && files.journal_midi.is_none() {
            remove_noting(store, &paths.journal_sidecar, &mut notes);
        }
        notes.push(format!(
            "{} has no sidecar: opened as an import, Save to give it one",
            paths.midi.display()
        ));
        let mut document = document_from(&paths.midi, midi, None)?;
        document.imported_from = Some(paths.midi.clone());
        return Ok(OpenedSong { document, notes });
    }
    if let Some(journal_midi) = &files.journal_midi {
        let sidecar = files
            .journal_sidecar
            .as_ref()
            .and_then(|bytes| parse_sidecar(&paths.journal_sidecar, bytes).ok());
        let mut document = document_from(&paths.journal_midi, journal_midi, sidecar)?;
        document.imported_from = Some(paths.journal_midi.clone());
        notes.push(format!(
            "{} is all an interrupted first save left: opened as an import, Save to keep it",
            paths.journal_midi.display()
        ));
        return Ok(OpenedSong { document, notes });
    }
    if files.journal_sidecar.is_some() {
        remove_noting(store, &paths.journal_sidecar, &mut notes);
    }
    Err(SongOpenError::Missing {
        path: paths.midi.clone(),
    })
}

fn import_from(store: &impl SongStore, path: &Path) -> Result<SongDocument, SongOpenError> {
    let midi = read_optional(store, path)?.ok_or_else(|| SongOpenError::Missing {
        path: path.to_path_buf(),
    })?;
    let sidecar_path = with_suffix(path, ".ron");
    let sidecar = match read_optional(store, &sidecar_path)? {
        Some(bytes) => Some(parse_sidecar(&sidecar_path, &bytes)?),
        None => None,
    };
    let mut document = document_from(path, &midi, sidecar)?;
    document.imported_from = Some(path.to_path_buf());
    Ok(document)
}

/// A never-saved document over these bytes; a sidecar's fingerprint is dropped, since it
/// describes a file this document has not written.
fn document_from(
    path: &Path,
    midi: &[u8],
    sidecar: Option<SongFile>,
) -> Result<SongDocument, SongOpenError> {
    let mut file = sidecar.unwrap_or_else(default_song_file);
    file.midi_fingerprint = None;
    SongDocument::from_smf(midi, file).map_err(|error| SongOpenError::Midi {
        path: path.to_path_buf(),
        error,
    })
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

/// Remove a leftover the open settled, noting it; a failed removal is noted too and
/// opening goes on — the leftover is settled the same way next time.
fn remove_noting(store: &mut impl SongStore, path: &Path, notes: &mut Vec<String>) {
    match store.remove(path) {
        Ok(()) => notes.push(format!(
            "removed {}, left by an interrupted save",
            path.display()
        )),
        Err(error) => notes.push(format!("could not remove {}: {error}", path.display())),
    }
}
