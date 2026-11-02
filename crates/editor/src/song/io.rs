//! Saving a song as a recoverable journal; opening one is [`open`].
//!
//! A save is one of two kinds. **A map save** — to the document's own path — writes the
//! sidecar and the home `LICENSE` stanza and leaves the `.mid` alone: the score belongs to
//! the program that wrote it, and the sidecar fingerprints the bytes the document loaded,
//! never a re-encoding of them. **A Save As** — to any other target — copies the whole
//! song: the loaded bytes, or `write_smf`'s once the engine edited the score, and the map.
//!
//! Immediately before its first write a map save reads both files through the store. A
//! sidecar that changed since the document read it refuses the save with
//! [`SongSaveError::SidecarChanged`] unless the [`SaveIntent`] overwrites it; the compare
//! is by content, so a rewrite that kept the file's length and stamp is still caught. A
//! score that changed is adopted first ([`SongDocument::adopt_score`]) and the sidecar
//! fingerprints the disk's bytes — unless the engine edited the score, whose save takes the
//! rule at step 5 instead, so no edit is lost to an adoption. A `.mid` gone from disk is
//! written again from the document.
//!
//! Nothing on either target renames a file, so a save cannot swap its files in at once.
//! It writes a journal instead, in this order, and refuses before the first write
//! (`commit.rs` holds steps 2 to 7):
//!
//! 1. **Refuse, writing nothing**: a path outside the song folders or against
//!    [`saveable_song_path`], a song or a `Sample` instrument without provenance, a sidecar
//!    `check_schema` refuses, a sample that cannot be read, a sample folder another song
//!    owns, a sidecar changed on disk.
//! 2. **The new samples**, into the song's own folder, [`sample_directory`]. A sample
//!    whose name is taken there by other bytes gets a content-hash suffix, and its
//!    instrument's path follows. Nothing is removed yet.
//! 3. **The journal**: `<name>.ron.new`, carrying the fingerprint of the score the song
//!    names, then `<name>.new` when the save writes the `.mid`. A map save's journal is
//!    the sidecar's alone.
//! 4. **The home folder's `LICENSE`**, the song's own stanza, written only when it changed.
//! 5. **The finals**: the `.mid` when the save writes it, then the sidecar. Before the
//!    `.mid` of a score the engine edited, the `.mid` on disk is compared with the one the
//!    document loaded or last wrote: another program's rewrite is left where it is, the
//!    engine's score kept beside it as `<stem>.recovered-<fingerprint>.<ext>` with a note,
//!    and the sidecar saved against the disk's bytes, which the document then adopts. The
//!    window between that compare and the write is accepted until the engine no longer
//!    writes scores at all.
//! 6. **The journal removed.**
//! 7. **The prune**: every file of the song's sample folder that neither the song nor a
//!    `<stem>.recovered-*` sidecar beside it names is removed, and the folder's `LICENSE`
//!    rewritten with exactly those samples' stanzas under a `Source:` header naming the
//!    song; then the watermark is marked saved.
//!
//! The prune runs last, so an interruption at any step leaves every sample each file
//! beside the song names on disk: an open keeps a journal as a recovery and never promotes
//! it over the finals ([`open`]), and a kept recovery's samples are the prune's roots. A
//! prune cut short leaves files nothing names, still listed by the old `LICENSE` and so
//! unseen by the provenance gate, until the next save's prune removes them; a new sample
//! the old `LICENSE` never named is what the gate reports meanwhile.

mod commit;
mod init;
pub(super) mod open;
mod paths;
mod recover;
mod samples;

pub use init::{init_song, SongInitError};
pub use open::{OpenedSong, SongOpenError};
pub use paths::{sample_directory, saveable_song_path, HOME_DIRECTORIES};
use paths::{home_of, is_grandfathered, is_in_home_directory, slash_path};

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use music_midi::{
    write_song_file, InstrumentSource, Provenance, SongError, SongFile, SongFileError,
    SongFileProblem,
};

use super::SongDocument;

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
    #[error("{0}")]
    PathRule(String),
    #[error("instruments['{instrument}'] has no {field}: a sample needs its author and license before the song is saved")]
    MissingProvenance {
        instrument: String,
        field: &'static str,
    },
    #[error("the song has no {field}: a song needs its author and license before it is saved (the Song section's Author and License)")]
    MissingSongProvenance { field: &'static str },
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
    #[error(
        "{folder} holds the samples of {owner}, not of {song}; Save As to another path",
        folder = folder.display()
    )]
    SampleFolderTaken {
        folder: PathBuf,
        owner: String,
        song: String,
    },
    #[error(
        "{folder} already holds samples its LICENSE names no song for, and {song} may only adopt a folder named after itself; Save As to another path",
        folder = folder.display()
    )]
    SampleFolderUnowned { folder: PathBuf, song: String },
    #[error("writing {} failed: {reason}", path.display())]
    Write { path: PathBuf, reason: String },
    /// The sidecar on disk is not the one the document read or last wrote: another
    /// program, or a hand, changed the map. Each fingerprint is `None` for a file absent.
    #[error(
        "the map changed on disk since you loaded it: {} — save with Overwrite to replace it with this song's map",
        sidecar.display()
    )]
    SidecarChanged {
        sidecar: PathBuf,
        loaded: Option<u64>,
        on_disk: Option<u64>,
        midi: PathBuf,
        score_loaded: u64,
        score_on_disk: Option<u64>,
    },
    #[error("{} changed on disk and cannot be read: {reason}", path.display())]
    ChangedScoreUnreadable { path: PathBuf, reason: String },
}

/// Whether a save may replace a sidecar that changed on disk since the document read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidecarIntent {
    /// Refuse with [`SongSaveError::SidecarChanged`]: what a save does unless told.
    #[default]
    Keep,
    /// Write over it: the answer a person gave the conflict's prompt or the API's line.
    Overwrite,
}

/// What a save was asked to do past the checks it makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SaveIntent {
    pub sidecar: SidecarIntent,
}

impl SaveIntent {
    /// Replace a sidecar changed on disk.
    pub const OVERWRITE: SaveIntent = SaveIntent {
        sidecar: SidecarIntent::Overwrite,
    };
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

/// Everything a save writes, prepared before the first write.
struct SavePlan {
    sample_directory: PathBuf,
    /// The song's `.mid` under the asset base, as its sample folder's `Source:` names it.
    song: String,
    samples: Vec<samples::PlacedSample>,
    /// The home folder's `LICENSE`, when the song's stanza changed it: its path, the text to
    /// write, and the text it held — `None` for no file — which a refusal after the write puts back.
    home_license: Option<(PathBuf, String, Option<String>)>,
    rendered: RenderedFiles,
    /// For a score the engine edited, saved in place: the fingerprint the `.mid` on disk
    /// must still have when the finals are written.
    score_guard: Option<u64>,
    /// For a save in place that keeps the sidecar: what the sidecar on disk must still be
    /// when the finals are written — its fingerprint, or `None` for no file.
    sidecar_guard: Option<Option<u64>>,
    /// The fingerprint of the score the document follows, for a refusal's message.
    score_loaded: u64,
}

/// A song's files as bytes, and the sidecar they were rendered from.
struct RenderedFiles {
    /// `None` for a map save, which leaves the `.mid` alone.
    midi_bytes: Option<Vec<u8>>,
    sidecar_text: String,
    file: SongFile,
}

/// Render `file` with `midi_fingerprint` set to `fingerprint`, the score it describes.
/// `label` names the sidecar in an error.
fn render_sidecar(
    mut file: SongFile,
    fingerprint: u64,
    label: &str,
) -> Result<(SongFile, String), SongSaveError> {
    file.midi_fingerprint = Some(fingerprint);
    let text = write_song_file(label, &file)?;
    Ok((file, text))
}

/// The refusal for a provenance missing a field, or `Ok`.
fn check_provenance<E>(
    provenance: Option<&Provenance>,
    missing: impl Fn(&'static str) -> E,
) -> Result<(), E> {
    let provenance = provenance.ok_or_else(|| missing("provenance"))?;
    if provenance.author.trim().is_empty() {
        return Err(missing("provenance.author"));
    }
    if provenance.license.trim().is_empty() {
        return Err(missing("provenance.license"));
    }
    Ok(())
}

/// The file at `path` through the store; `None` when there is none.
fn read_optional(store: &impl SongStore, path: &Path) -> Result<Option<Vec<u8>>, SongSaveError> {
    match store.read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(SongSaveError::Write {
            path: path.to_path_buf(),
            reason: format!("reading it failed: {error}"),
        }),
    }
}

impl SongDocument {
    /// Save the song to `target` (a `.mid` in a song folder under `asset_base`) through
    /// `common::vfs`, as the module docs lay out, refusing over a sidecar changed on disk.
    /// Returns what the save settled — an adopted score's track diff, a recovery — for the
    /// status bar. A refused save writes nothing; a save stopped by a failed write leaves
    /// files [`SongDocument::open`] settles, and the song stays dirty.
    pub fn save(&mut self, target: &Path, asset_base: &Path) -> Result<Vec<String>, SongSaveError> {
        self.save_with(target, asset_base, SaveIntent::default())
    }

    /// [`SongDocument::save`] past the checks `intent` overrides.
    pub fn save_with(
        &mut self,
        target: &Path,
        asset_base: &Path,
        intent: SaveIntent,
    ) -> Result<Vec<String>, SongSaveError> {
        self.save_to(&mut VfsStore, target, asset_base, intent)
    }

    pub(crate) fn save_to(
        &mut self,
        store: &mut impl SongStore,
        target: &Path,
        asset_base: &Path,
        intent: SaveIntent,
    ) -> Result<Vec<String>, SongSaveError> {
        let mut notes = Vec::new();
        let in_place = self
            .path
            .as_deref()
            .is_some_and(|own| own.components().eq(target.components()));
        let midi_on_disk = if in_place {
            self.follow_disk(store, target, intent, &mut notes)?
        } else {
            false
        };
        let score_authored = self.is_score_authored();
        let writes_midi = !in_place || score_authored || !midi_on_disk;
        let score_guard = (in_place && score_authored && midi_on_disk).then_some(self.score_fingerprint);
        // Compared again right before the finals: the samples' reads and the licenses lie
        // between the look above and the write, and a map rewritten in that span would be
        // overwritten unseen.
        let sidecar_guard = (in_place && intent.sidecar == SidecarIntent::Keep).then_some(self.map_fingerprint);
        let plan = self.plan_save(store, target, asset_base, writes_midi, score_guard, sidecar_guard)?;
        let committed = commit::commit(store, target, asset_base, &plan)?;
        self.path = Some(target.to_path_buf());
        self.imported_from = None;
        self.map_fingerprint = Some(midi_fingerprint(committed.sidecar_text.as_bytes()));
        self.file = committed.file;
        match committed.recovered {
            Some(recovered) => {
                notes.push(format!(
                    "{} changed on disk before the save could write it: it is left as it is, \
                     and this song's score is kept as {}",
                    target.display(),
                    recovered.path.display()
                ));
                match self.adopt_score(recovered.disk_score) {
                    Ok(lines) => notes.extend(lines),
                    Err(error) => notes.push(format!(
                        "{} cannot be read, so the song keeps its own score: {error}",
                        target.display()
                    )),
                }
            }
            None => {
                if let Some(bytes) = plan.rendered.midi_bytes {
                    // Written from `write_smf` when the score was the engine's: those bytes are
                    // the export, and the baseline needs no second one.
                    let export = if score_authored { Some(bytes.clone()) } else { None };
                    self.take_loaded_score(bytes, export);
                }
            }
        }
        self.history.mark_saved();
        Ok(notes)
    }

    /// A map save's look at disk before its first write: the sidecar refused when it
    /// changed and `intent` keeps it, then a changed score adopted unless the engine edited
    /// its own. Returns whether the `.mid` is on disk at all.
    fn follow_disk(
        &mut self,
        store: &impl SongStore,
        target: &Path,
        intent: SaveIntent,
        notes: &mut Vec<String>,
    ) -> Result<bool, SongSaveError> {
        let sidecar = crate::external_tools::midi_sidecar_path(target);
        let sidecar_on_disk = read_optional(store, &sidecar)?.map(|bytes| midi_fingerprint(&bytes));
        let disk_score = read_optional(store, target)?;
        if sidecar_on_disk != self.map_fingerprint && intent.sidecar == SidecarIntent::Keep {
            return Err(SongSaveError::SidecarChanged {
                sidecar,
                loaded: self.map_fingerprint,
                on_disk: sidecar_on_disk,
                midi: target.to_path_buf(),
                score_loaded: self.score_fingerprint,
                score_on_disk: disk_score.as_deref().map(midi_fingerprint),
            });
        }
        let Some(disk_score) = disk_score else {
            return Ok(false);
        };
        if !self.is_score_authored() && midi_fingerprint(&disk_score) != self.score_fingerprint {
            let lines = self.adopt_score(disk_score).map_err(|error| {
                SongSaveError::ChangedScoreUnreadable {
                    path: target.to_path_buf(),
                    reason: error.to_string(),
                }
            })?;
            notes.push(format!(
                "{} changed on disk: the song follows it, its map kept",
                target.display()
            ));
            notes.extend(lines);
        }
        Ok(true)
    }

    /// The song's two files as the document holds them — the score it would save and the
    /// sidecar with its fingerprint set — with nothing refused and no sample moved: what a
    /// backup of an unsaved song carries, rendered as a save renders them.
    pub fn rendered_files(&self) -> Result<(Vec<u8>, String), SongSaveError> {
        let label = self.path.as_deref().map_or_else(
            || "the song in memory".to_string(),
            |path| crate::external_tools::midi_sidecar_path(path).display().to_string(),
        );
        let midi_bytes = self.score_bytes()?;
        let (_, sidecar_text) =
            render_sidecar(self.file.clone(), midi_fingerprint(&midi_bytes), &label)?;
        Ok((midi_bytes, sidecar_text))
    }

    fn plan_save(
        &self,
        store: &impl SongStore,
        target: &Path,
        asset_base: &Path,
        writes_midi: bool,
        score_guard: Option<u64>,
        sidecar_guard: Option<Option<u64>>,
    ) -> Result<SavePlan, SongSaveError> {
        if !crate::external_tools::is_midi_path(target) {
            return Err(SongSaveError::NotMidi {
                path: target.to_path_buf(),
            });
        }
        let Some((relative, home)) = target
            .strip_prefix(asset_base)
            .ok()
            .and_then(|relative| Some((relative, home_of(relative)?)))
        else {
            return Err(SongSaveError::OutsideHome {
                path: target.to_path_buf(),
            });
        };
        if !is_grandfathered(relative) {
            saveable_song_path(relative).map_err(SongSaveError::PathRule)?;
        }
        check_provenance(self.file.provenance.as_ref(), |field| {
            SongSaveError::MissingSongProvenance { field }
        })?;
        for instrument in &self.file.instruments {
            if matches!(instrument.source, InstrumentSource::Sample { .. }) {
                check_provenance(instrument.provenance.as_ref(), |field| {
                    SongSaveError::MissingProvenance {
                        instrument: instrument.name.clone(),
                        field,
                    }
                })?;
            }
        }
        self.file.check_schema()?;

        let song = slash_path(relative);
        let folder = sample_directory(relative).ok_or_else(|| SongSaveError::OutsideHome {
            path: target.to_path_buf(),
        })?;
        let sample_directory = asset_base.join(&folder);
        samples::check_folder_owner(store, &sample_directory, &song, relative)?;
        let mut file = self.file.clone();
        let mut samples: Vec<samples::PlacedSample> = Vec::new();
        for instrument in &mut file.instruments {
            let (InstrumentSource::Sample { path, .. }, Some(provenance)) =
                (&mut instrument.source, &instrument.provenance)
            else {
                continue;
            };
            let bytes = store.read(&asset_base.join(path.as_str())).map_err(|error| {
                SongSaveError::SampleUnreadable {
                    instrument: instrument.name.clone(),
                    path: path.clone(),
                    reason: error.to_string(),
                }
            })?;
            let file_name = samples::place_sample(
                store,
                &sample_directory,
                &mut samples,
                path,
                bytes,
                provenance,
            )?;
            *path = format!("{folder}/{file_name}");
        }

        let label = crate::external_tools::midi_sidecar_path(target)
            .display()
            .to_string();
        let midi_bytes = writes_midi.then(|| self.score_bytes()).transpose()?;
        let fingerprint = midi_bytes
            .as_deref()
            .map_or(self.score_fingerprint, midi_fingerprint);
        let (file, sidecar_text) = render_sidecar(file, fingerprint, &label)?;
        let home_license = match self.file.provenance.as_ref() {
            Some(provenance) => {
                samples::home_license(store, &asset_base.join(home), home, relative, provenance)?
            }
            None => None,
        };
        Ok(SavePlan {
            sample_directory,
            song,
            samples,
            home_license,
            rendered: RenderedFiles {
                midi_bytes,
                sidecar_text,
                file,
            },
            score_guard,
            sidecar_guard,
            score_loaded: self.score_fingerprint,
        })
    }
}
