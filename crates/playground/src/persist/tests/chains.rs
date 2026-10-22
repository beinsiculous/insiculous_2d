use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::task::{Context, Waker};

use super::{pong_chains, settle, store_another_tabs_file, stored_bytes, test_manifest, write_project_file, GatedStore};
use crate::persist::{Chains, PathState, SaveState, SaveStatus};
use crate::store::memory::MemoryStore;
use crate::store::{ProjectStore, StoredFile};

#[test]
fn test_take_started_operation_moves_the_future_and_keeps_the_state() {
    let memory = MemoryStore::new();
    let mut chains = Chains::new(
        "pong".to_string(),
        "/projects/pong".to_string(),
        "v1".to_string(),
        test_manifest("pong"),
        Arc::new(memory),
        None,
    );

    let path = std::path::Path::new("/projects/pong/scenes/main.ron");
    chains.on_vfs_write(path, b"test content").unwrap();
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::InFlight);

    let (taken_path, future) = chains.take_started_operation().unwrap();
    assert_eq!(taken_path, "scenes/main.ron");
    assert!(chains.take_started_operation().is_none(), "one operation runs at a time");

    // Chain is still InFlight while future is taken
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::InFlight);

    let result = pollster::block_on(future);
    assert_eq!(result, Ok(1));
    chains.handle_completion(&taken_path, result);

    assert_eq!(chains.path_state("scenes/main.ron"), PathState::Idle);
    assert_eq!(chains.base_revision("scenes/main.ron"), 1);
}

#[test]
fn test_two_puts_chain_with_gated_store() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let pending_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut chains = Chains::new(
        "pong".to_string(),
        "/playground/v1/assets/projects/pong".to_string(),
        "v1".to_string(),
        test_manifest("pong"),
        Arc::new(gated.clone()),
        Some(pending_flag.clone()),
    );

    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    // 1. Pause gate and start first write
    gated.pause();
    let full_path = std::path::Path::new("/playground/v1/assets/projects/pong/scenes/main.ron");
    chains.on_vfs_write(full_path, b"edit 1").unwrap();
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::InFlight);
    assert!(pending_flag.load(Ordering::Relaxed));

    // 2. Second write for same path while first is in flight -> becomes Queued
    chains.on_vfs_write(full_path, b"edit 2").unwrap();
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::Queued);

    // 3. Release gate and poll
    gated.release();
    assert!(chains.poll_head(&mut context));
    // First put finished (rev 1), second put automatically started (InFlight)
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::InFlight);
    assert_eq!(chains.base_revision("scenes/main.ron"), 1);

    // 4. Poll again for second put completion
    assert!(chains.poll_head(&mut context));
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::Idle);
    assert_eq!(chains.base_revision("scenes/main.ron"), 2);
    assert!(!pending_flag.load(Ordering::Relaxed));

    // Verify stored content has edit 2 and revision 2
    pollster::block_on(async {
        let loaded = memory.load_project("pong").await.unwrap();
        assert_eq!(loaded[0].revision, 2);
        assert_eq!(loaded[0].bytes, b"edit 2");
    });
}

#[test]
fn test_seed_then_save_advances_revision_without_conflict() {
    let memory = MemoryStore::new();
    let mut chains = Chains::new(
        "pong".to_string(),
        "/projects/pong".to_string(),
        "v1".to_string(),
        test_manifest("pong"),
        Arc::new(memory.clone()),
        None,
    );

    let seeded = vec![StoredFile {
        project: "pong".to_string(),
        path: "scenes/main.ron".to_string(),
        bytes: b"loaded_rev_3".to_vec(),
        revision: 3,
        bundle_version: "v1".to_string(),
        deleted: false,
    }];
    pollster::block_on(async {
        memory.replace_project("pong", seeded.clone(), test_manifest("pong")).await.unwrap();
    });
    chains.seed(&seeded);
    assert_eq!(chains.base_revision("scenes/main.ron"), 3);

    let path = std::path::Path::new("/projects/pong/scenes/main.ron");
    chains.on_vfs_write(path, b"saved_rev_4").unwrap();

    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    chains.poll_head(&mut context);

    assert_eq!(chains.base_revision("scenes/main.ron"), 4);
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::Idle);
}

#[test]
fn test_writes_during_a_drain_are_refused_until_the_epoch_is_restored() {
    let memory = MemoryStore::new();
    let mut chains = Chains::new(
        "pong".to_string(),
        "/projects/pong".to_string(),
        "v1".to_string(),
        test_manifest("pong"),
        Arc::new(memory),
        None,
    );

    let epoch = chains.write_epoch();
    chains.start_drain();
    assert_eq!(chains.write_epoch(), epoch + 1);

    let path = std::path::Path::new("/projects/pong/scenes/main.ron");
    let error = chains.on_vfs_write(path, b"data").unwrap_err();
    assert!(error.contains("project is being replaced"));

    chains.restore_epoch();
    assert_eq!(chains.write_epoch(), epoch);
    assert!(chains.on_vfs_write(path, b"data").is_ok());
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::InFlight);
}

#[test]
fn test_conflicted_path_never_reissued() {
    let memory = MemoryStore::new();
    let mut chains = pong_chains(Arc::new(memory.clone()));
    store_another_tabs_file(&memory, "a.txt", 2);

    write_project_file(&mut chains, "a.txt", b"mine");
    settle(&mut chains);
    assert_eq!(chains.path_state("a.txt"), PathState::Conflicted);

    chains.reissue_stranded();
    assert_eq!(chains.path_state("a.txt"), PathState::Conflicted);
    write_project_file(&mut chains, "a.txt", b"mine again");
    settle(&mut chains);
    assert_eq!(chains.path_state("a.txt"), PathState::Conflicted);
    assert_eq!(stored_bytes(&memory, "a.txt"), Some(b"other tab".to_vec()), "no put reached the store");
}

#[test]
fn test_drain_finishes_the_queue_written_before_it_and_refuses_new_writes() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = pong_chains(Arc::new(gated.clone()));

    gated.pause();
    write_project_file(&mut chains, "scenes/main.ron", b"v1");
    write_project_file(&mut chains, "scenes/main.ron", b"v2");
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::Queued);

    chains.start_drain();
    let write_error = chains
        .on_vfs_write(std::path::Path::new("/projects/pong/scenes/other.ron"), b"v3")
        .unwrap_err();
    assert!(write_error.contains("project is being replaced"));
    assert_eq!(chains.path_state("scenes/other.ron"), PathState::Idle);
    assert!(chains.has_active(), "the drain waits while anything written before it is left");

    gated.release();
    settle(&mut chains);
    assert!(!chains.has_active());
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::Idle, "v2 landed before the drain ended");
    assert_eq!(chains.base_revision("scenes/main.ron"), 2);
    assert_eq!(stored_bytes(&memory, "scenes/main.ron"), Some(b"v2".to_vec()));

    chains.restore_epoch();
    assert!(!chains.is_pending());
}

#[test]
fn test_a_drain_retries_a_stranded_head_once_and_a_second_failure_leaves_it_stranded_not_active() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = pong_chains(Arc::new(gated.clone()));

    gated.set_fail_next_operation(true);
    write_project_file(&mut chains, "scenes/main.ron", b"v1");
    settle(&mut chains);
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::Stranded);

    // The switch is the visitor acting, so the stranded head is tried again; the store is back.
    chains.start_drain();
    assert!(chains.has_active(), "the stranded head was retried");
    settle(&mut chains);
    assert!(!chains.has_active());
    assert_eq!(stored_bytes(&memory, "scenes/main.ron"), Some(b"v1".to_vec()));
    chains.restore_epoch();

    gated.set_fail_next_operation(true);
    write_project_file(&mut chains, "scenes/main.ron", b"v2");
    settle(&mut chains);
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::Stranded);

    // The store is still down: the retry fails and the switch goes on without it.
    gated.set_fail_next_operation(true);
    chains.start_drain();
    settle(&mut chains);
        assert!(!chains.has_active(), "a head stranded again is not active: the drain reads it as stranded and refuses");
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::Stranded);
    assert_eq!(stored_bytes(&memory, "scenes/main.ron"), Some(b"v1".to_vec()));
}

#[test]
fn test_first_time_put_born_during_drain_is_refused() {
    let memory = MemoryStore::new();
    let mut chains = Chains::new(
        "pong".to_string(),
        "/projects/pong".to_string(),
        "v1".to_string(),
        test_manifest("pong"),
        Arc::new(memory.clone()),
        None,
    );

    chains.start_drain();
    let new_path = std::path::Path::new("/projects/pong/brand_new.ron");
    let error = chains.on_vfs_write(new_path, b"content").unwrap_err();
    assert!(error.contains("project is being replaced"));
    assert!(!chains.chains.contains_key("brand_new.ron"));

    pollster::block_on(async {
        memory.remove_project("pong").await.unwrap();
        let loaded = memory.load_project("pong").await.unwrap();
        assert!(loaded.is_empty());
    });
}

#[test]
fn test_a_failed_put_strands_the_head_and_the_put_behind_it_waits_for_the_retry() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = pong_chains(Arc::new(gated.clone()));

    gated.pause();
    gated.set_fail_next_operation(true);
    write_project_file(&mut chains, "file.txt", b"v1");
    write_project_file(&mut chains, "file.txt", b"v2");
    gated.release();
    settle(&mut chains);

    assert_eq!(chains.path_state("file.txt"), PathState::Stranded);
    assert!(chains.is_pending(), "a stranded head keeps the unload prompt armed");
    assert_eq!(stored_bytes(&memory, "file.txt"), None, "v2 never overtook the failed v1");

    chains.reissue_stranded();
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    assert!(chains.poll_head(&mut context));
    assert_eq!(stored_bytes(&memory, "file.txt"), Some(b"v1".to_vec()), "v1 lands first");
    settle(&mut chains);
    assert_eq!(chains.path_state("file.txt"), PathState::Idle);
    assert_eq!(chains.base_revision("file.txt"), 2);
    assert_eq!(stored_bytes(&memory, "file.txt"), Some(b"v2".to_vec()));
}

#[test]
fn test_a_stranded_head_retries_on_the_next_write_and_lands_before_it() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = pong_chains(Arc::new(gated.clone()));

    gated.set_fail_next_operation(true);
    write_project_file(&mut chains, "stranded.txt", b"stranded_bytes");
    settle(&mut chains);
    assert_eq!(chains.path_state("stranded.txt"), PathState::Stranded);

    // The next write is evidence the tab is alive: the head is issued again at once,
    // with the new write queued behind it.
    gated.pause();
    write_project_file(&mut chains, "busy.txt", b"busy");
    assert_eq!(chains.path_state("stranded.txt"), PathState::InFlight);
    assert_eq!(chains.path_state("busy.txt"), PathState::Queued);

    gated.release();
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    assert!(chains.poll_head(&mut context));
    assert_eq!(stored_bytes(&memory, "stranded.txt"), Some(b"stranded_bytes".to_vec()));
    assert_eq!(stored_bytes(&memory, "busy.txt"), None, "the retried head lands first");
    settle(&mut chains);
    assert_eq!(stored_bytes(&memory, "busy.txt"), Some(b"busy".to_vec()));
    assert!(!chains.is_pending());
}

#[test]
fn test_put_resolving_after_drain_timeout_updates_base_without_conflict() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory.clone());
    let mut chains = Chains::new(
        "pong".to_string(),
        "/projects/pong".to_string(),
        "v1".to_string(),
        test_manifest("pong"),
        Arc::new(gated.clone()),
        None,
    );
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    gated.pause();
    let path = std::path::Path::new("/projects/pong/scene.ron");
    chains.on_vfs_write(path, b"content").unwrap();
    assert_eq!(chains.path_state("scene.ron"), PathState::InFlight);

    // Timeout path: start_drain then restore_epoch
    chains.start_drain();
    chains.restore_epoch();

    // Now gate releases and put resolves Ok(1)
    gated.release();
    chains.poll_head(&mut context);

    assert_eq!(chains.path_state("scene.ron"), PathState::Idle);
    assert_eq!(chains.base_revision("scene.ron"), 1);
}

#[test]
fn test_write_under_a_sibling_slug_sharing_the_root_prefix_is_ignored() {
    let memory = MemoryStore::new();
    let mut chains = Chains::new(
        "pong".to_string(),
        "/projects/pong".to_string(),
        "v1".to_string(),
        test_manifest("pong"),
        Arc::new(memory),
        None,
    );

    let sibling_path = std::path::Path::new("/projects/pong2/scenes/x.ron");
    chains.on_vfs_write(sibling_path, b"sibling").unwrap();

    assert!(!chains.is_pending(), "a sibling slug's write must not start a put under this project");
    assert!(!chains.chains.contains_key("2/scenes/x.ron"), "the root is a directory boundary, not a string prefix");
}

#[test]
fn test_a_stranded_head_is_retried_by_the_drain_once_not_by_visibility_during_it_and_again_after_restore() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory);
    let mut chains = Chains::new(
        "pong".to_string(),
        "/projects/pong".to_string(),
        "v1".to_string(),
        test_manifest("pong"),
        Arc::new(gated.clone()),
        None,
    );
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    gated.pause();
    gated.set_fail_next_operation(true);
    let stranded_path = std::path::Path::new("/projects/pong/stranded.txt");
    chains.on_vfs_write(stranded_path, b"stranded_bytes").unwrap();
    gated.release();
    chains.poll_head(&mut context);
    assert_eq!(chains.path_state("stranded.txt"), PathState::Stranded);

    // The drain's own retry, which fails again here, so the head is stranded under the drain.
    gated.set_fail_next_operation(true);
    chains.start_drain();
    assert_eq!(chains.path_state("stranded.txt"), PathState::InFlight, "the drain retried it once");
    chains.poll_head(&mut context);
    assert_eq!(chains.path_state("stranded.txt"), PathState::Stranded);

    // A visibility change during the drain must not retry it: the switch is going ahead.
    chains.reissue_stranded();
    assert_eq!(chains.path_state("stranded.txt"), PathState::Stranded);

    chains.restore_epoch();
    gated.pause();
    chains.reissue_stranded();
    assert_eq!(chains.path_state("stranded.txt"), PathState::InFlight);
}

#[test]
fn test_a_conflicted_put_holds_the_queue_and_names_its_path() {
    let memory = MemoryStore::new();
    let mut chains = pong_chains(Arc::new(memory.clone()));
    store_another_tabs_file(&memory, "z.ron", 1);

    write_project_file(&mut chains, "z.ron", b"mine");
    write_project_file(&mut chains, "a.txt", b"behind the conflict");
    settle(&mut chains);

    assert_eq!(chains.conflicted_paths(), vec!["z.ron".to_string()]);
    assert_eq!(chains.path_state("a.txt"), PathState::Queued, "nothing issues behind a conflict");
    assert!(chains.is_pending());
    assert_eq!(stored_bytes(&memory, "a.txt"), None);
    assert_eq!(
        chains.save_status(false),
        SaveStatus { state: SaveState::Failed, reason: "conflicted: z.ron".to_string() }
    );
}

#[test]
fn test_save_status_walks_the_four_states_and_the_dirty_flag() {
    let memory = MemoryStore::new();
    let gated = GatedStore::new(memory);
    let mut chains = Chains::new(
        "pong".to_string(),
        "/projects/pong".to_string(),
        "v1".to_string(),
        test_manifest("pong"),
        Arc::new(gated.clone()),
        None,
    );
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let path = std::path::Path::new("/projects/pong/scenes/main.ron");

    // Nothing written, nothing pending: the editor's flag decides on its own.
    assert_eq!(
        chains.save_status(false),
        SaveStatus { state: SaveState::Saved, reason: String::new() }
    );
    assert_eq!(
        chains.save_status(true),
        SaveStatus { state: SaveState::Unsaved, reason: String::new() }
    );

    // In flight, and then queued behind it: saving either way, flag or no flag.
    gated.pause();
    chains.on_vfs_write(path, b"edit 1").unwrap();
    assert_eq!(chains.save_status(false).state, SaveState::Saving);
    assert_eq!(chains.save_status(true).state, SaveState::Saving);

    chains.on_vfs_write(path, b"edit 2").unwrap();
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::Queued);
    assert_eq!(chains.save_status(false).state, SaveState::Saving);
    assert_eq!(chains.save_status(true).state, SaveState::Saving);

    // Both puts land: the flag is alone again.
    gated.release();
    while chains.has_active() {
        chains.poll_head(&mut context);
    }
    assert_eq!(chains.save_status(false).state, SaveState::Saved);
    assert_eq!(chains.save_status(true).state, SaveState::Unsaved);

    // A backend failure strands the newest bytes: failed, naming that path.
    gated.pause();
    gated.set_fail_next_operation(true);
    chains.on_vfs_write(path, b"edit 3").unwrap();
    gated.release();
    chains.poll_head(&mut context);
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::Stranded);
    assert_eq!(
        chains.save_status(false),
        SaveStatus {
            state: SaveState::Failed,
            reason: "stranded: scenes/main.ron; press Save to retry".to_string(),
        }
    );
    assert_eq!(chains.save_status(true).state, SaveState::Failed);
}


