//! Sound data types and playback settings.

/// Unique identifier for a loaded sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundHandle {
    pub(crate) id: u32,
}

impl SoundHandle {
    /// Wrap a manager-allocated id in a handle.
    ///
    /// Ids are allocated by the owning [`crate::AudioManager`] from an
    /// instance-local counter, so handles are unique within one manager but
    /// deterministic across managers (both start from the same base).
    pub(crate) fn from_id(id: u32) -> Self {
        Self { id }
    }

    /// Get the numeric ID of this handle.
    #[must_use]
    pub fn id(&self) -> u32 {
        self.id
    }
}

/// The volume bus a caller-built source plays under (see
/// [`crate::AudioManager::play_source`]): its sink volume is that bus's volume
/// times master, re-derived whenever either changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bus {
    /// Under [`crate::AudioManager::music_volume`].
    Music,
    /// Under [`crate::AudioManager::sfx_volume`].
    Sfx,
}

/// Settings for sound playback.
#[derive(Debug, Clone)]
pub struct SoundSettings {
    /// Volume level (0.0 = silent, 1.0 = full volume).
    pub volume: f32,
    /// Playback speed (1.0 = normal, 2.0 = double speed).
    pub speed: f32,
    /// Whether the sound should loop.
    pub looping: bool,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            volume: 1.0,
            speed: 1.0,
            looping: false,
        }
    }
}

impl SoundSettings {
    /// Create new sound settings with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the volume level (clamped to 0.0..=1.0).
    ///
    /// Note: the fields are public, so values are also re-clamped at the
    /// point of use in [`crate::AudioManager::play_with_settings`].
    #[must_use]
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    /// Set the playback speed (floored at 0.1).
    ///
    /// Note: the fields are public, so values are also re-clamped at the
    /// point of use in [`crate::AudioManager::play_with_settings`].
    #[must_use]
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.1);
        self
    }

    /// Set whether the sound should loop.
    #[must_use]
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }
}
