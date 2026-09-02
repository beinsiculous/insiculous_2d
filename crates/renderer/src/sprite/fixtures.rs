//! Shared fixtures for the sprite submodules' tests. Built through the live
//! `Sprite` → `to_instance` path, so they depend on no test-only constructor.

use glam::Vec2;

use super::{Sprite, SpriteBatch};
use crate::sprite_data::SpriteInstance;
use crate::texture::TextureHandle;

/// A white-texture instance at `(x, 0)` — distinct bytes per `x`, so the
/// instance cache can tell two of them apart.
pub(crate) fn instance(x: f32) -> SpriteInstance {
    Sprite::new(TextureHandle::WHITE)
        .with_position(Vec2::new(x, 0.0))
        .to_instance()
}

/// A white-texture instance at the origin with the given depth.
pub(crate) fn instance_at_depth(depth: f32) -> SpriteInstance {
    Sprite::new(TextureHandle::WHITE).with_depth(depth).to_instance()
}

/// A batch on `texture` holding `instances` in the given order.
pub(crate) fn batch_with(instances: &[SpriteInstance], texture: TextureHandle) -> SpriteBatch {
    let mut batch = SpriteBatch::new(texture);
    for instance in instances {
        batch.add_instance(*instance);
    }
    batch
}
