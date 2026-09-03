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

    /// Get total sprite count (used by tests)
    #[cfg(test)]
    pub fn sprite_count(&self) -> usize {
        self.batches.values().map(|batch| batch.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::fixtures::{batch_with, instance_at_depth};

    fn depths(batch: &SpriteBatch) -> Vec<f32> {
        batch.instances.iter().map(|instance| instance.depth).collect()
    }

    // === SpriteBatch: depth order ===

    /// Alpha blending needs back-to-front order; `total_cmp` puts NaN last
    /// instead of panicking the way `partial_cmp().unwrap()` did.
    #[test]
    fn test_sort_by_depth_orders_ascending_with_nan_last() {
        let mut batch = batch_with(
            &[
                instance_at_depth(3.0),
                instance_at_depth(f32::NAN),
                instance_at_depth(1.0),
                instance_at_depth(2.0),
            ],
            TextureHandle::WHITE,
        );

        batch.sort_by_depth();

        let sorted = depths(&batch);
        assert_eq!(&sorted[..3], &[1.0, 2.0, 3.0]);
        assert!(sorted[3].is_nan(), "NaN sorts after every real depth, got {sorted:?}");
        assert!(batch.sorted);
    }

    /// The `sorted` flag is the whole point of the guard in `sort_by_depth`:
    /// a sorted batch is not re-sorted (mutating `instances` behind its back
    /// proves the skip), and only an add or a clear makes it dirty again.
    #[test]
    fn test_sorted_flag_skips_resort_until_an_add_or_clear() {
        let mut batch = batch_with(
            &[instance_at_depth(2.0), instance_at_depth(1.0)],
            TextureHandle::WHITE,
        );
        batch.sort_by_depth();

        batch.instances.swap(0, 1);
        batch.sort_by_depth();
        assert_eq!(depths(&batch), [2.0, 1.0], "a sorted batch must not re-sort");

        batch.add_instance(instance_at_depth(0.0));
        assert!(!batch.sorted, "an add dirties the flag");
        batch.sort_by_depth();
        assert_eq!(depths(&batch), [0.0, 1.0, 2.0]);

        batch.clear();
        assert!(!batch.sorted, "a clear dirties the flag");
        assert!(batch.is_empty());
    }

    // === SpriteBatcher: grouping ===

    /// The game path: no clip is ever set, so sprites group purely by
    /// texture and every batch is unclipped.
    #[test]
    fn test_sprites_group_into_one_unclipped_batch_per_texture() {
        let mut batcher = SpriteBatcher::new();
        let (one, two, three) = (
            TextureHandle { id: 1 },
            TextureHandle { id: 2 },
            TextureHandle { id: 3 },
        );

        for texture in [one, one, two, two, three] {
            batcher.add_sprite(&Sprite::new(texture));
        }

        assert_eq!(batcher.batches().len(), 3, "one batch per texture");
        assert_eq!(batcher.sprite_count(), 5);
        for (texture, expected) in [(one, 2), (two, 2), (three, 1)] {
            let batch = batcher.batch_for(texture).expect("a batch per texture");
            assert_eq!(batch.len(), expected, "sprites on texture {}", texture.id);
            assert_eq!(batch.texture_handle, texture);
            assert_eq!(batch.clip, None, "game batches never carry a clip");
        }
    }

    /// `sort_all_batches` is what `engine_core` calls once per frame; every
    /// group must come out depth-ordered, not just the first.
    #[test]
    fn test_sort_all_batches_orders_every_texture_group_by_depth() {
        let mut batcher = SpriteBatcher::new();
        let (one, two) = (TextureHandle { id: 1 }, TextureHandle { id: 2 });
        for (texture, depth) in [(one, 3.0), (one, 1.0), (two, 5.0), (two, 2.0)] {
            batcher.add_sprite(&Sprite::new(texture).with_depth(depth));
        }

        batcher.sort_all_batches();

        for (texture, expected) in [(one, [1.0, 3.0]), (two, [2.0, 5.0])] {
            let batch = batcher.batch_for(texture).expect("a batch per texture");
            assert_eq!(depths(batch), expected, "texture {}", texture.id);
            assert!(batch.sorted);
        }
    }

    /// Clipped UI (issue #41): the same texture under two clip states is two
    /// draws, each carrying its own scissor.
    #[test]
    fn test_same_texture_under_two_clip_states_splits_into_two_batches() {
        let mut batcher = SpriteBatcher::new();
        let texture = TextureHandle { id: 1 };
        let clip = [10, 20, 100, 50];

        batcher.add_sprite(&Sprite::new(texture));
        batcher.set_clip(Some(clip));
        batcher.add_sprite(&Sprite::new(texture));
        batcher.add_sprite(&Sprite::new(texture));
        batcher.set_clip(None);
        batcher.add_sprite(&Sprite::new(texture));

        assert_eq!(batcher.batches().len(), 2, "same texture, two clip states");
        let unclipped = batcher.batch_for(texture).expect("the unclipped batch");
        assert_eq!((unclipped.len(), unclipped.clip), (2, None));
        let clipped = batcher
            .batches()
            .get(&(texture, Some(clip)))
            .expect("the clipped batch");
        assert_eq!((clipped.len(), clipped.clip), (2, Some(clip)));
    }

    /// An unbalanced push in one frame must not clip the next; the batches
    /// themselves stay allocated (emptied, not dropped) so a steady frame
    /// never reallocates.
    #[test]
    fn test_clear_resets_the_clip_cursor_and_keeps_batches_allocated() {
        let mut batcher = SpriteBatcher::new();
        let texture = TextureHandle { id: 1 };
        let clip = [0, 0, 10, 10];
        batcher.set_clip(Some(clip));
        batcher.add_sprite(&Sprite::new(texture));

        batcher.clear();
        batcher.add_sprite(&Sprite::new(texture));

        assert_eq!(batcher.sprite_count(), 1);
        let next_frame = batcher.batch_for(texture).expect("the new sprite is unclipped");
        assert_eq!((next_frame.len(), next_frame.clip), (1, None));
        let stale = batcher
            .batches()
            .get(&(texture, Some(clip)))
            .expect("kept, not dropped");
        assert!(stale.is_empty(), "cleared, not dropped");
    }
}
