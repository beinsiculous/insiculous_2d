//! Asset browser data: filesystem scan and per-entry state.
//!
//! Pure data + vfs scan only — thumbnail loading and drawing live in
//! `editor_integration` (which can reach the engine's `AssetManager`).
//!
//! The scan follows the configured asset root — a project whose root is itself a link to
//! a shared checkout lists, as `vfs::list_files` lists it — and never a symlink entry
//! below it (`common::vfs::list_entries`): a link out of the project would otherwise dump
//! someone's home directory into the panel and into every export. Assets are copies by
//! convention — the art repo's sync script copies, never links — so a linked entry is not
//! a supported layout; it lists as nothing, not as an error.

use std::path::{Path, PathBuf};

use common::clock::Instant;
use glam::Vec2;

use crate::double_click::DoubleClick;
use crate::external_tools::{Location, OpenRequest};

mod tree;
pub use tree::{scan_assets, kind_root, validate_name, validate_folder, route_new_file, companion_path, AssetFolder, AssetScan};
#[cfg(test)]
use tree::kind_for_path;

/// What kind of asset a scanned file is — plus `Folder`, `Font` and `Locale`, which the scan
/// never produces and the creation convention's table names for their roots and rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssetKind {
    /// An image usable as a sprite texture (png/jpg/jpeg/bmp)
    Image,
    /// A scene file (.ron)
    Scene,
    /// A script file (.rhai or .rs)
    Script,
    /// A recorded sound the audio manager decodes (wav/mp3/ogg/flac), dropped
    /// on a `SoundEmitter`'s sound slot — or a `.wav` on the Conductor panel, as a sample
    Sound,
    /// A song's MIDI file (.mid or .midi), dropped on the Conductor panel to open it
    Song,
    /// A music workstation's project file, by the external tools' extension list
    DawProject,
    /// A SoundFont bank (.sf2), dropped on the Conductor panel as an instrument
    SoundFont,
    /// A sidecar belonging to an image or MIDI file; never a separate tile.
    Companion,
    /// A folder name, used by the creation validator.
    Folder,
    /// A vendored font name, preserved as distributed.
    Font,
    /// A locale table.
    Locale,
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
    /// Whether the primary has its companion beside it.
    pub has_sidecar: bool,
    /// Image dimensions read from the loaded thumbnail by the host.
    pub image_size: Option<(u32, u32)>,
}

/// Whether an identity denotes a directory or a primary file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileKind { Folder, File }

/// A tile's identity across sorting and rescans.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TileId { pub kind: TileKind, pub path: String }

impl TileId {
    /// Identity for a primary file relative to the asset root.
    pub fn file(path: &str) -> Self { Self { kind: TileKind::File, path: path.into() } }
    /// Identity for a folder relative to the asset root, including the empty root.
    pub fn folder(path: &str) -> Self { Self { kind: TileKind::Folder, path: path.into() } }
}

/// A visible folder or file, identified without an index into the file list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile { pub id: TileId, pub name: String }

fn parent_path(path: &str) -> &str { path.rsplit_once('/').map_or("", |(parent, _)| parent) }

/// Asset browser panel state (a field on `EditorContext`).
#[derive(Debug, Default)]
pub struct AssetBrowserState {
    /// Scanned entries, sorted by (kind, name)
    pub entries: Vec<AssetEntry>,
    /// Whether an initial scan has run
    pub scanned: bool,
    /// Stable identity of the selected folder or file, independent of grid order.
    pub selected: Option<TileId>,
    /// Scanned folders at every depth, including empty directories.
    pub folders: Vec<AssetFolder>,
    /// Current folder relative to the asset root; empty means root.
    pub current_folder: String,
    /// Vertical scroll (shared panel pattern)
    pub scroll: crate::ScrollState,
    /// The last left press on a tile, by relative path, waiting for its second
    pub last_press: DoubleClick<TileId>,
    /// Whether the browser holds the keyboard: a press inside the panel takes it, and
    /// while it does its keys decide before the editor's
    pub holds_keyboard: bool,
    /// The grid's columns when it was last drawn: Up and Down step a row of them
    pub columns: usize,
    /// Scroll the selected tile wholly into view at the next render
    pub reveal_selected: bool,
    /// Open the selected tile's context menu at the tile at the next render, which
    /// knows where the tile is
    pub menu_requested: bool,
}

/// One arrow key's step through the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileStep {
    Left,
    Right,
    Up,
    Down,
}

impl AssetBrowserState {
    /// Replace the entries with a fresh scan, carrying over loaded texture
    /// handles and failure flags by relative path (rescans must not re-load
    /// textures — the texture manager does not dedupe by path).
    pub fn apply_scan(&mut self, scan: AssetScan) {
        let previous = std::mem::take(&mut self.entries);
        self.entries = scan.files.into_iter().map(|mut entry| {
            if let Some(old) = previous.iter().find(|old| old.relative_path == entry.relative_path) {
                entry.texture_handle = old.texture_handle;
                entry.load_failed = old.load_failed;
                entry.image_size = old.image_size;
            }
            entry
        }).collect();
        // The scan is the only word on which folders exist: natively `read_dir` lists an
        // empty directory and on the web its `.keep` does, so a session cache of created
        // folders would only keep alive one that another program removed.
        self.folders = scan.folders;
        if !self.folder_exists(&self.current_folder) { self.current_folder.clear(); }
        if self.selected.as_ref().is_some_and(|selected| !self.visible_tiles().iter().any(|tile| tile.id == *selected)) {
            self.selected = None;
        }
        self.scanned = true;
    }

    /// The immediate children of the current folder, parent first, folders by name,
    /// then files by kind and name. Hidden companions never enter the grid.
    pub fn visible_tiles(&self) -> Vec<Tile> {
        let mut folders: Vec<Tile> = self.folders.iter()
            .filter(|folder| parent_path(&folder.relative_path) == self.current_folder)
            .map(|folder| Tile { id: TileId::folder(&folder.relative_path), name: folder.name.clone() }).collect();
        folders.sort_by(|first, second| first.name.cmp(&second.name));
        if !self.current_folder.is_empty() {
            folders.insert(0, Tile { id: TileId::folder(parent_path(&self.current_folder)), name: "..".into() });
        }
        let mut files: Vec<&AssetEntry> = self.entries.iter().filter(|entry| {
            entry.kind != AssetKind::Companion && entry.name != ".keep"
                && parent_path(&entry.relative_path) == self.current_folder
        }).collect();
        files.sort_by(|first, second| first.kind.cmp(&second.kind).then(first.name.cmp(&second.name)));
        folders.extend(files.into_iter().map(|entry| Tile { id: TileId::file(&entry.relative_path), name: entry.name.clone() }));
        folders
    }

    fn folder_exists(&self, folder: &str) -> bool {
        folder.is_empty() || self.folders.iter().any(|entry| entry.relative_path == folder)
    }

    /// Enter a known folder; a stale saved folder falls back to the root.
    pub fn enter(&mut self, folder: &str) {
        self.current_folder = if self.folder_exists(folder) { folder.into() } else { String::new() };
        self.selected = None;
        self.scroll.scroll_to(0.0);
        self.last_press.forget();
        self.menu_requested = false;
    }

    /// Go to the current folder's parent, staying at the root when already there.
    pub fn up(&mut self) {
        let parent = parent_path(&self.current_folder).to_string();
        self.enter(&parent);
    }

    /// The file entry selected, or None for a folder.
    pub fn selected_entry(&self) -> Option<&AssetEntry> {
        let selected = self.selected.as_ref().filter(|selected| selected.kind == TileKind::File)?;
        self.entries.iter().find(|entry| entry.relative_path == selected.path)
    }

    /// The selected tile's current position, for drawing and keyboard movement only.
    pub fn selected_index(&self) -> Option<usize> {
        let selected = self.selected.as_ref()?;
        self.visible_tiles().iter().position(|tile| tile.id == *selected)
    }

    /// Record a press by stable identity. A double press enters a folder or opens a file.
    pub fn press_tile(&mut self, tile: &TileId, at: Instant, position: Vec2) -> Option<OpenRequest> {
        if !self.visible_tiles().iter().any(|visible| visible.id == *tile) { return None; }
        if self.last_press.press(tile.clone(), at, position) { self.open_tile(tile) } else { None }
    }

    fn open_tile(&mut self, tile: &TileId) -> Option<OpenRequest> {
        match tile.kind {
            TileKind::Folder => { self.enter(&tile.path); None }
            TileKind::File => Some(open_file_request(&tile.path)),
        }
    }

    /// Enter the selected folder or request the selected file's external tool.
    pub fn open_selected(&mut self) -> Option<OpenRequest> {
        let selected = self.selected.clone()?;
        self.open_tile(&selected)
    }

    /// Step through visible tiles, with Up and Down moving one grid row.
    pub fn step_selection(&mut self, step: TileStep) {
        let tiles = self.visible_tiles();
        let Some(last) = tiles.len().checked_sub(1) else { return; };
        let columns = self.columns.max(1);
        let index = match (self.selected_index(), step) {
            (None, _) => 0,
            (Some(index), TileStep::Left) => index.saturating_sub(1),
            (Some(index), TileStep::Right) => (index + 1).min(last),
            (Some(index), TileStep::Up) => index.checked_sub(columns).unwrap_or(index),
            (Some(index), TileStep::Down) => if index + columns <= last { index + columns } else { index },
        };
        self.selected = Some(tiles[index].id.clone());
        self.reveal_selected = true;
    }

    /// Let the keyboard go, with any request for a menu it made.
    pub fn release_keyboard(&mut self) {
        self.holds_keyboard = false;
        self.menu_requested = false;
    }
}

/// The request that opens an asset by its relative path, at no particular place in it.
pub fn open_file_request(relative_path: &str) -> OpenRequest {
    OpenRequest { path: PathBuf::from(relative_path), location: Location::None }
}

/// The request that opens the folder holding an asset; a file at the asset root opens
/// the root.
pub fn open_folder_request(relative_path: &str) -> OpenRequest {
    let folder = Path::new(relative_path).parent().filter(|parent| !parent.as_os_str().is_empty());
    OpenRequest { path: folder.map_or_else(|| PathBuf::from("."), Path::to_path_buf), location: Location::None }
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
mod tests;
