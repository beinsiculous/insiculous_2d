//! Saving a song as a recoverable journal; opening one is [`open`].
//!
//! Nothing on either target renames a file, so a save cannot swap its two files in at
//! once. It writes a journal instead, in this order, and refuses before the first write:
//!
//! 1. **Refuse, writing nothing**: a `Sample` instrument without provenance, a sidecar
//!    `check_schema` refuses, a sample that cannot be read, a path outside the song folders.
//! 2. **The new samples**, into `music/samples/<stem>/` — the song's own folder. A sample
//!    whose name is taken there by other bytes gets a content-hash suffix, and its
//!    instrument's path follows. Nothing is removed yet.
//! 3. **The pair**: `<stem>.mid.ron.new`, carrying the fingerprint of the MIDI bytes about
//!    to be written, then `<stem>.mid.new`. The pair is the commit point.
//! 4. **The prune**: every file of that folder the song no longer names is removed, and its
//!    `LICENSE` rewritten with exactly the referenced samples' stanzas.
//! 5. **The finals**: `<stem>.mid`, then `<stem>.mid.ron`.
//! 6. **The pair removed**, then the watermark marked saved.
//!
//! The prune waits for the pair: an interruption before the pair leaves the old song with
//! every sample it names, and one after it is recovered as the new song, which names
//! nothing the prune removed; pruning before the pair would delete a removed instrument's
//! sample while the old finals still name it. A prune cut short leaves files the new song
//! does not name, still listed by the old `LICENSE` and so unseen by the provenance gate,
//! until the next save's prune removes them; a new sample the old `LICENSE` never named is
//! what the gate reports meanwhile. Open reads the state any interruption leaves and
//! settles it (see [`open`]).

pub(super) mod open;
mod samples;

pub use open::{OpenedSong, SongOpenError};

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use music_midi::{
    write_song_file, InstrumentSource, SongError, SongFile, SongFileError, SongFileProblem,
};

use super::SongDocument;

/// The folders a song saves into, under the asset folder: the human's, and the agents'
/// (whose songs carry the `ai_` prefix until a human pass promotes them).
pub const HOME_DIRECTORIES: [&str; 2] = ["music", "ai/music"];

/// Where a song's samples live, under the asset folder, each song in `<stem>/`.
pub const SAMPLE_DIRECTORY: &str = "music/samples";

/// Why a save was refused or stopped.
#[derive(Debug, thiserror::Error)]
pub enum SongSaveError {
    #[error("{} is not a .mid file", path.display())]
    NotMidi { path: PathBuf },
    #[error(
        "{} is outside the song folders; save the song under music/ or ai/music/ in the asset folder",
        path.display()
    )]
    OutsideHome { path: PathBuf },
    #[error("instruments['{instrument}'] has no {field}: a sample needs its author and license before the song is saved")]
    MissingProvenance {
        instrument: String,
        field: &'static str,
    },
    #[error("the song file would not load: {0}")]
    Schema(#[from] SongFileProblem),
    #[error("instruments['{instrument}'] plays {path}, which cannot be read: {reason}")]
    SampleUnreadable {
        instrument: String,
        path: String,
        reason: String,
    },
    #[error("{0}")]
    Midi(#[from] SongError),
    #[error("{0}")]
    Sidecar(#[from] SongFileError),
    #[error("{} holds other bytes under both names a sample could take", directory.display())]
    SampleNameTaken {
        directory: PathBuf,
        file_name: String,
    },
    #[error("writing {} failed: {reason}", path.display())]
    Write { path: PathBuf, reason: String },
}

/// The file operations a save and an open make: `common::vfs` on both targets
/// ([`VfsStore`]), or a store a test interrupts.
pub(crate) trait SongStore {
    /// The file's bytes; `NotFound` when there is none.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn write(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    /// Remove exactly this file or folder; a missing one is `Ok`.
    fn remove(&mut self, path: &Path) -> io::Result<()>;
    /// Every file under `directory`; empty when it does not exist.
    fn list_files(&self, directory: &Path) -> io::Result<Vec<PathBuf>>;
}

/// The store every real save and open uses.
pub(crate) struct VfsStore;

impl SongStore for VfsStore {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        common::vfs::read(path)
    }

    fn write(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        common::vfs::write(path, bytes)
    }

    fn remove(&mut self, path: &Path) -> io::Result<()> {
        common::vfs::remove_prefix(path)
    }

    fn list_files(&self, directory: &Path) -> io::Result<Vec<PathBuf>> {
        match common::vfs::list_files(directory) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            listed => listed,
        }
    }
}

/// FNV-1a (64-bit) over `bytes`: the `midi_fingerprint` a saved sidecar carries. A file
/// format value, so it never changes with the platform or the release.
pub fn midi_fingerprint(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    bytes.iter().fold(OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

/// `path` with `suffix` appended to its last component (`theme.mid` → `theme.mid.new`).
pub(super) fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut joined = OsString::from(path.as_os_str());
    joined.push(suffix);
    PathBuf::from(joined)
}

/// Whether `path` lies in a song folder under `asset_base`. A path not under
/// `asset_base` at all — a relative one included — is outside: the samples are placed
/// under `asset_base`, so a target resolved anywhere else would split a song in two.
pub(super) fn is_in_home_directory(path: &Path, asset_base: &Path) -> bool {
    path.strip_prefix(asset_base).is_ok_and(|relative| {
        HOME_DIRECTORIES
            .iter()
            .any(|home| relative.starts_with(home))
    })
}

/// Everything a save writes, prepared before the first write.
struct SavePlan {
    sample_directory: PathBuf,
    samples: Vec<samples::PlacedSample>,
    midi_bytes: Vec<u8>,
    sidecar_text: String,
    file: SongFile,
}

impl SongDocument {
    /// Save the song to `target` (a `.mid` in a song folder under `asset_base`) through
    /// `common::vfs`, as the module docs lay out. A refused save writes nothing; a save
    /// stopped by a failed write leaves files [`SongDocument::open`] settles, and the song
    /// stays dirty.
    pub fn save(&mut self, target: &Path, asset_base: &Path) -> Result<(), SongSaveError> {
        self.save_to(&mut VfsStore, target, asset_base)
    }

    pub(crate) fn save_to(
        &mut self,
        store: &mut impl SongStore,
        target: &Path,
        asset_base: &Path,
    ) -> Result<(), SongSaveError> {
        let plan = self.plan_save(store, target, asset_base)?;
        commit(store, target, &plan)?;
        self.file = plan.file;
        self.pending_samples.clear();
        self.path = Some(target.to_path_buf());
        self.imported_from = None;
        self.history.mark_saved();
        Ok(())
    }

    fn plan_save(
        &self,
        store: &impl SongStore,
        target: &Path,
        asset_base: &Path,
    ) -> Result<SavePlan, SongSaveError> {
        let stem = match (target.extension(), target.file_stem()) {
            (Some(extension), Some(stem)) if extension == "mid" => {
                stem.to_string_lossy().into_owned()
            }
            _ => {
                return Err(SongSaveError::NotMidi {
                    path: target.to_path_buf(),
                })
            }
        };
        if !is_in_home_directory(target, asset_base) {
            return Err(SongSaveError::OutsideHome {
                path: target.to_path_buf(),
            });
        }
        for instrument in &self.file.instruments {
            if !matches!(instrument.source, InstrumentSource::Sample { .. }) {
                continue;
            }
            let missing = |field| SongSaveError::MissingProvenance {
                instrument: instrument.name.clone(),
                field,
            };
            let provenance = instrument
                .provenance
                .as_ref()
                .ok_or_else(|| missing("provenance"))?;
            if provenance.author.trim().is_empty() {
                return Err(missing("provenance.author"));
            }
            if provenance.license.trim().is_empty() {
                return Err(missing("provenance.license"));
            }
        }
        self.file.check_schema()?;

        let sample_directory = asset_base.join(SAMPLE_DIRECTORY).join(&stem);
        let mut file = self.file.clone();
        let mut samples: Vec<samples::PlacedSample> = Vec::new();
        for instrument in &mut file.instruments {
            let (InstrumentSource::Sample { path, .. }, Some(provenance)) =
                (&mut instrument.source, &instrument.provenance)
            else {
                continue;
            };
            let bytes = match self.pending_samples.get(path.as_str()) {
                Some(bytes) => bytes.clone(),
                None => store
                    .read(&asset_base.join(path.as_str()))
                    .map_err(|error| SongSaveError::SampleUnreadable {
                        instrument: instrument.name.clone(),
                        path: path.clone(),
                        reason: error.to_string(),
                    })?,
            };
            let file_name = samples::place_sample(
                store,
                &sample_directory,
                &mut samples,
                path,
                bytes,
                provenance,
            )?;
            *path = format!("{SAMPLE_DIRECTORY}/{stem}/{file_name}");
        }

        let midi_bytes = self.data.write_smf()?;
        file.midi_fingerprint = Some(midi_fingerprint(&midi_bytes));
        let label = with_suffix(target, ".ron").display().to_string();
        let sidecar_text = write_song_file(&label, &file)?;
        Ok(SavePlan {
            sample_directory,
            samples,
            midi_bytes,
            sidecar_text,
            file,
        })
    }
}

/// A write that names its file when it fails.
fn write_file(store: &mut impl SongStore, path: &Path, bytes: &[u8]) -> Result<(), SongSaveError> {
    store
        .write(path, bytes)
        .map_err(|error| SongSaveError::Write {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })
}

/// A removal that names its file when it fails.
fn remove_file(store: &mut impl SongStore, path: &Path) -> Result<(), SongSaveError> {
    store.remove(path).map_err(|error| SongSaveError::Write {
        path: path.to_path_buf(),
        reason: format!("removing it failed: {error}"),
    })
}

/// Steps 2 to 6 of the module docs; the first failed write stops the save.
fn commit(store: &mut impl SongStore, target: &Path, plan: &SavePlan) -> Result<(), SongSaveError> {
    for sample in plan.samples.iter().filter(|sample| !sample.already_there) {
        write_file(
            store,
            &plan.sample_directory.join(&sample.file_name),
            &sample.bytes,
        )?;
    }
    let sidecar = with_suffix(target, ".ron");
    let journal_sidecar = with_suffix(&sidecar, ".new");
    let journal_midi = with_suffix(target, ".new");
    write_file(store, &journal_sidecar, plan.sidecar_text.as_bytes())?;
    write_file(store, &journal_midi, &plan.midi_bytes)?;
    prune(store, plan)?;
    write_file(store, target, &plan.midi_bytes)?;
    write_file(store, &sidecar, plan.sidecar_text.as_bytes())?;
    remove_file(store, &journal_midi)?;
    remove_file(store, &journal_sidecar)
}

/// Step 4: remove what the song no longer names from its folder and rewrite its `LICENSE`
/// (the folder itself goes with its last sample).
fn prune(store: &mut impl SongStore, plan: &SavePlan) -> Result<(), SongSaveError> {
    let referenced: BTreeSet<&str> = plan
        .samples
        .iter()
        .map(|sample| sample.file_name.as_str())
        .collect();
    let listed =
        store
            .list_files(&plan.sample_directory)
            .map_err(|error| SongSaveError::Write {
                path: plan.sample_directory.clone(),
                reason: format!("listing it failed: {error}"),
            })?;
    for path in listed {
        let relative = path.strip_prefix(&plan.sample_directory).unwrap_or(&path);
        let relative = relative.to_string_lossy();
        if relative != "LICENSE" && !referenced.contains(relative.as_ref()) {
            remove_file(store, &path)?;
        }
    }
    if plan.samples.is_empty() {
        remove_file(store, &plan.sample_directory)?;
    } else {
        let license = samples::license_text(&plan.samples);
        write_file(
            store,
            &plan.sample_directory.join("LICENSE"),
            license.as_bytes(),
        )?;
    }
    Ok(())
}
