//! Reloads at the observer and durable queue boundaries of asset mutations.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::task::{Context, Waker};

use common::vfs::MemFs;

use super::{pong_chains, PONG_ROOT};
use crate::persist::Chains;
use crate::store::memory::MemoryStore;
use crate::store::ProjectStore;

fn path(relative: &str) -> PathBuf {
    Path::new(PONG_ROOT).join(relative)
}

fn bundled_files() -> MemFs {
    let mut files = MemFs::default();
    files.insert(path("assets/scripts/group/first.rhai").to_string_lossy().into_owned(), b"first".to_vec());
    files.insert(path("assets/scripts/group/second.rhai").to_string_lossy().into_owned(), b"second".to_vec());
    files
}

fn reload(memory: &MemoryStore) -> MemFs {
    let records = pollster::block_on(memory.load_project("pong")).unwrap();
    let has_manifest = pollster::block_on(memory.manifests()).unwrap().manifests.iter().any(|manifest| manifest.slug == "pong");
    let selected = crate::store::stored_files_for_boot(records, has_manifest)
        .expect("a successful mutation always stores the manifest with its record");
    let mut files = bundled_files();
    for record in selected {
        let file_path = path(&record.path);
        if record.deleted {
            files.remove_prefix(&file_path);
        } else {
            files.insert(file_path.to_string_lossy().into_owned(), record.bytes);
        }
    }
    files
}

fn poll_once(chains: &RefCell<Chains>) -> bool {
    chains.borrow_mut().poll_head(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn bundled_only_delete_through_the_observer_stays_deleted_after_reload() {
    let memory = MemoryStore::new();
    let chains = RefCell::new(pong_chains(Arc::new(memory.clone())));
    let files = RefCell::new(bundled_files());
    let removed = path("assets/scripts/group/first.rhai");
    MemFs::remove_file_observed(&files, &removed, |removed_path| {
        chains.borrow_mut().on_vfs_remove(removed_path);
    }).unwrap();
    assert!(poll_once(&chains));
    assert!(!chains.borrow().is_pending());
    assert!(reload(&memory).read(&removed).is_err());
    assert_eq!(reload(&memory).read(&path("assets/scripts/group/second.rhai")).unwrap(), b"second");
    let tombstone = pollster::block_on(memory.load_project("pong")).unwrap().remove(0);
    assert!(tombstone.deleted);
    assert_eq!(tombstone.revision, 1);
}

#[test]
fn rename_failure_preserves_duplicates_until_retry_finishes_every_removal() {
    for successful_puts in [1, 2] {
        let memory = MemoryStore::new();
        memory.fail_after(successful_puts);
        let chains = RefCell::new(pong_chains(Arc::new(memory.clone())));
        let files = RefCell::new(bundled_files());
        MemFs::rename_observed(
            &files,
            &path("assets/scripts/group"),
            &path("assets/scripts/renamed"),
            |written_path| {
                let bytes = files.borrow().read(written_path).unwrap();
                chains.borrow_mut().on_vfs_write(written_path, &bytes).unwrap();
            },
            |removed_path| chains.borrow_mut().on_vfs_remove(removed_path),
        ).unwrap();

        for completed in 0..successful_puts {
            assert!(poll_once(&chains));
            let reopened = reload(&memory);
            assert_eq!(reopened.read(&path("assets/scripts/group/first.rhai")).unwrap(), b"first");
            assert_eq!(reopened.read(&path("assets/scripts/group/second.rhai")).unwrap(), b"second");
            assert_eq!(reopened.read(&path("assets/scripts/renamed/first.rhai")).unwrap(), b"first");
            if completed == 1 {
                assert_eq!(reopened.read(&path("assets/scripts/renamed/second.rhai")).unwrap(), b"second");
            }
        }
        assert!(poll_once(&chains), "the next operation fails at the chosen boundary");
        assert!(!chains.borrow().has_active());
        assert_eq!(chains.borrow().stranded_paths().len(), 1);
        let reopened = reload(&memory);
        assert_eq!(reopened.read(&path("assets/scripts/group/first.rhai")).unwrap(), b"first");
        assert_eq!(reopened.read(&path("assets/scripts/group/second.rhai")).unwrap(), b"second");
        assert_eq!(reopened.read(&path("assets/scripts/renamed/first.rhai")).unwrap(), b"first");

        chains.borrow_mut().reissue_stranded();
        for _ in 0..4 {
            if !chains.borrow().has_active() { break; }
            assert!(poll_once(&chains));
            let reopened = reload(&memory);
            assert_eq!(reopened.read(&path("assets/scripts/renamed/first.rhai")).unwrap(), b"first");
            assert_eq!(reopened.read(&path("assets/scripts/renamed/second.rhai")).unwrap(), b"second");
        }
        assert!(!chains.borrow().is_pending());
        let reopened = reload(&memory);
        assert!(reopened.read(&path("assets/scripts/group/first.rhai")).is_err());
        assert!(reopened.read(&path("assets/scripts/group/second.rhai")).is_err());
    }
}
