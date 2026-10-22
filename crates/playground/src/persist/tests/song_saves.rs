//! A song's save journal through the queue: every durable prefix opens.
//!
//! The composer saves `x.mid.ron.new`, `x.mid.new`, then `x.mid`, `x.mid.ron`, then
//! removes the pair (`docs/SONG_FORMAT.md` § How the composer saves). These tests read the
//! store back after each operation lands and settle it with [`opened_song`], a model of
//! the open-time rules over bytes that name their version.

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
const JOURNAL_MIDI: &str = "music/theme.mid.new";
const JOURNAL_SIDECAR: &str = "music/theme.mid.ron.new";

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

/// What opening the song would show, by the open-time rules: a pair whose MIDI matches
/// its fingerprint is the truth; else finals that match each other stand; a `.mid` with
/// no sidecar opens as an import; finals that disagree with no valid pair are refused.
fn opened_song(files: &BTreeMap<String, Vec<u8>>) -> Result<String, String> {
    let file = |path: &str| files.get(path);
    if let (Some(sidecar), Some(midi)) = (file(JOURNAL_SIDECAR), file(JOURNAL_MIDI)) {
        if fingerprints(sidecar, midi) {
            return Ok(String::from_utf8_lossy(midi).into_owned());
        }
    }
    match (file(SIDECAR), file(MIDI)) {
        (Some(sidecar), Some(midi)) if fingerprints(sidecar, midi) => {
            Ok(String::from_utf8_lossy(midi).into_owned())
        }
        (Some(_), Some(_)) => Err("the finals disagree and no pair is valid".to_string()),
        (None, Some(midi)) => Ok(format!("import of {}", String::from_utf8_lossy(midi))),
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

fn save_song(chains: &mut Chains, version: &str) {
    write_project_file(chains, JOURNAL_SIDECAR, &sidecar_bytes(version));
    write_project_file(chains, JOURNAL_MIDI, &midi_bytes(version));
    write_project_file(chains, MIDI, &midi_bytes(version));
    write_project_file(chains, SIDECAR, &sidecar_bytes(version));
    remove_project_file(chains, JOURNAL_MIDI);
    remove_project_file(chains, JOURNAL_SIDECAR);
}

fn poll_once(chains: &mut Chains) -> bool {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    chains.poll_head(&mut context)
}

#[test]
fn test_save_reseed_save_lands_because_the_journal_tombstones_carry_their_revisions() {
    let memory = MemoryStore::new();
    let mut chains = pong_chains(Arc::new(memory.clone()));
    save_song(&mut chains, "v1");
    settle(&mut chains);
    assert!(stored_file(&memory, JOURNAL_MIDI).unwrap().deleted);

    let mut reloaded = pong_chains(Arc::new(memory.clone()));
    reloaded.seed(&pollster::block_on(memory.load_project("pong")).unwrap());
    save_song(&mut reloaded, "v2");
    settle(&mut reloaded);

    assert!(!reloaded.is_pending(), "nothing conflicted or stranded");
    assert_eq!(opened_song(&reloaded_files(&memory)), Ok("midi v2".to_string()));
    let journal = stored_file(&memory, JOURNAL_SIDECAR).unwrap();
    assert_eq!((journal.deleted, journal.revision), (true, 4), "put, tombstone, put, tombstone");
}

#[test]
fn test_a_failed_final_strands_the_head_its_removals_never_issue_and_the_retry_lands_the_rest() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = pong_chains(Arc::new(gated.clone()));
    save_song(&mut chains, "v1");

    assert!(poll_once(&mut chains), "the journal sidecar lands");
    gated.set_fail_next_operation(true);
    assert!(poll_once(&mut chains), "the journal MIDI lands, and the final MIDI issues to fail");
    settle(&mut chains);

    assert_eq!(chains.stranded_paths(), vec![MIDI.to_string()]);
    assert_eq!(chains.save_status(false).state, SaveState::Failed);
    assert_eq!(stored_bytes(&memory, JOURNAL_MIDI), Some(midi_bytes("v1")), "the pair was not removed");
    assert_eq!(opened_song(&reloaded_files(&memory)), Ok("midi v1".to_string()), "the pair opens the save");

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
    assert_eq!(landed_in_order, vec![MIDI, SIDECAR, JOURNAL_MIDI, JOURNAL_SIDECAR]);
    assert_eq!(stored_bytes(&memory, SIDECAR), Some(sidecar_bytes("v1")));
    assert_eq!(stored_bytes(&memory, JOURNAL_MIDI), None);
}

#[test]
fn test_overlapping_saves_reopen_the_latest_song_whose_pair_landed_after_every_operation() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = pong_chains(Arc::new(gated.clone()));
    save_song(&mut chains, "v0");
    settle(&mut chains);

    save_song(&mut chains, "v1");
    assert!(poll_once(&mut chains), "v1's journal sidecar lands");
    // v2's save arrives while v1's journal MIDI is in flight; v1's removals stay queued in order.
    gated.pause();
    save_song(&mut chains, "v2");
    gated.release();

    let mut latest_pair_landed = "midi v0".to_string();
    let mut operations = 0;
    loop {
        let files = reloaded_files(&memory);
        if let (Some(sidecar), Some(midi)) = (files.get(JOURNAL_SIDECAR), files.get(JOURNAL_MIDI)) {
            if fingerprints(sidecar, midi) {
                latest_pair_landed = String::from_utf8_lossy(midi).into_owned();
            }
        }
        assert_eq!(
            opened_song(&files),
            Ok(latest_pair_landed.clone()),
            "after {operations} operations the store reopens the latest saved pair"
        );
        if !poll_once(&mut chains) {
            break;
        }
        operations += 1;
    }
    assert!(!chains.is_pending());
    assert_eq!(latest_pair_landed, "midi v2");
    assert_eq!(operations, 11, "v1's pair, finals and two removals, then v2's whole save, in write order");
}

#[test]
fn test_a_replaced_upload_removes_the_sidecar_and_pair_first_and_every_boundary_opens() {
    let memory = MemoryStore::new();
    let interrupted = [
        (JOURNAL_SIDECAR, sidecar_bytes("v1")),
        (JOURNAL_MIDI, midi_bytes("v1")),
        (MIDI, midi_bytes("v1")),
        (SIDECAR, sidecar_bytes("v1")),
    ];
    let files = interrupted
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

    remove_project_file(&mut chains, SIDECAR);
    remove_project_file(&mut chains, JOURNAL_SIDECAR);
    remove_project_file(&mut chains, JOURNAL_MIDI);
    write_project_file(&mut chains, MIDI, b"midi uploaded");

    let mut opened_at_each_boundary = vec![opened_song(&reloaded_files(&memory))];
    while poll_once(&mut chains) {
        opened_at_each_boundary.push(opened_song(&reloaded_files(&memory)));
    }
    assert_eq!(
        opened_at_each_boundary,
        vec![
            Ok("midi v1".to_string()),
            Ok("midi v1".to_string()),
            Ok("import of midi v1".to_string()),
            Ok("import of midi v1".to_string()),
            Ok("import of midi uploaded".to_string()),
        ]
    );
}
