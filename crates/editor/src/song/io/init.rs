//! `song init`: the scaffold sidecar written beside an unmapped `.mid` in a song folder, so a
//! score another program wrote becomes a song the engine plays.

use std::io;
use std::path::{Path, PathBuf};

use music_midi::{write_song_file, SongData};

use super::{home_of, is_grandfathered, midi_fingerprint, saveable_song_path, SongStore, VfsStore};
use crate::external_tools::{is_midi_path, midi_sidecar_path};
use crate::song::scaffold_song_file;

/// Why `song init` wrote nothing.
#[derive(Debug, thiserror::Error)]
pub enum SongInitError {
    #[error("{} is not a .mid file", path.display())]
    NotMidi { path: PathBuf },
    #[error(
        "{} is outside the song folders; map a .mid under music/ or ai/music/ in the asset folder",
        path.display()
    )]
    OutsideHome { path: PathBuf },
    #[error("{} already has its sidecar, {}: open the song instead", midi.display(), sidecar.display())]
    Mapped { midi: PathBuf, sidecar: PathBuf },
    #[error("{reason}")]
    InvalidPath { reason: String },
    #[error("{} does not exist", path.display())]
    Missing { path: PathBuf },
    #[error("{} cannot be read as a song: {reason}", path.display())]
    Unreadable { path: PathBuf, reason: String },
    #[error("writing {} failed: {reason}", path.display())]
    Write { path: PathBuf, reason: String },
}

/// Write the scaffold sidecar ([`scaffold_song_file`], fingerprinting the `.mid` as it is)
/// beside the unmapped `.mid` at `path` through `common::vfs`; returns the sidecar's path.
/// A `.mid` that already has a sidecar is refused, and nothing is written.
pub fn init_song(path: &Path, asset_base: &Path) -> Result<PathBuf, SongInitError> {
    init_from(&mut VfsStore, path, asset_base)
}

pub(crate) fn init_from(
    store: &mut impl SongStore,
    path: &Path,
    asset_base: &Path,
) -> Result<PathBuf, SongInitError> {
    if !is_midi_path(path) {
        return Err(SongInitError::NotMidi {
            path: path.to_path_buf(),
        });
    }
    let outside_home = || SongInitError::OutsideHome {
        path: path.to_path_buf(),
    };
    let relative = path.strip_prefix(asset_base).map_err(|_| outside_home())?;
    if home_of(relative).is_none() {
        return Err(outside_home());
    }
    // The rule Save holds a song's path to, grandfather clause included: a sidecar under a
    // sample folder, or at a name a save refuses, would be a song that cannot be saved and
    // that another song's prune removes.
    if !is_grandfathered(relative) {
        saveable_song_path(relative).map_err(|reason| SongInitError::InvalidPath { reason })?;
    }
    let sidecar = midi_sidecar_path(path);
    let unreadable = |path: &Path, reason: String| SongInitError::Unreadable {
        path: path.to_path_buf(),
        reason,
    };
    match store.read(&sidecar) {
        Ok(_) => {
            return Err(SongInitError::Mapped {
                midi: path.to_path_buf(),
                sidecar,
            })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(unreadable(&sidecar, error.to_string())),
    }
    let bytes = match store.read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(SongInitError::Missing {
                path: path.to_path_buf(),
            })
        }
        Err(error) => return Err(unreadable(path, error.to_string())),
    };
    SongData::from_smf(&bytes).map_err(|error| unreadable(path, error.to_string()))?;
    let mut file = scaffold_song_file(asset_base);
    file.midi_fingerprint = Some(midi_fingerprint(&bytes));
    let write_failed = |reason: String| SongInitError::Write {
        path: sidecar.clone(),
        reason,
    };
    let text = write_song_file(&sidecar.display().to_string(), &file)
        .map_err(|error| write_failed(error.to_string()))?;
    store
        .write(&sidecar, text.as_bytes())
        .map_err(|error| write_failed(error.to_string()))?;
    Ok(sidecar)
}
