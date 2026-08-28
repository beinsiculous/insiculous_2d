//! Asset browser data: filesystem scan and per-entry state.
//!
//! Pure data + std::fs only — thumbnail loading and drawing live in
//! `editor_integration` (which can reach the engine's `AssetManager`).

use std::path::Path;

/// Maximum directory depth `scan_assets` descends (guards symlink cycles).
const MAX_SCAN_DEPTH: usize = 6;

/// What kind of asset a scanned file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssetKind {
    /// An image usable as a sprite texture (png/jpg/jpeg/bmp)
    Image,
    /// A scene file (.ron)
    Scene,
}

/// One file found under the asset root.
#[derive(Debug, Clone)]
pub struct AssetEntry {
    /// File name (for the tile label)
    pub name: String,
    /// Path relative to the asset root — exactly what `assets.load_texture`
    /// takes (uses `/` separators)
    pub relative_path: String,
    /// Asset kind by extension
    pub kind: AssetKind,
    /// Loaded renderer texture handle for the thumbnail (filled lazily by
    /// the integration layer; `None` until loaded)
    pub texture_handle: Option<u32>,
    /// Set when a thumbnail load failed so it is never retried every frame
    pub load_failed: bool,
}

/// Asset browser panel state (a field on `EditorContext`).
#[derive(Debug, Default)]
pub struct AssetBrowserState {
    /// Scanned entries, sorted by (kind, name)
    pub entries: Vec<AssetEntry>,
    /// Whether an initial scan has run
    pub scanned: bool,
    /// Vertical scroll (shared panel pattern — audit §3.3)
    pub scroll: crate::ScrollState,
}

impl AssetBrowserState {
    /// Replace the entries with a fresh scan, carrying over loaded texture
    /// handles and failure flags by relative path (rescans must not re-load
    /// textures — the texture manager does not dedupe by path).
    pub fn apply_scan(&mut self, new_entries: Vec<AssetEntry>) {
        let old: Vec<AssetEntry> = std::mem::take(&mut self.entries);
        self.entries = new_entries
            .into_iter()
            .map(|mut e| {
                if let Some(prev) = old.iter().find(|o| o.relative_path == e.relative_path) {
                    e.texture_handle = prev.texture_handle;
                    e.load_failed = prev.load_failed;
                }
                e
            })
            .collect();
        self.scanned = true;
    }
}

/// Classify a file by extension (case-insensitive).
fn kind_for_extension(ext: &str) -> Option<AssetKind> {
    match ext.to_ascii_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "bmp" => Some(AssetKind::Image),
        "ron" => Some(AssetKind::Scene),
        _ => None,
    }
}

/// Recursively scan `base` for known asset files. Never panics: missing or
/// unreadable directories yield an empty (or partial) list. Results are
/// sorted by (kind, name) for a stable grid.
pub fn scan_assets(base: &Path) -> Vec<AssetEntry> {
    let mut entries = Vec::new();
    // Explicit stack instead of recursion; depth cap guards symlink cycles.
    let mut stack: Vec<(std::path::PathBuf, usize)> = vec![(base.to_path_buf(), 0)];

    while let Some((dir, depth)) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if depth < MAX_SCAN_DEPTH {
                    stack.push((path, depth + 1));
                }
                continue;
            }
            let Some(kind) = path
                .extension()
                .and_then(|e| e.to_str())
                .and_then(kind_for_extension)
            else {
                continue;
            };
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Ok(relative) = path.strip_prefix(base) else {
                continue;
            };
            let relative_path = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            entries.push(AssetEntry {
                name: name.to_string(),
                relative_path,
                kind,
                texture_handle: None,
                load_failed: false,
            });
        }
    }

    entries.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.name.cmp(&b.name)));
    entries
}

/// Aspect-preserving fit of a `tex_w`×`tex_h` image inside `slot`, centered.
pub fn fit_rect(tex_w: u32, tex_h: u32, slot: common::Rect) -> common::Rect {
    if tex_w == 0 || tex_h == 0 {
        return slot;
    }
    let scale = (slot.width / tex_w as f32).min(slot.height / tex_h as f32);
    let w = tex_w as f32 * scale;
    let h = tex_h as f32 * scale;
    common::Rect::new(
        slot.x + (slot.width - w) / 2.0,
        slot.y + (slot.height - h) / 2.0,
        w,
        h,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"x").unwrap();
    }

    #[test]
    fn test_scan_finds_images_and_scenes_recursively() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("player.png"));
        touch(&dir.path().join("brick.JPG"));
        touch(&dir.path().join("scenes/level1.scene.ron"));
        touch(&dir.path().join("fonts/font.ttf")); // ignored
        touch(&dir.path().join("notes.txt")); // ignored

        let entries = scan_assets(dir.path());
        assert_eq!(entries.len(), 3);
        // Sorted: images first (by name), then scenes
        assert_eq!(entries[0].name, "brick.JPG");
        assert_eq!(entries[0].kind, AssetKind::Image);
        assert_eq!(entries[1].name, "player.png");
        assert_eq!(entries[2].kind, AssetKind::Scene);
        assert_eq!(entries[2].relative_path, "scenes/level1.scene.ron");
    }

    #[test]
    fn test_scan_missing_dir_is_empty_not_panic() {
        let entries = scan_assets(Path::new("/definitely/not/a/real/dir"));
        assert!(entries.is_empty());
    }

    #[test]
    fn test_relative_paths_are_load_compatible() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("sub/tex.png"));
        let entries = scan_assets(dir.path());
        assert_eq!(entries[0].relative_path, "sub/tex.png", "forward slashes, no base prefix");
    }

    #[test]
    fn test_apply_scan_preserves_loaded_handles_by_path() {
        let mut state = AssetBrowserState::default();
        state.apply_scan(vec![AssetEntry {
            name: "a.png".into(),
            relative_path: "a.png".into(),
            kind: AssetKind::Image,
            texture_handle: None,
            load_failed: false,
        }]);
        state.entries[0].texture_handle = Some(5);
        state.entries[0].load_failed = false;

        // Rescan finds the same file plus a new one
        state.apply_scan(vec![
            AssetEntry {
                name: "a.png".into(),
                relative_path: "a.png".into(),
                kind: AssetKind::Image,
                texture_handle: None,
                load_failed: false,
            },
            AssetEntry {
                name: "b.png".into(),
                relative_path: "b.png".into(),
                kind: AssetKind::Image,
                texture_handle: None,
                load_failed: false,
            },
        ]);

        assert_eq!(state.entries[0].texture_handle, Some(5), "handle survives rescan");
        assert_eq!(state.entries[1].texture_handle, None, "new entry starts unloaded");
        assert!(state.scanned);
    }

    #[test]
    fn test_fit_rect_preserves_aspect_and_centers() {
        let slot = common::Rect::new(10.0, 10.0, 64.0, 64.0);
        // Wide image: width-bound
        let wide = fit_rect(128, 64, slot);
        assert_eq!(wide.width, 64.0);
        assert_eq!(wide.height, 32.0);
        assert_eq!(wide.y, 10.0 + 16.0, "vertically centered");
        // Tall image: height-bound
        let tall = fit_rect(32, 64, slot);
        assert_eq!(tall.height, 64.0);
        assert_eq!(tall.width, 32.0);
        assert_eq!(tall.x, 10.0 + 16.0, "horizontally centered");
        // Degenerate sizes fall back to the slot
        assert_eq!(fit_rect(0, 10, slot), slot);
    }
}
