//! Project persistence abstraction for the Web Playground.
//!
//! Provides the [`ProjectStore`] trait and data types implemented by
//! [`DirectoryStore`] (natively, with project-level locking for CAS tests),
//! [`MemoryStore`] (fallback when browser storage fails to open), and
//! [`IndexedDbStore`] (on wasm).

use std::future::Future;
use std::pin::Pin;
use serde::{Deserialize, Serialize};

use crate::projects::ProjectManifest;

/// Pinned boxed future alias to avoid async-trait macros.
pub type Fut<T> = Pin<Box<dyn Future<Output = T>>>;

/// A stored file within a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredFile {
    /// Project identifier slug.
    pub project: String,
    /// Path relative to the project root, e.g. `assets/scenes/behavior_demo.scene.ron`.
    pub path: String,
    /// Raw file contents.
    pub bytes: Vec<u8>,
    /// Monotonically increasing revision number, starting at 1.
    pub revision: u64,
    /// Bundle version string under which this file was written.
    pub bundle_version: String,
    /// A removed file is kept as a tombstone — empty bytes, one revision up — so a put
    /// that recreates it still names a revision, and a stale tab's removal stays stale.
    #[serde(default)]
    pub deleted: bool,
}

/// Errors returned by persistence operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// Storage backend is unavailable (e.g. storage quota exceeded or disabled).
    Unavailable,
    /// Compare-and-swap conflict: stored revision does not match the tab's base revision.
    StaleRevision { stored: u64, base: u64 },
    /// Storage backend reported a failure.
    Backend(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => write!(f, "storage backend unavailable"),
            Self::StaleRevision { stored, base } => {
                write!(f, "stale revision: stored {stored} != base {base}")
            }
            Self::Backend(err) => write!(f, "storage backend error: {err}"),
        }
    }
}

impl std::error::Error for StoreError {}

/// Contract for playground project storage.
pub trait ProjectStore {
    /// Load all stored files belonging to the given project slug, tombstones included,
    /// so the chains learn every revision the store holds.
    fn load_project(&self, slug: &str) -> Fut<Result<Vec<StoredFile>, StoreError>>;

    /// Compare-and-swap write of a single file.
    ///
    /// Refuses with [`StoreError::StaleRevision`] unless the stored revision
    /// equals `base_revision` (or 0 if absent). On success, increments and
    /// returns the new revision. Upserts `manifest` if none is stored yet.
    fn put(
        &self,
        file: StoredFile,
        base_revision: u64,
        manifest: &ProjectManifest,
    ) -> Fut<Result<u64, StoreError>>;

    /// Compare-and-swap removal of a single file, written as a tombstone.
    ///
    /// Refuses with [`StoreError::StaleRevision`] unless the stored revision equals
    /// `base_revision` (zero for an absent record). Writes empty bytes marked deleted
    /// at the returned revision `base_revision + 1`, even for a bundled-only file.
    /// Upserts `manifest` in the same transaction, exactly as `put` does.
    fn remove_file(
        &self,
        slug: &str,
        path: &str,
        base_revision: u64,
        manifest: &ProjectManifest,
    ) -> Fut<Result<u64, StoreError>>;

    /// Atomically replace all files and manifest for a project.
    fn replace_project(
        &self,
        slug: &str,
        files: Vec<StoredFile>,
        manifest: ProjectManifest,
    ) -> Fut<Result<(), StoreError>>;

    /// Atomically remove all files and manifest for a project.
    fn remove_project(&self, slug: &str) -> Fut<Result<(), StoreError>>;

    /// Return manifests for all stored projects.
    fn manifests(&self) -> Fut<Vec<ProjectManifest>>;

    /// Remove stored files whose project slug has no manifest and is not in `bundled`.
    fn sweep_orphans(&self, bundled: &[String]) -> Fut<Result<(), StoreError>>;
}

/// Select stored records for boot, including tombstones. Manifest-less records belong
/// to an interrupted import and must be removed instead of shadowing the bundle.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn stored_files_for_boot(
    stored_files: Vec<StoredFile>,
    has_stored_manifest: bool,
) -> Option<Vec<StoredFile>> {
    if !stored_files.is_empty() && !has_stored_manifest {
        None
    } else {
        Some(stored_files)
    }
}

pub mod directory;
pub mod memory;

#[cfg(target_arch = "wasm32")]
pub mod idb_transaction;
#[cfg(target_arch = "wasm32")]
pub mod indexed_db;

#[cfg(test)]
mod tests;
