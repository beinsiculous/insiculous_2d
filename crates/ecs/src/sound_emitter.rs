//! `SoundEmitter` — a sound a scene names on an entity.
//!
//! The component is data only: what to play (a recorded file by path, or a binding of
//! the playing song by name), how loud, whether it starts on its own and loops, and
//! whether its level and pan follow its distance from the main camera. Nothing here knows
//! audio. The engine's emitter system (`engine_core::sound_emitters`) resolves the sound
//! the first time it is played and keeps every runtime fact — first sight, the playing
//! instance, the last mix — in a table of its own, so the component saves, snapshots and
//! copies as plain data.

use serde::{Deserialize, Serialize};

use crate::component_registry::ComponentMeta;
use crate::DeriveComponentMeta;

/// What a [`SoundEmitter`] plays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoundRef {
    /// A recorded file (WAV, MP3, OGG or FLAC), relative to the game's asset base like a
    /// texture path.
    Path(String),
    /// A binding of the playing song, by name: the song's part on its instrument, placed
    /// on the beat grid. The conductor owns its dynamics, so a binding emitter neither
    /// loops nor follows the camera.
    Binding(String),
}

impl Default for SoundRef {
    fn default() -> Self {
        SoundRef::Path(String::new())
    }
}

impl SoundRef {
    /// The path or the binding name.
    pub fn name(&self) -> &str {
        match self {
            SoundRef::Path(name) | SoundRef::Binding(name) => name,
        }
    }

    pub fn is_binding(&self) -> bool {
        matches!(self, SoundRef::Binding(_))
    }
}

/// Distance attenuation and pan from the main camera, in world units (pixels over
/// `RENDER_UNIT`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Spatial {
    /// The distance at which the sound falls silent; at half of it the level is halved,
    /// and an emitter half of it to the camera's right pans halfway right. Floored at
    /// [`Spatial::MIN_MAX_DISTANCE`] wherever it is used.
    pub max_distance: f32,
}

impl Spatial {
    /// The smallest `max_distance`: zero would divide the attenuation by zero.
    pub const MIN_MAX_DISTANCE: f32 = 0.01;
}

impl Default for Spatial {
    fn default() -> Self {
        Self { max_distance: 10.0 }
    }
}

/// A sound named on an entity; see the module docs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DeriveComponentMeta)]
#[serde(default)]
pub struct SoundEmitter {
    pub sound: SoundRef,
    /// Level of a recorded sound, 0..=1; a binding's level is the song's.
    pub volume: f32,
    /// Play the first frame the engine runs with this entity in the world. On the web a
    /// one-shot played before the page's first gesture is dropped — the audio manager holds
    /// only loops for the gesture — so a one-shot spawn sound in the first scene is silent
    /// there; a loop waits and starts.
    pub play_on_spawn: bool,
    /// Loop a recorded sound until it is stopped or its entity goes.
    pub looping: bool,
    /// Follow the main camera's distance and side; `None` plays flat.
    pub spatial: Option<Spatial>,
}

impl Default for SoundEmitter {
    /// A recorded-sound emitter with no path yet, at full volume, that waits to be played.
    fn default() -> Self {
        Self {
            sound: SoundRef::default(),
            volume: 1.0,
            play_on_spawn: false,
            looping: false,
            spatial: None,
        }
    }
}
