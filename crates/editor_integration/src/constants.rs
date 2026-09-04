//! Shared editor-integration constants.
//!
//! Centralizes values that would otherwise be scattered as magic numbers
//! across the editor wrapper, entity operations, and panel rendering.

use engine_core::GameConfig;
use glam::Vec2;

/// Default scene file path used until a file picker exists (Phase 2+).
pub(crate) const DEFAULT_SCENE_PATH: &str = "scenes/scene.ron";

/// Editor preferences file (camera, grid, panel layout), saved on exit.
pub(crate) const EDITOR_PREFS_PATH: &str = "editor_prefs.json";

/// Minimum window width for the editor to be usable.
pub(crate) const MIN_EDITOR_WINDOW_WIDTH: u32 = 1024;

/// Minimum window height for the editor to be usable.
pub(crate) const MIN_EDITOR_WINDOW_HEIGHT: u32 = 720;

/// Enlarge a game config so the editor window is at least the usable minimum.
pub(crate) fn clamp_editor_window_size(mut config: GameConfig) -> GameConfig {
    config.width = config.width.max(MIN_EDITOR_WINDOW_WIDTH);
    config.height = config.height.max(MIN_EDITOR_WINDOW_HEIGHT);
    config
}

/// Smallest allowed entity scale when dragging the scale gizmo
/// (prevents zero/negative scale).
pub(crate) const MIN_ENTITY_SCALE: f32 = 0.01;

/// World-space offset applied to duplicated entities so the copy is visible
/// next to the original.
pub(crate) const DUPLICATE_OFFSET: Vec2 = Vec2::new(20.0, -20.0);

/// First `.ron` scene in a directory, in sorted order.
pub fn find_first_scene(scenes_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    common::vfs::list_dir_files(scenes_dir, "ron").into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_window_is_enlarged_to_the_usable_minimum_and_never_shrunk() {
        let cases = [
            ((640, 480), (1024, 720)),
            ((1920, 1080), (1920, 1080)),
            ((1100, 600), (1100, 720)),
        ];
        for ((width, height), expected) in cases {
            let config = clamp_editor_window_size(GameConfig::new("Test").with_size(width, height));
            assert_eq!((config.width, config.height), expected, "requested {width}x{height}");
        }
    }

    #[test]
    fn test_first_scene_is_the_byte_wise_first_ron_file_whatever_the_dir_order() -> std::io::Result<()> {
        // Which scene opens on launch must not depend on `read_dir`.
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("zeta.ron"), "x")?;
        std::fs::write(dir.path().join("alpha.RON"), "x")?;
        std::fs::write(dir.path().join("aaa.txt"), "x")?;

        let first = find_first_scene(dir.path()).expect("a scene exists");
        assert_eq!(first.file_name().and_then(|n| n.to_str()), Some("alpha.RON"));

        let empty = tempfile::tempdir()?;
        assert_eq!(find_first_scene(empty.path()), None, "no .ron file, no scene");
        assert_eq!(find_first_scene(&dir.path().join("missing")), None, "unreadable dir, no scene");
        Ok(())
    }
}
