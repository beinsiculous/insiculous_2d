//! Filesystem-backed project store implementation for native targets.
//!
//! Serves as the native test double for IndexedDB: each project uses a `.lock`
//! file to ensure compare-and-swap (CAS) operations are honest and race-free.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::projects::{ProjectManifest, ProjectOrigin};
use crate::store::{Fut, ProjectStore, StoredFile, StoreError};

/// Filesystem-backed project store rooted at `root`.
#[derive(Clone)]
pub struct DirectoryStore {
    root: PathBuf,
}

impl DirectoryStore {
    /// Create a new directory store at `root`, creating the directory if missing.
    pub fn new(root: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub(crate) fn project_directory(&self, slug: &str) -> PathBuf {
        self.root.join(slug)
    }

    pub(crate) fn manifest_path(&self, slug: &str) -> PathBuf {
        self.project_directory(slug).join("manifest.json")
    }

    fn file_path(&self, slug: &str, relative_path: &str) -> PathBuf {
        // Flat file naming: replace '/' with '__' and append '.json'
        let sanitized = relative_path.replace('/', "__");
        self.project_directory(slug).join("files").join(format!("{sanitized}.json"))
    }
}

struct ProjectLock {
    lock_file: PathBuf,
}

impl ProjectLock {
    fn acquire(directory: &Path) -> Result<Self, StoreError> {
        fs::create_dir_all(directory).map_err(|error| StoreError::Backend(error.to_string()))?;
        let lock_file = directory.join(".lock");
        for _ in 0..500 {
            match fs::OpenOptions::new().write(true).create_new(true).open(&lock_file) {
                Ok(_) => return Ok(Self { lock_file }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(error) => return Err(StoreError::Backend(error.to_string())),
            }
        }
        Err(StoreError::Backend("timed out acquiring project lock".to_string()))
    }
}

impl Drop for ProjectLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_file);
    }
}

impl ProjectStore for DirectoryStore {
    fn load_project(&self, slug: &str) -> Fut<Result<Vec<StoredFile>, StoreError>> {
        let files_directory = self.project_directory(slug).join("files");
        Box::pin(async move {
            if !files_directory.exists() {
                return Ok(Vec::new());
            }
            let read_directory = fs::read_dir(&files_directory).map_err(|error| StoreError::Backend(error.to_string()))?;
            let mut stored_files = Vec::new();
            for entry in read_directory.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
                if file_name.ends_with(".new") || file_name.ends_with(".old") {
                    continue;
                }
                if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
                    let content = fs::read_to_string(&path)
                        .map_err(|error| StoreError::Backend(error.to_string()))?;
                    let file: StoredFile = serde_json::from_str(&content)
                        .map_err(|error| StoreError::Backend(error.to_string()))?;
                    stored_files.push(file);
                }
            }
            stored_files.sort_by(|first, second| first.path.cmp(&second.path));
            Ok(stored_files)
        })
    }

    fn put(
        &self,
        file: StoredFile,
        base_revision: u64,
        manifest: &ProjectManifest,
    ) -> Fut<Result<u64, StoreError>> {
        let project_directory = self.project_directory(&file.project);
        let manifest_path = self.manifest_path(&file.project);
        let file_path = self.file_path(&file.project, &file.path);
        let manifest = manifest.clone();
        Box::pin(async move {
            let _lock = ProjectLock::acquire(&project_directory)?;

            if file_path.exists() {
                let content = fs::read_to_string(&file_path)
                    .map_err(|error| StoreError::Backend(error.to_string()))?;
                let existing: StoredFile = serde_json::from_str(&content)
                    .map_err(|error| StoreError::Backend(error.to_string()))?;
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
            if !manifest_path.exists() {
                let mut upserted_manifest = manifest;
                upserted_manifest.origin = ProjectOrigin::Saved;
                let json_content = serde_json::to_string_pretty(&upserted_manifest)
                    .map_err(|error| StoreError::Backend(error.to_string()))?;
                fs::write(&manifest_path, json_content).map_err(|error| StoreError::Backend(error.to_string()))?;
            }

            let mut stored = file;
            stored.revision = new_revision;
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).map_err(|error| StoreError::Backend(error.to_string()))?;
            }
            let json_content = serde_json::to_string_pretty(&stored)
                .map_err(|error| StoreError::Backend(error.to_string()))?;
            fs::write(&file_path, json_content).map_err(|error| StoreError::Backend(error.to_string()))?;
            Ok(new_revision)
        })
    }

    fn replace_project(
        &self,
        slug: &str,
        files: Vec<StoredFile>,
        manifest: ProjectManifest,
    ) -> Fut<Result<(), StoreError>> {
        let project_directory = self.project_directory(slug);
        let files_directory = project_directory.join("files");
        let files_new_directory = project_directory.join("files.new");
        let files_old_directory = project_directory.join("files.old");
        let manifest_path = self.manifest_path(slug);
        let manifest_new_path = project_directory.join("manifest.json.new");
        let slug_string = slug.to_string();
        Box::pin(async move {
            let _lock = ProjectLock::acquire(&project_directory)?;

            if files_new_directory.exists() {
                fs::remove_dir_all(&files_new_directory).map_err(|error| StoreError::Backend(error.to_string()))?;
            }
            if files_old_directory.exists() {
                fs::remove_dir_all(&files_old_directory).map_err(|error| StoreError::Backend(error.to_string()))?;
            }
            if manifest_new_path.exists() {
                fs::remove_file(&manifest_new_path).map_err(|error| StoreError::Backend(error.to_string()))?;
            }

            fs::create_dir_all(&files_new_directory).map_err(|error| StoreError::Backend(error.to_string()))?;
            for mut file in files {
                file.project = slug_string.clone();
                let sanitized = file.path.replace('/', "__");
                let file_path = files_new_directory.join(format!("{sanitized}.json"));
                let json_content = serde_json::to_string_pretty(&file)
                    .map_err(|error| StoreError::Backend(error.to_string()))?;
                fs::write(file_path, json_content).map_err(|error| StoreError::Backend(error.to_string()))?;
            }

            let manifest_json = serde_json::to_string_pretty(&manifest)
                .map_err(|error| StoreError::Backend(error.to_string()))?;
            fs::write(&manifest_new_path, manifest_json)
                .map_err(|error| StoreError::Backend(error.to_string()))?;

            // Atomic stage swap: rename current files -> files.old, files.new -> files, manifest.json.new -> manifest.json
            if files_directory.exists() {
                fs::rename(&files_directory, &files_old_directory)
                    .map_err(|error| StoreError::Backend(error.to_string()))?;
            }
            fs::rename(&files_new_directory, &files_directory)
                .map_err(|error| StoreError::Backend(error.to_string()))?;
            fs::rename(&manifest_new_path, &manifest_path)
                .map_err(|error| StoreError::Backend(error.to_string()))?;

            if files_old_directory.exists() {
                let _ = fs::remove_dir_all(&files_old_directory);
            }

            Ok(())
        })
    }

    fn remove_project(&self, slug: &str) -> Fut<Result<(), StoreError>> {
        let project_directory = self.project_directory(slug);
        Box::pin(async move {
            if project_directory.exists() {
                let _lock = ProjectLock::acquire(&project_directory)?;
                fs::remove_dir_all(&project_directory).map_err(|error| StoreError::Backend(error.to_string()))?;
            }
            Ok(())
        })
    }

    fn manifests(&self) -> Fut<Vec<ProjectManifest>> {
        let root = self.root.clone();
        Box::pin(async move {
            let Ok(read_directory) = fs::read_dir(&root) else {
                return Vec::new();
            };
            let mut manifests_list = Vec::new();
            for entry in read_directory.flatten() {
                let path = entry.path();
                let directory_name = entry.file_name().to_string_lossy().into_owned();
                if directory_name.ends_with(".new") || directory_name.ends_with(".old") || directory_name.starts_with('.') {
                    continue;
                }
                if path.is_dir() {
                    let manifest_file = path.join("manifest.json");
                    if manifest_file.exists() {
                        if let Ok(content) = fs::read_to_string(&manifest_file) {
                            if let Ok(manifest) = serde_json::from_str::<ProjectManifest>(&content) {
                                manifests_list.push(manifest);
                            }
                        }
                    }
                }
            }
            manifests_list.sort_by(|first, second| first.slug.cmp(&second.slug));
            manifests_list
        })
    }

    fn sweep_orphans(&self, bundled_slugs: &[String]) -> Fut<Result<(), StoreError>> {
        let root = self.root.clone();
        let bundled_slugs = bundled_slugs.to_vec();
        Box::pin(async move {
            let read_directory = fs::read_dir(&root).map_err(|error| StoreError::Backend(error.to_string()))?;
            for entry in read_directory.flatten() {
                let path = entry.path();
                let directory_name = entry.file_name().to_string_lossy().into_owned();
                if directory_name.ends_with(".new") || directory_name.ends_with(".old") || directory_name.starts_with('.') {
                    continue;
                }
                if path.is_dir() {
                    let slug = directory_name;
                    let manifest_file = path.join("manifest.json");
                    if !manifest_file.exists() && !bundled_slugs.contains(&slug) {
                        fs::remove_dir_all(&path).map_err(|error| StoreError::Backend(error.to_string()))?;
                    }
                }
            }
            Ok(())
        })
    }
}
