//! In-memory project store implementation.
//!
//! Used as a fallback on the web when IndexedDB fails to open (private browsing,
//! sandboxed iframe) and as a test double across all targets.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::projects::{ProjectManifest, ProjectOrigin};
use crate::store::{replacement_revision, Fut, ManifestList, ProjectStore, StoredFile, StoreError};

#[derive(Default)]
struct MemoryState {
    files: HashMap<(String, String), StoredFile>,
    manifests: HashMap<String, ProjectManifest>,
    #[cfg(test)]
    fail_after: Option<usize>,
    #[cfg(test)]
    read_faults: ReadFaults,
}

/// The failures a test injects, each standing for what the browser's store can hand
/// back: a manifest read that fails whole, a manifest record or a file record that is
/// stored but will not decode, a sweep whose transaction fails. A marked record keeps
/// its value, so clearing the faults shows whether anything touched it.
#[cfg(test)]
#[derive(Default)]
struct ReadFaults {
    manifest_read_fails: bool,
    sweep_fails: bool,
    undecodable_manifests: std::collections::HashSet<String>,
    undecodable_files: std::collections::HashSet<(String, String)>,
}

#[cfg(test)]
impl MemoryState {
    fn manifest_read_fails(&self) -> bool {
        self.read_faults.manifest_read_fails
    }

    fn sweep_fails(&self) -> bool {
        self.read_faults.sweep_fails
    }

    fn manifest_is_undecodable(&self, slug: &str) -> bool {
        self.read_faults.undecodable_manifests.contains(slug)
    }

    fn file_is_undecodable(&self, slug: &str, path: &str) -> bool {
        self.read_faults.undecodable_files.contains(&(slug.to_string(), path.to_string()))
    }

    fn project_has_undecodable_file(&self, slug: &str) -> bool {
        self.read_faults.undecodable_files.iter().any(|(project_slug, _)| project_slug == slug)
    }
}

#[cfg(not(test))]
impl MemoryState {
    fn manifest_read_fails(&self) -> bool {
        false
    }

    fn sweep_fails(&self) -> bool {
        false
    }

    fn manifest_is_undecodable(&self, _slug: &str) -> bool {
        false
    }

    fn file_is_undecodable(&self, _slug: &str, _path: &str) -> bool {
        false
    }

    fn project_has_undecodable_file(&self, _slug: &str) -> bool {
        false
    }
}

fn undecodable_record(slug: &str, path: &str) -> StoreError {
    StoreError::Backend(format!("the stored record {slug}/{path} cannot be decoded"))
}

/// Thread-safe in-memory project store.
#[derive(Clone, Default)]
pub struct MemoryStore {
    state: Arc<Mutex<MemoryState>>,
}

impl MemoryStore {
    /// Create a new empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fail one mutation after `successful_mutations` more successful attempts.
    #[cfg(test)]
    pub(crate) fn fail_after(&self, successful_mutations: usize) {
        self.state.lock().expect("memory store lock").fail_after = Some(successful_mutations);
    }

    /// Every manifest read fails until the faults are cleared.
    #[cfg(test)]
    pub(crate) fn fail_manifest_reads(&self) {
        self.state.lock().expect("memory store lock").read_faults.manifest_read_fails = true;
    }

    /// The stored manifest of `slug` reads as a record that will not decode.
    #[cfg(test)]
    pub(crate) fn mark_manifest_undecodable(&self, slug: &str) {
        let mut guard = self.state.lock().expect("memory store lock");
        assert!(guard.manifests.contains_key(slug), "only a stored manifest can be marked");
        guard.read_faults.undecodable_manifests.insert(slug.to_string());
    }

    /// The stored record at `slug`/`path` reads as a record that will not decode.
    #[cfg(test)]
    pub(crate) fn mark_file_undecodable(&self, slug: &str, path: &str) {
        let mut guard = self.state.lock().expect("memory store lock");
        let key = (slug.to_string(), path.to_string());
        assert!(guard.files.contains_key(&key), "only a stored record can be marked");
        guard.read_faults.undecodable_files.insert(key);
    }

    /// Every sweep fails, removing nothing, until the faults are cleared.
    #[cfg(test)]
    pub(crate) fn fail_sweeps(&self) {
        self.state.lock().expect("memory store lock").read_faults.sweep_fails = true;
    }

    /// The manifest of `slug` is gone and its files stay: an import an older release
    /// interrupted between the two, which the boot reads as an interrupted import.
    #[cfg(test)]
    pub(crate) fn lose_manifest(&self, slug: &str) {
        let removed = self.state.lock().expect("memory store lock").manifests.remove(slug);
        assert!(removed.is_some(), "only a stored manifest can be lost");
    }

    /// Every injected read fault is lifted; the marked records read as they were stored.
    #[cfg(test)]
    pub(crate) fn clear_read_faults(&self) {
        self.state.lock().expect("memory store lock").read_faults = ReadFaults::default();
    }
}

impl ProjectStore for MemoryStore {
    fn load_project(&self, slug: &str) -> Fut<Result<Vec<StoredFile>, StoreError>> {
        let state = self.state.clone();
        let slug = slug.to_string();
        Box::pin(async move {
            let guard = state.lock().map_err(|error| StoreError::Backend(error.to_string()))?;
            let mut result = Vec::new();
            for ((project_slug, path), file) in &guard.files {
                if project_slug != &slug {
                    continue;
                }
                if guard.file_is_undecodable(project_slug, path) {
                    return Err(undecodable_record(project_slug, path));
                }
                result.push(file.clone());
            }
            result.sort_by(|first, second| first.path.cmp(&second.path));
            Ok(result)
        })
    }

    fn put(
        &self,
        file: StoredFile,
        base_revision: u64,
        manifest: &ProjectManifest,
    ) -> Fut<Result<u64, StoreError>> {
        let state = self.state.clone();
        let manifest = manifest.clone();
        Box::pin(async move {
            let mut guard = state.lock().map_err(|error| StoreError::Backend(error.to_string()))?;
            #[cfg(test)]
            if let Some(remaining) = guard.fail_after.as_mut() {
                if *remaining == 0 {
                    guard.fail_after = None;
                    return Err(StoreError::Backend("injected mutation failure".to_string()));
                }
                *remaining -= 1;
            }
            let key = (file.project.clone(), file.path.clone());
            if guard.file_is_undecodable(&file.project, &file.path) {
                return Err(undecodable_record(&file.project, &file.path));
            }
            if let Some(existing) = guard.files.get(&key) {
                if existing.revision != base_revision {
                    return Err(StoreError::StaleRevision {
                        stored: existing.revision,
                        base: base_revision,
                    });
                }
            } else if base_revision != 0 {
                return Err(StoreError::StaleRevision {
                    stored: 0,
                    base: base_revision,
                });
            }

            let new_revision = base_revision + 1;
            if !guard.manifests.contains_key(&file.project) {
                let mut upserted_manifest = manifest;
                upserted_manifest.origin = ProjectOrigin::Saved;
                guard.manifests.insert(file.project.clone(), upserted_manifest);
            }

            let mut stored = file;
            stored.revision = new_revision;
            guard.files.insert(key, stored);
            Ok(new_revision)
        })
    }

    fn remove_file(
        &self,
        slug: &str,
        path: &str,
        base_revision: u64,
        manifest: &ProjectManifest,
    ) -> Fut<Result<u64, StoreError>> {
        self.put(
            StoredFile {
                project: slug.to_string(),
                path: path.to_string(),
                bytes: Vec::new(),
                revision: 0,
                bundle_version: manifest.bundle_version.clone(),
                deleted: true,
            },
            base_revision,
            manifest,
        )
    }

    fn replace_project(
        &self,
        slug: &str,
        files: Vec<StoredFile>,
        manifest: ProjectManifest,
    ) -> Fut<Result<(), StoreError>> {
        let state = self.state.clone();
        let slug = slug.to_string();
        Box::pin(async move {
            let mut guard = state.lock().map_err(|error| StoreError::Backend(error.to_string()))?;
            if guard.project_has_undecodable_file(&slug) {
                return Err(StoreError::Backend(format!(
                    "a stored record of {slug} cannot be decoded, so its highest revision is unknown"
                )));
            }
            let revision = replacement_revision(
                guard
                    .files
                    .iter()
                    .filter(|((project_slug, _), _)| project_slug == &slug)
                    .map(|(_, file)| file.revision),
            );
            guard.files.retain(|(project_slug, _), _| project_slug != &slug);
            for mut file in files {
                file.project = slug.clone();
                file.revision = revision;
                #[cfg(test)]
                if let Some(remaining) = guard.fail_after.as_mut() {
                    if *remaining == 0 {
                        guard.fail_after = None;
                        return Err(StoreError::Backend("injected mutation failure".to_string()));
                    }
                    *remaining -= 1;
                }
                let key = (file.project.clone(), file.path.clone());
                guard.files.insert(key, file);
            }
            guard.manifests.insert(slug, manifest);
            Ok(())
        })
    }

    fn remove_project(&self, slug: &str) -> Fut<Result<(), StoreError>> {
        let state = self.state.clone();
        let slug = slug.to_string();
        Box::pin(async move {
            let mut guard = state.lock().map_err(|error| StoreError::Backend(error.to_string()))?;
            guard.files.retain(|(project_slug, _), _| project_slug != &slug);
            guard.manifests.remove(&slug);
            Ok(())
        })
    }

    fn manifests(&self) -> Fut<Result<ManifestList, StoreError>> {
        let state = self.state.clone();
        Box::pin(async move {
            let guard = state.lock().map_err(|error| StoreError::Backend(error.to_string()))?;
            if guard.manifest_read_fails() {
                return Err(StoreError::Backend("injected manifest read failure".to_string()));
            }
            let mut list = ManifestList::default();
            for (slug, manifest) in &guard.manifests {
                if guard.manifest_is_undecodable(slug) {
                    list.undecodable.push(slug.clone());
                } else {
                    list.manifests.push(manifest.clone());
                }
            }
            list.manifests.sort_by(|first, second| first.slug.cmp(&second.slug));
            list.undecodable.sort();
            Ok(list)
        })
    }

    fn sweep_orphans(&self, known: &[String]) -> Fut<Result<(), StoreError>> {
        let state = self.state.clone();
        let known = known.to_vec();
        Box::pin(async move {
            let mut guard = state.lock().map_err(|error| StoreError::Backend(error.to_string()))?;
            if guard.sweep_fails() {
                return Err(StoreError::Backend("injected sweep failure".to_string()));
            }
            let projects: std::collections::HashSet<String> = guard
                .files
                .keys()
                .map(|(project_slug, _)| project_slug.clone())
                .collect();
            for project_slug in &projects {
                if !guard.manifests.contains_key(project_slug) && !known.contains(project_slug) {
                    guard.files.retain(|(stored_slug, _), _| stored_slug != project_slug);
                }
            }
            Ok(())
        })
    }
}
