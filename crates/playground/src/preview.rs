//! Unpacking a snapshot archive for the preview window.
//!
//! Target-agnostic on purpose: the refusals are the importer's, and they are
//! worth testing headless even though only the wasm entry calls this.

use std::path::PathBuf;

use crate::archive::{import_project, ArchiveError};
use crate::projects::project_root;

/// A snapshot archive laid out as VFS keys, ready to insert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnpackedPreview {
    /// The project root every key is under.
    pub root: String,
    /// The scene the preview is to load, named by the editor, never inferred.
    pub scene: PathBuf,
    /// Every file of the project, keyed by its full VFS path.
    pub files: Vec<(String, Vec<u8>)>,
}

/// Validate a snapshot archive and key its files under the project root.
///
/// Every refusal `import_project` has applies here — an unsafe path, an
/// invalid scene, a bad script — plus one of its own: the entry the editor
/// named must actually be in the archive, or the preview would boot into an
/// empty world and blame the scene.
pub fn unpack_preview(
    bytes: &[u8],
    scene_entry: &str,
    asset_base: &str,
    bundle_version: &str,
) -> Result<UnpackedPreview, ArchiveError> {
    let (manifest, stored_files) = import_project(bytes, bundle_version)?;
    let root = project_root(asset_base, &manifest.slug);

    if !stored_files.iter().any(|file| file.path == scene_entry) {
        return Err(ArchiveError::MissingScene(scene_entry.to_string()));
    }

    let files = stored_files
        .into_iter()
        .map(|file| (format!("{root}/{}", file.path), file.bytes))
        .collect();

    Ok(UnpackedPreview {
        scene: PathBuf::from(format!("{root}/{scene_entry}")),
        root,
        files,
    })
}

/// What the page's readiness question is answered with.
///
/// `Ok` from the load export means the runtime was scheduled, not that it
/// runs, so a page that took that for readiness would show a black canvas as
/// a working preview. Failure is reported from either of the two places one
/// can come from: the scene the preview was handed, and the boot status. The
/// status is read by phase: before the first frame the engine writes progress
/// there and one failure ("Renderer init failed"); once a frame has stepped
/// it has cleared the status, and anything it writes afterwards — a lost
/// graphics device, a game that ended — is terminal.
pub fn preview_state(
    loaded: bool,
    started: bool,
    scene_error: Option<&str>,
    boot_status: Option<&str>,
) -> String {
    if let Some(error) = scene_error {
        return format!("failed: {error}");
    }
    let status = boot_status.unwrap_or("").trim();
    if loaded && started {
        if !status.is_empty() {
            return format!("failed: {status}");
        }
        return "running".to_string();
    }
    if status.to_ascii_lowercase().contains("failed") {
        return format!("failed: {status}");
    }
    "booting".to_string()
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::*;
    use crate::projects::{ProjectManifest, ProjectOrigin};

    const SCENE: &str = r#"SceneData(name: "preview", entities: [])"#;

    fn manifest_ron(slug: &str) -> String {
        let manifest = ProjectManifest {
            slug: slug.to_string(),
            title: "Preview".to_string(),
            bundle_version: "v1".to_string(),
            content_hash: String::new(),
            origin: ProjectOrigin::Bundled,
        };
        ron::ser::to_string_pretty(&manifest, ron::ser::PrettyConfig::default()).unwrap()
    }

    fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
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
    fn test_unpack_preview_keys_every_file_under_the_project_root_and_names_the_scene() {
        let bytes = zip_of(&[
            ("project.ron", manifest_ron("demo").as_bytes()),
            ("assets/scenes/main.scene.ron", SCENE.as_bytes()),
            ("assets/scripts/ball.rhai", b"fn update(me, view, params, cmd, dt) {}"),
        ]);

        let unpacked =
            unpack_preview(&bytes, "assets/scenes/main.scene.ron", "/playground/v1/assets", "v1")
                .expect("a valid snapshot unpacks");

        assert_eq!(unpacked.root, "/playground/v1/assets/projects/demo");
        assert_eq!(
            unpacked.scene,
            PathBuf::from("/playground/v1/assets/projects/demo/assets/scenes/main.scene.ron")
        );
        assert_eq!(unpacked.files.len(), 2, "both asset files travel");
        assert!(
            unpacked.files.iter().all(|(key, _)| key.starts_with(&unpacked.root)),
            "every key is under the project root: {:?}",
            unpacked.files.iter().map(|(key, _)| key).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_unpack_preview_refuses_a_zip_escaping_the_root() {
        let bytes = zip_of(&[
            ("project.ron", manifest_ron("demo").as_bytes()),
            ("assets/scenes/main.scene.ron", SCENE.as_bytes()),
            ("../escaped.txt", b"nope"),
        ]);

        let error = unpack_preview(
            &bytes,
            "assets/scenes/main.scene.ron",
            "/playground/v1/assets",
            "v1",
        )
        .expect_err("a traversal entry is refused");
        assert!(matches!(error, ArchiveError::UnsafePath(_)), "{error}");
    }

    #[test]
    fn test_unpack_preview_refuses_a_snapshot_without_a_scene() {
        let bytes = zip_of(&[
            ("project.ron", manifest_ron("demo").as_bytes()),
            ("assets/scenes/other.scene.ron", SCENE.as_bytes()),
        ]);

        let error = unpack_preview(
            &bytes,
            "assets/scenes/main.scene.ron",
            "/playground/v1/assets",
            "v1",
        )
        .expect_err("a snapshot without the named scene is refused");
        assert!(
            matches!(&error, ArchiveError::MissingScene(entry) if entry == "assets/scenes/main.scene.ron"),
            "{error}"
        );
    }

    #[test]
    fn test_preview_state_reports_running_only_after_the_first_successful_frame_and_failed_on_a_scene_load_error(
    ) {
        assert_eq!(preview_state(false, false, None, Some("")), "booting");
        assert_eq!(
            preview_state(true, false, None, Some("")),
            "booting",
            "a scheduled runtime is not a running one"
        );
        assert_eq!(preview_state(true, true, None, Some("")), "running");
        assert_eq!(
            preview_state(true, false, Some("no such scene"), Some("")),
            "failed: no such scene"
        );
        assert_eq!(
            preview_state(true, false, None, Some("Renderer init failed: no adapter")),
            "failed: Renderer init failed: no adapter"
        );
        assert_eq!(
            preview_state(true, true, Some("scene gone"), Some("")),
            "failed: scene gone",
            "a scene failure outranks frames that kept running"
        );
        assert_eq!(preview_state(true, true, None, None), "running", "a page without the status element");
        assert_eq!(
            preview_state(true, true, None, Some("Graphics device lost — reload the page to continue")),
            "failed: Graphics device lost — reload the page to continue",
            "after the first frame any status the engine writes is terminal"
        );
        assert_eq!(
            preview_state(true, true, None, Some("Game ended — reload the page to play again")),
            "failed: Game ended — reload the page to play again"
        );
        assert_eq!(
            preview_state(true, false, None, Some("Starting the preview…")),
            "booting",
            "before the first frame the status is progress, not failure"
        );
    }

    #[test]
    fn test_bundled_projects_keep_their_scenes_directly_under_assets_scenes() {
        // The preview loads the entry it is handed, and the editor's default
        // scene comes from `SceneLoader::first_scene_in(<root>/assets/scenes)`.
        // A bundled project that nested its scenes deeper would make those two
        // disagree without any test failing.
        let projects_directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../insiculous_web/public/playground/v1/assets/projects");
        let Ok(entries) = std::fs::read_dir(&projects_directory) else {
            // The deployed bundle is not in every checkout of the engine.
            return;
        };
        for entry in entries.flatten() {
            let scenes = entry.path().join("assets/scenes");
            let Ok(scene_files) = std::fs::read_dir(&scenes) else {
                continue;
            };
            for scene in scene_files.flatten() {
                assert!(
                    scene.path().is_file(),
                    "{} nests its scenes; the editor's default scene reads {} flat",
                    entry.path().display(),
                    scenes.display()
                );
            }
        }
    }
}
