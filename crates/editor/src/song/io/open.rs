//! Opening a song, and keeping whatever an interrupted save left beside it.
//!
//! **Recovery keeps, never promotes.** Each journal file present — `<name>.new` and
//! `<name>.ron.new` — is renamed beside the song to `<stem>.recovered-<fingerprint>.<ext>`
//! and that name's sidecar, under a name no file holds (`recover.rs`), with a note naming
//! it; then the finals open as they are. A pair takes its MIDI file's fingerprint for both,
//! so the recovered sidecar sits beside the score it describes and the recovery opens as a
//! song; a lone file takes its own bytes' fingerprint. Nothing is deleted and the finals
//! are never judged against the journal: the person opens the recovery if they want it.
//!
//! - **No final at all** — a first save interrupted before them — is the one case a
//!   journal becomes the finals: it is the song's only copy, nothing is replaced, and a
//!   note says the save was finished. A lone `.mid.ron.new` there holds no score, and the
//!   open is `Missing` with nothing written.
//! - **A `.mid` its sidecar fingerprints differently** is the score changed outside: it
//!   opens on the disk's bytes with its sidecar, a note saying so, and the track diff's
//!   lines against no old score; the next save rewrites the fingerprint.
//! - **A `.mid` with no sidecar** opens in place on a scaffold sidecar
//!   ([`scaffold_song_file`]), the map unsaved, so Save writes it.
//! - **A sidecar with no `.mid`** is no song to open: `Missing`, nothing written.
//!
//! A `.mid` outside the song folders opens as an import and none of its neighbours is
//! touched. On the web the project store persists every write and removal in the order
//! they were made, so a reload finds a state a native crash could leave, which these rules
//! settle the same way.

use std::io;
use std::path::{Path, PathBuf};

use music_midi::{parse_song_file, SongError, SongFile, SongFileError};

use super::{is_in_home_directory, midi_fingerprint, recover, with_suffix, SongStore, VfsStore};
use crate::external_tools::{diff_tracks, midi_sidecar_path, TrackListing};
use crate::song::{default_song_file, scaffold_song_file, SongDocument};

/// A song opened, and what opening it settled, for the status bar.
#[derive(Debug)]
pub struct OpenedSong {
    pub document: SongDocument,
    pub notes: Vec<String>,
}

/// Why a song could not be opened. Nothing was written, unless finishing a first save
/// was what failed.
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
        let sidecar = midi_sidecar_path(midi);
        Self {
            journal_midi: with_suffix(midi, ".new"),
            journal_sidecar: with_suffix(&sidecar, ".new"),
            midi: midi.to_path_buf(),
            sidecar,
        }
    }
}

impl SongDocument {
    /// Open the song at `path` (its `.mid`) through `common::vfs`, keeping what an
    /// interrupted save left as the module docs lay out. `asset_base` places the song
    /// folders and the bank a scaffold sidecar plays.
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
    if files.midi.is_none() && files.sidecar.is_none() {
        return finish_first_save(store, &paths, &files, asset_base);
    }
    let Some(midi) = &files.midi else {
        return Err(SongOpenError::Missing { path: paths.midi });
    };
    let mut notes = Vec::new();
    let mut document = match &files.sidecar {
        Some(sidecar_bytes) => {
            let sidecar = parse_sidecar(&paths.sidecar, sidecar_bytes)?;
            let changed_outside = sidecar
                .midi_fingerprint
                .is_some_and(|fingerprint| fingerprint != midi_fingerprint(midi));
            let document = document_from(&paths.midi, midi, Some(sidecar), Some(sidecar_bytes))?;
            if changed_outside {
                notes.push(format!(
                    "{} changed outside the editor since its sidecar was saved: opened on the \
                     new score with its map — Save to record it",
                    paths.midi.display()
                ));
                let listing = TrackListing::of(document.file(), document.data());
                notes.extend(diff_tracks(&TrackListing::empty(), &listing).texts());
            }
            document
        }
        None => {
            notes.push(format!(
                "{} has no sidecar: opened with a new one, unsaved — Save writes it",
                paths.midi.display()
            ));
            document_from(&paths.midi, midi, Some(scaffold_song_file(asset_base)), None)?
        }
    };
    let mut kept = keep_journals(store, &paths, &files);
    kept.append(&mut notes);
    document.path = Some(paths.midi.clone());
    Ok(OpenedSong {
        document,
        notes: kept,
    })
}

/// No final at all: the journal, the song's only copy, becomes the finals.
fn finish_first_save(
    store: &mut impl SongStore,
    paths: &SongPaths,
    files: &SongFiles,
    asset_base: &Path,
) -> Result<OpenedSong, SongOpenError> {
    let Some(journal_midi) = &files.journal_midi else {
        return Err(SongOpenError::Missing {
            path: paths.midi.clone(),
        });
    };
    let journal_sidecar = files.journal_sidecar.as_deref();
    let sidecar = journal_sidecar
        .map(|bytes| parse_sidecar(&paths.journal_sidecar, bytes))
        .transpose()?;
    let mut document = document_from(
        &paths.journal_midi,
        journal_midi,
        Some(sidecar.unwrap_or_else(|| scaffold_song_file(asset_base))),
        journal_sidecar,
    )?;
    let recovery_failed = |path: &Path, error: io::Error| SongOpenError::Recovery {
        path: path.to_path_buf(),
        reason: error.to_string(),
    };
    store
        .write(&paths.midi, journal_midi)
        .map_err(|error| recovery_failed(&paths.midi, error))?;
    if let Some(bytes) = journal_sidecar {
        store
            .write(&paths.sidecar, bytes)
            .map_err(|error| recovery_failed(&paths.sidecar, error))?;
    }
    for journal in [&paths.journal_midi, &paths.journal_sidecar] {
        store
            .remove(journal)
            .map_err(|error| recovery_failed(journal, error))?;
    }
    document.path = Some(paths.midi.clone());
    Ok(OpenedSong {
        document,
        notes: vec![format!(
            "finished the first save interrupted at {}",
            paths.midi.display()
        )],
    })
}

/// Keep each journal file beside the finals as a recovery, noting each; one that cannot be
/// kept is left where it is and noted, and the open goes on.
fn keep_journals(store: &mut impl SongStore, paths: &SongPaths, files: &SongFiles) -> Vec<String> {
    let Some(named_by) = files.journal_midi.as_deref().or(files.journal_sidecar.as_deref()) else {
        return Vec::new();
    };
    let Some(recovered_midi) = recover::recovery_path(store, &paths.midi, midi_fingerprint(named_by))
    else {
        return vec![format!(
            "{} holds what an interrupted save left, and no free name was left to keep it under",
            paths.midi.display()
        )];
    };
    let journals = [
        (&paths.journal_midi, &files.journal_midi, recovered_midi.clone()),
        (&paths.journal_sidecar, &files.journal_sidecar, midi_sidecar_path(&recovered_midi)),
    ];
    let mut notes = Vec::new();
    for (journal, bytes, kept_at) in journals {
        let Some(bytes) = bytes else {
            continue;
        };
        let map_alone = journal == &paths.journal_sidecar && files.journal_midi.is_none();
        notes.push(match recover::keep(store, bytes, &kept_at, journal) {
            Ok(()) if map_alone => format!(
                "{} was left by an interrupted map save: kept as {} — a map alone, this song's \
                 score left as it is; copy it over the song's sidecar to take that map",
                journal.display(),
                kept_at.display()
            ),
            Ok(()) => format!(
                "{} was left by an interrupted save: kept as {} — open it to see that save",
                journal.display(),
                kept_at.display()
            ),
            Err(error) => format!(
                "{} was left by an interrupted save and could not be kept: {error}",
                journal.display()
            ),
        });
    }
    notes
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
