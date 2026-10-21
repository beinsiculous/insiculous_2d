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

/// One playback of a sound, returned by [`crate::AudioManager::play_with_settings`] so
/// that instance alone can be stopped, re-mixed or asked about while other instances of
/// the same sound play on. Ids come from a manager-local counter and are never reused; a
/// disabled manager mints them too, so a caller's bookkeeping is the same with or without
/// a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlaybackId {
    pub(crate) id: u64,
}

impl PlaybackId {
    /// The numeric id of this playback.
    #[must_use]
    pub fn id(&self) -> u64 {
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
    /// Stereo position: −1 hard left, 0 centred (the sound as recorded), 1 hard right.
    /// Changeable while the sound plays through [`crate::AudioManager::set_playback_mix`].
    pub pan: f32,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            volume: 1.0,
            speed: 1.0,
            looping: false,
            pan: 0.0,
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

    /// Set the stereo position (clamped to −1.0..=1.0; a non-finite pan centres).
    ///
    /// Note: the fields are public, so the pan is also re-clamped where the playing
    /// source reads it.
    #[must_use]
    pub fn with_pan(mut self, pan: f32) -> Self {
        self.pan = crate::pan_source::clamp_pan(pan);
        self
    }
}
