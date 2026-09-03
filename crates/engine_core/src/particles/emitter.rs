//! ECS component for continuous particle emission.
//!
//! Attach a [`ParticleEmitter`] to an entity that has a `Transform2D`. Each
//! frame, [`ParticleSystem::update`](super::ParticleSystem::update) emits
//! particles from the entity's transform position at the configured rate.
//!
//! For one-shot bursts (explosions, score effects), call
//! [`ParticleManager::spawn_burst`](super::ParticleManager::spawn_burst)
//! directly instead — no component needed.

use super::particle::ParticleConfig;

/// Continuous particle source. The component is purely data — the engine's
/// [`ParticleSystem`](super::ParticleSystem) reads it each frame to decide
/// when to spawn particles.
///
/// Particles are transient runtime state, so this component is not
/// serialized into scene files — emitters are expected to be set up in code
/// during `Game::init()`.
#[derive(Debug, Clone)]
pub struct ParticleEmitter {
    /// Particles per second.
    pub emission_rate: f32,
    /// Internal time accumulator. Don't touch from gameplay code.
    pub accumulator: f32,
    /// When false, the system skips this emitter (use it to pause without
    /// removing the component).
    pub active: bool,
    /// Configuration for each emitted particle.
    pub config: Option<ParticleConfig>,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            emission_rate: 30.0,
            accumulator: 0.0,
            active: true,
            config: None,
        }
    }
}

impl ParticleEmitter {
    /// New emitter with the given rate and per-particle config.
    pub fn new(emission_rate: f32, config: ParticleConfig) -> Self {
        Self {
            emission_rate,
            accumulator: 0.0,
            active: true,
            config: Some(config),
        }
    }

    /// Pause emission without removing the component.
    pub fn pause(&mut self) {
        self.active = false;
    }

    /// Resume emission.
    pub fn resume(&mut self) {
        self.active = true;
    }
}
