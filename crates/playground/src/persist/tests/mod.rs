use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::pin::Pin;

use crate::projects::ProjectManifest;
use crate::store::memory::MemoryStore;
use crate::store::{Fut, ProjectStore, StoredFile, StoreError};

#[derive(Clone)]
pub(crate) struct GatedStore {
    inner: MemoryStore,
    paused: Arc<AtomicBool>,
    fail_next_put: Arc<AtomicBool>,
    completions: Arc<Mutex<Vec<Waker>>>,
}

impl GatedStore {
    pub(crate) fn new(inner: MemoryStore) -> Self {
        Self {
            inner,
            paused: Arc::new(AtomicBool::new(false)),
            fail_next_put: Arc::new(AtomicBool::new(false)),
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

    pub(crate) fn set_fail_next_put(&self, fail: bool) {
        self.fail_next_put.store(fail, Ordering::SeqCst);
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
        let inner_fut = self.inner.put(file, base_revision, manifest);
        let paused = self.paused.clone();
        let fail_on_completion = self.fail_next_put.swap(false, Ordering::SeqCst);
        let completions = self.completions.clone();

        Box::pin(GatedPut {
            inner_fut,
            paused,
            fail_on_completion,
            completions,
        })
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

struct GatedPut {
    inner_fut: Fut<Result<u64, StoreError>>,
    paused: Arc<AtomicBool>,
    fail_on_completion: bool,
    completions: Arc<Mutex<Vec<Waker>>>,
}

impl std::future::Future for GatedPut {
    type Output = Result<u64, StoreError>;

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
mod stores;
