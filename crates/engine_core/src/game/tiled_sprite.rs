//! Expansion of an animated sprite rectangle into native-size repeated patches.

use ecs::SpriteTiling;
use glam::Vec2;
use renderer::sprite::{Sprite, SpriteBatcher};
use std::sync::atomic::{AtomicBool, Ordering};

static REPORTED_INVALID_LAYOUT: AtomicBool = AtomicBool::new(false);

/// Append native-size patches for a fully transformed renderer sprite, preserving
/// signed scale, flipped UVs and a shared pixel-snap anchor. Zero-size sprites draw nothing.
/// Expansion stops when adding the assembly would exceed the batch's patch budget.
pub fn append_tiled_sprite(sprite: &Sprite, tiling: &SpriteTiling, batcher: &mut SpriteBatcher) {
    if sprite.scale.x == 0.0 || sprite.scale.y == 0.0 {
        return;
    }
    let mut region = sprite.tex_region;
    let mut mirror = sprite.scale.signum();
    for axis in 0..2 {
        if region[axis + 2] < 0.0 {
            region[axis] += region[axis + 2];
            region[axis + 2] = -region[axis + 2];
            mirror[axis] = -mirror[axis];
        }
    }
    let patch_count = tiling.patch_count(sprite.scale.abs(), region);
    let remaining = SpriteTiling::MAX_PATCHES.saturating_sub(batcher.sprite_count());
    if patch_count == 0 || patch_count > remaining {
        if !REPORTED_INVALID_LAYOUT.swap(true, Ordering::Relaxed) {
            log::warn!("A tiled sprite cannot render: check source/destination dimensions and the {}-patch batch budget; texture {:?}, position {:?}, source {:?}, destination {:?}. Further layout warnings are suppressed.",
                SpriteTiling::MAX_PATCHES, sprite.texture_handle, sprite.position, tiling.source_size, sprite.scale);
        }
        return;
    }
    for tile in tiling.tiles(sprite.scale.abs(), region) {
        let offset = Vec2::from_angle(sprite.rotation).rotate(tile.offset * mirror);
        let mut patch = sprite.clone();
        patch.position += offset;
        patch.scale = tile.size * mirror;
        patch.tex_region = tile.tex_region;
        patch.snap_anchor = Some(sprite.position);
        batcher.add_sprite(&patch);
    }
}
