//! The boot aliases over a `MemFs` laid out as the browser lays it: the bundle's preload, the
//! aliases, then the stored records as `web_entry.rs` step 6 lays them. Every stored record is
//! written and read back through a `MemoryStore`, so a fixture is spelled as the store writes it,
//! and the stored scene is the bundled pong's own scene from before the rename.

use std::cell::RefCell;
use std::path::Path;

use common::vfs::MemFs;

use super::{lay_aliases, selected_aliases, LEGACY_PATHS};
use crate::projects::{ProjectManifest, ProjectOrigin};
use crate::store::memory::MemoryStore;
use crate::store::{ProjectStore, StoredFile};

const PONG_ROOT: &str = "/playground/v5/assets/projects/pong";
const EXAMPLES_ROOT: &str = "/playground/v5/assets/projects/examples";
const OLD_PADDLE: &str = "assets/images/paddle_16px.png";
const NEW_PADDLE: &str = "assets/sprites/paddle_16px.png";
const OLD_BALL: &str = "assets/images/ball_8px.png";
const PONG_SCENE: &str = "assets/scenes/pong.scene.ron";
/// The bundled pong's scene as it was before the rename, naming `images/` paths.
const PRE_CONVENTION_PONG_SCENE: &[u8] = include_bytes!("../../tests/fixtures/pre_convention_pong.scene.ron");
const CURRENT_PONG_SCENE: &[u8] = include_bytes!("../../assets/projects/pong/assets/scenes/pong.scene.ron");
const ENGINE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn manifest(slug: &str, bundle_version: &str) -> ProjectManifest {
    ProjectManifest {
        slug: slug.to_string(),
        title: slug.to_string(),
        bundle_version: bundle_version.to_string(),
        content_hash: String::new(),
        origin: ProjectOrigin::Saved,
    }
}

/// A project's store, written as the persistence chains write it: a first put at base 0, a
/// removal at the record's current revision (0 for a bundled-only file), the manifest upserted.
struct StoredProject {
    store: MemoryStore,
    manifest: ProjectManifest,
}

impl StoredProject {
    fn new(slug: &str, bundle_version: &str) -> Self {
        Self { store: MemoryStore::new(), manifest: manifest(slug, bundle_version) }
    }

    fn put(&self, path: &str, bytes: &[u8]) -> u64 {
        let file = StoredFile {
            project: self.manifest.slug.clone(),
            path: path.to_string(),
            bytes: bytes.to_vec(),
            revision: 0,
            bundle_version: self.manifest.bundle_version.clone(),
            deleted: false,
        };
        pollster::block_on(self.store.put(file, 0, &self.manifest)).expect("the put lands")
    }

    fn remove(&self, path: &str, base_revision: u64) {
        pollster::block_on(self.store.remove_file(&self.manifest.slug, path, base_revision, &self.manifest))
            .expect("the removal lands");
    }

    fn files(&self) -> Vec<StoredFile> {
        pollster::block_on(self.store.load_project(&self.manifest.slug)).expect("the project loads")
    }
}

/// The bundle's preload: every file of the project's tracked tree under its root key.
fn preloaded(root: &str, project_directory: &Path, leave_out: &[&str]) -> RefCell<MemFs> {
    let mut filesystem = MemFs::default();
    for file in common::vfs::list_files(project_directory).expect("the bundled project lists") {
        let relative = file.strip_prefix(project_directory).expect("under the project").to_string_lossy().replace('\\', "/");
        if leave_out.contains(&relative.as_str()) {
            continue;
        }
        filesystem.insert(format!("{root}/{relative}"), std::fs::read(&file).expect("a bundled file"));
    }
    RefCell::new(filesystem)
}

fn bundled_pong() -> RefCell<MemFs> {
    preloaded(PONG_ROOT, &Path::new(ENGINE_ROOT).join("crates/playground/assets/projects/pong"), &[])
}

/// Step 6 as `web_entry.rs` runs it: the aliases, then each stored record laid over them — a
/// live one inserted, a tombstone dropping its path.
fn boot(filesystem: &RefCell<MemFs>, root: &str, slug: &str, stored_files: &[StoredFile]) {
    lay_aliases(
        root,
        &selected_aliases(slug, stored_files),
        |path| filesystem.borrow().read(path),
        |path| filesystem.borrow().exists(path),
        |key, bytes| filesystem.borrow_mut().insert(key, bytes),
    );
    for file in stored_files {
        let key = format!("{root}/{}", file.path);
        if file.deleted {
            filesystem.borrow_mut().remove_prefix(Path::new(&key));
        } else {
            filesystem.borrow_mut().insert(key, file.bytes.clone());
        }
    }
}

fn read(filesystem: &RefCell<MemFs>, root: &str, relative: &str) -> Option<Vec<u8>> {
    filesystem.borrow().read(Path::new(&format!("{root}/{relative}"))).ok()
}

#[test]
fn a_stored_pre_convention_pong_scene_reads_its_images_paths_as_the_bundles_sprites() {
    let project = StoredProject::new("pong", "v4");
    project.put(PONG_SCENE, PRE_CONVENTION_PONG_SCENE);
    let filesystem = bundled_pong();
    boot(&filesystem, PONG_ROOT, "pong", &project.files());

    let bundle_paddle = read(&filesystem, PONG_ROOT, NEW_PADDLE).expect("the bundle's paddle");
    assert_eq!(read(&filesystem, PONG_ROOT, OLD_PADDLE), Some(bundle_paddle), "the scene's images/paddle_16px.png loads");
    assert!(read(&filesystem, PONG_ROOT, OLD_BALL).is_some(), "and so does its ball");
    assert!(read(&filesystem, PONG_ROOT, "assets/images/paddle_16px.sheet.ron").is_none(), "a new sidecar is never aliased");
}

#[test]
fn the_same_stored_scene_under_a_manifest_stamped_v5_is_aliased_all_the_same() {
    let project = StoredProject::new("pong", "v5");
    project.put(PONG_SCENE, PRE_CONVENTION_PONG_SCENE);
    let filesystem = bundled_pong();
    boot(&filesystem, PONG_ROOT, "pong", &project.files());

    assert_eq!(read(&filesystem, PONG_ROOT, OLD_PADDLE), read(&filesystem, PONG_ROOT, NEW_PADDLE));
}

#[test]
fn a_stored_customised_paddle_at_the_old_key_wins_over_the_alias() {
    let project = StoredProject::new("pong", "v4");
    project.put(PONG_SCENE, PRE_CONVENTION_PONG_SCENE);
    project.put(OLD_PADDLE, b"the visitor's own paddle");
    let filesystem = bundled_pong();
    boot(&filesystem, PONG_ROOT, "pong", &project.files());

    assert_eq!(read(&filesystem, PONG_ROOT, OLD_PADDLE).as_deref(), Some(&b"the visitor's own paddle"[..]));
}

#[test]
fn a_stored_tombstone_at_the_old_key_leaves_it_absent() {
    let project = StoredProject::new("pong", "v4");
    project.put(PONG_SCENE, PRE_CONVENTION_PONG_SCENE);
    project.remove(OLD_PADDLE, 0);
    let filesystem = bundled_pong();
    boot(&filesystem, PONG_ROOT, "pong", &project.files());

    assert!(read(&filesystem, PONG_ROOT, OLD_PADDLE).is_none(), "an old key the visitor deleted stays deleted");
    assert!(read(&filesystem, PONG_ROOT, OLD_BALL).is_some(), "the scene's other old key is aliased");
}

#[test]
fn a_tombstoned_scene_selects_nothing() {
    let project = StoredProject::new("pong", "v4");
    let revision = project.put(PONG_SCENE, PRE_CONVENTION_PONG_SCENE);
    project.remove(PONG_SCENE, revision);
    let stored_files = project.files();
    assert!(stored_files.iter().all(|file| file.deleted), "the store holds the scene's tombstone");

    assert!(selected_aliases("pong", &stored_files).is_empty());
    let filesystem = bundled_pong();
    boot(&filesystem, PONG_ROOT, "pong", &stored_files);
    assert!(read(&filesystem, PONG_ROOT, OLD_PADDLE).is_none());
}

#[test]
fn a_scene_naming_only_sprites_paths_a_project_with_nothing_stored_and_an_unbundled_slug_get_no_old_key() {
    let project = StoredProject::new("pong", "v5");
    project.put(PONG_SCENE, CURRENT_PONG_SCENE);
    let filesystem = bundled_pong();
    boot(&filesystem, PONG_ROOT, "pong", &project.files());
    assert!(read(&filesystem, PONG_ROOT, OLD_PADDLE).is_none(), "a scene naming sprites/ alone");
    assert!(read(&filesystem, PONG_ROOT, OLD_BALL).is_none());

    assert!(selected_aliases("pong", &[]).is_empty(), "nothing stored");
    let elsewhere = StoredProject::new("my-game", "v4");
    elsewhere.put(PONG_SCENE, PRE_CONVENTION_PONG_SCENE);
    assert!(selected_aliases("my-game", &elsewhere.files()).is_empty(), "a project the bundle never shipped");
}

#[test]
fn a_stored_script_naming_the_old_jump_sound_selects_the_examples_row() {
    let project = StoredProject::new("examples", "v4");
    project.put("assets/scripts/jumper.rhai", b"fn update(me, view, params, cmd, dt) {\n    let sound = \"sounds/snd_jump.wav\";\n}\n");
    let filesystem = preloaded(EXAMPLES_ROOT, &Path::new(ENGINE_ROOT).join("examples"), &[]);
    boot(&filesystem, EXAMPLES_ROOT, "examples", &project.files());

    assert_eq!(selected_aliases("examples", &project.files()), [("assets/sounds/snd_jump.wav", "assets/sounds/jump.wav")]);
    assert_eq!(
        read(&filesystem, EXAMPLES_ROOT, "assets/sounds/snd_jump.wav"),
        read(&filesystem, EXAMPLES_ROOT, "assets/sounds/jump.wav"),
    );
}

#[test]
fn a_row_whose_bundle_file_is_missing_inserts_nothing() {
    let project = StoredProject::new("pong", "v4");
    project.put(PONG_SCENE, PRE_CONVENTION_PONG_SCENE);
    let filesystem = preloaded(
        PONG_ROOT,
        &Path::new(ENGINE_ROOT).join("crates/playground/assets/projects/pong"),
        &[NEW_PADDLE],
    );
    boot(&filesystem, PONG_ROOT, "pong", &project.files());

    assert!(read(&filesystem, PONG_ROOT, OLD_PADDLE).is_none());
    assert!(read(&filesystem, PONG_ROOT, OLD_BALL).is_some(), "the row whose file is there is aliased");
}

#[test]
fn a_stored_scene_named_in_capitals_selects_as_its_lowercase_twin_does() {
    let project = StoredProject::new("pong", "v4");
    project.put("assets/scenes/LEVEL.SCENE.RON", PRE_CONVENTION_PONG_SCENE);
    let filesystem = bundled_pong();
    boot(&filesystem, PONG_ROOT, "pong", &project.files());

    assert_eq!(
        selected_aliases("pong", &project.files()),
        [(OLD_BALL, "assets/sprites/ball_8px.png"), (OLD_PADDLE, NEW_PADDLE)],
    );
    assert!(read(&filesystem, PONG_ROOT, OLD_PADDLE).is_some());
}

/// The stated cost, pinned so it is never mistaken for a bug: with no stored scene naming the
/// old key, a customised old-key file stays the visitor's and the bundle's new file stays stock.
#[test]
fn a_customised_old_paddle_without_a_stored_scene_keeps_its_bytes_beside_the_stock_sprite() {
    let project = StoredProject::new("pong", "v4");
    project.put(OLD_PADDLE, b"the visitor's own paddle");
    let filesystem = bundled_pong();
    boot(&filesystem, PONG_ROOT, "pong", &project.files());

    assert_eq!(read(&filesystem, PONG_ROOT, OLD_PADDLE).as_deref(), Some(&b"the visitor's own paddle"[..]));
    let stock = std::fs::read(Path::new(ENGINE_ROOT).join("crates/playground/assets/projects/pong").join(NEW_PADDLE))
        .expect("the bundled paddle");
    assert_eq!(read(&filesystem, PONG_ROOT, NEW_PADDLE), Some(stock));
}

/// The table read against the tree: every row's bundle file is there and no old key survives.
/// The template is a sibling repository, absent from a standalone engine checkout, so its row is
/// checked only when it is there.
#[test]
fn every_row_names_a_bundle_file_in_the_tree_and_no_old_key_is_left_there() {
    let engine_root = Path::new(ENGINE_ROOT);
    let template = engine_root.join("../games/game-template");
    for row in LEGACY_PATHS {
        let project_directory = match row.slug {
            "examples" => engine_root.join("examples"),
            "pong" => engine_root.join("crates/playground/assets/projects/pong"),
            "game-template" if template.is_dir() => template.clone(),
            "game-template" => continue,
            other => panic!("a row for {other}, which the bundle does not carry"),
        };
        assert!(project_directory.join(row.bundle_file).is_file(), "{}: {} is in the tree", row.slug, row.bundle_file);
        assert!(!project_directory.join(row.old_key).exists(), "{}: {} has moved", row.slug, row.old_key);
    }
}
