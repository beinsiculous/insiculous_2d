//! Change detection for sprite instance uploads.
//!
//! Flattening batches and uploading the instance buffer every frame is pure
//! waste when nothing on screen moved. [`InstanceCache`] stages the flattened
//! instances into a reusable buffer and reports whether they (or the batch
//! layout — texture boundaries) differ from what was last staged; the GPU
//! upload is skipped when they don't. Instances are compared as raw bytes
//! (they're `bytemuck::Pod`), so the check is exact and NaN-safe.

use bytemuck::cast_slice;

use crate::sprite::SpriteBatch;
use crate::sprite_data::SpriteInstance;
use crate::texture::TextureHandle;

/// Staging buffer + last-uploaded snapshot for sprite instances.
#[derive(Default)]
pub struct InstanceCache {
    /// Instances as last staged for upload.
    instances: Vec<SpriteInstance>,
    /// Batch layout as last staged: (texture, instance count) per batch.
    layout: Vec<(TextureHandle, usize)>,
    /// Scratch buffers reused across frames (no per-frame allocations).
    staging: Vec<SpriteInstance>,
    staging_layout: Vec<(TextureHandle, usize)>,
    uploads_performed: u64,
    uploads_skipped: u64,
}

impl InstanceCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Flatten `batches` into the staging buffer and report whether the
    /// result differs from what was last staged (i.e. whether a GPU upload
    /// is needed). On change the staged data becomes the new snapshot.
    pub fn stage(&mut self, batches: &[&SpriteBatch]) -> bool {
        self.staging.clear();
        self.staging_layout.clear();
        for batch in batches {
            self.staging.extend_from_slice(&batch.instances);
            self.staging_layout.push((batch.texture_handle, batch.instances.len()));
        }

        let unchanged = self.staging_layout == self.layout
            && cast_slice::<SpriteInstance, u8>(&self.staging)
                == cast_slice::<SpriteInstance, u8>(&self.instances);

        if unchanged {
            self.uploads_skipped += 1;
            false
        } else {
            std::mem::swap(&mut self.instances, &mut self.staging);
            std::mem::swap(&mut self.layout, &mut self.staging_layout);
            self.uploads_performed += 1;
            true
        }
    }

    /// The instances staged by the last [`stage`](Self::stage) call that
    /// reported a change — the data to upload.
    pub fn staged(&self) -> &[SpriteInstance] {
        &self.instances
    }

    /// Total number of `stage` calls that required an upload.
    pub fn uploads_performed(&self) -> u64 {
        self.uploads_performed
    }

    /// Total number of `stage` calls skipped because nothing changed.
    pub fn uploads_skipped(&self) -> u64 {
        self.uploads_skipped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::fixtures::{batch_with, instance};

    /// The upload is skipped only while nothing changed: an identical restage
    /// skips, a moved sprite uploads (and the snapshot follows it), and
    /// empty ↔ content in either direction counts as a change.
    #[test]
    fn test_upload_is_skipped_only_while_the_staged_instances_are_unchanged() {
        let mut cache = InstanceCache::new();
        let empty: [&SpriteBatch; 0] = [];
        let batch = batch_with(&[instance(1.0), instance(2.0)], TextureHandle::WHITE);
        let moved = batch_with(&[instance(5.0), instance(2.0)], TextureHandle::WHITE);

        assert!(!cache.stage(&empty), "an empty first frame stages nothing new");
        assert!(cache.stage(&[&batch]), "content after empty must upload");
        assert!(!cache.stage(&[&batch]), "an identical restage must skip");
        assert!(!cache.stage(&[&batch]), "and keep skipping");
        assert!(cache.stage(&[&moved]), "a moved instance must re-upload");
        assert_eq!(cache.staged()[0].position, [5.0, 0.0], "the snapshot follows the move");
        assert!(cache.stage(&empty), "content -> empty is a change");
        assert!(cache.staged().is_empty());

        assert_eq!((cache.uploads_performed(), cache.uploads_skipped()), (3, 3));
    }

    /// The subtle half: the flattened bytes can be identical while the batch
    /// boundaries (the draw ranges) moved — that must still re-upload.
    #[test]
    fn test_same_bytes_with_different_batch_boundaries_still_upload() {
        let mut cache = InstanceCache::new();
        let one = batch_with(&[instance(1.0), instance(2.0)], TextureHandle::WHITE);
        assert!(cache.stage(&[&one]));

        let a = batch_with(&[instance(1.0)], TextureHandle::WHITE);
        let b = batch_with(&[instance(2.0)], TextureHandle { id: 7 });

        assert!(
            cache.stage(&[&a, &b]),
            "same bytes with different batch boundaries must re-upload"
        );
    }
}
