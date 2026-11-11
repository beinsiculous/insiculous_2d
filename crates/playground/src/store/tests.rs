use std::sync::atomic::{AtomicU64, Ordering};

use super::directory::DirectoryStore;
use super::memory::MemoryStore;
use super::{ProjectStore, StoredFile, StoreError};
use crate::projects::{ProjectManifest, ProjectOrigin};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

fn dummy_manifest(slug: &str) -> ProjectManifest {
    ProjectManifest {
        slug: slug.to_string(),
        title: slug.to_uppercase(),
        bundle_version: "v1".to_string(),
        content_hash: "test_hash".to_string(),
        origin: ProjectOrigin::Bundled,
    }
}

pub async fn run_store_contract_suite(store: &dyn ProjectStore) {
    let alpha_manifest = dummy_manifest("alpha");
    let beta_manifest = dummy_manifest("beta");

    // 1. put/load round trip
    let file_alpha = StoredFile {
        project: "alpha".to_string(),
        path: "scenes/main.ron".to_string(),
        bytes: b"alpha v1".to_vec(),
        revision: 0,
        bundle_version: "v1".to_string(),
        deleted: false,
    };
    let revision = store.put(file_alpha, 0, &alpha_manifest).await.expect("put succeeds");
    assert_eq!(revision, 1);

    let loaded_alpha = store.load_project("alpha").await.expect("load succeeds");
    assert_eq!(loaded_alpha.len(), 1);
    assert_eq!(loaded_alpha[0].revision, 1);
    assert_eq!(loaded_alpha[0].bytes, b"alpha v1");

    // 2. manifest upsert: manifest for "alpha" was upserted with origin Saved
    let manifests = store.manifests().await.expect("manifests read").manifests;
    let alpha_saved = manifests.iter().find(|manifest| manifest.slug == "alpha").expect("alpha manifest found");
    assert_eq!(alpha_saved.origin, ProjectOrigin::Saved);

    // 3. stale base refused with bytes untouched
    let stale_file = StoredFile {
        project: "alpha".to_string(),
        path: "scenes/main.ron".to_string(),
        bytes: b"alpha conflict".to_vec(),
        revision: 0,
        bundle_version: "v1".to_string(),
        deleted: false,
    };
    let stale_error = store.put(stale_file, 0, &alpha_manifest).await.expect_err("stale base refused");
    assert_eq!(stale_error, StoreError::StaleRevision { stored: 1, base: 0 });

    let reloaded_alpha = store.load_project("alpha").await.expect("load succeeds");
    assert_eq!(reloaded_alpha.len(), 1);
    assert_eq!(reloaded_alpha[0].bytes, b"alpha v1");
    assert_eq!(reloaded_alpha[0].revision, 1);

    // 4. second project "beta"
    let file_beta = StoredFile {
        project: "beta".to_string(),
        path: "scenes/other.ron".to_string(),
        bytes: b"beta original".to_vec(),
        revision: 0,
        bundle_version: "v1".to_string(),
        deleted: false,
    };
    store.put(file_beta, 0, &beta_manifest).await.expect("beta put succeeds");

    // 5. replace_project isolation: replace "alpha", verify "beta" remains untouched
    let replacement_file = StoredFile {
        project: "alpha".to_string(),
        path: "scenes/replaced.ron".to_string(),
        bytes: b"alpha replaced".to_vec(),
        revision: 10,
        bundle_version: "v1".to_string(),
        deleted: false,
    };
    let mut new_alpha_manifest = alpha_manifest.clone();
    new_alpha_manifest.title = "Alpha Replaced".to_string();
    store.replace_project("alpha", vec![replacement_file], new_alpha_manifest).await.expect("replace succeeds");

    let alpha_after_replace = store.load_project("alpha").await.expect("load alpha");
    assert_eq!(alpha_after_replace.len(), 1);
    assert_eq!(alpha_after_replace[0].path, "scenes/replaced.ron");
    assert_eq!(alpha_after_replace[0].bytes, b"alpha replaced");
    assert_eq!(
        alpha_after_replace[0].revision, 2,
        "a replaced project's files are stamped one above the highest revision it held, never as given"
    );

    let beta_after_replace = store.load_project("beta").await.expect("load beta");
    assert_eq!(beta_after_replace.len(), 1);
    assert_eq!(beta_after_replace[0].path, "scenes/other.ron");
    assert_eq!(beta_after_replace[0].bytes, b"beta original");

    // 6. remove_project
    store.remove_project("alpha").await.expect("remove succeeds");
    let alpha_removed = store.load_project("alpha").await.expect("load empty");
    assert!(alpha_removed.is_empty());

    let beta_still_present = store.load_project("beta").await.expect("beta intact");
    assert_eq!(beta_still_present.len(), 1);

    // 7. sweep_orphans
    store.sweep_orphans(&["bundled_slug".to_string()]).await.expect("sweep succeeds");
    let beta_kept = store.load_project("beta").await.expect("beta kept");
    assert_eq!(beta_kept.len(), 1);

    run_removal_contract(store).await;
}

/// A removal is a tombstone one revision up that `load_project` still returns, so a
/// put recreating the file names that revision and a stale removal stays stale.
async fn run_removal_contract(store: &dyn ProjectStore) {
    let manifest = dummy_manifest("gamma");
    let file = |bytes: &[u8]| StoredFile {
        project: "gamma".to_string(),
        path: "music/theme.mid.new".to_string(),
        bytes: bytes.to_vec(),
        revision: 0,
        bundle_version: "v1".to_string(),
        deleted: false,
    };

    assert_eq!(
        store.remove_file("gamma", "music/never.wav", 0, &manifest).await,
        Ok(1),
        "a bundled-only file needs a tombstone to suppress its bundled copy"
    );
    assert_eq!(
        store.remove_file("gamma", "music/never.wav", 3, &manifest).await,
        Err(StoreError::StaleRevision { stored: 1, base: 3 })
    );
    let bundled_tombstone = store.load_project("gamma").await.expect("load");
    assert_eq!(bundled_tombstone.len(), 1);
    assert!(bundled_tombstone[0].deleted);
    assert!(bundled_tombstone[0].bytes.is_empty());
    assert_eq!(bundled_tombstone[0].revision, 1);
    assert_eq!(bundled_tombstone[0].bundle_version, manifest.bundle_version);
    assert!(store.manifests().await.expect("manifests read").manifests.iter().any(|saved| saved.slug == "gamma" && saved.origin == ProjectOrigin::Saved));

    store.put(file(b"journal"), 0, &manifest).await.expect("first put");
    assert_eq!(
        store.remove_file("gamma", "music/theme.mid.new", 0, &manifest).await,
        Err(StoreError::StaleRevision { stored: 1, base: 0 }),
        "a removal from a tab that never saw revision 1 is stale"
    );
    store.remove_file("gamma", "music/theme.mid.new", 1, &manifest).await.expect("removal at the stored revision");

    let loaded = store.load_project("gamma").await.expect("load");
    assert_eq!(loaded.len(), 2, "both tombstones are returned with their revisions");
    let loaded = loaded.iter().filter(|file| file.path == "music/theme.mid.new").collect::<Vec<_>>();
    assert!(loaded[0].deleted);
    assert!(loaded[0].bytes.is_empty());
    assert_eq!(loaded[0].revision, 2);

    assert_eq!(
        store.put(file(b"again"), 0, &manifest).await,
        Err(StoreError::StaleRevision { stored: 2, base: 0 }),
        "a recreate must name the tombstone's revision"
    );
    assert_eq!(store.put(file(b"again"), 2, &manifest).await, Ok(3));
    let recreated = store.load_project("gamma").await.expect("load");
    let recreated = recreated.iter().filter(|file| file.path == "music/theme.mid.new").collect::<Vec<_>>();
    assert!(!recreated[0].deleted);
    assert_eq!(recreated[0].bytes, b"again");

    run_import_over_a_held_project_contract(store).await;
}

/// An import stamps every file revision 1. A page still holding the project at revision
/// 1 must meet a conflict on its next save, not write over the import — so the replaced
/// files land one above every revision the project held, its tombstones included.
async fn run_import_over_a_held_project_contract(store: &dyn ProjectStore) {
    let manifest = dummy_manifest("delta");
    let saved = |path: &str, bytes: &[u8]| StoredFile {
        project: "delta".to_string(),
        path: path.to_string(),
        bytes: bytes.to_vec(),
        revision: 0,
        bundle_version: "v1".to_string(),
        deleted: false,
    };
    assert_eq!(store.put(saved("scenes/main.ron", b"held"), 0, &manifest).await, Ok(1));
    assert_eq!(store.put(saved("scripts/old.rhai", b"one"), 0, &manifest).await, Ok(1));
    assert_eq!(store.put(saved("scripts/old.rhai", b"two"), 1, &manifest).await, Ok(2));
    assert_eq!(store.remove_file("delta", "scripts/old.rhai", 2, &manifest).await, Ok(3));

    let mut imported = saved("scenes/main.ron", b"imported");
    imported.revision = 1;
    store.replace_project("delta", vec![imported], manifest.clone()).await.expect("import replaces");

    let after_import = store.load_project("delta").await.expect("load");
    assert_eq!(after_import.len(), 1, "the import's file set is the whole project");
    assert_eq!(after_import[0].revision, 4, "one above the tombstone's 3, the highest the project held");
    assert_eq!(
        store.put(saved("scenes/main.ron", b"the held page saves"), 1, &manifest).await,
        Err(StoreError::StaleRevision { stored: 4, base: 1 }),
        "the page that held revision 1 conflicts instead of overwriting the import"
    );
    assert_eq!(
        store.put(saved("scripts/old.rhai", b"recreated"), 3, &manifest).await,
        Err(StoreError::StaleRevision { stored: 0, base: 3 }),
        "a path the import dropped is absent, which no non-zero base matches"
    );
    assert_eq!(store.load_project("delta").await.expect("load")[0].bytes, b"imported");
}

#[test]
fn test_memory_store_satisfies_store_contract_suite() {
    pollster::block_on(async {
        let store = MemoryStore::new();
        run_store_contract_suite(&store).await;
    });
}

#[test]
fn test_directory_store_satisfies_store_contract_suite() {
    pollster::block_on(async {
        let unique_id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary_directory = std::env::temp_dir().join(format!("insiculous_store_suite_{}_{}", std::process::id(), unique_id));
        let _ = std::fs::remove_dir_all(&temporary_directory);

        let store = DirectoryStore::new(temporary_directory.clone()).expect("directory store created");
        run_store_contract_suite(&store).await;

        let _ = std::fs::remove_dir_all(&temporary_directory);
    });
}

#[test]
fn test_directory_store_first_put_upserts_manifest_and_sweep_orphans_keeps_files() {
    pollster::block_on(async {
        let unique_id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary_directory = std::env::temp_dir().join(format!("insiculous_store_first_put_{}_{}", std::process::id(), unique_id));
        let _ = std::fs::remove_dir_all(&temporary_directory);

        let store = DirectoryStore::new(temporary_directory.clone()).expect("directory store created");

        let bundled_manifest = ProjectManifest {
            slug: "starter".to_string(),
            title: "Starter".to_string(),
            bundle_version: "v1".to_string(),
            content_hash: "hash123".to_string(),
            origin: ProjectOrigin::Bundled,
        };

        let file = StoredFile {
            project: "starter".to_string(),
            path: "scenes/demo.scene.ron".to_string(),
            bytes: b"demo".to_vec(),
            revision: 0,
            bundle_version: "v1".to_string(),
            deleted: false,
        };

        let revision = store.put(file, 0, &bundled_manifest).await.expect("put succeeds");
        assert_eq!(revision, 1);

        let manifests = store.manifests().await.expect("manifests read").manifests;
        let starter_manifest = manifests.iter().find(|manifest| manifest.slug == "starter").expect("manifest exists");
        assert_eq!(starter_manifest.origin, ProjectOrigin::Saved);

        store.sweep_orphans(&["starter".to_string()]).await.expect("sweep succeeds");
        let loaded = store.load_project("starter").await.expect("load succeeds");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].bytes, b"demo");

        let _ = std::fs::remove_dir_all(&temporary_directory);
    });
}

#[test]
fn test_first_bundled_only_delete_survives_boot_selection_and_marks_project_modified() {
    pollster::block_on(async {
        let store = MemoryStore::new();
        let manifest = dummy_manifest("bundled");
        assert_eq!(store.remove_file("bundled", "assets/sprites/hero.png", 0, &manifest).await, Ok(1));
        let manifests = store.manifests().await.expect("manifests read").manifests;
        let records = store.load_project("bundled").await.unwrap();
        let selected = super::stored_files_for_boot(
            records.clone(), manifests.iter().any(|saved| saved.slug == "bundled")
        ).expect("a tombstone-only store has a manifest, so boot keeps it");
        assert_eq!(selected.len(), 1);
        assert!(selected[0].deleted);
        assert!(selected[0].bytes.is_empty());
        assert_eq!(selected[0].path, "assets/sprites/hero.png");
        let projects = crate::projects::list_projects(&[manifest], &manifests);
        assert!(projects[0].has_stored_files, "the project picker marks even a delete-only project modified");
        assert!(super::stored_files_for_boot(records, false).is_none(), "interrupted imports still load bundled-only");
        assert!(super::stored_files_for_boot(Vec::new(), false).unwrap().is_empty());
    });
}
