use super::*;
use std::cell::RefCell;
use std::io::Write;

struct TempDirectory(PathBuf);
impl TempDirectory {
    fn new() -> io::Result<Self> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let identifier = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "insiculous_vfs_mutate_{}_{identifier}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)?;
        Ok(Self(path))
    }
}
impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn test_file_removal_does_not_remove_a_directory_or_a_prefix_sibling() -> io::Result<()> {
    let temporary = TempDirectory::new()?;
    write(&temporary.0.join("hero.png"), b"image")?;
    write(&temporary.0.join("hero.png.backup"), b"backup")?;
    remove_file(&temporary.0.join("hero.png"))?;
    assert!(!temporary.0.join("hero.png").exists());
    assert_eq!(read(&temporary.0.join("hero.png.backup"))?, b"backup");
    assert!(remove_file(&temporary.0).is_err());
    let mut filesystem = MemFs::default();
    filesystem.create_dir_all(Path::new("/assets/sprites"))?;
    filesystem.write_new(Path::new("/assets/sprites/hero.png"), b"image")?;
    filesystem.remove_file(Path::new("/assets/sprites/hero.png"))?;
    assert_eq!(
        filesystem
            .remove_file(Path::new("/assets/sprites"))
            .map_err(|error| error.kind()),
        Err(io::ErrorKind::IsADirectory)
    );
    Ok(())
}

#[test]
fn test_rename_refuses_an_existing_file_or_empty_directory() -> io::Result<()> {
    let temporary = TempDirectory::new()?;
    let source = temporary.0.join("source.rhai");
    let destination = temporary.0.join("destination.rhai");
    write(&source, b"source")?;
    write(&destination, b"destination")?;
    assert_eq!(
        rename(&source, &destination).map_err(|error| error.kind()),
        Err(io::ErrorKind::AlreadyExists)
    );
    assert_eq!(read(&source)?, b"source");
    assert_eq!(read(&destination)?, b"destination");
    create_dir_all(&temporary.0.join("source"))?;
    create_dir_all(&temporary.0.join("destination"))?;
    assert_eq!(
        rename(
            &temporary.0.join("source"),
            &temporary.0.join("destination")
        )
        .map_err(|error| error.kind()),
        Err(io::ErrorKind::AlreadyExists)
    );
    assert!(temporary.0.join("source").is_dir());
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_rename_preserves_a_destination_created_between_preflight_and_link() -> io::Result<()> {
    let temporary = TempDirectory::new()?;
    let source = temporary.0.join("hero.png");
    let destination = temporary.0.join("deion.png");
    write(&source, b"source")?;
    let result = entries::rename_native(
        &source,
        &destination,
        || std::fs::write(&destination, b"external"),
        |from, to| std::fs::hard_link(from, to),
    );
    assert_eq!(
        result.map_err(|error| error.kind()),
        Err(io::ErrorKind::AlreadyExists)
    );
    assert_eq!(read(&source)?, b"source");
    assert_eq!(read(&destination)?, b"external");
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_rename_preserves_an_empty_folder_created_between_preflight_and_rename() -> io::Result<()> {
    let temporary = TempDirectory::new()?;
    let source = temporary.0.join("props");
    let destination = temporary.0.join("scenery");
    std::fs::create_dir(&source)?;
    write(&source.join("crate.png"), b"crate")?;
    let result = entries::rename_native(
        &source,
        &destination,
        || std::fs::create_dir(&destination),
        |from, to| std::fs::hard_link(from, to),
    );
    assert_eq!(
        result.map_err(|error| error.kind()),
        Err(io::ErrorKind::AlreadyExists)
    );
    assert_eq!(read(&source.join("crate.png"))?, b"crate");
    assert!(destination.is_dir() && list_entries(&destination)?.is_empty(), "the folder that appeared is left as it was");
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_rename_keeps_a_source_replaced_before_the_identity_check() -> io::Result<()> {
    let temporary = TempDirectory::new()?;
    let source = temporary.0.join("hero.png");
    let destination = temporary.0.join("deion.png");
    write(&source, b"original")?;
    let result = entries::rename_native(
        &source,
        &destination,
        || Ok(()),
        |from, to| {
            std::fs::hard_link(from, to)?;
            let replacement = temporary.0.join("external.png");
            std::fs::write(&replacement, b"external")?;
            std::fs::rename(replacement, from)
        },
    );
    assert!(result.unwrap_err().to_string().contains("both"));
    assert_eq!(read(&source)?, b"external");
    assert_eq!(read(&destination)?, b"original");
    Ok(())
}

#[test]
fn test_rename_refuses_unsupported_hard_links_without_copying_or_removing() -> io::Result<()> {
    let temporary = TempDirectory::new()?;
    let source = temporary.0.join("hero.png");
    let destination = temporary.0.join("deion.png");
    write(&source, b"source")?;
    let result = entries::rename_native(
        &source,
        &destination,
        || Ok(()),
        |_, _| {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "test filesystem rejects links",
            ))
        },
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("this filesystem cannot rename safely"));
    assert_eq!(read(&source)?, b"source");
    assert!(!destination.exists());
    Ok(())
}

#[test]
fn test_exclusive_publish_never_exposes_a_partial_file_under_the_final_name() -> io::Result<()> {
    let temporary = TempDirectory::new()?;
    let destination = temporary.0.join("scene.ron");
    let result = entries::publish_new_with(&destination, |file| {
        file.write_all(b"partial")?;
        assert!(!destination.exists());
        Err(io::Error::other("injected full disk"))
    }, |from, to| std::fs::hard_link(from, to));
    assert!(result.is_err());
    assert!(!destination.exists());
    assert!(
        list_entries(&temporary.0)?.is_empty(),
        "the failed temporary sibling was cleaned"
    );
    write_new(&destination, b"complete")?;
    assert_eq!(read(&destination)?, b"complete");
    assert_eq!(list_entries(&temporary.0)?.len(), 1);
    Ok(())
}

#[test]
fn test_exclusive_publish_creates_directly_where_the_filesystem_refuses_links() -> io::Result<()> {
    let temporary = TempDirectory::new()?;
    let destination = temporary.0.join("scene.ron");
    let no_links = |_: &Path, _: &Path| Err(io::Error::new(io::ErrorKind::PermissionDenied, "this filesystem keeps no links"));
    entries::publish_new_with(&destination, |file| file.write_all(b"complete"), no_links)?;
    assert_eq!(read(&destination)?, b"complete");
    assert_eq!(list_entries(&temporary.0)?.len(), 1, "no temporary sibling stays behind");
    let refused = entries::publish_new_with(&destination, |file| file.write_all(b"again"), no_links);
    assert_eq!(refused.map_err(|error| error.kind()), Err(io::ErrorKind::AlreadyExists));
    assert_eq!(read(&destination)?, b"complete", "the direct create is exclusive too");
    let other = temporary.0.join("script.rhai");
    let mount_answer = |_: &Path, _: &Path| Err(io::Error::new(io::ErrorKind::InvalidInput, "EINVAL from a mount"));
    entries::publish_new_with(&other, |file| file.write_all(b"complete"), mount_answer)?;
    assert_eq!(read(&other)?, b"complete", "any refused link falls back, not a listed kind");
    let denied = temporary.0.join("denied.rhai");
    let result = entries::publish_new_with(&denied, |_| Err(io::Error::new(io::ErrorKind::PermissionDenied, "the write itself")), |from, to| std::fs::hard_link(from, to));
    assert_eq!(result.map_err(|error| error.kind()), Err(io::ErrorKind::PermissionDenied), "a failed write is the error, never a fallback");
    assert!(!denied.exists());
    assert_eq!(list_entries(&temporary.0)?.len(), 2, "no temporary stays behind");
    Ok(())
}

#[test]
fn test_remove_dir_takes_an_empty_folder_and_refuses_one_that_holds_a_file() -> io::Result<()> {
    let temporary = TempDirectory::new()?;
    let folder = temporary.0.join("props");
    std::fs::create_dir(&folder)?;
    write(&folder.join("late.png"), b"arrived")?;
    assert!(entries::remove_dir(&folder).is_err(), "a folder that holds a file is refused");
    assert!(folder.join("late.png").exists());
    std::fs::remove_file(folder.join("late.png"))?;
    entries::remove_dir(&folder)?;
    assert!(!folder.exists());
    let mut memory = MemFs::default();
    memory.insert("/project/assets/props/late.png".into(), b"arrived".to_vec());
    assert_eq!(memory.remove_dir(Path::new("/project/assets/props")).map_err(|error| error.kind()), Err(io::ErrorKind::DirectoryNotEmpty));
    memory.remove_file(Path::new("/project/assets/props/late.png"))?;
    memory.remove_dir(Path::new("/project/assets/props"))?;
    memory.insert("/project/assets/props".into(), b"a file wearing the folder's name".to_vec());
    assert_eq!(memory.remove_dir(Path::new("/project/assets/props")).map_err(|error| error.kind()), Err(io::ErrorKind::NotADirectory), "a file at the path is refused, as natively");
    assert!(entries::remove_dir(&temporary.0.join("props.png")).is_err() || !temporary.0.join("props.png").exists());
    Ok(())
}

#[test]
fn test_memory_folder_rename_observes_every_put_before_any_remove() -> io::Result<()> {
    thread_local! {
        static CHANGES: RefCell<Vec<(bool, PathBuf)>> = const { RefCell::new(Vec::new()) };
    }
    fn written(path: &Path) {
        CHANGES.with(|changes| changes.borrow_mut().push((true, path.to_path_buf())));
    }
    fn removed(path: &Path) {
        CHANGES.with(|changes| changes.borrow_mut().push((false, path.to_path_buf())));
    }
    let filesystem = RefCell::new(MemFs::default());
    let root = Path::new("/assets");
    {
        let mut filesystem = filesystem.borrow_mut();
        filesystem.create_dir_all(&root.join("sprites/props/nested"))?;
        filesystem.write_new(&root.join("sprites/props/hero.png"), b"image")?;
        filesystem.write_new(&root.join("sprites/props/hero.sheet.ron"), b"sheet")?;
    }
    set_write_observer(written);
    set_remove_observer(removed);
    MemFs::rename_observed(
        &filesystem,
        &root.join("sprites/props"),
        &root.join("sprites/actors"),
        |path| {
            assert!(filesystem
                .borrow()
                .exists(&root.join("sprites/props/hero.png")));
            super::notify_write_observer(path);
        },
        super::notify_remove_observer,
    )?;
    let changes = CHANGES.with(|changes| changes.borrow().clone());
    assert_eq!(
        changes,
        [
            (true, root.join("sprites/actors/hero.png")),
            (true, root.join("sprites/actors/hero.sheet.ron")),
            (true, root.join("sprites/actors/nested/.keep")),
            (false, root.join("sprites/props/hero.png")),
            (false, root.join("sprites/props/hero.sheet.ron")),
            (false, root.join("sprites/props/nested/.keep")),
        ]
    );
    let filesystem = filesystem.borrow();
    assert_eq!(
        filesystem.read(&root.join("sprites/actors/hero.png"))?,
        b"image"
    );
    assert_eq!(
        filesystem.read(&root.join("sprites/actors/hero.sheet.ron"))?,
        b"sheet"
    );
    assert!(!filesystem.exists(&root.join("sprites/props")));
    Ok(())
}

#[test]
fn test_memory_rename_refuses_a_live_destination_and_a_move_inside_itself() -> io::Result<()> {
    let mut filesystem = MemFs::default();
    let root = Path::new("/assets");
    filesystem.create_dir_all(&root.join("sprites"))?;
    filesystem.write_new(&root.join("sprites/hero.png"), b"image")?;
    filesystem.write_new(&root.join("sprites/deion.png"), b"existing")?;
    assert_eq!(
        filesystem
            .rename(
                &root.join("sprites/hero.png"),
                &root.join("sprites/deion.png")
            )
            .map_err(|error| error.kind()),
        Err(io::ErrorKind::AlreadyExists)
    );
    assert_eq!(filesystem.read(&root.join("sprites/hero.png"))?, b"image");
    assert_eq!(
        filesystem.read(&root.join("sprites/deion.png"))?,
        b"existing"
    );
    assert_eq!(
        filesystem
            .rename(&root.join("sprites"), &root.join("sprites/nested"))
            .map_err(|error| error.kind()),
        Err(io::ErrorKind::InvalidInput)
    );
    Ok(())
}
