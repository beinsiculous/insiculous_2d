//! A boot that cannot read the store stops before it writes anything, and one that
//! can opens what it read. Every record here is written through the store's own `put`
//! or `replace_project`; the faults stand for what the browser's store hands back.

use super::boot::{is_move_boot, memory_fallback_refusal, plan_boot, BootPlan, OpenedProject, MOVE_BOOT_STORAGE_REFUSAL};
use super::memory::MemoryStore;
use super::{ManifestList, ProjectStore, StoredFile, StoreError};
use crate::projects::{ProjectManifest, ProjectOrigin};

const BUNDLE_VERSION: &str = "v6";
const SCENE: &str = "assets/scenes/main.scene.ron";
const EVERY_SLUG: [&str; 5] = ["examples", "pong", "mine", "third", "lost"];

fn manifest(slug: &str, origin: ProjectOrigin) -> ProjectManifest {
    ProjectManifest {
        slug: slug.to_string(),
        title: slug.to_uppercase(),
        bundle_version: BUNDLE_VERSION.to_string(),
        content_hash: String::new(),
        origin,
    }
}

fn bundled() -> Vec<ProjectManifest> {
    vec![manifest("examples", ProjectOrigin::Bundled), manifest("pong", ProjectOrigin::Bundled)]
}

fn scene(slug: &str, bytes: &[u8]) -> StoredFile {
    StoredFile {
        project: slug.to_string(),
        path: SCENE.to_string(),
        bytes: bytes.to_vec(),
        revision: 0,
        bundle_version: BUNDLE_VERSION.to_string(),
        deleted: false,
    }
}

/// A visitor's store: an edit over the bundled pong, two imported projects, and the
/// files of an import an older release interrupted before its manifest.
fn visitors_store() -> MemoryStore {
    let store = MemoryStore::new();
    pollster::block_on(async {
        store.put(scene("pong", b"pong edited"), 0, &manifest("pong", ProjectOrigin::Bundled)).await.expect("pong saved");
        for slug in ["mine", "third"] {
            let mut imported = scene(slug, format!("{slug} imported").as_bytes());
            imported.revision = 1;
            store
                .replace_project(slug, vec![imported], manifest(slug, ProjectOrigin::Imported))
                .await
                .expect("imported");
        }
        store.put(scene("lost", b"half an import"), 0, &manifest("lost", ProjectOrigin::Imported)).await.expect("lost saved");
    });
    store.lose_manifest("lost");
    store
}

/// Everything the store holds, read with every fault lifted.
fn everything(store: &MemoryStore) -> (ManifestList, Vec<Vec<StoredFile>>) {
    store.clear_read_faults();
    pollster::block_on(async {
        let manifests = store.manifests().await.expect("manifests read");
        let mut records = Vec::new();
        for slug in EVERY_SLUG {
            records.push(store.load_project(slug).await.expect("records read"));
        }
        (manifests, records)
    })
}

fn boot(store: &MemoryStore, requested: Option<&str>) -> Result<BootPlan, String> {
    pollster::block_on(plan_boot(store, &bundled(), requested))
}

fn opened(plan: Result<BootPlan, String>) -> OpenedProject {
    match plan {
        Ok(BootPlan::Open(project)) => project,
        other => panic!("expected the project to open, got {other:?}"),
    }
}

fn stored_scene(project: &OpenedProject) -> Vec<u8> {
    project.stored_files.iter().find(|file| file.path == SCENE).expect("the stored scene").bytes.clone()
}

#[test]
fn a_failed_manifest_read_stops_the_boot_with_every_record_intact() {
    let store = visitors_store();
    let before = everything(&store);

    store.fail_manifest_reads();
    let stopped = boot(&store, Some("pong")).expect_err("a failed manifest read stops the boot");
    assert!(stopped.contains("could not be read"), "{stopped}");

    assert_eq!(everything(&store), before, "nothing swept, removed or redirected: the orphan is still there");
}

#[test]
fn a_failed_file_read_stops_the_boot_with_every_record_intact() {
    let store = visitors_store();
    let before = everything(&store);

    store.mark_file_undecodable("pong", SCENE);
    let stopped = boot(&store, Some("pong")).expect_err("an unreadable record of the opened project stops the boot");
    assert!(stopped.contains("\"pong\""), "{stopped}");

    assert_eq!(everything(&store), before, "the sweep never ran and the record kept its bytes");
}

#[test]
fn two_healthy_projects_beside_an_undecodable_manifest_both_open_and_its_files_survive() {
    let store = visitors_store();
    let before = everything(&store);
    store.mark_manifest_undecodable("third");

    let pong = opened(boot(&store, Some("pong")));
    assert_eq!(stored_scene(&pong), b"pong edited");
    let mine = opened(boot(&store, Some("mine")));
    assert_eq!(stored_scene(&mine), b"mine imported");
    assert_eq!(mine.stored_manifest.map(|manifest| manifest.origin), Some(ProjectOrigin::Imported));
    assert!(
        !mine.stored_manifests.iter().any(|manifest| manifest.slug == "third"),
        "the picker lists what decoded"
    );

    assert_eq!(
        everything(&store),
        before,
        "the third project's files survive, and no sweep ran while a manifest would not decode"
    );
}

#[test]
fn an_unknown_slug_while_a_manifest_will_not_decode_stops_instead_of_redirecting() {
    let store = visitors_store();
    let before = everything(&store);
    store.mark_manifest_undecodable("third");

    for requested in ["third", "nowhere"] {
        let stopped = boot(&store, Some(requested)).expect_err("the requested slug may be the undecodable one");
        assert!(stopped.contains(requested), "{stopped}");
    }

    assert_eq!(everything(&store), before);
}

#[test]
fn a_bundled_project_whose_own_manifest_will_not_decode_stops_the_boot_even_with_no_files() {
    let store = MemoryStore::new();
    pollster::block_on(store.replace_project("examples", Vec::new(), manifest("examples", ProjectOrigin::Bundled)))
        .expect("a manifest with no files");
    store.mark_manifest_undecodable("examples");

    for requested in [Some("examples"), None] {
        let stopped = boot(&store, requested).expect_err("a save here would lock the project out of every later boot");
        assert!(stopped.contains("\"examples\""), "{stopped}");
    }
    assert!(matches!(boot(&store, Some("pong")), Ok(BootPlan::Open(_))), "another bundled project still opens");
}

#[test]
fn stored_files_with_no_manifest_while_a_manifest_will_not_decode_stop_the_boot() {
    let store = visitors_store();
    store.lose_manifest("pong");
    let before = everything(&store);
    store.mark_manifest_undecodable("third");

    let stopped = boot(&store, Some("pong")).expect_err("pong's manifest may be the record that will not decode");
    assert!(stopped.contains("\"pong\""), "{stopped}");

    assert_eq!(everything(&store), before, "pong's files are not taken for an interrupted import");
}

#[test]
fn with_every_manifest_decoded_the_sweep_and_the_interrupted_import_removal_still_run() {
    let store = visitors_store();
    store.lose_manifest("pong");

    let pong = opened(boot(&store, Some("pong")));
    assert!(pong.stored_files.is_empty(), "an interrupted import loads the bundle only");

    let (manifests, records) = everything(&store);
    assert_eq!(
        manifests.manifests.iter().map(|manifest| manifest.slug.as_str()).collect::<Vec<_>>(),
        ["mine", "third"]
    );
    let count_of = |slug: &str| records[EVERY_SLUG.iter().position(|known| *known == slug).expect("listed")].len();
    assert_eq!(count_of("pong"), 0, "the interrupted import was removed");
    assert_eq!(count_of("lost"), 0, "the orphan was swept");
    assert_eq!((count_of("mine"), count_of("third")), (1, 1));
}

#[test]
fn an_unknown_slug_with_every_manifest_decoded_redirects_to_the_first_bundled_project() {
    let store = visitors_store();
    match boot(&store, Some("nowhere")) {
        Ok(BootPlan::Redirect(slug)) => assert_eq!(slug, "examples"),
        other => panic!("expected a redirect, got {other:?}"),
    }
    let first = opened(boot(&store, None));
    assert_eq!(first.slug, "examples", "no parameter opens the first bundled project");
}

#[test]
fn a_failed_sweep_stops_the_boot() {
    let store = visitors_store();
    let before = everything(&store);

    store.fail_sweeps();
    let stopped = boot(&store, Some("pong")).expect_err("the sweep's own failure stops the boot");
    assert!(stopped.contains("injected sweep failure"), "{stopped}");

    assert_eq!(everything(&store), before);
}

#[test]
fn a_move_boot_never_falls_back_to_the_memory_store() {
    assert!(is_move_boot(Some("1")));
    assert!(!is_move_boot(None));
    assert!(!is_move_boot(Some("0")));
    assert_eq!(memory_fallback_refusal(true), Some(MOVE_BOOT_STORAGE_REFUSAL));
    assert_eq!(memory_fallback_refusal(false), None, "an ordinary boot keeps today's fallback");
}

#[test]
fn a_record_that_will_not_decode_is_never_written_over_and_never_replaced() {
    let store = visitors_store();
    let before = everything(&store);
    store.mark_file_undecodable("pong", SCENE);

    let pong_manifest = manifest("pong", ProjectOrigin::Bundled);
    for base_revision in [0, 1] {
        let refused = pollster::block_on(store.put(scene("pong", b"over it"), base_revision, &pong_manifest));
        assert!(
            matches!(refused, Err(StoreError::Backend(_))),
            "an unknown revision is never read as absent or as a match: {refused:?}"
        );
    }
    let removed = pollster::block_on(store.remove_file("pong", SCENE, 1, &pong_manifest));
    assert!(matches!(removed, Err(StoreError::Backend(_))), "{removed:?}");
    let replaced = pollster::block_on(store.replace_project("pong", vec![scene("pong", b"import")], pong_manifest));
    assert!(matches!(replaced, Err(StoreError::Backend(_))), "{replaced:?}");

    assert_eq!(everything(&store), before);
}
