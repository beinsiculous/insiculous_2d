//! Audio components for ECS integration
//!
//! These components allow entities to emit sounds and receive audio.

use crate::component_registry::ComponentMeta;
use ecs_macros::ComponentMeta as DeriveComponentMeta;
use serde::{Deserialize, Serialize};

/// Component for entities that can emit sounds.
///
/// An AudioSource represents a point in the game world that can play sounds.
/// When combined with a Transform2D, it enables positional audio effects.
#[derive(Debug, Clone, Serialize, Deserialize, DeriveComponentMeta)]
pub struct AudioSource {
    /// Sound handle ID (from AudioManager::load_sound)
    pub sound_id: u32,
    /// Volume multiplier (0.0 = silent, 1.0 = full volume)
    pub volume: f32,
    /// Playback speed multiplier (1.0 = normal)
    pub pitch: f32,
    /// Whether the sound should loop
    pub looping: bool,
    /// Whether to play the sound on spawn
    pub play_on_spawn: bool,
    /// Whether the sound is currently playing
    pub playing: bool,
    /// Whether to use spatial (2D positional) audio
    pub spatial: bool,
    /// Maximum distance at which the sound can be heard (for spatial audio)
    pub max_distance: f32,
    /// Reference distance for attenuation (for spatial audio)
    pub reference_distance: f32,
    /// Rolloff factor for distance attenuation (for spatial audio)
    pub rolloff_factor: f32,
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            sound_id: 0,
            volume: 1.0,
            pitch: 1.0,
            looping: false,
            play_on_spawn: false,
            playing: false,
            spatial: false,
            max_distance: 1000.0,
            reference_distance: 100.0,
            rolloff_factor: 1.0,
        }
    }
}

impl AudioSource {
    /// Create a new audio source with a sound ID.
    pub fn new(sound_id: u32) -> Self {
        Self {
            sound_id,
            ..Default::default()
        }
    }

    /// Set the volume.
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    /// Set the pitch/speed.
    pub fn with_pitch(mut self, pitch: f32) -> Self {
        self.pitch = pitch.max(0.1);
        self
    }

    /// Set whether the sound should loop.
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Set whether to play on spawn.
    pub fn with_play_on_spawn(mut self, play: bool) -> Self {
        self.play_on_spawn = play;
        self.playing = play;
        self
    }

    /// Enable spatial audio with default settings.
    pub fn with_spatial(mut self) -> Self {
        self.spatial = true;
        self
    }

    /// Configure spatial audio parameters.
    pub fn with_spatial_settings(
        mut self,
        max_distance: f32,
        reference_distance: f32,
        rolloff_factor: f32,
    ) -> Self {
        self.spatial = true;
        self.max_distance = max_distance;
        self.reference_distance = reference_distance;
        self.rolloff_factor = rolloff_factor;
        self
    }

    /// Start playing the sound.
    pub fn play(&mut self) {
        self.playing = true;
    }

    /// Stop playing the sound.
    pub fn stop(&mut self) {
        self.playing = false;
    }

    /// Check if the source is set to play.
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Calculate volume attenuation based on distance from listener.
    ///
    /// Uses inverse distance attenuation model:
    /// attenuation = reference_distance / (reference_distance + rolloff_factor * (distance - reference_distance))
    pub fn calculate_attenuation(&self, distance: f32) -> f32 {
        if !self.spatial {
            return 1.0;
        }

        if distance >= self.max_distance {
            return 0.0;
        }

        if distance <= self.reference_distance {
            return 1.0;
        }

        let attenuation = self.reference_distance
            / (self.reference_distance
                + self.rolloff_factor * (distance - self.reference_distance));

        attenuation.clamp(0.0, 1.0)
    }
}

/// Component for the entity that "hears" sounds.
///
/// Typically attached to the player or camera entity. There should only be
/// one active AudioListener in a scene. The listener's position determines
/// how spatial audio is processed.
#[derive(Debug, Clone, Serialize, Deserialize, DeriveComponentMeta)]
pub struct AudioListener {
    /// Whether this listener is active (only one should be active at a time)
    pub active: bool,
    /// Volume multiplier applied to all sounds heard by this listener
    pub volume: f32,
}

impl Default for AudioListener {
    fn default() -> Self {
        Self {
            active: true,
            volume: 1.0,
        }
    }
}

impl AudioListener {
    /// Create a new audio listener.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether this listener is active.
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Set the listener volume.
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }
}

/// One-shot sound effect request.
///
/// This component triggers a sound effect to play once and then the component
/// is typically removed. Useful for events like explosions, jumps, or pickups.
#[derive(Debug, Clone, Serialize, Deserialize, DeriveComponentMeta)]
pub struct PlaySoundEffect {
    /// Sound handle ID to play
    pub sound_id: u32,
    /// Volume for this specific play
    pub volume: f32,
    /// Pitch for this specific play
    pub pitch: f32,
    /// Whether to use spatial audio from entity position
    pub spatial: bool,
}

impl Default for PlaySoundEffect {
    fn default() -> Self {
        Self::new(0)
    }
}

impl PlaySoundEffect {
    /// Create a new sound effect request.
    pub fn new(sound_id: u32) -> Self {
        Self {
            sound_id,
            volume: 1.0,
            pitch: 1.0,
            spatial: false,
        }
    }

    /// Set the volume.
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    /// Set the pitch.
    pub fn with_pitch(mut self, pitch: f32) -> Self {
        self.pitch = pitch.max(0.1);
        self
    }

    /// Enable spatial audio.
    pub fn with_spatial(mut self) -> Self {
        self.spatial = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_source_attenuation() {
        let source = AudioSource::new(1).with_spatial_settings(1000.0, 100.0, 1.0);
        let attenuation_at = |distance: f32| source.calculate_attenuation(distance);

        // Inside the reference distance: full volume.
        assert!((attenuation_at(50.0) - 1.0).abs() < f32::EPSILON);
        assert!((attenuation_at(100.0) - 1.0).abs() < f32::EPSILON);
        // Inverse distance model: reference / (reference + rolloff * (distance - reference))
        // = 100 / (100 + 1 * (200 - 100)) = 0.5.
        assert!((attenuation_at(200.0) - 0.5).abs() < f32::EPSILON);
        // At and beyond the max distance: silent.
        assert!(attenuation_at(1000.0).abs() < f32::EPSILON);
        assert!(attenuation_at(2000.0).abs() < f32::EPSILON);

        // A non-spatial source never attenuates, whatever the distance.
        let flat = AudioSource::new(1);
        assert!((flat.calculate_attenuation(10_000.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_volume_builders_clamp_to_the_unit_range() {
        // Scene-authored volumes outside 0..=1 must never reach the mixer.
        assert_eq!(AudioSource::new(1).with_volume(2.0).volume, 1.0);
        assert_eq!(AudioSource::new(1).with_volume(-1.0).volume, 0.0);
        assert_eq!(AudioListener::new().with_volume(2.0).volume, 1.0);
        assert_eq!(AudioListener::new().with_volume(-1.0).volume, 0.0);
        assert_eq!(PlaySoundEffect::new(1).with_volume(2.0).volume, 1.0);
        assert_eq!(PlaySoundEffect::new(1).with_volume(-1.0).volume, 0.0);
    }
}
