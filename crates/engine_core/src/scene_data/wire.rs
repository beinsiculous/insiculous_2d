//! The scene's small row-shaped wire types — the leaf values of a component
//! variant, re-exported from `scene_data` so `scene_data::ClipData` stays the
//! canonical import path. A child module purely for file size.

use common::SheetGrid;
use ecs::sprite_components::AnimationClip;
use serde::{Deserialize, Serialize};

/// Wire form of a [`SheetGrid`](common::SheetGrid) in scene RON: the cell
/// counts only.
///
/// The grid's normalized cell size is derived, never written — scene files
/// speak in columns and rows, `.sheet.ron` sidecars speak in pixel cell sizes,
/// and neither spells out UVs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridData {
    #[serde(default = "default_grid_axis")]
    pub cols: u32,
    #[serde(default = "default_grid_axis")]
    pub rows: u32,
}

impl Default for GridData {
    fn default() -> Self {
        Self { cols: 1, rows: 1 }
    }
}

fn default_grid_axis() -> u32 {
    1
}

impl From<GridData> for SheetGrid {
    fn from(data: GridData) -> Self {
        SheetGrid::new(data.cols, data.rows)
    }
}

impl From<SheetGrid> for GridData {
    fn from(grid: SheetGrid) -> Self {
        Self {
            cols: grid.cols,
            rows: grid.rows,
        }
    }
}

/// Wire form of an [`AnimationClip`] — one shape shared by scene RON and
/// `.sheet.ron` sidecars, so a clip reads and writes identically wherever it
/// appears.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipData {
    /// Sheet cell indices, in playback order.
    pub frames: Vec<u32>,
    pub fps: f32,
    /// Omitted means looping: most clips repeat, and the plain serde default
    /// for a bool would silently make every clip a one-shot.
    #[serde(default = "default_looping")]
    pub looping: bool,
}

/// Clips loop unless a file says otherwise.
pub fn default_looping() -> bool {
    true
}

impl From<ClipData> for AnimationClip {
    fn from(data: ClipData) -> Self {
        Self {
            frame_indices: data.frames,
            fps: data.fps,
            looping: data.looping,
        }
    }
}

impl From<AnimationClip> for ClipData {
    fn from(clip: AnimationClip) -> Self {
        Self {
            frames: clip.frame_indices,
            fps: clip.fps,
            looping: clip.looping,
        }
    }
}
