//! Tilemap → sprite-batch expansion (PROJECT_ROADMAP Phase B, Gap 3).
//!
//! Every entity with a `Tilemap` + `Transform2D` is expanded to one sprite
//! instance per non-zero tile, all sharing the tileset texture — the whole
//! map lands in a single batch, and the renderer's `InstanceCache` skips
//! the GPU upload on frames where the map didn't change.
//!
//! Tiles are emitted in pixel units directly (no `RENDER_UNIT` multiply):
//! `Tilemap.tile_size` and `Transform2D.position` share the same pixel
//! space. Transform rotation/scale are ignored (see `ecs::Tilemap` docs).

use ecs::sprite_components::Transform2D;
use ecs::{Tilemap, World};
use glam::Vec2;
use renderer::sprite::SpriteBatcher;
use renderer::texture::TextureHandle;

/// Append one sprite per non-zero tile of every tilemap entity to the
/// game batcher. Called by the default `Game::render` before the entity
/// sprite loop so equal-depth sprites draw over tiles.
pub(crate) fn append_tilemap_sprites(world: &World, sprites: &mut SpriteBatcher) {
    for entity in world.entities() {
        let Some(tilemap) = world.get::<Tilemap>(entity) else { continue };
        let Some(transform) = world.get::<Transform2D>(entity) else { continue };

        let texture = TextureHandle { id: tilemap.tileset };
        for tile in tilemap.sprite_instances() {
            let sprite = renderer::Sprite::new(texture)
                .with_position(transform.position + tile.offset)
                .with_scale(Vec2::splat(tilemap.tile_size))
                .with_tex_region(
                    tile.tex_region[0],
                    tile.tex_region[1],
                    tile.tex_region[2],
                    tile.tex_region[3],
                )
                .with_depth(tilemap.depth);
            sprites.add_sprite(&sprite);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_with_map(anchor: Vec2) -> World {
        let mut world = World::new();
        let entity = world.create_entity();
        let mut map = Tilemap::new(3, 2, 40.0);
        map.tileset = 7;
        map.tile_uv_size = Vec2::new(0.25, 0.25);
        map.set_tile(0, 0, 1);
        map.set_tile(2, 1, 6);
        world.add_component(&entity, map).unwrap();
        world.add_component(&entity, Transform2D::new(anchor)).unwrap();
        world
    }

    #[test]
    fn test_tilemap_expands_into_one_batch_with_correct_instances() {
        let world = world_with_map(Vec2::new(100.0, 200.0));
        let mut batcher = SpriteBatcher::new();

        append_tilemap_sprites(&world, &mut batcher);

        let batches = batcher.batches();
        assert_eq!(batches.len(), 1, "whole map should share one batch");
        let batch = batcher.batch_for(TextureHandle { id: 7 }).unwrap();
        assert_eq!(batch.instances.len(), 2);

        // Tile (0,0): at the anchor, tileset cell 0.
        let first = &batch.instances[0];
        assert_eq!(first.position, [100.0, 200.0]);
        assert_eq!(first.scale, [40.0, 40.0]);
        assert_eq!(first.tex_region, [0.0, 0.0, 0.25, 0.25]);
        assert_eq!(first.depth, -1.0);

        // Tile (2,1): two columns right, one row DOWN; value 6 -> cell 5 ->
        // col 1, row 1 of the 4x4 tileset.
        let second = &batch.instances[1];
        assert_eq!(second.position, [180.0, 160.0]);
        assert_eq!(second.tex_region, [0.25, 0.25, 0.25, 0.25]);
    }

    #[test]
    fn test_tilemap_without_transform_is_skipped() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(&entity, Tilemap::new(2, 2, 32.0)).unwrap();

        let mut batcher = SpriteBatcher::new();
        append_tilemap_sprites(&world, &mut batcher);
        assert!(batcher.batches().is_empty());
    }
}
