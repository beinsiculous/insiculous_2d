//! Grid-of-tiles component rendered via the sprite batch pipeline
//! (PROJECT_ROADMAP Phase B, Gap 3).
//!
//! A [`Tilemap`] holds a row-major grid of tile indices into a tileset
//! texture. Each non-zero tile becomes one sprite instance sharing the
//! tileset texture — the whole map batches into a single draw call. The
//! engine_core render path expands maps every frame via
//! [`Tilemap::sprite_instances`]; this crate only produces plain data
//! (ecs has no renderer dependency).
//!
//! Conventions:
//! - `Transform2D.position` anchors the **center of tile (0, 0)**; row 0 is
//!   the top row and rows grow downward (world Y is up). Transform rotation
//!   and scale are ignored.
//! - `tile_size` is in pixels — the same space as `Transform2D.position`
//!   (only sprite *scale* goes through `RENDER_UNIT`, positions never do).
//! - Tile value `0` = empty; value `t` selects tileset cell `t - 1`, counted
//!   row-major across a tileset with `1 / tile_uv_size.x` columns — the shared
//!   [`common::SheetGrid`] owns that math.

use common::SheetGrid;
use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::component_registry::ComponentMeta;
use crate::DeriveComponentMeta;

/// One tile expanded to renderable data: where it sits relative to the map
/// anchor and which tileset UV region it samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileInstance {
    /// Offset of the tile center from the map entity's position, in pixels.
    pub offset: Vec2,
    /// Normalized UV region `(x, y, width, height)` into the tileset.
    pub tex_region: [f32; 4],
}

/// Component: a grid of tile indices drawn from a tileset texture.
#[derive(Debug, Clone, Serialize, Deserialize, DeriveComponentMeta)]
pub struct Tilemap {
    /// Grid width in tiles.
    pub width: u32,
    /// Grid height in tiles.
    pub height: u32,
    /// Edge length of one square tile, in pixels.
    pub tile_size: f32,
    /// Texture handle of the tileset (same id space as `Sprite.texture_handle`).
    pub tileset: u32,
    /// Row-major tile values, `width * height` entries; `0` = empty,
    /// `t` = tileset cell `t - 1`.
    pub tiles: Vec<u32>,
    /// Fraction of the tileset texture per tile, e.g. `(0.25, 0.25)` for a
    /// 4x4 tileset.
    pub tile_uv_size: Vec2,
    /// Render depth for every tile; defaults to -1.0 so maps draw behind
    /// default-depth (0.0) sprites.
    #[serde(default = "default_tilemap_depth")]
    pub depth: f32,
}

fn default_tilemap_depth() -> f32 {
    -1.0
}

impl Default for Tilemap {
    fn default() -> Self {
        Self::new(0, 0, 32.0)
    }
}

impl Tilemap {
    /// Create an empty (all-zero) map of `width` x `height` tiles.
    pub fn new(width: u32, height: u32, tile_size: f32) -> Self {
        Self {
            width,
            height,
            tile_size,
            tileset: 0,
            tiles: vec![0; (width * height) as usize],
            tile_uv_size: Vec2::ONE,
            depth: default_tilemap_depth(),
        }
    }

    /// Set the tile value at `(col, row)`; out-of-grid coordinates are ignored.
    pub fn set_tile(&mut self, col: u32, row: u32, value: u32) {
        if col < self.width && row < self.height {
            if let Some(slot) = self.tiles.get_mut((row * self.width + col) as usize) {
                *slot = value;
            }
        }
    }

    /// Expand every non-zero tile to a [`TileInstance`] (offset from the map
    /// anchor + tileset UV region). A `tiles` Vec shorter than
    /// `width * height` yields only the tiles it holds; extra entries are
    /// ignored.
    pub fn sprite_instances(&self) -> impl Iterator<Item = TileInstance> + '_ {
        let grid = SheetGrid::from_uv_size(self.tile_uv_size);

        self.tiles
            .iter()
            .take((self.width * self.height) as usize)
            .enumerate()
            .filter(|(_, &value)| value != 0)
            .map(move |(i, &value)| {
                let col = i as u32 % self.width;
                let row = i as u32 / self.width;
                TileInstance {
                    offset: Vec2::new(
                        col as f32 * self.tile_size,
                        -(row as f32) * self.tile_size,
                    ),
                    tex_region: grid.uv_rect(value - 1),
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_instances_place_non_zero_tiles_with_row_zero_on_top() {
        let mut map = Tilemap::new(4, 3, 50.0);
        map.tile_uv_size = Vec2::new(0.25, 0.25); // 4x4 tileset
        map.set_tile(0, 0, 1); // tileset cell 0
        map.set_tile(2, 1, 6); // tileset cell 5 -> column 1, row 1
        map.set_tile(5, 0, 9); // outside the grid: ignored

        let instances: Vec<TileInstance> = map.sprite_instances().collect();

        // Zero tiles emit nothing; the out-of-grid write was dropped.
        assert_eq!(
            instances,
            [
                TileInstance { offset: Vec2::ZERO, tex_region: [0.0, 0.0, 0.25, 0.25] },
                // Columns grow right, rows grow DOWN (world Y is up).
                TileInstance {
                    offset: Vec2::new(100.0, -50.0),
                    tex_region: [0.25, 0.25, 0.25, 0.25],
                },
            ]
        );
        assert_eq!(Tilemap::new(16, 16, 32.0).sprite_instances().count(), 0, "an all-zero map draws nothing");
    }

    #[test]
    fn test_short_tiles_vec_is_truncated_not_a_panic() {
        let mut map = Tilemap::new(4, 4, 32.0);
        map.tiles = vec![1, 0, 2]; // only 3 of 16 entries
        assert_eq!(map.sprite_instances().count(), 2);

        map.tiles = vec![1; 100]; // more entries than cells
        assert_eq!(map.sprite_instances().count(), 16);
    }

    #[test]
    fn test_tilemap_ron_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let mut map = Tilemap::new(2, 2, 40.0);
        map.tileset = 3;
        map.tile_uv_size = Vec2::new(0.5, 0.5);
        map.set_tile(0, 1, 4);

        let serialized = ron::to_string(&map)?;
        let restored: Tilemap = ron::from_str(&serialized)?;

        assert_eq!(restored.width, 2);
        assert_eq!(restored.height, 2);
        assert_eq!(restored.tile_size, 40.0);
        assert_eq!(restored.tileset, 3);
        assert_eq!(restored.tiles, [0, 0, 4, 0]);
        assert_eq!(restored.tile_uv_size, Vec2::new(0.5, 0.5));
        assert_eq!(restored.depth, -1.0, "maps draw behind default-depth sprites");
        Ok(())
    }
}
