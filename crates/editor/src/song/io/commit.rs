//! Steps 2 to 7 of a save (the parent module's docs): the new samples, the journal, the home
//! `LICENSE`, the finals with the engine's score guarded, the journal removed, and the
//! prune, last, with every recovery beside the song as one of its roots.

use std::path::{Path, PathBuf};

use music_midi::{parse_song_file, InstrumentSource, Provenance, SongFile};

use super::{
    midi_fingerprint, read_optional, recover, render_sidecar, samples, with_suffix, SavePlan,
    SongSaveError, SongStore,
};
use crate::external_tools::midi_sidecar_path;

/// What a finished commit wrote.
pub(super) struct Committed {
    /// The sidecar as written, with the fingerprint of the score it was saved against.
    pub file: SongFile,
    pub sidecar_text: String,
    /// The engine's score, kept beside a `.mid` another program rewrote under the save.
    pub recovered: Option<Recovered>,
}

pub(super) struct Recovered {
    pub path: PathBuf,
    /// The `.mid` the save found on disk and left there.
    pub disk_score: Vec<u8>,
}

/// The first failed write stops the commit.
pub(super) fn commit(
    store: &mut impl SongStore,
    target: &Path,
    asset_base: &Path,
    plan: &SavePlan,
) -> Result<Committed, SongSaveError> {
    for sample in plan.samples.iter().filter(|sample| !sample.already_there) {
        write_file(
            store,
            &plan.sample_directory.join(&sample.file_name),
            &sample.bytes,
        )?;
    }
    let sidecar = midi_sidecar_path(target);
    let journal_sidecar = with_suffix(&sidecar, ".new");
    let journal_midi = with_suffix(target, ".new");
    let rendered = &plan.rendered;
    write_file(store, &journal_sidecar, rendered.sidecar_text.as_bytes())?;
    let mut journals_written = vec![journal_sidecar.as_path()];
    if let Some(midi) = &rendered.midi_bytes {
        write_file(store, &journal_midi, midi)?;
        journals_written.push(journal_midi.as_path());
    }
    // Before the home `LICENSE`, so a refused save has written nothing it keeps, and again
    // after it, right before the finals, so the `LICENSE`'s write is no window either; only
    // the journal files this save wrote go — a `.mid.new` an earlier interrupted save left
    // is a recovery, not this save's.
    guard_sidecar(store, target, &sidecar, &journals_written, plan)?;
    if let Some((path, text, previous)) = &plan.home_license {
        write_file(store, path, text.as_bytes())?;
        if let Err(refusal) = guard_sidecar(store, target, &sidecar, &journals_written, plan) {
            // The `LICENSE` is no journal, so a refusal here puts back what it held. The
            // refusal is what the caller hears either way — it is what the save must
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
    let mut file = rendered.file.clone();
    let mut sidecar_text = rendered.sidecar_text.clone();
    let mut recovered = None;
    if let Some(midi) = &rendered.midi_bytes {
        match changed_under_the_save(store, target, plan.score_guard)? {
            Some(disk_score) => {
                let path = recover::recovery_path(store, target, midi_fingerprint(midi))
                    .ok_or_else(|| SongSaveError::Write {
                        path: target.to_path_buf(),
                        reason: "it changed on disk, and no free name was left to keep this song's score under".to_string(),
                    })?;
                write_file(store, &path, midi)?;
                // The recovery's own sidecar, fingerprinting the engine's score: without it
                // the recovery opens unmapped and names no sample for the prune to keep.
                write_file(store, &midi_sidecar_path(&path), rendered.sidecar_text.as_bytes())?;
                let label = sidecar.display().to_string();
                (file, sidecar_text) = render_sidecar(file, midi_fingerprint(&disk_score), &label)?;
                recovered = Some(Recovered { path, disk_score });
            }
            None => write_file(store, target, midi)?,
        }
    }
    write_file(store, &sidecar, sidecar_text.as_bytes())?;
    if rendered.midi_bytes.is_some() {
        remove_file(store, &journal_midi)?;
    }
    remove_file(store, &journal_sidecar)?;
    prune(store, target, asset_base, plan)?;
    Ok(Committed {
        file,
        sidecar_text,
        recovered,
    })
}

/// Right before the home `LICENSE`, and again right before the finals: the sidecar on disk
/// against what the save's first look found, for a save that keeps it. A sidecar changed in
/// between refuses the save as the first look would have, the journal files this save wrote
/// removed, the samples left — nothing names them yet, and the next prune takes them.
fn guard_sidecar(
    store: &mut impl SongStore,
    target: &Path,
    sidecar: &Path,
    journals: &[&Path],
    plan: &SavePlan,
) -> Result<(), SongSaveError> {
    let Some(expected) = plan.sidecar_guard else {
        return Ok(());
    };
    let on_disk = read_optional(store, sidecar)?.map(|bytes| midi_fingerprint(&bytes));
    if on_disk == expected {
        return Ok(());
    }
    for journal in journals {
        remove_file(store, journal)?;
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

/// The `.mid` on disk when it is no longer the one `guard` fingerprints: another program
/// rewrote it since the document read or last wrote it. A file gone is not guarded.
fn changed_under_the_save(
    store: &impl SongStore,
    target: &Path,
    guard: Option<u64>,
) -> Result<Option<Vec<u8>>, SongSaveError> {
    let Some(expected) = guard else {
        return Ok(None);
    };
    Ok(read_optional(store, target)?.filter(|bytes| midi_fingerprint(bytes) != expected))
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

/// Step 7: remove what neither the song nor a recovery beside it names from the song's
/// folder, and rewrite its `LICENSE` with exactly those samples' stanzas (the folder itself
/// goes with its last sample). A recovery that cannot be read leaves the folder as it is:
/// its samples are unknown, and the next save prunes once it is gone.
fn prune(
    store: &mut impl SongStore,
    target: &Path,
    asset_base: &Path,
    plan: &SavePlan,
) -> Result<(), SongSaveError> {
    let Some(recovered) = recovery_samples(store, target, asset_base, &plan.sample_directory)
    else {
        log::warn!(
            "a recovery beside {} cannot be read, so its samples are kept unpruned",
            target.display()
        );
        return Ok(());
    };
    let stanzas: Vec<(&str, &Provenance)> = plan
        .samples
        .iter()
        .map(|sample| (sample.file_name.as_str(), &sample.provenance))
        .chain(
            recovered
                .iter()
                .map(|(file_name, provenance)| (file_name.as_str(), provenance)),
        )
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

/// Every sample in `sample_directory` a recovery's sidecar beside the song at `target`
/// names, by its file name there, with the provenance it names it under; `None` when a
/// recovery cannot be read or parsed.
fn recovery_samples(
    store: &impl SongStore,
    target: &Path,
    asset_base: &Path,
    sample_directory: &Path,
) -> Option<Vec<(String, Provenance)>> {
    let folder = target.parent()?;
    let listed = store.list_files(folder).ok()?;
    // A journal sidecar still beside the song is an open's recovery that could not be
    // kept whole: its samples are roots until it is gone.
    let journal_sidecar = with_suffix(&midi_sidecar_path(target), ".new");
    let leftover_journal = match read_optional(store, &journal_sidecar) {
        Ok(Some(_)) => Some(journal_sidecar.clone()),
        Ok(None) => None,
        // Unreadable is not absent: its samples are unknown, so nothing is pruned.
        Err(_) => return None,
    };
    let mut named = Vec::new();
    for sidecar in listed
        .iter()
        .filter(|path| recover::is_recovery_sidecar(target, path))
        .chain(leftover_journal.iter())
    {
        let text = String::from_utf8(store.read(sidecar).ok()?).ok()?;
        let file = parse_song_file(&sidecar.display().to_string(), &text).ok()?;
        for instrument in file.instruments {
            let (InstrumentSource::Sample { path, .. }, Some(provenance)) =
                (instrument.source, instrument.provenance)
            else {
                continue;
            };
            if let Ok(file_name) = asset_base.join(&path).strip_prefix(sample_directory) {
                named.push((file_name.to_string_lossy().into_owned(), provenance));
            }
        }
    }
    Some(named)
}
