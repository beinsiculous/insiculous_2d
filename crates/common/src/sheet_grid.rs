//! Row-major cell grid over a sprite sheet / tileset, and the UV math that
//! turns a cell index into a texture region.
//!
//! One sheet is cut into `cols` x `rows` equally sized cells, numbered
//! row-major from 0 (left to right, then top to bottom). [`SheetGrid::uv_rect`]
//! maps a cell index to the normalized `(x, y, width, height)` region that
//! samples it — the same shape as `Sprite.tex_region`.
//!
//! Consumers: `ecs::Tilemap` (tileset cells), and the sprite-animation /
//! sheet-import work that follows.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// A sheet cut into `cols` x `rows` row-major cells.
///
/// Build one with [`SheetGrid::new`], [`SheetGrid::from_cell_size`], or
/// [`SheetGrid::from_uv_size`] — the constructors keep the cell UV size
/// consistent with the grid dimensions, so there is no struct-literal form.
///
/// ```
/// use common::SheetGrid;
///
/// let grid = SheetGrid::new(4, 4);
/// // Cell 5 is column 1 of row 1.
/// assert_eq!(grid.uv_rect(5), [0.25, 0.25, 0.25, 0.25]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SheetGrid {
    /// Number of cells per row. The constructors guarantee at least 1.
    pub cols: u32,
    /// Number of rows. The constructors guarantee at least 1.
    pub rows: u32,
    /// Normalized size of one cell.
    ///
    /// Normally `(1 / cols, 1 / rows)`, but [`SheetGrid::from_uv_size`] stores
    /// the caller's value verbatim: a UV size that is not the exact reciprocal
    /// of an integer (`0.3` rounds to 3 columns, yet `1.0 / 3` is not `0.3`)
    /// must keep producing the regions it always did.
    cell_uv: Vec2,
}

impl SheetGrid {
    /// Create a grid of `cols` x `rows` equally sized cells.
    ///
    /// Zero dimensions are clamped to 1 rather than rejected, so a grid is
    /// always safe to sample from.
    #[inline]
    pub fn new(cols: u32, rows: u32) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        Self {
            cols,
            rows,
            cell_uv: Vec2::new(1.0 / cols as f32, 1.0 / rows as f32),
        }
    }

    /// Derive the grid from pixel sizes: how many whole `cell_w` x `cell_h`
    /// cells fit in a `sheet_w` x `sheet_h` sheet.
    ///
    /// Division truncates — a partial trailing cell (a 100px sheet of 30px
    /// cells has 10px left over) is excluded. The cell UV size is
    /// pixel-exact (`cell / sheet`), so each cell samples exactly its own
    /// pixels and the excluded remainder is never stretched over. A zero
    /// cell dimension, or a sheet smaller than one cell, yields 1 cell
    /// covering the full axis.
    #[inline]
    pub fn from_cell_size(sheet_w: u32, sheet_h: u32, cell_w: u32, cell_h: u32) -> Self {
        // checked_div: None on a zero cell dimension → 1 cell (same result
        // as the old explicit `> 0` guard; rust 1.98's manual_checked_ops
        // lint requires this spelling).
        let cols = sheet_w.checked_div(cell_w).map_or(1, |c| c.max(1));
        let rows = sheet_h.checked_div(cell_h).map_or(1, |r| r.max(1));
        let uv_x = if cell_w > 0 && cell_w <= sheet_w {
            cell_w as f32 / sheet_w as f32
        } else {
            1.0
        };
        let uv_y = if cell_h > 0 && cell_h <= sheet_h {
            cell_h as f32 / sheet_h as f32
        } else {
            1.0
        };
        Self {
            cols,
            rows,
            cell_uv: Vec2::new(uv_x, uv_y),
        }
    }

    /// Derive the grid from the normalized size of one cell, e.g. `(0.25, 0.25)`
    /// for a 4x4 sheet.
    ///
    /// The dimensions come from rounding `1 / uv`, and `uv` itself is kept as
    /// the cell UV size — degenerate values (zero or negative) are stored as
    /// given, on a 1-cell axis. This is the compatibility constructor: it
    /// reproduces the tileset math `ecs::Tilemap` has always used, for every
    /// input including the degenerate ones.
    #[inline]
    pub fn from_uv_size(uv: Vec2) -> Self {
        let axis = |size: f32| {
            if size > 0.0 {
                ((1.0 / size).round() as u32).max(1)
            } else {
                1
            }
        };
        Self {
            cols: axis(uv.x),
            rows: axis(uv.y),
            cell_uv: uv,
        }
    }

    /// Normalized `(x, y, width, height)` region of cell `index`, counted
    /// row-major from 0.
    ///
    /// Not bounds checked: an index past [`SheetGrid::cell_count`] keeps
    /// stepping down past the bottom of the sheet and yields coordinates at or
    /// beyond 1.0, which the sampler clamps. Use
    /// [`SheetGrid::uv_rect_checked`] when the index comes from data that
    /// might be wrong.
    #[inline]
    pub fn uv_rect(&self, index: u32) -> [f32; 4] {
        let cols = self.cols.max(1);
        [
            (index % cols) as f32 * self.cell_uv.x,
            (index / cols) as f32 * self.cell_uv.y,
            self.cell_uv.x,
            self.cell_uv.y,
        ]
    }

    /// [`SheetGrid::uv_rect`] for indices inside the grid, `None` past the
    /// last cell.
    #[inline]
    pub fn uv_rect_checked(&self, index: u32) -> Option<[f32; 4]> {
        (index < self.cell_count()).then(|| self.uv_rect(index))
    }

    /// Normalized size of one cell.
    #[inline]
    pub fn cell_uv_size(&self) -> Vec2 {
        self.cell_uv
    }

    /// Total number of cells (`cols * rows`).
    #[inline]
    pub fn cell_count(&self) -> u32 {
        self.cols.saturating_mul(self.rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uv_rect_maps_index_to_row_major_cell() {
        let grid = SheetGrid::new(4, 4);

        assert_eq!(grid.uv_rect(0), [0.0, 0.0, 0.25, 0.25]);
        // Cell 5 -> column 1, row 1.
        assert_eq!(grid.uv_rect(5), [0.25, 0.25, 0.25, 0.25]);
        // Last cell of the first row.
        assert_eq!(grid.uv_rect(3), [0.75, 0.0, 0.25, 0.25]);
    }

    #[test]
    fn test_new_clamps_zero_dimensions() {
        let grid = SheetGrid::new(0, 0);

        assert_eq!((grid.cols, grid.rows), (1, 1));
        assert_eq!(grid.cell_uv_size(), Vec2::ONE);
        assert_eq!(grid.cell_count(), 1);
    }

    #[test]
    fn test_from_cell_size_truncates_partial_trailing_cells() {
        // 100 / 30 = 3 whole cells per axis, 10px of remainder ignored.
        let grid = SheetGrid::from_cell_size(100, 100, 30, 30);

        assert_eq!((grid.cols, grid.rows), (3, 3));
        assert_eq!(grid.cell_count(), 9);
        // Pixel-exact UVs: each cell covers 30/100 of the sheet, NOT 1/3 —
        // the 10px remainder is excluded from sampling, not stretched over.
        assert_eq!(grid.cell_uv_size(), Vec2::new(0.3, 0.3));
        assert_eq!(grid.uv_rect(1), [0.3, 0.0, 0.3, 0.3]);
    }

    #[test]
    fn test_from_cell_size_guards_degenerate_inputs() {
        assert_eq!(SheetGrid::from_cell_size(64, 64, 0, 0), SheetGrid::new(1, 1));
        // Sheet smaller than one cell still yields a usable 1x1 grid.
        assert_eq!(SheetGrid::from_cell_size(10, 10, 16, 16), SheetGrid::new(1, 1));
    }

    #[test]
    fn test_uv_rect_passes_through_out_of_range_index() {
        let grid = SheetGrid::new(4, 4);

        // Cell 19 is one row past the bottom of the sheet: v = 1.0, left as-is
        // for the sampler to clamp rather than being wrapped or zeroed.
        assert_eq!(grid.uv_rect(19), [0.75, 1.0, 0.25, 0.25]);
    }

    #[test]
    fn test_uv_rect_checked_is_none_past_cell_count() {
        let grid = SheetGrid::new(4, 4);

        assert_eq!(grid.uv_rect_checked(15), Some(grid.uv_rect(15)));
        assert_eq!(grid.uv_rect_checked(16), None);
    }

    #[test]
    fn test_from_uv_size_derives_dimensions() {
        let grid = SheetGrid::from_uv_size(Vec2::new(0.25, 1.0));

        assert_eq!((grid.cols, grid.rows), (4, 1));
        assert_eq!(grid.uv_rect(2), [0.5, 0.0, 0.25, 1.0]);
    }

    #[test]
    fn test_from_uv_size_preserves_non_reciprocal_cell_size() {
        // 1 / 0.3 rounds to 3 columns, but the cell size must stay 0.3 —
        // 1.0 / 3 would shift every region.
        let grid = SheetGrid::from_uv_size(Vec2::new(0.3, 1.0));

        assert_eq!(grid.cols, 3);
        assert_eq!(grid.cell_uv_size().x, 0.3);
        assert_eq!(grid.uv_rect(1), [0.3, 0.0, 0.3, 1.0]);
    }

    #[test]
    fn test_from_uv_size_guards_zero_uv() {
        let grid = SheetGrid::from_uv_size(Vec2::ZERO);

        assert_eq!((grid.cols, grid.rows), (1, 1));
        // Degenerate in, degenerate out — no division by zero, no panic.
        assert_eq!(grid.uv_rect(0), [0.0, 0.0, 0.0, 0.0]);
    }
}
