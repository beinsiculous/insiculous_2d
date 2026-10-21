//! Each playing instance of a sound, on its own: stopped, re-mixed and asked about by the
//! [`PlaybackId`] that [`AudioManager::play_with_settings`] returned, while other
//! instances of the same sound play on.
//!
//! **One contract with or without a device.** A disabled manager keeps a ledger of what
//! it was asked to play, so a caller's bookkeeping and a headless test see what a device
//! would give them: a loop is reported playing until it is stopped, a one-shot ends at
//! the next `update`. Before the web's gesture a loop is *held* — its sound, settings and
//! pan — and [`AudioManager::enable_output`] starts it, as it starts pending music; a
//! one-shot from before the gesture is dropped, since its moment has passed.

use std::sync::Arc;

use common::clamp_volume;

use crate::pan_source::PanLevel;
use crate::sound::{PlaybackId, SoundHandle, SoundSettings};

use super::{effective_volume, AudioManager};

/// A playback requested while no device was connected.
pub(super) struct HeldPlayback {
    pub(super) playback: PlaybackId,
    pub(super) handle: SoundHandle,
    pub(super) settings: SoundSettings,
    /// The pan the source will read once it starts; written by `set_playback_mix` while
    /// held, so a loop starts where its emitter is by then.
    pub(super) pan: Arc<PanLevel>,
}

impl AudioManager {
    /// Stop one playback. Other instances of its sound play on; an ended or unknown
    /// playback is a no-op.
    pub fn stop_playback(&mut self, playback: PlaybackId) {
        self.active_sounds.retain(|active| {
            if active.playback == playback {
                active.sink.stop();
                false
            } else {
                true
            }
        });
        self.held_playbacks.retain(|held| held.playback != playback);
    }

    /// Set one playback's volume (its base, under the sfx bus and master, clamped to
    /// 0.0..=1.0) and its pan (clamped to −1.0..=1.0 where the source reads it). Takes
    /// effect at once on a playing sound and when a held loop starts; an ended or unknown
    /// playback is a no-op.
    pub fn set_playback_mix(&mut self, playback: PlaybackId, volume: f32, pan: f32) {
        let base_volume = clamp_volume(volume);
        if let Some(active) = self.active_sounds.iter_mut().find(|active| active.playback == playback) {
            active.base_volume = base_volume;
            active
                .sink
                .set_volume(effective_volume(base_volume, self.sfx_volume, self.master_volume));
            active.pan.set(pan);
        } else if let Some(held) = self.held_playbacks.iter_mut().find(|held| held.playback == playback) {
            held.settings.volume = base_volume;
            held.pan.set(pan);
        }
    }

    /// Whether `playback` is still playing: its sound has not ended (`update` frees an
    /// ended one) and nobody stopped it. Without a device the ledger answers — a loop
    /// until it is stopped, a one-shot until the next `update` (see the module docs).
    #[must_use]
    pub fn is_playing(&self, playback: PlaybackId) -> bool {
        self.active_sounds.iter().any(|active| active.playback == playback)
            || self.held_playbacks.iter().any(|held| held.playback == playback)
    }

    /// Start every loop held while disabled, under its own playback id; drop the held
    /// one-shots. A loop that fails to start is logged, not returned: the device upgrade
    /// itself succeeded.
    pub(super) fn start_held_loops(&mut self) {
        for held in std::mem::take(&mut self.held_playbacks) {
            if !held.settings.looping {
                continue;
            }
            let playback = held.playback;
            if let Err(error) = self.start_playback(held) {
                log::warn!("Held loop {} failed to start: {error}", playback.id());
            }
        }
    }
}
