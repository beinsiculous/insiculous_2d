use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::pin::Pin;

use crate::persist::Chains;
use crate::projects::ProjectManifest;
use crate::store::memory::MemoryStore;
use crate::store::{Fut, ProjectStore, StoredFile, StoreError};

#[derive(Clone)]
pub(crate) struct GatedStore {
    inner: MemoryStore,
    paused: Arc<AtomicBool>,
    fail_next_operation: Arc<AtomicBool>,
    completions: Arc<Mutex<Vec<Waker>>>,
}

impl GatedStore {
    pub(crate) fn new(inner: MemoryStore) -> Self {
        Self {
            inner,
            paused: Arc::new(AtomicBool::new(false)),
            fail_next_operation: Arc::new(AtomicBool::new(false)),
            completions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(crate) fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub(crate) fn release(&self) {
        self.paused.store(false, Ordering::SeqCst);
        let mut guard = self.completions.lock().unwrap();
        for waker in guard.drain(..) {
            waker.wake();
        }
    }

    /// The next put or removal issued fails with a backend error once released.
    pub(crate) fn set_fail_next_operation(&self, fail: bool) {
        self.fail_next_operation.store(fail, Ordering::SeqCst);
    }

    fn gate<T: 'static>(&self, inner_fut: Fut<Result<T, StoreError>>) -> Fut<Result<T, StoreError>> {
        Box::pin(Gated {
            inner_fut,
            paused: self.paused.clone(),
            fail_on_completion: self.fail_next_operation.swap(false, Ordering::SeqCst),
            completions: self.completions.clone(),
        })
    }
}

impl ProjectStore for GatedStore {
    fn load_project(&self, slug: &str) -> Fut<Result<Vec<StoredFile>, StoreError>> {
        self.inner.load_project(slug)
    }

    fn put(
        &self,
        file: StoredFile,
        base_revision: u64,
        manifest: &ProjectManifest,
    ) -> Fut<Result<u64, StoreError>> {
        self.gate(self.inner.put(file, base_revision, manifest))
    }

    fn remove_file(&self, slug: &str, path: &str, base_revision: u64) -> Fut<Result<(), StoreError>> {
        self.gate(self.inner.remove_file(slug, path, base_revision))
    }

    fn replace_project(
        &self,
        slug: &str,
        files: Vec<StoredFile>,
        manifest: ProjectManifest,
    ) -> Fut<Result<(), StoreError>> {
        self.inner.replace_project(slug, files, manifest)
    }

    fn remove_project(&self, slug: &str) -> Fut<Result<(), StoreError>> {
        self.inner.remove_project(slug)
    }

    fn manifests(&self) -> Fut<Vec<ProjectManifest>> {
        self.inner.manifests()
    }

    fn sweep_orphans(&self, bundled_slugs: &[String]) -> Fut<Result<(), StoreError>> {
        self.inner.sweep_orphans(bundled_slugs)
    }
}

struct Gated<T> {
    inner_fut: Fut<Result<T, StoreError>>,
    paused: Arc<AtomicBool>,
    fail_on_completion: bool,
    completions: Arc<Mutex<Vec<Waker>>>,
}

impl<T> std::future::Future for Gated<T> {
    type Output = Result<T, StoreError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.paused.load(Ordering::SeqCst) {
            self.completions.lock().unwrap().push(cx.waker().clone());
            Poll::Pending
        } else if self.fail_on_completion {
            Poll::Ready(Err(StoreError::Backend("simulated backend failure".to_string())))
        } else {
            self.inner_fut.as_mut().poll(cx)
        }
    }
}

pub(crate) fn test_manifest(slug: &str) -> ProjectManifest {
    ProjectManifest {
        slug: slug.to_string(),
        title: slug.to_uppercase(),
        bundle_version: "v1".to_string(),
        content_hash: "test_hash".to_string(),
        origin: crate::projects::ProjectOrigin::Bundled,
    }
}

/// The project root every helper below writes under.
pub(crate) const PONG_ROOT: &str = "/projects/pong";

/// Chains for the "pong" project over `store`, rooted at [`PONG_ROOT`].
pub(crate) fn pong_chains(store: Arc<dyn ProjectStore>) -> Chains {
    Chains::new(
        "pong".to_string(),
        PONG_ROOT.to_string(),
        "v1".to_string(),
        test_manifest("pong"),
        store,
        None,
    )
}

/// The VFS write of `bytes` at `relative_path` under the project, as the observer sees it.
pub(crate) fn write_project_file(chains: &mut Chains, relative_path: &str, bytes: &[u8]) {
    let path = std::path::Path::new(PONG_ROOT).join(relative_path);
    chains.on_vfs_write(&path, bytes).unwrap();
}

/// The VFS removal of `relative_path` under the project, as the observer sees it.
pub(crate) fn remove_project_file(chains: &mut Chains, relative_path: &str) {
    chains.on_vfs_remove(&std::path::Path::new(PONG_ROOT).join(relative_path));
}

/// Poll the head until nothing is in flight: every queued operation has landed, or one
/// holds the queue.
pub(crate) fn settle(chains: &mut Chains) {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    while chains.has_active() {
        chains.poll_head(&mut context);
    }
}

/// The record stored at `relative_path` in "pong", tombstone or not.
pub(crate) fn stored_file(store: &dyn ProjectStore, relative_path: &str) -> Option<StoredFile> {
    pollster::block_on(store.load_project("pong"))
        .unwrap()
        .into_iter()
        .find(|file| file.path == relative_path)
}

/// The live bytes at `relative_path` in "pong": `None` when absent or a tombstone.
pub(crate) fn stored_bytes(store: &dyn ProjectStore, relative_path: &str) -> Option<Vec<u8>> {
    stored_file(store, relative_path)
        .filter(|file| !file.deleted)
        .map(|file| file.bytes)
}

/// Another tab's save of `relative_path` at `revision`, written straight to the store.
pub(crate) fn store_another_tabs_file(store: &MemoryStore, relative_path: &str, revision: u64) {
    let file = StoredFile {
        project: "pong".to_string(),
        path: relative_path.to_string(),
        bytes: b"other tab".to_vec(),
        revision,
        bundle_version: "v1".to_string(),
        deleted: false,
    };
    pollster::block_on(store.replace_project("pong", vec![file], test_manifest("pong"))).unwrap();
}

pub(crate) struct TestTempDir(std::path::PathBuf);
impl TestTempDir {
    pub(crate) fn new(prefix: &str) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let identifier = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "insiculous_{prefix}_{}_{}",
            std::process::id(),
            identifier
        ));
        let _ = std::fs::create_dir_all(&path);
        Self(path)
    }
    pub(crate) fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for TestTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn temp_directory_store() -> (TestTempDir, crate::store::directory::DirectoryStore) {
    let directory = TestTempDir::new("dir_store_test");
    let store = crate::store::directory::DirectoryStore::new(directory.path().to_path_buf()).unwrap();
    (directory, store)
}

mod chains;
mod queue;
mod song_saves;
mod stores;
