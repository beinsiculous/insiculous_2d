//! The one queue: write order, no coalescing, removals as tombstones, failure at the head.

use std::sync::Arc;
use std::task::{Context, Waker};

use super::{
    pong_chains, remove_project_file, settle, stored_bytes, stored_file, write_project_file,
    GatedStore,
};
use crate::persist::{PathState, SaveState};
use crate::store::memory::MemoryStore;
use crate::store::ProjectStore;

fn poll_once(chains: &mut crate::persist::Chains) -> bool {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    chains.poll_head(&mut context)
}

#[test]
fn test_two_paths_written_in_order_land_in_that_order_and_never_overlap_in_flight() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = pong_chains(Arc::new(gated.clone()));

    gated.pause();
    write_project_file(&mut chains, "music/theme.mid.ron.new", b"sidecar");
    write_project_file(&mut chains, "music/theme.mid.new", b"midi");
    assert_eq!(chains.path_state("music/theme.mid.ron.new"), PathState::InFlight);
    assert_eq!(
        chains.path_state("music/theme.mid.new"),
        PathState::Queued,
        "the second path waits for the first, never runs beside it"
    );

    gated.release();
    assert!(poll_once(&mut chains));
    assert_eq!(stored_bytes(&memory, "music/theme.mid.ron.new"), Some(b"sidecar".to_vec()));
    assert_eq!(stored_bytes(&memory, "music/theme.mid.new"), None);
    assert_eq!(chains.path_state("music/theme.mid.new"), PathState::InFlight);
    assert!(poll_once(&mut chains));
    assert_eq!(stored_bytes(&memory, "music/theme.mid.new"), Some(b"midi".to_vec()));
    assert!(!chains.is_pending());
}

#[test]
fn test_a_path_written_twice_lands_twice_in_order() {
    let memory = MemoryStore::new();
    let mut chains = pong_chains(Arc::new(memory.clone()));

    write_project_file(&mut chains, "scenes/main.ron", b"first");
    write_project_file(&mut chains, "scenes/main.ron", b"second");

    assert!(poll_once(&mut chains));
    assert_eq!(stored_file(&memory, "scenes/main.ron").map(|file| (file.bytes, file.revision)), Some((b"first".to_vec(), 1)));
    assert!(poll_once(&mut chains));
    assert_eq!(stored_file(&memory, "scenes/main.ron").map(|file| (file.bytes, file.revision)), Some((b"second".to_vec(), 2)));
}

#[test]
fn test_a_removal_behind_a_put_waits_for_it_and_lands_as_a_tombstone_a_later_put_must_name() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = pong_chains(Arc::new(gated.clone()));

    gated.pause();
    write_project_file(&mut chains, "music/samples/theme/blip.wav", b"blip");
    remove_project_file(&mut chains, "music/samples/theme/blip.wav");
    assert_eq!(chains.path_state("music/samples/theme/blip.wav"), PathState::Queued);
    gated.release();

    assert!(poll_once(&mut chains));
    assert_eq!(stored_bytes(&memory, "music/samples/theme/blip.wav"), Some(b"blip".to_vec()), "the put lands first");
    assert!(poll_once(&mut chains));
    let tombstone = stored_file(&memory, "music/samples/theme/blip.wav").unwrap();
    assert!(tombstone.deleted);
    assert!(tombstone.bytes.is_empty());
    assert_eq!(tombstone.revision, 2, "one revision above the put");
    assert_eq!(chains.base_revision("music/samples/theme/blip.wav"), 2);

    write_project_file(&mut chains, "music/samples/theme/blip.wav", b"blip again");
    settle(&mut chains);
    assert_eq!(chains.path_state("music/samples/theme/blip.wav"), PathState::Idle, "no conflict");
    let recreated = stored_file(&memory, "music/samples/theme/blip.wav").unwrap();
    assert_eq!((recreated.deleted, recreated.revision), (false, 3));
}

#[test]
fn test_a_write_behind_its_own_queued_removal_waits_for_it_and_lands_above_the_tombstone() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = pong_chains(Arc::new(gated.clone()));
    write_project_file(&mut chains, "music/theme.mid.new", b"old journal");
    settle(&mut chains);

    gated.pause();
    write_project_file(&mut chains, "scenes/main.ron", b"holds the head");
    remove_project_file(&mut chains, "music/theme.mid.new");
    write_project_file(&mut chains, "music/theme.mid.new", b"new journal");
    gated.release();

    assert!(poll_once(&mut chains), "the head lands");
    assert!(poll_once(&mut chains), "the removal lands next, in write order");
    assert!(
        stored_file(&memory, "music/theme.mid.new").unwrap().deleted,
        "the queued removal was never cancelled: a store state out of write order is a state no open-time rule settles"
    );
    settle(&mut chains);
    let journal = stored_file(&memory, "music/theme.mid.new").unwrap();
    assert_eq!((journal.deleted, journal.bytes, journal.revision), (false, b"new journal".to_vec(), 3));
}

#[test]
fn test_a_write_behind_an_issued_removal_waits_and_lands_one_revision_above_the_tombstone() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = pong_chains(Arc::new(gated.clone()));
    write_project_file(&mut chains, "music/theme.mid.new", b"old journal");
    settle(&mut chains);

    gated.pause();
    remove_project_file(&mut chains, "music/theme.mid.new");
    write_project_file(&mut chains, "music/theme.mid.new", b"new journal");
    assert_eq!(chains.path_state("music/theme.mid.new"), PathState::Queued);
    gated.release();

    assert!(poll_once(&mut chains));
    assert!(stored_file(&memory, "music/theme.mid.new").unwrap().deleted, "the issued removal lands");
    settle(&mut chains);
    let journal = stored_file(&memory, "music/theme.mid.new").unwrap();
    assert_eq!((journal.deleted, journal.revision), (false, 3));
}

#[test]
fn test_a_removal_of_a_never_persisted_path_enqueues_nothing() {
    let memory = MemoryStore::new();
    let mut chains = pong_chains(Arc::new(memory.clone()));

    // A bundled-only path in this project now needs a tombstone; the no-op case is
    // a path owned by another project, which these chains must never persist.
    chains.on_vfs_remove(std::path::Path::new("/projects/another/music/samples/theme/never.wav"));
    assert!(!chains.is_pending(), "a path outside this project never queues a removal");
    assert!(pollster::block_on(memory.load_project("pong")).unwrap().is_empty());
}

#[test]
fn test_bundled_only_delete_then_recreate_without_reseed_lands_at_revision_two() {
    let memory = MemoryStore::new();
    let mut chains = pong_chains(Arc::new(memory.clone()));
    remove_project_file(&mut chains, "assets/scripts/paddle.rhai");
    write_project_file(&mut chains, "assets/scripts/paddle.rhai", b"recreated");
    for _ in 0..4 {
        if !chains.has_active() { break; }
        poll_once(&mut chains);
    }
    assert!(!chains.is_pending());
    let recreated = stored_file(&memory, "assets/scripts/paddle.rhai").unwrap();
    assert_eq!(recreated.revision, 2);
    assert!(!recreated.deleted);
    assert_eq!(recreated.bytes, b"recreated");
    assert_eq!(chains.base_revision("assets/scripts/paddle.rhai"), 2);
}

#[test]
fn test_bundled_only_tombstone_records_the_running_bundle_version() {
    let memory = MemoryStore::new();
    let mut chains = crate::persist::Chains::new(
        "pong".to_string(), super::PONG_ROOT.to_string(), "v4".to_string(),
        super::test_manifest("pong"), Arc::new(memory.clone()), None,
    );
    remove_project_file(&mut chains, "assets/scripts/paddle.rhai");
    assert!(poll_once(&mut chains));
    assert_eq!(stored_file(&memory, "assets/scripts/paddle.rhai").unwrap().bundle_version, "v4");
}

#[test]
fn test_a_removal_from_a_tab_that_loaded_an_older_revision_is_dropped_and_the_queue_moves_on() {
    let memory = MemoryStore::new();
    let mut first_tab = pong_chains(Arc::new(memory.clone()));
    write_project_file(&mut first_tab, "music/samples/theme/blip.wav", b"blip");
    settle(&mut first_tab);

    // Another tab loads the same project, removes the sample and records it again.
    let mut second_tab = pong_chains(Arc::new(memory.clone()));
    second_tab.seed(&pollster::block_on(memory.load_project("pong")).unwrap());
    remove_project_file(&mut second_tab, "music/samples/theme/blip.wav");
    write_project_file(&mut second_tab, "music/samples/theme/blip.wav", b"recorded again");
    settle(&mut second_tab);

    remove_project_file(&mut first_tab, "music/samples/theme/blip.wav");
    write_project_file(&mut first_tab, "scenes/main.ron", b"behind the stale removal");
    settle(&mut first_tab);

    assert_eq!(
        stored_bytes(&memory, "music/samples/theme/blip.wav"),
        Some(b"recorded again".to_vec()),
        "the other tab's write wins"
    );
    assert!(first_tab.conflicted_paths().is_empty(), "never Conflicted over a removal");
    assert_eq!(stored_bytes(&memory, "scenes/main.ron"), Some(b"behind the stale removal".to_vec()));
    assert!(!first_tab.is_pending());
}

#[test]
fn test_remove_reseed_recreate_lands_at_the_tombstones_next_revision() {
    let memory = MemoryStore::new();
    let mut chains = pong_chains(Arc::new(memory.clone()));
    write_project_file(&mut chains, "music/samples/theme/blip.wav", b"blip");
    remove_project_file(&mut chains, "music/samples/theme/blip.wav");
    settle(&mut chains);

    let mut reloaded = pong_chains(Arc::new(memory.clone()));
    reloaded.seed(&pollster::block_on(memory.load_project("pong")).unwrap());
    assert_eq!(reloaded.base_revision("music/samples/theme/blip.wav"), 2, "the tombstone carries its revision");
    write_project_file(&mut reloaded, "music/samples/theme/blip.wav", b"blip again");
    settle(&mut reloaded);

    assert_eq!(reloaded.path_state("music/samples/theme/blip.wav"), PathState::Idle);
    assert_eq!(stored_bytes(&memory, "music/samples/theme/blip.wav"), Some(b"blip again".to_vec()));
}

#[test]
fn test_a_failed_removal_strands_the_head_and_the_retry_lands_it() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = pong_chains(Arc::new(gated.clone()));
    write_project_file(&mut chains, "music/theme.mid.new", b"journal");
    settle(&mut chains);

    gated.set_fail_next_operation(true);
    remove_project_file(&mut chains, "music/theme.mid.new");
    settle(&mut chains);
    assert_eq!(chains.path_state("music/theme.mid.new"), PathState::Stranded);
    assert_eq!(chains.save_status(false).state, SaveState::Failed);

    chains.reissue_stranded();
    settle(&mut chains);
    assert!(stored_file(&memory, "music/theme.mid.new").unwrap().deleted);
    assert!(!chains.is_pending());
}

#[test]
fn test_exclusive_create_recreates_a_tombstone_and_refuses_the_live_file() {
    let memory = MemoryStore::new();
    let relative_path = "assets/scripts/move.rhai";
    let tombstone = crate::store::StoredFile {
        project: "pong".to_string(),
        path: relative_path.to_string(),
        bytes: Vec::new(),
        revision: 1,
        bundle_version: "v1".to_string(),
        deleted: true,
    };
    pollster::block_on(memory.replace_project("pong", vec![tombstone], super::test_manifest("pong"))).unwrap();
    let mut chains = pong_chains(Arc::new(memory.clone()));
    chains.seed(&pollster::block_on(memory.load_project("pong")).unwrap());

    let mut files = common::vfs::MemFs::default();
    let path = std::path::Path::new(super::PONG_ROOT).join(relative_path);
    files.create_dir_all(path.parent().unwrap()).unwrap();
    files.write_new(&path, b"fn update(me, view, params, cmd, dt) {}").unwrap();
    chains.on_vfs_write(&path, &files.read(&path).unwrap()).unwrap();
    for _ in 0..8 {
        if !chains.has_active() { break; }
        poll_once(&mut chains);
    }
    assert!(!chains.is_pending(), "the tombstone's revision must be the create's CAS base");
    let stored = stored_file(&memory, relative_path).unwrap();
    assert_eq!(stored.revision, 2);
    assert!(!stored.deleted);
    assert_eq!(stored.bytes, files.read(&path).unwrap());

    let error = files.write_new(&path, b"replacement").unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(files.read(&path).unwrap(), stored.bytes);
    assert_eq!(stored_file(&memory, relative_path).unwrap().revision, 2);
}
