//! A song's save through the queue: every durable prefix opens.
//!
//! A save writes each final to a temporary file beside it and moves it over the final, which
//! on the web is the final's put, then the temporary file's removal; a Save As does the
//! `.mid` and then the sidecar (`docs/SONG_FORMAT.md` § How the composer saves). These tests
//! read the store back after each operation lands and settle it with [`opened_song`], a
//! model of the open-time rules over bytes that name their version.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::task::{Context, Waker};

use super::{
    pong_chains, remove_project_file, settle, stored_bytes, stored_file, write_project_file,
    GatedStore,
};
use crate::persist::{Chains, SaveState};
use crate::store::memory::MemoryStore;
use crate::store::{ProjectStore, StoredFile};

const MIDI: &str = "music/theme.mid";
const SIDECAR: &str = "music/theme.mid.ron";
const TEMPORARY_MIDI: &str = "music/.theme.mid.saving";
const TEMPORARY_SIDECAR: &str = "music/.theme.mid.ron.saving";

fn midi_bytes(version: &str) -> Vec<u8> {
    format!("midi {version}").into_bytes()
}

/// A sidecar carries the fingerprint of the MIDI bytes it was written beside.
fn sidecar_bytes(version: &str) -> Vec<u8> {
    format!("sidecar fingerprinting midi {version}").into_bytes()
}

fn fingerprints(sidecar: &[u8], midi: &[u8]) -> bool {
    sidecar.strip_prefix(b"sidecar fingerprinting ") == Some(midi)
}

/// What opening the song would show, by the open-time rules over the finals as they are: a
/// sidecar fingerprinting another score is the score changed outside, opened on the disk's
/// score with that map; a `.mid` with no sidecar opens on a new map; a temporary file is
/// no song's. Only a song with no `.mid` does not open.
fn opened_song(files: &BTreeMap<String, Vec<u8>>) -> Result<String, String> {
    let file = |path: &str| files.get(path).map(|bytes| String::from_utf8_lossy(bytes).into_owned());
    match (files.get(SIDECAR), file(MIDI)) {
        (Some(sidecar), Some(midi)) if fingerprints(sidecar, midi.as_bytes()) => Ok(midi),
        (Some(_), Some(midi)) => Ok(format!("{midi} under the map of {}", file(SIDECAR).unwrap_or_default())),
        (None, Some(midi)) => Ok(format!("{midi} on a new map")),
        _ => Err("no song".to_string()),
    }
}

/// The live files a reload would put in MemFs: tombstones are skipped.
fn reloaded_files(memory: &MemoryStore) -> BTreeMap<String, Vec<u8>> {
    pollster::block_on(memory.load_project("pong"))
        .unwrap()
        .into_iter()
        .filter(|file| !file.deleted)
        .map(|file| (file.path, file.bytes))
        .collect()
}

/// A Save As's writes on the web: each final through its temporary file, the `.mid` first.
fn save_song_as(chains: &mut Chains, version: &str) {
    for (temporary, final_path, bytes) in [
        (TEMPORARY_MIDI, MIDI, midi_bytes(version)),
        (TEMPORARY_SIDECAR, SIDECAR, sidecar_bytes(version)),
    ] {
        write_project_file(chains, temporary, &bytes);
        write_project_file(chains, final_path, &bytes);
        remove_project_file(chains, temporary);
    }
}

fn poll_once(chains: &mut Chains) -> bool {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    chains.poll_head(&mut context)
}

#[test]
fn test_save_reseed_save_lands_because_the_temporary_files_tombstones_carry_their_revisions() {
    let memory = MemoryStore::new();
    let mut chains = pong_chains(Arc::new(memory.clone()));
    save_song_as(&mut chains, "v1");
    settle(&mut chains);
    assert!(stored_file(&memory, TEMPORARY_MIDI).unwrap().deleted);

    let mut reloaded = pong_chains(Arc::new(memory.clone()));
    reloaded.seed(&pollster::block_on(memory.load_project("pong")).unwrap());
    save_song_as(&mut reloaded, "v2");
    settle(&mut reloaded);

    assert!(!reloaded.is_pending(), "nothing conflicted or stranded");
    assert_eq!(opened_song(&reloaded_files(&memory)), Ok("midi v2".to_string()));
    let temporary = stored_file(&memory, TEMPORARY_SIDECAR).unwrap();
    assert_eq!((temporary.deleted, temporary.revision), (true, 4), "put, tombstone, put, tombstone");
}

#[test]
fn test_a_failed_final_strands_the_head_its_removals_never_issue_and_the_retry_lands_the_rest() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = pong_chains(Arc::new(gated.clone()));
    save_song_as(&mut chains, "v0");
    settle(&mut chains);
    save_song_as(&mut chains, "v1");

    gated.set_fail_next_operation(true);
    assert!(poll_once(&mut chains), "the temporary MIDI lands, and the final MIDI issues to fail");
    settle(&mut chains);

    assert_eq!(chains.stranded_paths(), vec![MIDI.to_string()]);
    assert_eq!(chains.save_status(false).state, SaveState::Failed);
    assert_eq!(stored_bytes(&memory, TEMPORARY_MIDI), Some(midi_bytes("v1")), "its removal never issued");
    assert_eq!(opened_song(&reloaded_files(&memory)), Ok("midi v0".to_string()), "the old finals open whole");

    chains.reissue_stranded();
    let mut landed_in_order = Vec::new();
    while chains.has_active() {
        let head = chains.take_started_operation().map(|(path, future)| {
            let result = pollster::block_on(future);
            chains.handle_completion(&path, result);
            path
        });
        landed_in_order.extend(head);
    }
    assert_eq!(landed_in_order, vec![MIDI, TEMPORARY_MIDI, TEMPORARY_SIDECAR, SIDECAR, TEMPORARY_SIDECAR]);
    assert_eq!(opened_song(&reloaded_files(&memory)), Ok("midi v1".to_string()));
    assert_eq!(stored_bytes(&memory, TEMPORARY_MIDI), None);
}

#[test]
fn test_overlapping_saves_open_after_every_operation_on_the_finals_that_landed() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = pong_chains(Arc::new(gated.clone()));
    save_song_as(&mut chains, "v0");
    settle(&mut chains);

    save_song_as(&mut chains, "v1");
    assert!(poll_once(&mut chains), "v1's temporary MIDI lands");
    // v2's save arrives while v1's final MIDI is in flight; v1's later writes stay queued in order.
    gated.pause();
    save_song_as(&mut chains, "v2");
    gated.release();

    let mut operations = 0;
    loop {
        let files = reloaded_files(&memory);
        let landed_midi = String::from_utf8_lossy(&files[MIDI]).into_owned();
        let opened = opened_song(&files);
        assert!(
            opened.as_ref().is_ok_and(|song| song.starts_with(&landed_midi)),
            "after {operations} operations the song opens on the MIDI file that landed: {opened:?}"
        );
        if !poll_once(&mut chains) {
            break;
        }
        operations += 1;
    }
    assert!(!chains.is_pending());
    assert_eq!(opened_song(&reloaded_files(&memory)), Ok("midi v2".to_string()));
    assert_eq!(operations, 11, "v1's five writes left, then v2's six, in write order");
}

#[test]
fn test_a_replaced_upload_keeps_the_sidecar_and_every_boundary_opens() {
    let memory = MemoryStore::new();
    let saved = [(MIDI, midi_bytes("v1")), (SIDECAR, sidecar_bytes("v1"))];
    let files = saved
        .iter()
        .map(|(path, bytes)| StoredFile {
            project: "pong".to_string(),
            path: path.to_string(),
            bytes: bytes.clone(),
            revision: 1,
            bundle_version: "v1".to_string(),
            deleted: false,
        })
        .collect::<Vec<_>>();
    pollster::block_on(memory.replace_project("pong", files.clone(), super::test_manifest("pong"))).unwrap();
    let mut chains = pong_chains(Arc::new(memory.clone()));
    chains.seed(&files);

    write_project_file(&mut chains, MIDI, b"midi uploaded");

    let mut opened_at_each_boundary = vec![opened_song(&reloaded_files(&memory))];
    while poll_once(&mut chains) {
        opened_at_each_boundary.push(opened_song(&reloaded_files(&memory)));
    }
    assert_eq!(
        opened_at_each_boundary,
        vec![
            Ok("midi v1".to_string()),
            Ok("midi uploaded under the map of sidecar fingerprinting midi v1".to_string()),
        ]
    );
}
