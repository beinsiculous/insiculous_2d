//! Direct directory entries and exclusive asset creation on disk and in the web map.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use super::MemFs;

/// One direct child of a directory; symlinks are never entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub path: PathBuf,
    pub is_dir: bool,
}

/// List the direct files and directories in `dir`, sorted by path. The configured root is
/// followed as `list_files` follows it (a project whose asset root is a link to a shared
/// checkout still lists), and symlink entries below it are never followed. An entry whose
/// metadata cannot be read is skipped, not the directory: one racy removal must not hide
/// every sibling. The web map derives directories from its stored paths, `.keep` included.
pub fn list_entries(dir: &Path) -> io::Result<Vec<DirEntry>> {
    nonempty_path(dir)?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(dir)?.flatten() {
            let Ok(file_type) = entry.file_type() else { continue };
            if file_type.is_file() || file_type.is_dir() {
                entries.push(DirEntry { path: entry.path(), is_dir: file_type.is_dir() });
            }
        }
        entries.sort_by(|first, second| first.path.cmp(&second.path));
        Ok(entries)
    }
    #[cfg(target_arch = "wasm32")]
    {
        super::MEM_FS.with(|filesystem| filesystem.borrow().list_entries(dir))
    }
}

/// Create a directory and its missing parents. A web directory carries a hidden
/// `.keep` file so an empty folder survives export, import, and reload.
pub fn create_dir_all(dir: &Path) -> io::Result<()> {
    nonempty_path(dir)?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::create_dir_all(dir)
    }
    #[cfg(target_arch = "wasm32")]
    {
        super::MEM_FS.with(|filesystem| filesystem.borrow_mut().create_dir_all(dir))?;
        super::notify_write_observer(&dir.join(".keep"));
        Ok(())
    }
}

/// Whether `path` names a file or directory; web directories are implied by file keys.
pub fn exists(path: &Path) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        path.exists()
    }
    #[cfg(target_arch = "wasm32")]
    {
        super::MEM_FS.with(|filesystem| filesystem.borrow().exists(path))
    }
}

/// Write a new file exclusively, refusing a live file or directory at `path` with
/// `AlreadyExists`. Its parent must already exist. On the web the observer queues the
/// put with the persistence chain's current revision, including a tombstone's revision.
pub fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    nonempty_path(path)?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
        // A write that fails after the create (a full disk) must not leave a half file:
        // it would block every retry with AlreadyExists and parse as a broken asset. The
        // cleanup removes only the file this call created — by its inode, since another
        // program may have replaced the path in the meantime — and leaves anything else.
        if let Err(error) = file.write_all(bytes) {
            return Err(match remove_if_same_file(&file, path) {
                Ok(()) => error,
                Err(cleanup) => io::Error::new(error.kind(), format!("{error}; the half-written {} could not be removed: {cleanup}", path.display())),
            });
        }
    }
    #[cfg(target_arch = "wasm32")]
    super::MEM_FS.with(|filesystem| filesystem.borrow_mut().write_new(path, bytes))?;

    #[cfg(any(target_arch = "wasm32", test))]
    super::notify_write_observer(path);
    Ok(())
}

impl MemFs {
    /// The direct children implied by the live file keys, with each directory listed
    /// once and the files that keep an empty directory included.
    pub fn list_entries(&self, dir: &Path) -> io::Result<Vec<DirEntry>> {
        nonempty_path(dir)?;
        let prefix = directory_prefix(dir);
        let mut entries = BTreeMap::new();
        for key in self.files.keys() {
            let Some(relative) = key.strip_prefix(&prefix) else { continue };
            let Some(name) = relative.split('/').next().filter(|name| !name.is_empty()) else { continue };
            let is_dir = relative.contains('/');
            entries.entry(dir.join(name)).and_modify(|directory| *directory |= is_dir).or_insert(is_dir);
        }
        Ok(entries.into_iter().map(|(path, is_dir)| DirEntry { path, is_dir }).collect())
    }

    /// Keep an empty directory by a `.keep` entry, refusing a file in its ancestry.
    pub fn create_dir_all(&mut self, dir: &Path) -> io::Result<()> {
        nonempty_path(dir)?;
        self.refuse_file_ancestors(dir)?;
        self.files.entry(dir.join(".keep").to_string_lossy().into_owned()).or_default();
        Ok(())
    }

    /// A live file or a directory implied by a live file exists; a removed file does not.
    pub fn exists(&self, path: &Path) -> bool {
        if path.as_os_str().is_empty() {
            return false;
        }
        self.files.contains_key(path.to_string_lossy().as_ref())
            || self.files.keys().any(|key| key.starts_with(&directory_prefix(path)))
    }

    /// Insert only when no live file or directory occupies `path`, and its parent is a
    /// directory. Removed keys can be recreated; persistence owns their old revisions.
    pub fn write_new(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        nonempty_path(path)?;
        if self.exists(path) {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{} already exists", path.display())));
        }
        if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            self.refuse_file_ancestors(parent)?;
            if !self.exists(parent) {
                return Err(io::Error::new(io::ErrorKind::NotFound, format!("{} does not exist", parent.display())));
            }
        }
        self.files.insert(path.to_string_lossy().into_owned(), bytes.to_vec());
        Ok(())
    }

    fn refuse_file_ancestors(&self, path: &Path) -> io::Result<()> {
        for ancestor in path.ancestors() {
            if self.files.contains_key(ancestor.to_string_lossy().as_ref()) {
                return Err(io::Error::new(io::ErrorKind::NotADirectory, format!("{} is a file", ancestor.display())));
            }
        }
        Ok(())
    }
}

/// Remove `path` only while it still names the file `created` is open on. On Unix the inode
/// says so; elsewhere nothing can, so the half file stays and the error names it.
#[cfg(not(target_arch = "wasm32"))]
fn remove_if_same_file(created: &std::fs::File, path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let (created, on_disk) = (created.metadata()?, std::fs::symlink_metadata(path)?);
        if created.ino() != on_disk.ino() || created.dev() != on_disk.dev() {
            return Err(io::Error::other("another program replaced it, so it was left alone"));
        }
        std::fs::remove_file(path)
    }
    #[cfg(not(unix))]
    {
        let _ = created;
        Err(io::Error::other(format!("{} is left for you to remove", path.display())))
    }
}

fn directory_prefix(dir: &Path) -> String {
    format!("{}/", dir.to_string_lossy().trim_end_matches('/'))
}

fn nonempty_path(path: &Path) -> io::Result<()> {
    if path.as_os_str().is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty path"));
    }
    Ok(())
}
