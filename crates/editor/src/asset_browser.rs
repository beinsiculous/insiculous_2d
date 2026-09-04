//! Asset browser data: filesystem scan and per-entry state.
//!
//! Pure data + vfs scan only — thumbnail loading and drawing live in
//! `editor_integration` (which can reach the engine's `AssetManager`).
//!
//! The scan never follows symlinks (`common::vfs::list_files`): a link out of
//! the project would otherwise dump six levels of someone's home directory into
//! the panel and into every export. Assets are copies by convention — the art
//! repo's sync script copies, never links — so a linked tree is not a supported
//! layout; it lists as empty, not as an error.

use std::path::Path;

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
    /// Vertical scroll (shared panel pattern)
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
/// unreadable directories yield an empty list. Results are sorted by (kind, name)
/// for a stable grid.
pub fn scan_assets(base: &Path) -> Vec<AssetEntry> {
    let mut entries = Vec::new();
    let Ok(files) = common::vfs::list_files(base) else {
        return entries;
    };

    for path in files {
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

    fn touch(path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, b"x")
    }

    fn image_entry(name: &str) -> AssetEntry {
        AssetEntry {
            name: name.into(),
            relative_path: name.into(),
            kind: AssetKind::Image,
            texture_handle: None,
            load_failed: false,
        }
    }

    #[test]
    fn test_nested_images_and_scenes_listed_with_slash_joined_relative_paths_while_txt_is_ignored() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        for file in [
            "player.png",
            "brick.JPG",
            "sprites/nested/coin.png",
            "scenes/level1.scene.ron",
            "fonts/font.ttf",
            "notes.txt",
        ] {
            touch(&dir.path().join(file))?;
        }

        let entries = scan_assets(dir.path());

        let listed: Vec<(&str, AssetKind, &str)> = entries
            .iter()
            .map(|entry| (entry.name.as_str(), entry.kind, entry.relative_path.as_str()))
            .collect();
        assert_eq!(
            listed,
            vec![
                ("brick.JPG", AssetKind::Image, "brick.JPG"),
                ("coin.png", AssetKind::Image, "sprites/nested/coin.png"),
                ("player.png", AssetKind::Image, "player.png"),
                ("level1.scene.ron", AssetKind::Scene, "scenes/level1.scene.ron"),
            ],
            "images first by name, then scenes; nested files have forward-slash relative paths; txt is ignored"
        );
        assert!(
            scan_assets(Path::new("/definitely/not/a/real/dir")).is_empty(),
            "a missing folder is an empty browser, not a crash"
        );
        Ok(())
    }

    #[test]
    fn test_apply_scan_preserves_loaded_handles_by_path() {
        let mut state = AssetBrowserState::default();
        state.apply_scan(vec![image_entry("a.png")]);
        state.entries[0].texture_handle = Some(5);

        // A rescan finds the same file plus a new one.
        state.apply_scan(vec![image_entry("a.png"), image_entry("b.png")]);

        assert_eq!(state.entries[0].texture_handle, Some(5), "a rescan must not reload what is already on the GPU");
        assert_eq!(state.entries[1].texture_handle, None, "a new entry starts unloaded");
        assert!(state.scanned);
    }

    #[test]
    fn test_thumbnail_fit_preserves_aspect_and_centers_in_the_slot() {
        let slot = common::Rect::new(10.0, 10.0, 64.0, 64.0);

        let wide = fit_rect(128, 64, slot);
        assert_eq!((wide.width, wide.height, wide.y), (64.0, 32.0, 26.0), "width-bound, vertically centered");
        let tall = fit_rect(32, 64, slot);
        assert_eq!((tall.width, tall.height, tall.x), (32.0, 64.0, 26.0), "height-bound, horizontally centered");
        assert_eq!(fit_rect(0, 10, slot), slot, "degenerate sizes fall back to the slot");
    }
}
