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

/// Write `bytes` to `path` (native: `create_dir_all(parent)` + `write`; wasm: `MemFs::insert`).
pub fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, bytes)
    }
    #[cfg(target_arch = "wasm32")]
    {
        MEM_FS.with(|fs| {
            fs.borrow_mut().insert(path.to_string_lossy().into_owned(), bytes.to_vec());
        });
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
        // A missing root is the caller's error; a child that vanishes or
        // cannot be read mid-walk is skipped, so the probe below is deliberate.
        let _ = std::fs::read_dir(dir)?;
        let mut files = Vec::new();
        let mut stack: Vec<(PathBuf, usize)> = vec![(dir.to_path_buf(), 0)];

        while let Some((current_dir, depth)) = stack.pop() {
            let Ok(read) = std::fs::read_dir(&current_dir) else {
                continue;
            };
            for entry in read.flatten() {
                let path = entry.path();
                let Ok(meta) = std::fs::symlink_metadata(&path) else {
                    continue;
                };
                if meta.file_type().is_symlink() {
                    continue;
                }
                if meta.is_dir() {
                    if depth < MAX_LIST_DEPTH {
                        stack.push((path, depth + 1));
                    }
                } else if meta.is_file() {
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
            Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(prefix),
            Ok(_) => std::fs::remove_file(prefix),
            Err(e) => Err(e),
        };
        match result {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
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
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case(ext))
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

    /// Direct children of `dir` with extension `ext` (case-insensitive), sorted.
    /// A key is a direct child when, after stripping `dir` + `/`, the remainder
    /// has no further `/` — matching `read_dir`'s non-recursive semantics.
    pub fn list_dir_files(&self, dir: &Path, ext: &str) -> Vec<PathBuf> {
        let mut prefix = dir.to_string_lossy().into_owned();
        if !prefix.is_empty() && !prefix.ends_with('/') {
            prefix.push('/');
        }
        let mut files: Vec<PathBuf> = self
            .files
            .keys()
            .filter(|key| {
                key.strip_prefix(&prefix).is_some_and(|rest| {
                    !rest.contains('/')
                        && rest
                            .rsplit_once('.')
                            .is_some_and(|(_, e)| e.eq_ignore_ascii_case(ext))
                })
            })
            .map(PathBuf::from)
            .collect();
        files.sort();
        files
    }

    /// Recursively list all files under `prefix`, sorted by path.
    pub fn list_files(&self, prefix: &Path) -> Vec<PathBuf> {
        let mut p = prefix.to_string_lossy().into_owned();
        if !p.is_empty() && !p.ends_with('/') {
            p.push('/');
        }
        let mut files: Vec<PathBuf> = self
            .files
            .keys()
            .filter(|key| key.starts_with(&p))
            .map(PathBuf::from)
            .collect();
        files.sort();
        files
    }

    /// Remove every file whose canonical key starts with `prefix/`, and a file
    /// keyed exactly `prefix` (the map has no directories, so a "file at the
    /// prefix" is the wasm twin of native `remove_file`). Siblings stay.
    pub fn remove_prefix(&mut self, prefix: &Path) {
        let exact = prefix.to_string_lossy().into_owned();
        let mut p = exact.clone();
        if !p.is_empty() && !p.ends_with('/') {
            p.push('/');
        }
        self.files.retain(|k, _| *k != exact && !k.starts_with(&p));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::path::{Path, PathBuf};

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

    #[test]
    fn test_mem_fs_write_and_read_round_trip() -> io::Result<()> {
        let mut fs = MemFs::default();
        let file_path = Path::new(BASE).join("save.ron");

        fs.insert(file_path.to_string_lossy().into_owned(), b"first".to_vec());
        assert_eq!(fs.read(&file_path)?, b"first");
        assert_eq!(fs.read_to_string(&file_path)?, "first");

        // Overwrite semantics
        fs.insert(file_path.to_string_lossy().into_owned(), b"second".to_vec());
        assert_eq!(fs.read(&file_path)?, b"second");
        assert_eq!(fs.read_to_string(&file_path)?, "second");
        Ok(())
    }

    #[test]
    fn test_mem_fs_list_files_recursive_and_sorted() {
        let mut fs = MemFs::default();
        fs.insert(format!("{BASE}/b.txt"), vec![1]);
        fs.insert(format!("{BASE}/sub/c.txt"), vec![2]);
        fs.insert(format!("{BASE}/a.txt"), vec![3]);
        fs.insert("/other/root/x.txt".to_string(), vec![4]);

        let listed = fs.list_files(Path::new(BASE));
        assert_eq!(
            listed,
            vec![
                PathBuf::from(format!("{BASE}/a.txt")),
                PathBuf::from(format!("{BASE}/b.txt")),
                PathBuf::from(format!("{BASE}/sub/c.txt")),
            ],
            "recursive listing under prefix, sorted alphabetically"
        );
    }

    #[test]
    fn test_mem_fs_remove_prefix_leaves_siblings() {
        let mut fs = MemFs::default();
        fs.insert(format!("{BASE}/project1/scene.ron"), vec![1]);
        fs.insert(format!("{BASE}/project1/scripts/main.rhai"), vec![2]);
        fs.insert(format!("{BASE}/project2/scene.ron"), vec![3]);

        fs.remove_prefix(Path::new(&format!("{BASE}/project1")));

        assert!(fs.list_files(Path::new(&format!("{BASE}/project1"))).is_empty());
        let remaining = fs.list_files(Path::new(&format!("{BASE}/project2")));
        assert_eq!(remaining, vec![PathBuf::from(format!("{BASE}/project2/scene.ron"))]);
    }

    #[test]
    fn test_mem_fs_remove_prefix_drops_a_file_keyed_exactly_at_the_prefix() {
        let mut fs = MemFs::default();
        fs.insert(format!("{BASE}/notes.txt"), vec![1]);
        fs.insert(format!("{BASE}/notes.txt.bak"), vec![2]);

        fs.remove_prefix(Path::new(&format!("{BASE}/notes.txt")));

        assert!(fs.read(&Path::new(BASE).join("notes.txt")).is_err(), "the exact key is gone");
        assert!(
            fs.read(&Path::new(BASE).join("notes.txt.bak")).is_ok(),
            "a longer sibling key is not a child of the prefix"
        );
    }

    #[test]
    fn test_empty_path_is_refused_by_list_files_and_remove_prefix_on_every_target() {
        assert_eq!(list_files(Path::new("")).map_err(|e| e.kind()), Err(io::ErrorKind::InvalidInput));
        assert_eq!(remove_prefix(Path::new("")).map_err(|e| e.kind()), Err(io::ErrorKind::InvalidInput));
    }

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(prefix: &str) -> io::Result<Self> {
            static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "insiculous_{prefix}_{}_{}",
                std::process::id(),
                id
            ));
            std::fs::create_dir_all(&path)?;
            Ok(Self(path))
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn test_vfs_native_write_and_read_round_trip() -> io::Result<()> {
        let dir = TempDir::new("vfs_rw")?;
        let path = dir.path().join("nested/dir/test.txt");

        write_string(&path, "hello vfs")?;
        assert_eq!(read_to_string(&path)?, "hello vfs");
        assert_eq!(read(&path)?, b"hello vfs");

        write(&path, b"updated")?;
        assert_eq!(read(&path)?, b"updated");
        Ok(())
    }

    #[test]
    fn test_vfs_native_list_files_and_remove_prefix() -> io::Result<()> {
        let dir = TempDir::new("vfs_list_remove")?;
        let p1 = dir.path().join("p1");
        let p2 = dir.path().join("p2");

        write_string(&p1.join("b.txt"), "b")?;
        write_string(&p1.join("sub/c.txt"), "c")?;
        write_string(&p1.join("a.txt"), "a")?;
        write_string(&p2.join("x.txt"), "x")?;

        let listed = list_files(&p1)?;
        assert_eq!(
            listed,
            vec![p1.join("a.txt"), p1.join("b.txt"), p1.join("sub/c.txt")]
        );

        remove_prefix(&p1)?;
        assert!(!p1.exists());
        assert!(p2.join("x.txt").exists());

        // Removing already-absent prefix is an idempotent Ok(())
        remove_prefix(&p1)?;

        // A regular file at the prefix is removed too, matching the wasm map's
        // exact-key removal instead of failing with NotADirectory.
        remove_prefix(&p2.join("x.txt"))?;
        assert!(!p2.join("x.txt").exists());
        assert!(p2.exists(), "only the file went, not its parent");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_vfs_list_files_never_follows_symlinks() -> io::Result<()> {
        let dir = TempDir::new("vfs_symlink")?;
        let root = dir.path().join("root");
        let sub = root.join("sub");
        write_string(&root.join("a.txt"), "a")?;
        write_string(&sub.join("b.txt"), "b")?;

        // Symlink loop: root/sub/loop -> root
        std::os::unix::fs::symlink(&root, sub.join("loop"))?;

        let listed = list_files(&root)?;
        assert_eq!(listed, vec![root.join("a.txt"), sub.join("b.txt")]);
        Ok(())
    }
}
