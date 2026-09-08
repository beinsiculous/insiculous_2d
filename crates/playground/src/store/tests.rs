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
    };
    let revision = store.put(file_alpha, 0, &alpha_manifest).await.expect("put succeeds");
    assert_eq!(revision, 1);

    let loaded_alpha = store.load_project("alpha").await.expect("load succeeds");
    assert_eq!(loaded_alpha.len(), 1);
    assert_eq!(loaded_alpha[0].revision, 1);
    assert_eq!(loaded_alpha[0].bytes, b"alpha v1");

    // 2. manifest upsert: manifest for "alpha" was upserted with origin Saved
    let manifests = store.manifests().await;
    let alpha_saved = manifests.iter().find(|manifest| manifest.slug == "alpha").expect("alpha manifest found");
    assert_eq!(alpha_saved.origin, ProjectOrigin::Saved);

    // 3. stale base refused with bytes untouched
    let stale_file = StoredFile {
        project: "alpha".to_string(),
        path: "scenes/main.ron".to_string(),
        bytes: b"alpha conflict".to_vec(),
        revision: 0,
        bundle_version: "v1".to_string(),
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
    };
    store.put(file_beta, 0, &beta_manifest).await.expect("beta put succeeds");

    // 5. replace_project isolation: replace "alpha", verify "beta" remains untouched
    let replacement_file = StoredFile {
        project: "alpha".to_string(),
        path: "scenes/replaced.ron".to_string(),
        bytes: b"alpha replaced".to_vec(),
        revision: 10,
        bundle_version: "v1".to_string(),
    };
    let mut new_alpha_manifest = alpha_manifest.clone();
    new_alpha_manifest.title = "Alpha Replaced".to_string();
    store.replace_project("alpha", vec![replacement_file], new_alpha_manifest).await.expect("replace succeeds");

    let alpha_after_replace = store.load_project("alpha").await.expect("load alpha");
    assert_eq!(alpha_after_replace.len(), 1);
    assert_eq!(alpha_after_replace[0].path, "scenes/replaced.ron");
    assert_eq!(alpha_after_replace[0].bytes, b"alpha replaced");

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
        };

        let revision = store.put(file, 0, &bundled_manifest).await.expect("put succeeds");
        assert_eq!(revision, 1);

        let manifests = store.manifests().await;
        let starter_manifest = manifests.iter().find(|manifest| manifest.slug == "starter").expect("manifest exists");
        assert_eq!(starter_manifest.origin, ProjectOrigin::Saved);

        store.sweep_orphans(&["starter".to_string()]).await.expect("sweep succeeds");
        let loaded = store.load_project("starter").await.expect("load succeeds");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].bytes, b"demo");

        let _ = std::fs::remove_dir_all(&temporary_directory);
    });
}
