use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::task::{Context, Waker};

use super::{test_manifest, GatedStore};
use crate::persist::{Chains, PathState};
use crate::store::memory::MemoryStore;
use crate::store::{ProjectStore, StoredFile, StoreError};

#[test]
fn test_take_started_puts_moves_the_future_and_keeps_the_state() {
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

    let mut started = chains.take_started_puts();
    assert_eq!(started.len(), 1);
    let (taken_path, future) = started.pop().unwrap();
    assert_eq!(taken_path, "scenes/main.ron");

    // Chain is still InFlight while future is taken
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::InFlight);

    let result = pollster::block_on(future);
    assert_eq!(result, Ok(1));
    chains.handle_put_completion(&taken_path, result);

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
    assert!(chains.poll_all(&mut context));
    // First put finished (rev 1), second put automatically started (InFlight)
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::InFlight);
    assert_eq!(chains.base_revision("scenes/main.ron"), 1);

    // 4. Poll again for second put completion
    assert!(chains.poll_all(&mut context));
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
    chains.poll_all(&mut context);

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
    let mut chains = Chains::new(
        "pong".to_string(),
        "/projects/pong".to_string(),
        "v1".to_string(),
        test_manifest("pong"),
        Arc::new(memory),
        None,
    );

    // Simulate stale revision error
    chains.chains.insert("a.txt".to_string(), crate::persist::PathChain::new("a.txt".to_string(), 0));
    chains.handle_put_completion("a.txt", Err(StoreError::StaleRevision { stored: 2, base: 0 }));
    assert_eq!(chains.path_state("a.txt"), PathState::Conflicted);

    // Reissue stranded must not touch conflicted path
    chains.reissue_stranded();
    assert_eq!(chains.path_state("a.txt"), PathState::Conflicted);
}

#[test]
fn test_drain_with_inflight_and_queued_completes_both_and_refuses_new_writes() {
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
    let path = std::path::Path::new("/projects/pong/scenes/main.ron");
    chains.on_vfs_write(path, b"v1").unwrap();
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::InFlight);

    chains.on_vfs_write(path, b"v2").unwrap();
    assert_eq!(chains.path_state("scenes/main.ron"), PathState::Queued);

    chains.start_drain();

    let new_path = std::path::Path::new("/projects/pong/scenes/other.ron");
    let write_error = chains.on_vfs_write(new_path, b"v3").unwrap_err();
    assert!(write_error.contains("project is being replaced"));
    assert_eq!(chains.path_state("scenes/other.ron"), PathState::Idle);

    gated.release();
    while chains.has_active() {
        chains.poll_all(&mut context);
    }

    assert_eq!(chains.path_state("scenes/main.ron"), PathState::Idle);
    assert_eq!(chains.base_revision("scenes/main.ron"), 2);

    pollster::block_on(async {
        let loaded = memory.load_project("pong").await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].bytes, b"v2");
        assert_eq!(loaded[0].revision, 2);
    });
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
fn test_fail_first_put_with_queued_starts_second_put_at_once() {
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
    gated.set_fail_next_put(true);

    let path = std::path::Path::new("/projects/pong/file.txt");
    chains.on_vfs_write(path, b"v1").unwrap();
    assert_eq!(chains.path_state("file.txt"), PathState::InFlight);
    assert_eq!(chains.base_revision("file.txt"), 0);

    chains.on_vfs_write(path, b"v2").unwrap();
    assert_eq!(chains.path_state("file.txt"), PathState::Queued);

    gated.release();
    // Poll to complete v1 (which fails) and trigger v2
    assert!(chains.poll_all(&mut context));

    // After completion, path is InFlight with v2 at same base 0
    assert_eq!(chains.path_state("file.txt"), PathState::InFlight);
    assert_eq!(chains.base_revision("file.txt"), 0);

    // Poll to complete v2 (which succeeds)
    assert!(chains.poll_all(&mut context));
    assert_eq!(chains.path_state("file.txt"), PathState::Idle);
    assert_eq!(chains.base_revision("file.txt"), 1);

    pollster::block_on(async {
        let loaded = memory.load_project("pong").await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].bytes, b"v2");
        assert_eq!(loaded[0].revision, 1);
    });
}

#[test]
fn test_stranded_reissued_with_newest_bytes_and_inflight_queued_untouched() {
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

    // Make stranded path: single put fails with Backend
    gated.pause();
    gated.set_fail_next_put(true);
    let stranded_path = std::path::Path::new("/projects/pong/stranded.txt");
    chains.on_vfs_write(stranded_path, b"stranded_bytes").unwrap();
    gated.release();
    chains.poll_all(&mut context);
    assert_eq!(chains.path_state("stranded.txt"), PathState::Stranded);

    // Make busy path: in-flight + queued
    gated.pause();
    let busy_path = std::path::Path::new("/projects/pong/busy.txt");
    chains.on_vfs_write(busy_path, b"busy_v1").unwrap();
    chains.on_vfs_write(busy_path, b"busy_v2").unwrap();
    assert_eq!(chains.path_state("busy.txt"), PathState::Queued);

    // Call reissue_stranded
    chains.reissue_stranded();

    // Stranded path became InFlight
    assert_eq!(chains.path_state("stranded.txt"), PathState::InFlight);
    // Busy path is still Queued, untouched
    assert_eq!(chains.path_state("busy.txt"), PathState::Queued);

    gated.release();
    while chains.has_active() {
        chains.poll_all(&mut context);
    }

    assert_eq!(chains.path_state("stranded.txt"), PathState::Idle);
    assert_eq!(chains.path_state("busy.txt"), PathState::Idle);

    pollster::block_on(async {
        let loaded = memory.load_project("pong").await.unwrap();
        let stranded_file = loaded.iter().find(|file| file.path == "stranded.txt").unwrap();
        assert_eq!(stranded_file.bytes, b"stranded_bytes");
        assert_eq!(stranded_file.revision, 1);

        let busy_file = loaded.iter().find(|file| file.path == "busy.txt").unwrap();
        assert_eq!(busy_file.bytes, b"busy_v2");
        assert_eq!(busy_file.revision, 2);
    });
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
    chains.poll_all(&mut context);

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
