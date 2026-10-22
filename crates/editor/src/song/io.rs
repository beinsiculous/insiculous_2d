//! Saving a song as a recoverable journal; opening one is [`open`].
//!
//! Nothing on either target renames a file, so a save cannot swap its two files in at
//! once. It writes a journal instead, in this order, and refuses before the first write:
//!
//! 1. **Refuse, writing nothing**: a path outside the song folders or against
//!    [`saveable_song_path`], a song or a `Sample` instrument without provenance, a sidecar
//!    `check_schema` refuses, a sample that cannot be read, a sample folder another song
//!    owns.
//! 2. **The new samples**, into the song's own folder, [`sample_directory`]. A sample
//!    whose name is taken there by other bytes gets a content-hash suffix, and its
//!    instrument's path follows. Nothing is removed yet.
//! 3. **The pair**: `<stem>.mid.ron.new`, carrying the fingerprint of the MIDI bytes about
//!    to be written, then `<stem>.mid.new`. The pair is the commit point.
//! 4. **The prune**: every file of that folder the song no longer names is removed, and its
//!    `LICENSE` rewritten with exactly the referenced samples' stanzas under a `Source:`
//!    header naming the song; then the home folder's `LICENSE` gets the song's own stanza,
//!    written only when it changed.
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
use std::path::{Component, Path, PathBuf};

use music_midi::{
    write_song_file, InstrumentSource, Provenance, SongData, SongError, SongFile, SongFileError,
    SongFileProblem,
};

use super::SongDocument;

/// The folders a song saves into, under the asset folder: the human's, and the agents'
/// (whose songs carry the `ai_` prefix until a human pass promotes them).
pub const HOME_DIRECTORIES: [&str; 2] = ["music", "ai/music"];

/// The folder under each home that holds its songs' samples, one folder a song.
const SAMPLES_FOLDER: &str = "samples";

/// The separator a song's path under its home is flattened by into its sample folder's
/// name; [`saveable_song_path`] keeps it unambiguous.
const FLAT_SEPARATOR: &str = "__";

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
    path.strip_prefix(asset_base)
        .is_ok_and(|relative| home_of(relative).is_some())
}

/// The song folder `relative` (a path under the asset base) lies in.
fn home_of(relative: &Path) -> Option<&'static str> {
    HOME_DIRECTORIES
        .into_iter()
        .find(|home| relative.starts_with(home))
}

/// The path's components as text, the last with its extension dropped.
fn named_components(path: &Path) -> Vec<String> {
    let mut names: Vec<String> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    if let (Some(last), Some(stem)) = (names.last_mut(), path.file_stem()) {
        *last = stem.to_string_lossy().into_owned();
    }
    names
}

/// Whether a song at `relative` (a `.mid` under the asset base) may be saved by name: in a
/// song folder, not under that folder's `samples/` (a song there would sit inside another
/// song's sample folder, which a save prunes), and with no component starting or ending
/// with `_`, no `__` and no whitespace — so the `__` that flattens a song's path into its
/// sample folder's name is never ambiguous, and the provenance gate, which splits a
/// `Files:` field on whitespace, can name its files. The refusal says what to change.
pub fn saveable_song_path(relative: &Path) -> Result<(), String> {
    let shown = relative.display();
    let Some(home) = home_of(relative) else {
        return Err(format!(
            "{shown} is outside the song folders; save the song under music/ or ai/music/"
        ));
    };
    if relative.starts_with(Path::new(home).join(SAMPLES_FOLDER)) {
        return Err(format!(
            "{shown} is inside {home}/{SAMPLES_FOLDER}/, which holds the songs' samples; Save As to a path outside it"
        ));
    }
    for name in named_components(relative) {
        if name.chars().any(char::is_whitespace) {
            return Err(format!(
                "{shown} has whitespace in '{name}'; Save As to a name without it (write _ instead)"
            ));
        }
        if name.starts_with('_') || name.ends_with('_') || name.contains(FLAT_SEPARATOR) {
            return Err(format!(
                "{shown}: '{name}' starts or ends with _ or holds __, which name a song's sample folder; Save As to a name without them"
            ));
        }
    }
    Ok(())
}

/// Whether Save takes `relative` on the older, looser rule: a song directly in its home
/// keys its sample folder by its stem alone, so no flattening is involved — short of
/// whitespace, which the provenance gate cannot name.
fn is_grandfathered(relative: &Path) -> bool {
    let flat = home_of(relative).is_some_and(|home| relative.parent() == Some(Path::new(home)));
    flat && !relative.to_string_lossy().chars().any(char::is_whitespace)
}

/// Where the song at `relative` (a `.mid` under the asset base) keeps its samples, under
/// the asset base: `<home>/samples/<flat>`, `<flat>` its path under its home, extension
/// dropped, each `/` made `__` — `music/theme.mid` → `music/samples/theme`,
/// `music/a/theme.mid` → `music/samples/a__theme`. `None` outside the song folders.
pub fn sample_directory(relative: &Path) -> Option<String> {
    let home = home_of(relative)?;
    let under_home = relative.strip_prefix(home).ok()?;
    let flat = named_components(under_home).join(FLAT_SEPARATOR);
    Some(format!("{home}/{SAMPLES_FOLDER}/{flat}"))
}

/// `path` as the sidecar and the `LICENSE` files write it: `/`-separated, whatever the target.
pub(super) fn slash_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Everything a save writes, prepared before the first write.
struct SavePlan {
    sample_directory: PathBuf,
    /// The song's `.mid` under the asset base, as its sample folder's `Source:` names it.
    song: String,
    samples: Vec<samples::PlacedSample>,
    /// The home folder's `LICENSE`, when the song's stanza changed it.
    home_license: Option<(PathBuf, String)>,
    rendered: RenderedFiles,
}

/// A song's two files as bytes, and the sidecar they were rendered from.
struct RenderedFiles {
    midi_bytes: Vec<u8>,
    sidecar_text: String,
    file: SongFile,
}

/// Render `file` and `data` as a save writes them: the fingerprint of the MIDI bytes set in
/// the sidecar. `label` names the sidecar in an error.
fn render(data: &SongData, mut file: SongFile, label: &str) -> Result<RenderedFiles, SongSaveError> {
    let midi_bytes = data.write_smf()?;
    file.midi_fingerprint = Some(midi_fingerprint(&midi_bytes));
    let sidecar_text = write_song_file(label, &file)?;
    Ok(RenderedFiles {
        midi_bytes,
        sidecar_text,
        file,
    })
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
        self.file = plan.rendered.file;
        self.path = Some(target.to_path_buf());
        self.imported_from = None;
        self.history.mark_saved();
        Ok(())
    }

    /// The song's two files as the document holds them — the MIDI bytes and the sidecar
    /// with its fingerprint set — with nothing refused and no sample moved: what a backup
    /// of an unsaved song carries, rendered as a save renders them.
    pub fn rendered_files(&self) -> Result<(Vec<u8>, String), SongSaveError> {
        let label = self.path.as_deref().map_or_else(
            || "the song in memory".to_string(),
            |path| with_suffix(path, ".ron").display().to_string(),
        );
        let rendered = render(&self.data, self.file.clone(), &label)?;
        Ok((rendered.midi_bytes, rendered.sidecar_text))
    }

    fn plan_save(
        &self,
        store: &impl SongStore,
        target: &Path,
        asset_base: &Path,
    ) -> Result<SavePlan, SongSaveError> {
        if target.extension().is_none_or(|extension| extension != "mid") {
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

        let label = with_suffix(target, ".ron").display().to_string();
        let rendered = render(&self.data, file, &label)?;
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
            rendered,
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
    let rendered = &plan.rendered;
    write_file(store, &journal_sidecar, rendered.sidecar_text.as_bytes())?;
    write_file(store, &journal_midi, &rendered.midi_bytes)?;
    prune(store, plan)?;
    if let Some((path, text)) = &plan.home_license {
        write_file(store, path, text.as_bytes())?;
    }
    write_file(store, target, &rendered.midi_bytes)?;
    write_file(store, &sidecar, rendered.sidecar_text.as_bytes())?;
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
        let license = samples::license_text(&plan.samples, &plan.song);
        write_file(
            store,
            &plan.sample_directory.join("LICENSE"),
            license.as_bytes(),
        )?;
    }
    Ok(())
}
