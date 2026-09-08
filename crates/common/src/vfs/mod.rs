//! Virtual filesystem seam for asset reads.
//!
//! Natively every function is a thin `std::fs` passthrough. On
//! `wasm32-unknown-unknown` the same functions serve an in-memory map that
//! the web boot phase populates (via [`insert`]) before the game starts, so
//! loaders stay synchronous and identical across targets.
//!
//! # Canonical key scheme (wasm)
//! A file's key is the exact string its read path produces: the configured
//! asset base joined with the asset's relative name (e.g.
//! `/games/pong/v1/assets/locales/en.ron`). The boot phase MUST insert under
//! that same joined form — mismatched key conventions are a boot-time 404 for
//! every asset, which is why [`MemFs`] and its tests live here on all targets.

use std::io;
use std::path::{Path, PathBuf};

#[cfg(any(target_arch = "wasm32", test))]
thread_local! {
    static WRITE_OBSERVER: std::cell::Cell<Option<fn(&Path)>> = const { std::cell::Cell::new(None) };
}

/// Set a callback invoked whenever [`write`] completes on wasm.
///
/// Called exactly once per [`write`] or [`write_string`]. Boot-phase
/// [`insert`] does not invoke this observer.
#[cfg(any(target_arch = "wasm32", test))]
pub fn set_write_observer(observer: fn(&Path)) {
    WRITE_OBSERVER.with(|slot| slot.set(Some(observer)));
}

#[cfg(any(target_arch = "wasm32", test))]
fn notify_write_observer(path: &Path) {
    WRITE_OBSERVER.with(|slot| {
        if let Some(observer) = slot.get() {
            observer(path);
        }
    });
}

/// Read a file's bytes (native: `std::fs::read`; wasm: in-memory map).
pub fn read(path: &Path) -> io::Result<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read(path)
    }
    #[cfg(target_arch = "wasm32")]
    {
        MEM_FS.with(|fs| fs.borrow().read(path))
    }
}

/// Read a file as UTF-8 (native: `std::fs::read_to_string`; wasm: map).
pub fn read_to_string(path: &Path) -> io::Result<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string(path)
    }
    #[cfg(target_arch = "wasm32")]
    {
        MEM_FS.with(|fs| fs.borrow().read_to_string(path))
    }
}

/// Write `bytes` to `path` (native: `create_dir_all(parent)` + `write`; wasm: `MemFs::insert`).
pub fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, bytes)?;
        #[cfg(test)]
        notify_write_observer(path);
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        MEM_FS.with(|fs| {
            fs.borrow_mut().insert(path.to_string_lossy().into_owned(), bytes.to_vec());
        });
        notify_write_observer(path);
        Ok(())
    }
}

/// Write UTF-8 `text` to `path` (native: `create_dir_all(parent)` + `write`; wasm: `MemFs::insert`).
pub fn write_string(path: &Path, text: &str) -> io::Result<()> {
    write(path, text.as_bytes())
}

/// How deep [`list_files`] descends natively. The org's asset trees are shallow
/// copies (never links), so six levels is generous; the cap is what bounds a
/// pathological tree, since symlinks are never followed. The wasm scan has no
/// directories to descend, hence the cfg.
#[cfg(not(target_arch = "wasm32"))]
const MAX_LIST_DEPTH: usize = 6;

/// Recursively list all files under `dir`, sorted by path.
///
/// Symlinks are never followed — neither directories nor files (`symlink_metadata`
/// is inspected on every entry), so a link out of the project cannot pull `/home`
/// into an asset browser or an export. Depth is capped at [`MAX_LIST_DEPTH`]
/// natively. On wasm this is a prefix scan over [`MemFs`]. An empty `dir` is an
/// error on every target (native `read_dir("")` fails; the wasm scan would
/// otherwise match every key).
pub fn list_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    if dir.as_os_str().is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "list_files: empty path"));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = std::fs::read_dir(dir)?;
        let mut files = Vec::new();
        let mut stack: Vec<(PathBuf, usize)> = vec![(dir.to_path_buf(), 0)];

        while let Some((current_dir, depth)) = stack.pop() {
            let Ok(read) = std::fs::read_dir(&current_dir) else {
                continue;
            };
            for entry in read.flatten() {
                let path = entry.path();
                let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                    continue;
                };
                if metadata.file_type().is_symlink() {
                    continue;
                }
                if metadata.is_dir() {
                    if depth < MAX_LIST_DEPTH {
                        stack.push((path, depth + 1));
                    }
                } else if metadata.is_file() {
                    files.push(path);
                }
            }
        }
        files.sort();
        Ok(files)
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(MEM_FS.with(|fs| fs.borrow().list_files(dir)))
    }
}

/// Remove everything at and under `prefix`: a directory and its contents, or a
/// single file at exactly that path (native: `remove_dir_all` / `remove_file`;
/// wasm: `MemFs::remove_prefix`, which also drops a key equal to the prefix).
///
/// Used as the "replace this project" primitive. A missing prefix reports
/// `Ok(())`; an empty prefix is an error on every target.
pub fn remove_prefix(prefix: &Path) -> io::Result<()> {
    if prefix.as_os_str().is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "remove_prefix: empty path"));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let result = match std::fs::symlink_metadata(prefix) {
            Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(prefix),
            Ok(_) => std::fs::remove_file(prefix),
            Err(error) => Err(error),
        };
        match result {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        MEM_FS.with(|fs| fs.borrow_mut().remove_prefix(prefix));
        Ok(())
    }
}

/// List the direct children of `dir` whose extension is `ext` (no dot, case-insensitive),
/// sorted by path for deterministic ordering on every target.
///
/// An unreadable/missing dir yields an empty list; use
/// [`list_dir_files_checked`] when the caller wants to distinguish a
/// missing dir from a real IO failure for diagnostics.
pub fn list_dir_files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    list_dir_files_checked(dir, ext).unwrap_or_default()
}

/// [`list_dir_files`] that surfaces the directory-read failure. On wasm the
/// map's prefix scan cannot fail, so a missing prefix is `Ok(empty)` —
/// only native filesystem problems (permissions, missing dir) reach `Err`.
pub fn list_dir_files_checked(dir: &Path, ext: &str) -> std::io::Result<Vec<PathBuf>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|extension_os_str| extension_os_str.to_str())
                    .is_some_and(|extension_str| extension_str.eq_ignore_ascii_case(ext))
            })
            .collect();
        files.sort();
        Ok(files)
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(MEM_FS.with(|fs| fs.borrow().list_dir_files(dir, ext)))
    }
}

/// Insert fetched bytes under `path` (wasm boot phase only).
#[cfg(target_arch = "wasm32")]
pub fn insert(path: String, bytes: Vec<u8>) {
    MEM_FS.with(|fs| fs.borrow_mut().insert(path, bytes));
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static MEM_FS: std::cell::RefCell<MemFs> = std::cell::RefCell::new(MemFs::default());
}

/// The in-memory filesystem backing the wasm read path. Compiled (and unit
/// tested) on every target so the exact lookup semantics the browser will
/// use are verified by the native test suite.
#[derive(Default)]
pub struct MemFs {
    files: std::collections::HashMap<String, Vec<u8>>,
}

impl MemFs {
    /// Store `bytes` under the canonical joined-path key.
    ///
    /// Overwrites any existing entry at `path`.
    pub fn insert(&mut self, path: String, bytes: Vec<u8>) {
        self.files.insert(path, bytes);
    }

    /// Look up a file by the exact string its `Path` renders to.
    pub fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let key = path.to_string_lossy();
        self.files.get(key.as_ref()).cloned().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "vfs: '{}' not in the memory map — is it in manifest.json, and does the \
                     preload base match GameConfig.asset_base_path?",
                    path.display()
                ),
            )
        })
    }

    /// Look up a UTF-8 file by the exact string its `Path` renders to.
    pub fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let bytes = self.read(path)?;
        String::from_utf8(bytes).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("vfs: '{}' is not valid UTF-8: {error}", path.display()),
            )
        })
    }

    /// Recursively list all files matching `dir` as a prefix, sorted by path.
    pub fn list_files(&self, dir: &Path) -> Vec<PathBuf> {
        let mut prefix = dir.to_string_lossy().into_owned();
        if !prefix.is_empty() && !prefix.ends_with('/') {
            prefix.push('/');
        }
        let mut matches: Vec<PathBuf> = self
            .files
            .keys()
            .filter(|key| key.starts_with(&prefix))
            .map(PathBuf::from)
            .collect();
        matches.sort();
        matches
    }

    /// List files directly under `dir` whose extension is `ext` (no dot, case-insensitive).
    pub fn list_dir_files(&self, dir: &Path, ext: &str) -> Vec<PathBuf> {
        let mut prefix = dir.to_string_lossy().into_owned();
        if !prefix.is_empty() && !prefix.ends_with('/') {
            prefix.push('/');
        }
        let mut matches: Vec<PathBuf> = self
            .files
            .keys()
            .filter(|key| {
                if let Some(rest) = key.strip_prefix(&prefix) {
                    !rest.contains('/')
                        && Path::new(rest)
                            .extension()
                            .and_then(|extension_os_str| extension_os_str.to_str())
                            .is_some_and(|extension_str| extension_str.eq_ignore_ascii_case(ext))
                } else {
                    false
                }
            })
            .map(PathBuf::from)
            .collect();
        matches.sort();
        matches
    }

    /// Drop everything whose key begins with `prefix` (interpreted as a directory boundary)
    /// or exactly matches `prefix`.
    pub fn remove_prefix(&mut self, prefix: &Path) {
        let exact = prefix.to_string_lossy().into_owned();
        let mut prefix_with_slash = exact.clone();
        if !prefix_with_slash.is_empty() && !prefix_with_slash.ends_with('/') {
            prefix_with_slash.push('/');
        }
        self.files.retain(|key, _| *key != exact && !key.starts_with(&prefix_with_slash));
    }
}

#[cfg(test)]
mod tests;
