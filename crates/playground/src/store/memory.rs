//! In-memory project store implementation.
//!
//! Used as a fallback on the web when IndexedDB fails to open (private browsing,
//! sandboxed iframe) and as a test double across all targets.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::projects::{ProjectManifest, ProjectOrigin};
use crate::store::{Fut, ProjectStore, StoredFile, StoreError};

#[derive(Default)]
struct MemoryState {
    files: HashMap<(String, String), StoredFile>,
    manifests: HashMap<String, ProjectManifest>,
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
}

impl ProjectStore for MemoryStore {
    fn load_project(&self, slug: &str) -> Fut<Result<Vec<StoredFile>, StoreError>> {
        let state = self.state.clone();
        let slug = slug.to_string();
        Box::pin(async move {
            let guard = state.lock().map_err(|error| StoreError::Backend(error.to_string()))?;
            let mut result: Vec<StoredFile> = guard
                .files
                .iter()
                .filter(|((project_slug, _), _)| project_slug == &slug)
                .map(|(_, file)| file.clone())
                .collect();
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
            let key = (file.project.clone(), file.path.clone());
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
            guard.files.retain(|(project_slug, _), _| project_slug != &slug);
            for mut file in files {
                file.project = slug.clone();
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

    fn manifests(&self) -> Fut<Vec<ProjectManifest>> {
        let state = self.state.clone();
        Box::pin(async move {
            let Ok(guard) = state.lock() else {
                return Vec::new();
            };
            let mut manifests_list: Vec<ProjectManifest> = guard.manifests.values().cloned().collect();
            manifests_list.sort_by(|first, second| first.slug.cmp(&second.slug));
            manifests_list
        })
    }

    fn sweep_orphans(&self, bundled_slugs: &[String]) -> Fut<Result<(), StoreError>> {
        let state = self.state.clone();
        let bundled_slugs = bundled_slugs.to_vec();
        Box::pin(async move {
            let mut guard = state.lock().map_err(|error| StoreError::Backend(error.to_string()))?;
            let projects: std::collections::HashSet<String> = guard
                .files
                .keys()
                .map(|(project_slug, _)| project_slug.clone())
                .collect();
            for project_slug in &projects {
                if !guard.manifests.contains_key(project_slug) && !bundled_slugs.contains(project_slug) {
                    guard.files.retain(|(proj, _), _| proj != project_slug);
                }
            }
            Ok(())
        })
    }
}
