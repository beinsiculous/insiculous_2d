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
fn test_mem_fs_file_size_is_the_stored_length_and_a_missing_file_is_not_found() {
    let mut fs = MemFs::default();
    let file_path = Path::new(BASE).join("music/theme.mid");
    fs.insert(file_path.to_string_lossy().into_owned(), vec![0; 1234]);

    assert_eq!(fs.file_size(&file_path).ok(), Some(1234));
    assert_eq!(
        fs.file_size(&Path::new(BASE).join("music/missing.mid")).map_err(|error| error.kind()),
        Err(io::ErrorKind::NotFound)
    );
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

    let dropped = fs.remove_prefix(Path::new(&format!("{BASE}/project1")));

    assert_eq!(
        dropped,
        vec![
            PathBuf::from(format!("{BASE}/project1/scene.ron")),
            PathBuf::from(format!("{BASE}/project1/scripts/main.rhai")),
        ],
        "each dropped key is reported once, so the remove observer fires once per file"
    );
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
fn test_vfs_native_file_size_is_known_without_reading_and_a_missing_file_is_not_found() -> io::Result<()> {
    let directory = TempDir::new("vfs_size")?;
    let path = directory.path().join("sample.wav");
    write(&path, &[7; 4096])?;

    assert_eq!(file_size(&path)?, 4096);
    assert_eq!(
        file_size(&directory.path().join("missing.wav")).map_err(|error| error.kind()),
        Err(io::ErrorKind::NotFound)
    );
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

#[test]
fn test_change_stamp_changes_when_the_bytes_change_and_a_missing_file_is_not_found() -> io::Result<()> {
    let directory = TempDir::new("vfs_change_stamp")?;
    let file_path = directory.path().join("music/bank.sf2");
    write(&file_path, b"first")?;
    let first = change_stamp(&file_path)?;
    assert!(first.is_some(), "a native file has a modified time");
    assert_eq!(change_stamp(&file_path)?, first, "an untouched file keeps its stamp");

    write(&file_path, b"rewritten, longer")?;
    let second = change_stamp(&file_path)?;
    assert_ne!(second, first, "a rewrite at another length is a new stamp");
    assert_eq!(second.map(|stamp| stamp.length_bytes), Some(17));

    assert_eq!(
        change_stamp(&directory.path().join("absent.wav")).map_err(|error| error.kind()),
        Err(io::ErrorKind::NotFound)
    );
    Ok(())
}

#[test]
fn test_mem_fs_keeps_no_change_stamp_and_a_missing_file_is_not_found() {
    let mut fs = MemFs::default();
    let file_path = Path::new(BASE).join("music/bank.sf2");
    fs.insert(file_path.to_string_lossy().into_owned(), vec![1, 2, 3]);

    assert_eq!(fs.change_stamp(&file_path).ok(), Some(None), "the map holds bytes only");
    assert_eq!(
        fs.change_stamp(&Path::new(BASE).join("music/absent.sf2")).map_err(|error| error.kind()),
        Err(io::ErrorKind::NotFound)
    );
}

#[test]
fn test_native_remove_prefix_never_notifies_the_remove_observer() -> io::Result<()> {
    use std::cell::Cell;
    thread_local! {
        static REMOVE_COUNT: Cell<usize> = const { Cell::new(0) };
    }
    fn on_remove(_path: &Path) {
        REMOVE_COUNT.with(|count| count.set(count.get() + 1));
    }
    set_remove_observer(on_remove);

    let directory = TempDir::new("vfs_remove_notify")?;
    let path = directory.path().join("removed.txt");
    write_string(&path, "gone")?;
    remove_prefix(&path)?;

    assert!(read(&path).is_err(), "the file is removed from disk");
    assert_eq!(
        REMOVE_COUNT.with(Cell::get),
        0,
        "the filesystem is the only copy natively, so nothing is told of a removal"
    );
    Ok(())
}

#[test]
fn test_directory_entries_include_empty_folders_and_direct_files_only() -> io::Result<()> {
    let directory = TempDir::new("vfs_entries")?;
    let root = directory.path();
    create_dir_all(&root.join("empty"))?;
    write(&root.join("nested/deeper/file.rhai"), b"script")?;
    write(&root.join("root.png"), b"image")?;
    assert_eq!(list_entries(root)?, vec![
        DirEntry { path: root.join("empty"), is_dir: true },
        DirEntry { path: root.join("nested"), is_dir: true },
        DirEntry { path: root.join("root.png"), is_dir: false },
    ]);
    assert!(exists(&root.join("empty")));
    assert!(exists(&root.join("root.png")));
    assert!(!exists(&root.join("absent")));
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_directory_entries_follow_the_root_but_never_a_file_or_directory_symlink_entry() -> io::Result<()> {
    let directory = TempDir::new("vfs_entry_symlinks")?;
    let root = directory.path().join("root");
    write(&root.join("real/file.png"), b"image")?;
    std::os::unix::fs::symlink(root.join("real/file.png"), root.join("file_link"))?;
    std::os::unix::fs::symlink(root.join("real"), root.join("directory_link"))?;
    std::os::unix::fs::symlink(&root, directory.path().join("root_link"))?;
    assert_eq!(list_entries(&root)?, vec![DirEntry { path: root.join("real"), is_dir: true }]);
    // A project whose asset root is itself a link still lists, as `list_files` lists it.
    let root_link = directory.path().join("root_link");
    assert_eq!(list_entries(&root_link)?, vec![DirEntry { path: root_link.join("real"), is_dir: true }]);
    Ok(())
}

#[test]
fn test_memory_directory_entries_derive_each_folder_once_and_include_keep_files() -> io::Result<()> {
    let mut filesystem = MemFs::default();
    let root = Path::new(BASE);
    filesystem.insert(root.join("sprites/props/hero.png").to_string_lossy().into_owned(), vec![1]);
    filesystem.insert(root.join("sprites/props/coin.png").to_string_lossy().into_owned(), vec![2]);
    filesystem.insert(root.join("root.scene.ron").to_string_lossy().into_owned(), vec![3]);
    filesystem.create_dir_all(&root.join("scenes/levels"))?;
    assert_eq!(filesystem.list_entries(root)?, vec![
        DirEntry { path: root.join("root.scene.ron"), is_dir: false },
        DirEntry { path: root.join("scenes"), is_dir: true },
        DirEntry { path: root.join("sprites"), is_dir: true },
    ]);
    assert_eq!(filesystem.list_entries(&root.join("scenes/levels"))?, vec![
        DirEntry { path: root.join("scenes/levels/.keep"), is_dir: false },
    ]);
    assert_eq!(filesystem.read(&root.join("scenes/levels/.keep"))?, b"");
    assert!(filesystem.exists(&root.join("scenes/levels")));
    assert!(filesystem.exists(&root.join("root.scene.ron")));
    assert!(!filesystem.exists(&root.join("scene")), "directory prefixes have a boundary");
    assert!(!filesystem.exists(&root.join("absent")));
    Ok(())
}

#[test]
fn test_write_new_refuses_live_paths_preserves_bytes_and_notifies_only_success() -> io::Result<()> {
    thread_local! {
        static CREATED_PATHS: std::cell::RefCell<Vec<PathBuf>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    fn on_write(path: &Path) {
        CREATED_PATHS.with(|paths| paths.borrow_mut().push(path.to_path_buf()));
    }
    let directory = TempDir::new("vfs_exclusive")?;
    let path = directory.path().join("new.rhai");
    set_write_observer(on_write);
    write_new(&path, b"first")?;
    assert_eq!(write_new(&path, b"replacement").map_err(|error| error.kind()), Err(io::ErrorKind::AlreadyExists));
    assert_eq!(read(&path)?, b"first");
    assert_eq!(CREATED_PATHS.with(|paths| paths.borrow().clone()), vec![path]);
    assert_eq!(write_new(directory.path(), b"file").map_err(|error| error.kind()), Err(io::ErrorKind::AlreadyExists));
    Ok(())
}

#[test]
fn test_memory_exclusive_create_refuses_live_files_and_directory_collisions_but_recreates_removed_paths() -> io::Result<()> {
    let mut filesystem = MemFs::default();
    let root = Path::new(BASE);
    filesystem.create_dir_all(&root.join("scripts"))?;
    let path = root.join("scripts/hero.rhai");
    filesystem.write_new(&path, b"first")?;
    assert_eq!(filesystem.write_new(&path, b"replacement").map_err(|error| error.kind()), Err(io::ErrorKind::AlreadyExists));
    assert_eq!(filesystem.read(&path)?, b"first");
    assert_eq!(filesystem.write_new(&root.join("scripts"), b"file").map_err(|error| error.kind()), Err(io::ErrorKind::AlreadyExists));
    assert_eq!(filesystem.write_new(&path.join("child"), b"file").map_err(|error| error.kind()), Err(io::ErrorKind::NotADirectory));
    assert_eq!(filesystem.create_dir_all(&path.join("child")).map_err(|error| error.kind()), Err(io::ErrorKind::NotADirectory));
    assert_eq!(filesystem.write_new(&root.join("absent/child"), b"file").map_err(|error| error.kind()), Err(io::ErrorKind::NotFound));
    filesystem.remove_prefix(&path);
    assert!(!filesystem.exists(&path));
    filesystem.write_new(&path, b"recreated")?;
    assert_eq!(filesystem.read(&path)?, b"recreated");
    Ok(())
}

#[test]
fn test_directory_and_exclusive_create_operations_refuse_empty_paths() {
    let mut filesystem = MemFs::default();
    let empty = Path::new("");
    assert_eq!(list_entries(empty).map_err(|error| error.kind()), Err(io::ErrorKind::InvalidInput));
    assert_eq!(create_dir_all(empty).map_err(|error| error.kind()), Err(io::ErrorKind::InvalidInput));
    assert_eq!(write_new(empty, b"file").map_err(|error| error.kind()), Err(io::ErrorKind::InvalidInput));
    assert_eq!(filesystem.list_entries(empty).map_err(|error| error.kind()), Err(io::ErrorKind::InvalidInput));
    assert_eq!(filesystem.create_dir_all(empty).map_err(|error| error.kind()), Err(io::ErrorKind::InvalidInput));
    assert_eq!(filesystem.write_new(empty, b"file").map_err(|error| error.kind()), Err(io::ErrorKind::InvalidInput));
}
