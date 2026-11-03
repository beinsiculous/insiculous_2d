//! Steps 2 to 5 of a save (the parent module's docs): the new samples, the home `LICENSE`
//! between two looks at the sidecar, the finals each by a temporary file and a rename, and
//! the prune, last.

use std::path::{Path, PathBuf};

use music_midi::Provenance;

use super::{midi_fingerprint, read_optional, samples, SavePlan, SongSaveError, SongStore};
use crate::external_tools::midi_sidecar_path;

/// The first failed write stops the commit.
pub(super) fn commit(
    store: &mut impl SongStore,
    target: &Path,
    plan: &SavePlan,
) -> Result<(), SongSaveError> {
    for sample in plan.samples.iter().filter(|sample| !sample.already_there) {
        write_file(
            store,
            &plan.sample_directory.join(&sample.file_name),
            &sample.bytes,
        )?;
    }
    let sidecar = midi_sidecar_path(target);
    // Before the home `LICENSE`, so a refused save has written nothing it keeps, and again
    // after it, right before the finals, so the `LICENSE`'s write is no window either.
    guard_sidecar(store, target, &sidecar, plan)?;
    if let Some((path, text, previous)) = &plan.home_license {
        write_file(store, path, text.as_bytes())?;
        if let Err(refusal) = guard_sidecar(store, target, &sidecar, plan) {
            // The refusal is what the caller hears either way — it is what the save must
            // answer; a put-back that fails is named here and leaves the new text.
            let put_back = match previous {
                Some(previous) => write_file(store, path, previous.as_bytes()),
                None => remove_file(store, path),
            };
            if let Err(error) = put_back {
                log::warn!(
                    "the home LICENSE at {} keeps a refused save's stanza: {error}",
                    path.display()
                );
            }
            return Err(refusal);
        }
    }
    let rendered = &plan.rendered;
    if let Some(midi) = &rendered.midi_bytes {
        replace_file(store, target, midi)?;
    }
    replace_file(store, &sidecar, rendered.sidecar_text.as_bytes())?;
    prune(store, plan)
}

/// Right before the home `LICENSE`, and again right before the finals: the sidecar on disk
/// against what the save's first look found, for a save that keeps it. A sidecar changed in
/// between refuses the save as the first look would have, the samples left — nothing names
/// them yet, and the next prune takes them.
fn guard_sidecar(
    store: &impl SongStore,
    target: &Path,
    sidecar: &Path,
    plan: &SavePlan,
) -> Result<(), SongSaveError> {
    let Some(expected) = plan.sidecar_guard else {
        return Ok(());
    };
    let on_disk = read_optional(store, sidecar)?.map(|bytes| midi_fingerprint(&bytes));
    if on_disk == expected {
        return Ok(());
    }
    Err(SongSaveError::SidecarChanged {
        sidecar: sidecar.to_path_buf(),
        loaded: expected,
        on_disk,
        midi: target.to_path_buf(),
        score_loaded: plan.score_loaded,
        score_on_disk: read_optional(store, target)?.map(|bytes| midi_fingerprint(&bytes)),
    })
}

/// The temporary file a final is written to before it is renamed over `path`: beside it, in
/// its own folder, so the rename never crosses a filesystem; hidden by its leading dot, and
/// under an extension no scan of songs, sounds or images takes.
fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.saving"))
}

/// Write `bytes` over `path` whole: into the temporary file, then renamed over it. A failed
/// write or rename leaves `path` as it was, and the temporary file is removed when it can be.
fn replace_file(store: &mut impl SongStore, path: &Path, bytes: &[u8]) -> Result<(), SongSaveError> {
    let temporary = temporary_path(path);
    let written = store
        .write(&temporary, bytes)
        .and_then(|()| store.rename(&temporary, path));
    written.map_err(|error| {
        if let Err(removal) = store.remove(&temporary) {
            log::warn!(
                "the unfinished save's {} could not be removed: {removal}",
                temporary.display()
            );
        }
        SongSaveError::Write {
            path: path.to_path_buf(),
            reason: error.to_string(),
        }
    })
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

/// Step 5: remove what the song does not name from its folder, and rewrite its `LICENSE`
/// with exactly those samples' stanzas (the folder itself goes with its last sample).
fn prune(store: &mut impl SongStore, plan: &SavePlan) -> Result<(), SongSaveError> {
    let stanzas: Vec<(&str, &Provenance)> = plan
        .samples
        .iter()
        .map(|sample| (sample.file_name.as_str(), &sample.provenance))
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
        let named = stanzas.iter().any(|(file_name, _)| *file_name == relative);
        if relative != "LICENSE" && !named {
            remove_file(store, &path)?;
        }
    }
    if stanzas.is_empty() {
        remove_file(store, &plan.sample_directory)?;
    } else {
        let license = samples::license_text(stanzas, &plan.song);
        write_file(
            store,
            &plan.sample_directory.join("LICENSE"),
            license.as_bytes(),
        )?;
    }
    Ok(())
}
