use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::*;
use crate::projects::{ProjectManifest, ProjectOrigin};

fn valid_manifest_ron(slug: &str) -> String {
    let manifest = ProjectManifest {
        slug: slug.to_string(),
        title: "Test".to_string(),
        bundle_version: "v1".to_string(),
        content_hash: String::new(),
        origin: ProjectOrigin::Bundled,
    };
    ron::ser::to_string_pretty(&manifest, ron::ser::PrettyConfig::default()).unwrap()
}

fn create_test_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buffer = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(&mut buffer);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, content) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap();
    buffer.into_inner()
}

#[test]
fn test_export_then_import_round_trip_valid_project() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();
    let scenes_dir = root.join("assets/scenes");
    let images_dir = root.join("assets/images");
    std::fs::create_dir_all(&scenes_dir).unwrap();
    std::fs::create_dir_all(&images_dir).unwrap();

    let scene_content = include_str!("../../../../examples/assets/scenes/hello_world.scene.ron");
    let image_bytes = vec![1, 2, 3, 4, 5];
    let sheet_content = r#"SheetFile(
    version: 1,
    cell: (16, 16),
    clips: [
        ("idle", (frames: [0, 1], fps: 6.0, looping: true)),
    ],
)"#;

    std::fs::write(scenes_dir.join("hello.scene.ron"), scene_content).unwrap();
    std::fs::write(images_dir.join("x.png"), &image_bytes).unwrap();
    std::fs::write(images_dir.join("x.sheet.ron"), sheet_content).unwrap();

    let manifest = ProjectManifest {
        slug: "round-trip".to_string(),
        title: "Round Trip".to_string(),
        bundle_version: "v1".to_string(),
        content_hash: "initial_hash".to_string(),
        origin: ProjectOrigin::Bundled,
    };

    let zip_bytes = export_project(root, &manifest).unwrap();
    let (imported_manifest, stored_files) = import_project(&zip_bytes, "v1").unwrap();

    assert_eq!(imported_manifest.slug, "round-trip");
    assert_eq!(imported_manifest.title, "Round Trip");
    assert_eq!(imported_manifest.content_hash, "");
    assert_eq!(imported_manifest.origin, ProjectOrigin::Imported);
    assert_eq!(imported_manifest.bundle_version, "v1");

    let mut paths: Vec<String> = stored_files.iter().map(|file| file.path.clone()).collect();
    paths.sort();
    assert_eq!(
        paths,
        vec![
            "assets/images/x.png".to_string(),
            "assets/images/x.sheet.ron".to_string(),
            "assets/scenes/hello.scene.ron".to_string(),
        ]
    );

    let png_file = stored_files.iter().find(|file| file.path == "assets/images/x.png").unwrap();
    assert_eq!(png_file.bytes, image_bytes);
    assert_eq!(png_file.revision, 1);
    assert_eq!(png_file.project, "round-trip");

    let sheet_file = stored_files.iter().find(|file| file.path == "assets/images/x.sheet.ron").unwrap();
    assert_eq!(sheet_file.bytes, sheet_content.as_bytes());

    let scene_file = stored_files.iter().find(|file| file.path == "assets/scenes/hello.scene.ron").unwrap();
    assert_eq!(scene_file.bytes, scene_content.as_bytes());
}

#[test]
fn test_tree_holding_own_project_ron_exports_without_duplicate_entry() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();
    let assets_dir = root.join("assets");
    std::fs::create_dir_all(&assets_dir).unwrap();
    std::fs::write(root.join("project.ron"), "existing manifest").unwrap();
    std::fs::write(root.join("README.md"), "existing readme").unwrap();
    std::fs::write(assets_dir.join("test.txt"), b"test").unwrap();

    let manifest = ProjectManifest {
        slug: "tree-test".to_string(),
        title: "Tree Test".to_string(),
        bundle_version: "v1".to_string(),
        content_hash: "hash".to_string(),
        origin: ProjectOrigin::Saved,
    };

    let zip_bytes = export_project(root, &manifest).unwrap();
    let (imported_manifest, files) = import_project(&zip_bytes, "v1").unwrap();
    assert_eq!(imported_manifest.slug, "tree-test");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "assets/test.txt");
}

#[test]
fn test_leading_dot_slash_imports_as_normalized_path() {
    let manifest_text = valid_manifest_ron("dot-slash");
    let zip_bytes = create_test_zip(&[
        ("./project.ron", manifest_text.as_bytes()),
        ("./assets/data.txt", b"hello"),
    ]);

    let (manifest, files) = import_project(&zip_bytes, "v1").unwrap();
    assert_eq!(manifest.slug, "dot-slash");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "assets/data.txt");
}

#[test]
fn test_mid_path_dot_and_repeated_slash_import_canonical() {
    let manifest_text = valid_manifest_ron("dotty");
    let zip_bytes = create_test_zip(&[
        ("project.ron", manifest_text.as_bytes()),
        ("assets/./images//x.png", b"png"),
    ]);

    let (_, files) = import_project(&zip_bytes, "v1").unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "assets/images/x.png");
}

#[test]
fn test_unsafe_traversal_path_refused() {
    let manifest_text = valid_manifest_ron("unsafe-test");
    let zip_bytes = create_test_zip(&[
        ("project.ron", manifest_text.as_bytes()),
        ("assets/../x.txt", b"danger"),
    ]);

    let result = import_project(&zip_bytes, "v1");
    assert_eq!(result, Err(ArchiveError::UnsafePath("assets/../x.txt".to_string())));
}

#[test]
fn test_bad_sidecar_is_invalid_sheet_naming_entry() {
    let manifest_text = valid_manifest_ron("bad-sheet");
    let zip_bytes = create_test_zip(&[
        ("project.ron", manifest_text.as_bytes()),
        ("assets/bad.sheet.ron", b"SheetFile(version: 999)"),
    ]);

    let result = import_project(&zip_bytes, "v1");
    match result {
        Err(ArchiveError::InvalidSheet { entry, .. }) => {
            assert_eq!(entry, "assets/bad.sheet.ron");
        }
        other => panic!("expected InvalidSheet, got {other:?}"),
    }
}

#[test]
fn test_scene_naming_unknown_component_is_invalid_scene_naming_entry() {
    let manifest_text = valid_manifest_ron("unknown-comp");
    let broken_scene = r#"SceneData(
    name: "Broken",
    prefabs: {
        "Test": PrefabData(
            components: [
                UnknownComponentNonExistent(value: 123),
            ],
        ),
    },
)"#;
    let zip_bytes = create_test_zip(&[
        ("project.ron", manifest_text.as_bytes()),
        ("assets/scenes/broken.scene.ron", broken_scene.as_bytes()),
    ]);

    let result = import_project(&zip_bytes, "v1");
    match result {
        Err(ArchiveError::InvalidScene { entry, .. }) => {
            assert_eq!(entry, "assets/scenes/broken.scene.ron");
        }
        other => panic!("expected InvalidScene, got {other:?}"),
    }
}

#[test]
fn test_backslash_name_imports_under_slash() {
    let manifest_text = valid_manifest_ron("backslash-test");
    let zip_bytes = create_test_zip(&[
        ("project.ron", manifest_text.as_bytes()),
        ("assets\\sub\\file.txt", b"content"),
    ]);

    let (manifest, files) = import_project(&zip_bytes, "v1").unwrap();
    assert_eq!(manifest.slug, "backslash-test");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "assets/sub/file.txt");
}

#[test]
fn test_directory_entry_is_skipped() {
    let manifest_text = valid_manifest_ron("dir-test");
    let mut buffer = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(&mut buffer);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    writer.add_directory("assets/", options).unwrap();
    writer.add_directory("assets/sub/", options).unwrap();
    writer.start_file("project.ron", options).unwrap();
    writer.write_all(manifest_text.as_bytes()).unwrap();
    writer.start_file("assets/sub/item.txt", options).unwrap();
    writer.write_all(b"item").unwrap();
    writer.finish().unwrap();

    let zip_bytes = buffer.into_inner();
    let (manifest, files) = import_project(&zip_bytes, "v1").unwrap();
    assert_eq!(manifest.slug, "dir-test");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "assets/sub/item.txt");
}

#[test]
fn test_root_file_outside_project_is_refused() {
    let manifest_text = valid_manifest_ron("outside-test");
    let zip_bytes = create_test_zip(&[
        ("project.ron", manifest_text.as_bytes()),
        ("foo.txt", b"not allowed at root"),
    ]);

    let result = import_project(&zip_bytes, "v1");
    assert_eq!(result, Err(ArchiveError::OutsideProject("foo.txt".to_string())));
}

#[test]
fn test_missing_project_ron_manifest_is_refused() {
    let zip_bytes = create_test_zip(&[
        ("assets/file.txt", b"no manifest here"),
    ]);

    let result = import_project(&zip_bytes, "v1");
    assert_eq!(result, Err(ArchiveError::MissingManifest));
}

#[test]
fn test_import_project_refuses_archive_exceeding_decompressed_cap() {
    let manifest_text = valid_manifest_ron("zeros-test");
    let mut buffer = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(&mut buffer);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    writer.start_file("project.ron", options).unwrap();
    writer.write_all(manifest_text.as_bytes()).unwrap();

    // Write 70 MiB of zeros
    writer.start_file("assets/zeros.bin", options).unwrap();
    let zero_chunk = [0u8; 64 * 1024];
    let chunks = (70 * 1024 * 1024) / zero_chunk.len();
    for _ in 0..chunks {
        writer.write_all(&zero_chunk).unwrap();
    }
    writer.finish().unwrap();

    let zip_bytes = buffer.into_inner();
    assert!(zip_bytes.len() < 100 * 1024, "70 MiB of zeros should compress to tens of KiB");

    let result = import_project(&zip_bytes, "v1");
    assert_eq!(result, Err(ArchiveError::TooLarge));
}
