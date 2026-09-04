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
        relative_miss.map_err(|error| error.kind()),
        Err(io::ErrorKind::NotFound),
        "a relative key must miss: only the base-joined form is canonical"
    );
    assert_eq!(
        invalid_utf8.map_err(|error| error.kind()),
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
    assert_eq!(list_files(Path::new("")).map_err(|error| error.kind()), Err(io::ErrorKind::InvalidInput));
    assert_eq!(remove_prefix(Path::new("")).map_err(|error| error.kind()), Err(io::ErrorKind::InvalidInput));
}

struct TempDir(PathBuf);
impl TempDir {
    fn new(prefix: &str) -> io::Result<Self> {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let identifier = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "insiculous_{prefix}_{}_{}",
            std::process::id(),
            identifier
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
    let directory = TempDir::new("vfs_rw")?;
    let path = directory.path().join("nested/dir/test.txt");

    write_string(&path, "hello vfs")?;
    assert_eq!(read_to_string(&path)?, "hello vfs");
    assert_eq!(read(&path)?, b"hello vfs");

    write(&path, b"updated")?;
    assert_eq!(read(&path)?, b"updated");
    Ok(())
}

#[test]
fn test_vfs_native_list_files_and_remove_prefix() -> io::Result<()> {
    let directory = TempDir::new("vfs_list_remove")?;
    let project_one = directory.path().join("p1");
    let project_two = directory.path().join("p2");

    write_string(&project_one.join("b.txt"), "b")?;
    write_string(&project_one.join("sub/c.txt"), "c")?;
    write_string(&project_one.join("a.txt"), "a")?;
    write_string(&project_two.join("x.txt"), "x")?;

    let listed = list_files(&project_one)?;
    assert_eq!(
        listed,
        vec![project_one.join("a.txt"), project_one.join("b.txt"), project_one.join("sub/c.txt")]
    );

    remove_prefix(&project_one)?;
    assert!(!project_one.exists());
    assert!(project_two.join("x.txt").exists());

    // Removing already-absent prefix is an idempotent Ok(())
    remove_prefix(&project_one)?;

    // A regular file at the prefix is removed too, matching the wasm map's
    // exact-key removal instead of failing with NotADirectory.
    remove_prefix(&project_two.join("x.txt"))?;
    assert!(!project_two.join("x.txt").exists());
    assert!(project_two.exists(), "only the file went, not its parent");
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_vfs_list_files_never_follows_symlinks() -> io::Result<()> {
    let directory = TempDir::new("vfs_symlink")?;
    let root = directory.path().join("root");
    let sub = root.join("sub");
    write_string(&root.join("a.txt"), "a")?;
    write_string(&sub.join("b.txt"), "b")?;

    // Symlink loop: root/sub/loop -> root
    std::os::unix::fs::symlink(&root, sub.join("loop"))?;

    let listed = list_files(&root)?;
    assert_eq!(listed, vec![root.join("a.txt"), sub.join("b.txt")]);
    Ok(())
}

#[test]
fn test_write_string_notifies_observer_once() -> io::Result<()> {
    use std::cell::Cell;
    thread_local! {
        static NOTIFIED_PATH: Cell<Option<&'static str>> = const { Cell::new(None) };
        static NOTIFY_COUNT: Cell<usize> = const { Cell::new(0) };
    }
    fn on_write(_path: &Path) {
        NOTIFY_COUNT.with(|count| count.set(count.get() + 1));
    }
    set_write_observer(on_write);

    let directory = TempDir::new("vfs_notify")?;
    let path = directory.path().join("observer_test.txt");

    NOTIFY_COUNT.with(|count| count.set(0));
    write_string(&path, "test")?;

    assert_eq!(NOTIFY_COUNT.with(|count| count.get()), 1);
    Ok(())
}

#[test]
fn test_memfs_key_story_bundled_edit_and_relative_miss() -> io::Result<()> {
    let mut fs = MemFs::default();
    let asset_base = "/playground/v1/assets";
    let root = format!("{asset_base}/projects/examples");
    let scene_key = format!("{root}/assets/scenes/behavior_demo.scene.ron");

    // 1. Insert bundled file under the key the build script's copy produces
    fs.insert(scene_key.clone(), b"bundled content".to_vec());
    let lookup_path = Path::new(&root).join("assets").join("scenes/behavior_demo.scene.ron");
    assert_eq!(fs.read_to_string(&lookup_path)?, "bundled content");

    // 2. Overwrite edit at the same key
    fs.insert(scene_key.clone(), b"edited content".to_vec());
    assert_eq!(fs.read_to_string(&lookup_path)?, "edited content");

    // 3. Confirm a relative key never resolves
    let relative_path = Path::new("assets/scenes/behavior_demo.scene.ron");
    assert!(fs.read(relative_path).is_err());
    let relative_scenes_path = Path::new("scenes/behavior_demo.scene.ron");
    assert!(fs.read(relative_scenes_path).is_err());
    Ok(())
}
