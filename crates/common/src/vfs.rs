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

/// List the direct children of `dir` whose extension is `ext` (no dot),
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
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(ext))
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
                    "not in vfs: {key} — is it in manifest.json, and does the \
                     preload base match GameConfig.asset_base_path?"
                ),
            )
        })
    }

    /// [`MemFs::read`] + UTF-8 validation, mirroring `fs::read_to_string`.
    pub fn read_to_string(&self, path: &Path) -> io::Result<String> {
        String::from_utf8(self.read(path)?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Direct children of `dir` with extension `ext`, sorted. A key is a
    /// direct child when, after stripping `dir` + `/`, the remainder has no
    /// further `/` — matching `read_dir`'s non-recursive semantics.
    pub fn list_dir_files(&self, dir: &Path, ext: &str) -> Vec<PathBuf> {
        let mut prefix = dir.to_string_lossy().into_owned();
        if !prefix.ends_with('/') {
            prefix.push('/');
        }
        let suffix = format!(".{ext}");
        let mut files: Vec<PathBuf> = self
            .files
            .keys()
            .filter(|key| {
                key.strip_prefix(&prefix)
                    .is_some_and(|rest| !rest.contains('/') && rest.ends_with(&suffix))
            })
            .map(PathBuf::from)
            .collect();
        files.sort();
        files
    }
}

#[cfg(test)]
mod tests {
    use super::MemFs;
    use std::io;
    use std::path::Path;

    const BASE: &str = "/games/pong/v1/assets";

    /// Populate the map the way the web boot phase will (base-joined keys)
    /// and read back through AssetManager-style `Path::join` lookups.
    #[test]
    fn test_boot_phase_keys_resolve_through_base_joined_reads() -> io::Result<()> {
        let mut fs = MemFs::default();
        fs.insert(format!("{BASE}/paddle_16px.png"), vec![1, 2, 3]);
        fs.insert(format!("{BASE}/fonts/font.ttf"), vec![4, 5]);
        fs.insert(format!("{BASE}/locales/broken.ron"), vec![0xFF, 0xFE]);

        let flat = fs.read(&Path::new(BASE).join("paddle_16px.png"))?;
        let nested = fs.read(&Path::new(BASE).join("fonts/font.ttf"))?;
        let relative_miss = fs.read(Path::new("paddle_16px.png"));
        let invalid_utf8 = fs.read_to_string(&Path::new(BASE).join("locales/broken.ron"));

        assert_eq!(flat, [1, 2, 3]);
        assert_eq!(nested, [4, 5]);
        assert_eq!(
            relative_miss.map_err(|e| e.kind()),
            Err(io::ErrorKind::NotFound),
            "a relative key must miss: only the base-joined form is canonical"
        );
        assert_eq!(
            invalid_utf8.map_err(|e| e.kind()),
            Err(io::ErrorKind::InvalidData),
            "read_to_string mirrors fs::read_to_string's UTF-8 rejection"
        );
        Ok(())
    }

    #[test]
    fn test_list_dir_files_is_sorted_extension_filtered_direct_children_only() {
        let mut fs = MemFs::default();
        fs.insert(format!("{BASE}/locales/pirate.ron"), vec![2]);
        fs.insert(format!("{BASE}/locales/en.ron"), vec![1]);
        fs.insert(format!("{BASE}/locales/notes.txt"), vec![3]);
        fs.insert(format!("{BASE}/locales/extra/deep.ron"), vec![5]);
        fs.insert(format!("{BASE}/paddle_16px.png"), vec![4]);
        let dir = Path::new(BASE).join("locales");

        let files = fs.list_dir_files(&dir, "ron");

        assert_eq!(
            files,
            [dir.join("en.ron"), dir.join("pirate.ron")],
            "sorted, ron-only, direct children (no extra/deep.ron)"
        );
    }
}
