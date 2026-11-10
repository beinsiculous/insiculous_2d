//! Directory entries and non-overwriting asset mutations on disk and in the web map.

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
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_file() || file_type.is_dir() {
                entries.push(DirEntry {
                    path: entry.path(),
                    is_dir: file_type.is_dir(),
                });
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
/// `AlreadyExists`. Its parent must already exist. Native writes publish a complete
/// temporary sibling through a hard link, so incomplete bytes never have the final name;
/// where the filesystem refuses links (FAT, exFAT) the file is created directly instead,
/// a half-written file removed by its inode. On the web the observer queues the put with
/// the path's current persistence revision.
pub fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    nonempty_path(path)?;
    #[cfg(not(target_arch = "wasm32"))]
    publish_new(path, bytes)?;
    #[cfg(target_arch = "wasm32")]
    super::MEM_FS.with(|filesystem| filesystem.borrow_mut().write_new(path, bytes))?;

    #[cfg(any(target_arch = "wasm32", test))]
    super::notify_write_observer(path);
    Ok(())
}

/// Remove one file, reporting a missing path. The web remove observer runs after the
/// map's borrow ends so persistence can inspect the resulting filesystem.
pub fn remove_file(path: &Path) -> io::Result<()> {
    nonempty_path(path)?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::remove_file(path)
    }
    #[cfg(target_arch = "wasm32")]
    {
        super::MEM_FS.with(|filesystem| {
            MemFs::remove_file_observed(filesystem, path, super::notify_remove_observer)
        })
    }
}

/// Remove an empty directory, refusing one that holds anything: the caller that proved
/// it empty a moment ago must not take a file that arrived since. A web directory is
/// its keys, so one with none is already gone.
pub fn remove_dir(dir: &Path) -> io::Result<()> {
    nonempty_path(dir)?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::remove_dir(dir)
    }
    #[cfg(target_arch = "wasm32")]
    {
        super::MEM_FS.with(|filesystem| filesystem.borrow().remove_dir(dir))
    }
}

/// Rename without replacing an existing destination. Native files use a hard link,
/// verify identity and length, then unlink the source; directories check absence then
/// use the operating system rename. On the web every destination put is observed before
/// any source removal, so a persistence interruption leaves a recoverable duplicate.
pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
    nonempty_path(from)?;
    nonempty_path(to)?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        rename_native(
            from,
            to,
            || Ok(()),
            |source, destination| std::fs::hard_link(source, destination),
        )
    }
    #[cfg(target_arch = "wasm32")]
    {
        super::MEM_FS.with(|filesystem| {
            MemFs::rename_observed(
                filesystem,
                from,
                to,
                super::notify_write_observer,
                super::notify_remove_observer,
            )
        })
    }
}

struct MemoryRename {
    paths: Vec<(PathBuf, PathBuf)>,
    moving_directory: bool,
}

impl MemFs {
    /// The direct children implied by the live file keys, with each directory listed
    /// once and the files that keep an empty directory included.
    pub fn list_entries(&self, dir: &Path) -> io::Result<Vec<DirEntry>> {
        nonempty_path(dir)?;
        let prefix = directory_prefix(dir);
        let mut entries = BTreeMap::new();
        for key in self.files.keys() {
            let Some(relative) = key.strip_prefix(&prefix) else {
                continue;
            };
            let Some(name) = relative.split('/').next().filter(|name| !name.is_empty()) else {
                continue;
            };
            let is_dir = relative.contains('/');
            entries
                .entry(dir.join(name))
                .and_modify(|directory| *directory |= is_dir)
                .or_insert(is_dir);
        }
        Ok(entries
            .into_iter()
            .map(|(path, is_dir)| DirEntry { path, is_dir })
            .collect())
    }

    /// Keep an empty directory by a `.keep` entry, refusing a file in its ancestry.
    pub fn create_dir_all(&mut self, dir: &Path) -> io::Result<()> {
        nonempty_path(dir)?;
        self.refuse_file_ancestors(dir)?;
        self.files
            .entry(dir.join(".keep").to_string_lossy().into_owned())
            .or_default();
        Ok(())
    }

    /// A live file or a directory implied by a live file exists; a removed file does not.
    pub fn exists(&self, path: &Path) -> bool {
        if path.as_os_str().is_empty() {
            return false;
        }
        self.files.contains_key(path.to_string_lossy().as_ref())
            || self
                .files
                .keys()
                .any(|key| key.starts_with(&directory_prefix(path)))
    }

    /// Insert only when no live file or directory occupies `path`, and its parent is a
    /// directory. Removed keys can be recreated; persistence owns their old revisions.
    pub fn write_new(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.write_new_in_directory(path, bytes, false)
    }

    fn write_new_in_directory(
        &mut self,
        path: &Path,
        bytes: &[u8],
        moving_parent: bool,
    ) -> io::Result<()> {
        nonempty_path(path)?;
        if self.exists(path) {
            return Err(already_exists(path));
        }
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            self.refuse_file_ancestors(parent)?;
            if !moving_parent && !self.exists(parent) {
                return Err(super::not_in_map(parent));
            }
        }
        self.files
            .insert(path.to_string_lossy().into_owned(), bytes.to_vec());
        Ok(())
    }

    /// Remove a single live file, leaving descendants and longer sibling keys alone.
    pub fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        nonempty_path(path)?;
        if self.files.remove(path.to_string_lossy().as_ref()).is_some() {
            return Ok(());
        }
        if self.exists(path) {
            return Err(io::Error::new(
                io::ErrorKind::IsADirectory,
                format!("{} is a directory", path.display()),
            ));
        }
        Err(super::not_in_map(path))
    }

    /// Nothing to drop for an empty web directory; one with keys under it is refused.
    pub fn remove_dir(&self, dir: &Path) -> io::Result<()> {
        nonempty_path(dir)?;
        if self.files.contains_key(dir.to_string_lossy().as_ref()) {
            return Err(io::Error::new(io::ErrorKind::NotADirectory, format!("{} is a file", dir.display())));
        }
        if self.files.keys().any(|key| key.starts_with(&directory_prefix(dir))) {
            return Err(io::Error::new(io::ErrorKind::DirectoryNotEmpty, format!("{} is not empty", dir.display())));
        }
        Ok(())
    }

    /// Remove a file and notify after releasing the map borrow, allowing the observer
    /// to read the remaining filesystem. This is the web adapter, testable natively.
    pub fn remove_file_observed(
        filesystem: &std::cell::RefCell<Self>,
        path: &Path,
        removed: impl Fn(&Path),
    ) -> io::Result<()> {
        filesystem.borrow_mut().remove_file(path)?;
        removed(path);
        Ok(())
    }

    /// Move silently, returning the source/destination pairs. Use `rename_observed`
    /// when persistence must see each destination before any source is removed.
    pub fn rename(&mut self, from: &Path, to: &Path) -> io::Result<Vec<(PathBuf, PathBuf)>> {
        let plan = self.plan_rename(from, to)?;
        for (source, destination) in &plan.paths {
            let bytes = self.read(source)?;
            self.write_new_in_directory(destination, &bytes, plan.moving_directory)?;
        }
        for (source, _) in &plan.paths {
            self.remove_file(source)?;
        }
        Ok(plan.paths)
    }

    /// Publish every destination and notify after each borrow ends, then remove and
    /// notify every source. The same adapter drives wasm and native persistence tests.
    pub fn rename_observed(
        filesystem: &std::cell::RefCell<Self>,
        from: &Path,
        to: &Path,
        written: impl Fn(&Path),
        removed: impl Fn(&Path),
    ) -> io::Result<()> {
        let plan = filesystem.borrow().plan_rename(from, to)?;
        for (source, destination) in &plan.paths {
            let bytes = filesystem.borrow().read(source)?;
            filesystem.borrow_mut().write_new_in_directory(
                destination,
                &bytes,
                plan.moving_directory,
            )?;
            written(destination);
        }
        for (source, _) in &plan.paths {
            Self::remove_file_observed(filesystem, source, &removed)?;
        }
        Ok(())
    }

    fn plan_rename(&self, from: &Path, to: &Path) -> io::Result<MemoryRename> {
        nonempty_path(from)?;
        nonempty_path(to)?;
        if self.exists(to) {
            return Err(already_exists(to));
        }
        self.refuse_file_ancestors(to)?;
        let source_is_file = self.files.contains_key(from.to_string_lossy().as_ref());
        let sources = if source_is_file {
            vec![from.to_path_buf()]
        } else {
            self.list_files(from)
        };
        if sources.is_empty() {
            return Err(super::not_in_map(from));
        }
        if !source_is_file && to.starts_with(from) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a directory cannot move inside itself",
            ));
        }
        if let Some(parent) = to.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            if !self.exists(parent) {
                return Err(super::not_in_map(parent));
            }
        }
        let moved = sources
            .into_iter()
            .map(|source| {
                let destination = if source_is_file {
                    to.to_path_buf()
                } else {
                    to.join(source.strip_prefix(from).map_err(io::Error::other)?)
                };
                Ok((source, destination))
            })
            .collect::<io::Result<Vec<_>>>()?;
        // Source topology proves each new parent even before its first key is
        // published. Ordinary creates still require an existing parent.
        Ok(MemoryRename {
            paths: moved,
            moving_directory: !source_is_file,
        })
    }

    fn refuse_file_ancestors(&self, path: &Path) -> io::Result<()> {
        for ancestor in path.ancestors() {
            if self.files.contains_key(ancestor.to_string_lossy().as_ref()) {
                return Err(io::Error::new(
                    io::ErrorKind::NotADirectory,
                    format!("{} is a file", ancestor.display()),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn publish_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    publish_new_with(path, |file| file.write_all(bytes), |from, to| std::fs::hard_link(from, to))
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn publish_new_with(
    path: &Path,
    write: impl Fn(&mut std::fs::File) -> io::Result<()>,
    link: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    static NEXT_TEMPORARY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if path.file_name().is_none() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "missing file name"));
    }
    // The link is what makes the publish exclusive; this check only spares a large write
    // onto a name that is already taken.
    if std::fs::symlink_metadata(path).is_ok() {
        return Err(already_exists(path));
    }
    let (temporary, mut file) = loop {
        let identifier = NEXT_TEMPORARY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // Named independently of the final name: a long but valid asset name plus a
        // suffix would pass the filesystem's component limit and fail every create.
        let temporary = parent.join(format!(".publish-{}-{identifier}.new", std::process::id()));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => break (temporary, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let written = write(&mut file);
    drop(file);
    let with_cleanup = |result: io::Result<()>| match (result, std::fs::remove_file(&temporary)) {
        (Ok(()), _) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(cleanup)) => Err(io::Error::other(format!(
            "{error}; temporary file {} could not be removed: {cleanup}",
            temporary.display()
        ))),
    };
    // A failed write is the error, whatever its kind; only a failed link decides the
    // fallback, and only a destination that already exists is a refusal — every other
    // answer to `link` (FAT's EPERM, a mount's EINVAL, EMLINK) means the filesystem
    // keeps no links for this file, and the direct create is what worked before.
    if let Err(error) = written {
        return with_cleanup(Err(error));
    }
    match link(&temporary, path) {
        // The file is published whatever became of its temporary: a sibling that would
        // not go is a second name for the same bytes, not a failed create, and an error
        // here kept the prompt open on an asset that exists.
        Ok(()) => { let _ = std::fs::remove_file(&temporary); Ok(()) }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => with_cleanup(Err(already_exists(path))),
        Err(_) => { with_cleanup(Ok(()))?; create_directly(path, &write) }
    }
}

/// The direct exclusive create for a filesystem without hard links: a write that fails
/// after the create (a full disk) must not leave a half file, which would block every
/// retry with `AlreadyExists` and parse as a broken asset, so the cleanup removes only
/// the file this call created, by its inode — another program may have replaced the
/// path in the meantime. A crash mid-write can still leave one; that is the price of
/// those filesystems, and the linked publish above avoids it everywhere else.
#[cfg(not(target_arch = "wasm32"))]
fn create_directly(path: &Path, write: &impl Fn(&mut std::fs::File) -> io::Result<()>) -> io::Result<()> {
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    if let Err(error) = write(&mut file) {
        return Err(match remove_if_same_file(&file, path) {
            Ok(()) => error,
            Err(cleanup) => io::Error::new(error.kind(), format!("{error}; the half-written {} could not be removed: {cleanup}", path.display())),
        });
    }
    Ok(())
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

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn rename_native(
    from: &Path,
    to: &Path,
    before_link: impl FnOnce() -> io::Result<()>,
    link: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    if std::fs::symlink_metadata(to).is_ok() {
        return Err(already_exists(to));
    }
    let metadata = std::fs::symlink_metadata(from)?;
    if metadata.is_dir() {
        before_link()?;
        return rename_directory(from, to);
    }
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "only a regular file or directory can be renamed",
        ));
    }
    #[cfg(unix)]
    {
        before_link()?;
        link(from, to).map_err(safe_link_error)?;
        remove_linked_source(from, to)
    }
    #[cfg(not(unix))]
    {
        let _ = (before_link, link);
        Err(safe_link_error(io::Error::new(
            io::ErrorKind::Unsupported,
            "stable file identity is unavailable on this target",
        )))
    }
}

/// POSIX `rename` replaces an empty destination directory, so the destination is claimed
/// first with an exclusive `create_dir` — a folder that appeared there after the preflight
/// fails that create — and the rename then replaces only the empty folder this call made.
/// Windows refuses an existing destination on its own.
#[cfg(not(target_arch = "wasm32"))]
fn rename_directory(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::fs::create_dir(to).map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists => already_exists(to),
            _ => error,
        })?;
        if let Err(error) = std::fs::rename(from, to) {
            let _ = std::fs::remove_dir(to);
            return Err(error);
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::rename(from, to)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn safe_link_error(error: io::Error) -> io::Error {
    match error.kind() {
        io::ErrorKind::PermissionDenied
        | io::ErrorKind::Unsupported
        | io::ErrorKind::CrossesDevices => io::Error::new(
            error.kind(),
            format!("this filesystem cannot rename safely; a rename in your file manager works, but the editor cannot follow it — references, the LICENSE stanza and the open document stay as they are: {error}"),
        ),
        _ => error,
    }
}

#[cfg(unix)]
fn remove_linked_source(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::unix::fs::MetadataExt;
    let source = std::fs::symlink_metadata(from)?;
    let destination = std::fs::symlink_metadata(to)?;
    if source.ino() != destination.ino()
        || source.dev() != destination.dev()
        || source.len() != destination.len()
    {
        return Err(io::Error::other(format!(
            "another program replaced {}; both {} and {} were left in place",
            from.display(),
            from.display(),
            to.display()
        )));
    }
    std::fs::remove_file(from)
}

fn already_exists(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("{} already exists", path.display()),
    )
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
