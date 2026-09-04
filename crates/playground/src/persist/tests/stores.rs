use super::{temp_directory_store, test_manifest};
use crate::projects::ProjectManifest;
use crate::store::{ProjectStore, StoredFile, StoreError};

#[test]
fn test_directory_store_put_load_round_trip() {
    let (_temporary_directory, store) = temp_directory_store();
    pollster::block_on(async {
        let manifest = test_manifest("pong");
        let file = StoredFile {
            project: "pong".to_string(),
            path: "scenes/main.ron".to_string(),
            bytes: b"scene_content".to_vec(),
            revision: 0,
            bundle_version: "v1".to_string(),
        };

        let new_revision = store.put(file, 0, &manifest).await.unwrap();
        assert_eq!(new_revision, 1);

        let files = store.load_project("pong").await.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "scenes/main.ron");
        assert_eq!(files[0].bytes, b"scene_content");
        assert_eq!(files[0].revision, 1);
    });
}

#[test]
fn test_directory_store_stale_base_refused_and_bytes_untouched() {
    let (_temporary_directory, store) = temp_directory_store();
    pollster::block_on(async {
        let manifest = test_manifest("pong");
        let file = StoredFile {
            project: "pong".to_string(),
            path: "scenes/main.ron".to_string(),
            bytes: b"original".to_vec(),
            revision: 0,
            bundle_version: "v1".to_string(),
        };
        store.put(file, 0, &manifest).await.unwrap();

        let stale = StoredFile {
            project: "pong".to_string(),
            path: "scenes/main.ron".to_string(),
            bytes: b"corrupted".to_vec(),
            revision: 0,
            bundle_version: "v1".to_string(),
        };
        let error = store.put(stale, 0, &manifest).await.unwrap_err();
        assert_eq!(error, StoreError::StaleRevision { stored: 1, base: 0 });

        let files = store.load_project("pong").await.unwrap();
        assert_eq!(files[0].bytes, b"original");
    });
}

#[test]
fn test_two_writers_racing_from_same_base_exactly_one_wins() {
    let (_temporary_directory, store) = temp_directory_store();
    let first_store = store.clone();
    let second_store = store.clone();
    let manifest = test_manifest("pong");
    let first_manifest = manifest.clone();
    let second_manifest = manifest.clone();

    let first_thread = std::thread::spawn(move || {
        pollster::block_on(async move {
            let file = StoredFile {
                project: "pong".to_string(),
                path: "file.txt".to_string(),
                bytes: b"writer 1".to_vec(),
                revision: 0,
                bundle_version: "v1".to_string(),
            };
            first_store.put(file, 0, &first_manifest).await
        })
    });

    let second_thread = std::thread::spawn(move || {
        pollster::block_on(async move {
            let file = StoredFile {
                project: "pong".to_string(),
                path: "file.txt".to_string(),
                bytes: b"writer 2".to_vec(),
                revision: 0,
                bundle_version: "v1".to_string(),
            };
            second_store.put(file, 0, &second_manifest).await
        })
    });

    let first_result = first_thread.join().unwrap();
    let second_result = second_thread.join().unwrap();

    let wins = (first_result.is_ok() as usize) + (second_result.is_ok() as usize);
    assert_eq!(wins, 1, "exactly one writer racing from base 0 must win");
}

#[test]
fn test_sweep_orphans_removes_non_bundled_manifestless_slugs() {
    let (_temporary_directory, store) = temp_directory_store();
    pollster::block_on(async {
        let manifest = test_manifest("pong");
        let file_pong = StoredFile {
            project: "pong".to_string(),
            path: "scene.ron".to_string(),
            bytes: b"pong".to_vec(),
            revision: 0,
            bundle_version: "v1".to_string(),
        };
        store.put(file_pong, 0, &manifest).await.unwrap();

        let orphan_file = StoredFile {
            project: "orphan".to_string(),
            path: "file.txt".to_string(),
            bytes: b"orphan".to_vec(),
            revision: 0,
            bundle_version: "v1".to_string(),
        };
        let empty_manifest = ProjectManifest {
            slug: "orphan".to_string(),
            title: "".to_string(),
            bundle_version: "".to_string(),
            content_hash: "".to_string(),
            origin: crate::projects::ProjectOrigin::Saved,
        };
        store.put(orphan_file, 0, &empty_manifest).await.unwrap();
        let orphan_manifest_path = store.manifest_path("orphan");
        std::fs::remove_file(&orphan_manifest_path).unwrap();

        store.sweep_orphans(&["bundled_slug".to_string()]).await.unwrap();

        assert_eq!(store.load_project("pong").await.unwrap().len(), 1);
        assert!(store.load_project("orphan").await.unwrap().is_empty());
    });
}
