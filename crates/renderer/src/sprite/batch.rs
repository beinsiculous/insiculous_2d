//! CPU-side sprite batching: grouping sprites by texture before GPU upload.

use std::collections::HashMap;

use crate::sprite::Sprite;
use crate::sprite_data::SpriteInstance;
use crate::texture::TextureHandle;

/// A batch of sprites using the same texture
#[derive(Debug, Clone)]
pub struct SpriteBatch {
    /// Texture handle for this batch
    pub texture_handle: TextureHandle,
    /// Sprite instances
    pub instances: Vec<SpriteInstance>,
    /// Whether this batch is sorted by depth
    pub sorted: bool,
    /// GPU scissor rect (`[x, y, w, h]` in physical surface pixels) applied
    /// when drawing this batch, or `None` for the pass default. Set by the
    /// UI batcher from `PushClipRect`/`PopClipRect` (issue #41); game
    /// batches never carry a clip.
    pub clip: Option<[u32; 4]>,
}

impl SpriteBatch {
    /// Create a new sprite batch
    pub fn new(texture_handle: TextureHandle) -> Self {
        Self {
            texture_handle,
            instances: Vec::new(),
            sorted: false,
            clip: None,
        }
    }

    /// Add a sprite instance to the batch
    pub fn add_instance(&mut self, instance: SpriteInstance) {
        self.instances.push(instance);
        self.sorted = false;
    }

    /// Add multiple sprite instances
    pub fn add_instances(&mut self, instances: &[SpriteInstance]) {
        self.instances.extend_from_slice(instances);
        self.sorted = false;
    }

    /// Sort instances by depth (for proper alpha blending).
    ///
    /// Uses `total_cmp` so NaN depths sort deterministically instead of
    /// panicking.
    pub fn sort_by_depth(&mut self) {
        if !self.sorted {
            self.instances.sort_by(|a, b| a.depth.total_cmp(&b.depth));
            self.sorted = true;
        }
    }

    /// Get the number of instances
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Clear all instances
    pub fn clear(&mut self) {
        self.instances.clear();
        self.sorted = false;
    }
}

/// Sprite batcher for efficient rendering.
///
/// Batches are keyed by `(texture, clip)`: sprites sharing a texture AND
/// the active clip rect merge into one draw. Game rendering never sets a
/// clip, so its batching is identical to the old by-texture map; the UI
/// integration drives [`set_clip`](Self::set_clip) from its clip-rect
/// stack so clipped UI regions scissor on the GPU (issue #41).
#[derive(Default)]
pub struct SpriteBatcher {
    batches: HashMap<(TextureHandle, Option<[u32; 4]>), SpriteBatch>,
    /// Clip applied to sprites added from now on (a cursor, not a filter).
    current_clip: Option<[u32; 4]>,
}

impl SpriteBatcher {
    /// Create a new sprite batcher
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the clip rect (physical surface pixels) applied to every sprite
    /// added after this call. `None` = unclipped (the default).
    pub fn set_clip(&mut self, clip: Option<[u32; 4]>) {
        self.current_clip = clip;
    }

    /// Add a sprite to the batcher
    pub fn add_sprite(&mut self, sprite: &Sprite) {
        let clip = self.current_clip;
        let batch = self.batches
            .entry((sprite.texture_handle, clip))
            .or_insert_with(|| {
                let mut batch = SpriteBatch::new(sprite.texture_handle);
                batch.clip = clip;
                batch
            });

        batch.add_instance(sprite.to_instance());
    }

    /// Add multiple sprites
    pub fn add_sprites(&mut self, sprites: &[Sprite]) {
        for sprite in sprites {
            self.add_sprite(sprite);
        }
    }

    /// Sort all batches by depth
    pub fn sort_all_batches(&mut self) {
        for batch in self.batches.values_mut() {
            batch.sort_by_depth();
        }
    }

    /// Get all batches
    pub fn batches(&self) -> &HashMap<(TextureHandle, Option<[u32; 4]>), SpriteBatch> {
        &self.batches
    }

    /// Get mutable batches
    pub fn batches_mut(&mut self) -> &mut HashMap<(TextureHandle, Option<[u32; 4]>), SpriteBatch> {
        &mut self.batches
    }

    /// The unclipped batch for a texture, if any — the common case for game
    /// rendering, where no clip is ever set.
    pub fn batch_for(&self, texture: TextureHandle) -> Option<&SpriteBatch> {
        self.batches.get(&(texture, None))
    }

    /// Clear all batches. Also resets the clip cursor, so an unbalanced
    /// push/pop in one frame can never leak a clip into the next.
    pub fn clear(&mut self) {
        for batch in self.batches.values_mut() {
            batch.clear();
        }
        self.current_clip = None;
    }

    /// Get total sprite count
    pub fn sprite_count(&self) -> usize {
        self.batches.values().map(|batch| batch.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Vec2, Vec4};

    // ==================== SpriteBatch Tests ====================

    #[test]
    fn test_sprite_batch_new() {
        let handle = TextureHandle::new(5);
        let batch = SpriteBatch::new(handle);
        assert_eq!(batch.texture_handle.id, 5);
        assert!(batch.instances.is_empty());
        assert!(!batch.sorted);
    }

    #[test]
    fn test_sprite_batch_add_instance() {
        let mut batch = SpriteBatch::new(TextureHandle::default());
        let instance = SpriteInstance::new(
            Vec2::new(10.0, 20.0),
            0.0,
            Vec2::ONE,
            [0.0, 0.0, 1.0, 1.0],
            Vec4::ONE,
            0.0,
        );
        batch.add_instance(instance);
        assert_eq!(batch.len(), 1);
        assert!(!batch.sorted);
    }

    #[test]
    fn test_sprite_batch_add_instances() {
        let mut batch = SpriteBatch::new(TextureHandle::default());
        let instances = vec![
            SpriteInstance::new(Vec2::ZERO, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 0.0),
            SpriteInstance::new(Vec2::ONE, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 1.0),
            SpriteInstance::new(Vec2::new(2.0, 2.0), 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 2.0),
        ];
        batch.add_instances(&instances);
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn test_sprite_batch_sort_by_depth() {
        let mut batch = SpriteBatch::new(TextureHandle::default());
        batch.add_instance(SpriteInstance::new(Vec2::ZERO, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 3.0));
        batch.add_instance(SpriteInstance::new(Vec2::ZERO, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 1.0));
        batch.add_instance(SpriteInstance::new(Vec2::ZERO, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 2.0));

        assert!(!batch.sorted);
        batch.sort_by_depth();
        assert!(batch.sorted);

        // Verify sorted order (ascending)
        assert_eq!(batch.instances[0].depth, 1.0);
        assert_eq!(batch.instances[1].depth, 2.0);
        assert_eq!(batch.instances[2].depth, 3.0);
    }

    #[test]
    fn test_sprite_batch_sort_handles_nan_depth_without_panicking() {
        let mut batch = SpriteBatch::new(TextureHandle::default());
        batch.add_instance(SpriteInstance::new(Vec2::ZERO, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, f32::NAN));
        batch.add_instance(SpriteInstance::new(Vec2::ZERO, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 1.0));
        batch.add_instance(SpriteInstance::new(Vec2::ZERO, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 0.5));

        batch.sort_by_depth();

        // total_cmp orders NaN after all real numbers; the real values stay sorted.
        assert!(batch.sorted);
        assert_eq!(batch.instances[0].depth, 0.5);
        assert_eq!(batch.instances[1].depth, 1.0);
        assert!(batch.instances[2].depth.is_nan());
    }

    #[test]
    fn test_sprite_batch_sort_idempotent() {
        let mut batch = SpriteBatch::new(TextureHandle::default());
        batch.add_instance(SpriteInstance::new(Vec2::ZERO, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 2.0));
        batch.add_instance(SpriteInstance::new(Vec2::ZERO, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 1.0));

        batch.sort_by_depth();
        assert!(batch.sorted);

        // Sorting again should be a no-op since already sorted
        batch.sort_by_depth();
        assert!(batch.sorted);
        assert_eq!(batch.instances[0].depth, 1.0);
        assert_eq!(batch.instances[1].depth, 2.0);
    }

    #[test]
    fn test_sprite_batch_len_and_is_empty() {
        let mut batch = SpriteBatch::new(TextureHandle::default());
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);

        batch.add_instance(SpriteInstance::new(Vec2::ZERO, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 0.0));
        assert!(!batch.is_empty());
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn test_sprite_batch_clear() {
        let mut batch = SpriteBatch::new(TextureHandle::default());
        batch.add_instance(SpriteInstance::new(Vec2::ZERO, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 0.0));
        batch.add_instance(SpriteInstance::new(Vec2::ONE, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 1.0));
        batch.sort_by_depth();

        assert_eq!(batch.len(), 2);
        assert!(batch.sorted);

        batch.clear();
        assert!(batch.is_empty());
        assert!(!batch.sorted);
    }

    #[test]
    fn test_sprite_batch_sorted_flag_reset_on_add() {
        let mut batch = SpriteBatch::new(TextureHandle::default());
        batch.add_instance(SpriteInstance::new(Vec2::ZERO, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 0.0));
        batch.sort_by_depth();
        assert!(batch.sorted);

        // Adding should reset sorted flag
        batch.add_instance(SpriteInstance::new(Vec2::ONE, 0.0, Vec2::ONE, [0.0, 0.0, 1.0, 1.0], Vec4::ONE, 1.0));
        assert!(!batch.sorted);
    }

    // ==================== SpriteBatcher Tests ====================

    #[test]
    fn test_sprite_batcher_new() {
        let batcher = SpriteBatcher::new();
        assert_eq!(batcher.sprite_count(), 0);
        assert!(batcher.batches().is_empty());
    }

    #[test]
    fn test_sprite_batcher_add_sprite() {
        let mut batcher = SpriteBatcher::new();
        let sprite = Sprite::new(TextureHandle::new(1));
        batcher.add_sprite(&sprite);
        assert_eq!(batcher.sprite_count(), 1);
    }

    #[test]
    fn test_sprite_batcher_add_sprites() {
        let mut batcher = SpriteBatcher::new();
        let sprites = vec![
            Sprite::new(TextureHandle::new(1)),
            Sprite::new(TextureHandle::new(1)),
            Sprite::new(TextureHandle::new(2)),
        ];
        batcher.add_sprites(&sprites);
        assert_eq!(batcher.sprite_count(), 3);
    }

    #[test]
    fn test_sprite_batcher_groups_by_texture() {
        let mut batcher = SpriteBatcher::new();

        // Add sprites with different textures
        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));
        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));
        batcher.add_sprite(&Sprite::new(TextureHandle::new(2)));
        batcher.add_sprite(&Sprite::new(TextureHandle::new(2)));
        batcher.add_sprite(&Sprite::new(TextureHandle::new(3)));

        let batches = batcher.batches();
        assert_eq!(batches.len(), 3); // 3 different textures

        assert_eq!(batcher.batch_for(TextureHandle::new(1)).unwrap().len(), 2);
        assert_eq!(batcher.batch_for(TextureHandle::new(2)).unwrap().len(), 2);
        assert_eq!(batcher.batch_for(TextureHandle::new(3)).unwrap().len(), 1);
    }

    #[test]
    fn test_sprite_batcher_splits_same_texture_by_clip() {
        let mut batcher = SpriteBatcher::new();

        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));
        batcher.set_clip(Some([10, 20, 100, 50]));
        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));
        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));
        batcher.set_clip(None);
        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));

        // Same texture, two clip states -> two batches.
        assert_eq!(batcher.batches().len(), 2);
        let unclipped = batcher.batch_for(TextureHandle::new(1)).unwrap();
        assert_eq!(unclipped.len(), 2);
        assert_eq!(unclipped.clip, None);

        let clipped = batcher
            .batches()
            .get(&(TextureHandle::new(1), Some([10, 20, 100, 50])))
            .unwrap();
        assert_eq!(clipped.len(), 2);
        assert_eq!(clipped.clip, Some([10, 20, 100, 50]));
    }

    #[test]
    fn test_sprite_batcher_no_clip_batches_identically_to_before() {
        let mut batcher = SpriteBatcher::new();
        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));
        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));

        // Never touching set_clip yields exactly one batch per texture,
        // carrying no clip — the game path is unchanged.
        assert_eq!(batcher.batches().len(), 1);
        assert_eq!(batcher.batch_for(TextureHandle::new(1)).unwrap().clip, None);
    }

    #[test]
    fn test_sprite_batcher_clear_resets_clip_cursor() {
        let mut batcher = SpriteBatcher::new();
        batcher.set_clip(Some([0, 0, 10, 10]));
        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));

        // Unbalanced push (no set_clip(None)) must not leak across frames.
        batcher.clear();
        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));
        assert!(batcher.batch_for(TextureHandle::new(1)).is_some());
        assert_eq!(batcher.batch_for(TextureHandle::new(1)).unwrap().len(), 1);
    }

    #[test]
    fn test_sprite_batcher_sprite_count() {
        let mut batcher = SpriteBatcher::new();
        assert_eq!(batcher.sprite_count(), 0);

        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));
        assert_eq!(batcher.sprite_count(), 1);

        batcher.add_sprite(&Sprite::new(TextureHandle::new(2)));
        assert_eq!(batcher.sprite_count(), 2);

        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));
        assert_eq!(batcher.sprite_count(), 3);
    }

    #[test]
    fn test_sprite_batcher_sort_all_batches() {
        let mut batcher = SpriteBatcher::new();

        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)).with_depth(3.0));
        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)).with_depth(1.0));
        batcher.add_sprite(&Sprite::new(TextureHandle::new(2)).with_depth(5.0));
        batcher.add_sprite(&Sprite::new(TextureHandle::new(2)).with_depth(2.0));

        batcher.sort_all_batches();

        let batch1 = batcher.batch_for(TextureHandle::new(1)).unwrap();
        assert!(batch1.sorted);
        assert_eq!(batch1.instances[0].depth, 1.0);
        assert_eq!(batch1.instances[1].depth, 3.0);

        let batch2 = batcher.batch_for(TextureHandle::new(2)).unwrap();
        assert!(batch2.sorted);
        assert_eq!(batch2.instances[0].depth, 2.0);
        assert_eq!(batch2.instances[1].depth, 5.0);
    }

    #[test]
    fn test_sprite_batcher_clear() {
        let mut batcher = SpriteBatcher::new();
        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));
        batcher.add_sprite(&Sprite::new(TextureHandle::new(2)));
        assert_eq!(batcher.sprite_count(), 2);

        batcher.clear();
        assert_eq!(batcher.sprite_count(), 0);

        // Batches still exist but are empty
        assert!(!batcher.batches().is_empty());
        for batch in batcher.batches().values() {
            assert!(batch.is_empty());
        }
    }

    #[test]
    fn test_sprite_batcher_batches_mutable() {
        let mut batcher = SpriteBatcher::new();
        batcher.add_sprite(&Sprite::new(TextureHandle::new(1)));

        // Verify we can get mutable access
        let batches = batcher.batches_mut();
        if let Some(batch) = batches.get_mut(&(TextureHandle::new(1), None)) {
            batch.clear();
        }

        assert_eq!(batcher.sprite_count(), 0);
    }
}
