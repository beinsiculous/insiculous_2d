//! Where a kept recovery goes: beside its song as `<stem>.recovered-<fingerprint>.<ext>`,
//! its sidecar the MIDI file's whole name plus `.ron` (`theme.recovered-….mid.ron`, never
//! `theme.mid.recovered-….ron`), under a name no file holds yet. A recovery is never
//! promoted over the song: the person opens it if they want that save.

use std::io;
use std::path::{Path, PathBuf};

use super::SongStore;
use crate::external_tools::{midi_sidecar_path, sidecar_midi_path};

/// How many names a recovery tries before it gives up: `-2` to this are appended to the
/// fingerprint while a name is taken.
const MAXIMUM_RECOVERY_NAMES: u32 = 1000;

/// The first free recovery name beside `midi` for `fingerprint`: neither its MIDI file nor
/// its sidecar exists. `None` when every name tried is taken or unreadable.
pub(super) fn recovery_path(store: &impl SongStore, midi: &Path, fingerprint: u64) -> Option<PathBuf> {
    let stem = midi.file_stem()?.to_string_lossy();
    let extension = midi.extension()?.to_string_lossy();
    let base = format!("{stem}.recovered-{fingerprint:016x}");
    (1..=MAXIMUM_RECOVERY_NAMES)
        .map(|attempt| match attempt {
            1 => midi.with_file_name(format!("{base}.{extension}")),
            _ => midi.with_file_name(format!("{base}-{attempt}.{extension}")),
        })
        .find(|candidate| is_free(store, candidate) && is_free(store, &midi_sidecar_path(candidate)))
}

/// Whether `candidate` is a recovery's sidecar kept beside the song at `midi`.
pub(super) fn is_recovery_sidecar(midi: &Path, candidate: &Path) -> bool {
    let (Some(stem), Some(name)) = (midi.file_stem(), candidate.file_name()) else {
        return false;
    };
    let prefix = format!("{}.recovered-", stem.to_string_lossy());
    candidate.parent() == midi.parent()
        && name.to_string_lossy().starts_with(&prefix)
        && sidecar_midi_path(candidate).is_some()
}

/// Write `bytes` at `recovery`, then remove `journal`: a failed write leaves the journal.
pub(super) fn keep(
    store: &mut impl SongStore,
    bytes: &[u8],
    recovery: &Path,
    journal: &Path,
) -> io::Result<()> {
    store.write(recovery, bytes)?;
    store.remove(journal)
}

/// A file a read finds — or cannot rule out — is taken.
fn is_free(store: &impl SongStore, path: &Path) -> bool {
    matches!(store.read(path), Err(error) if error.kind() == io::ErrorKind::NotFound)
}
