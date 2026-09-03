//! Physics world wrapper for rapier2d integration
//!
//! This module provides a wrapper around rapier2d that integrates with the ECS.
//!
//! Split by responsibility:
//! - `mod.rs` — configuration, world struct, construction, unit conversion
//! - `bodies.rs` — body/collider add/remove and per-body accessors
//! - `stepping.rs` — simulation stepping and collision event extraction

mod bodies;
mod stepping;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;

use glam::Vec2;
use rapier2d::prelude::*;
use serde::{Deserialize, Serialize};

use ecs::EntityId;

use crate::components::CollisionData;

use self::stepping::CollisionPair;

/// Default pixels-per-meter scale used when an invalid value is supplied.
pub const DEFAULT_PIXELS_PER_METER: f32 = 100.0;

/// Configuration for the physics world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsConfig {
    /// Gravity vector in units per second squared
    pub gravity: Vec2,
    /// Number of solver iterations (maps to rapier's `num_solver_iterations`)
    pub solver_iterations: usize,
    /// Number of additional friction iterations (maps to rapier's
    /// `num_additional_friction_iterations`)
    pub friction_iterations: usize,
    /// Pixels per meter scale factor (must be finite and > 0; invalid values
    /// fall back to [`DEFAULT_PIXELS_PER_METER`])
    pub pixels_per_meter: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: Vec2::new(0.0, -980.0), // 9.8 m/s^2 * 100 pixels/meter
            solver_iterations: 16,   // Increased for better collision resolution
            friction_iterations: 8,  // Increased for better stacking stability
            pixels_per_meter: DEFAULT_PIXELS_PER_METER,
        }
    }
}

impl PhysicsConfig {
    /// Create a new physics config with the given gravity
    pub fn new(gravity: Vec2) -> Self {
        Self {
            gravity,
            ..Default::default()
        }
    }

    /// Set gravity
    pub fn with_gravity(mut self, gravity: Vec2) -> Self {
        self.gravity = gravity;
        self
    }

    /// Set solver iterations (solver and additional friction iterations)
    pub(crate) fn with_iterations(mut self, solver: usize, friction: usize) -> Self {
        self.solver_iterations = solver;
        self.friction_iterations = friction;
        self
    }

    /// Set pixels per meter scale.
    ///
    /// Non-finite or non-positive values are rejected with a warning and
    /// fall back to [`DEFAULT_PIXELS_PER_METER`] (a zero scale would produce
    /// NaN positions via divide-by-zero).
    pub fn with_scale(mut self, pixels_per_meter: f32) -> Self {
        self.pixels_per_meter = sanitize_pixels_per_meter(pixels_per_meter);
        self
    }
}

/// Clamp a degenerate pixels-per-meter value to the default, with a warning.
fn sanitize_pixels_per_meter(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        log::warn!(
            "Invalid pixels_per_meter ({value}); falling back to default ({DEFAULT_PIXELS_PER_METER})"
        );
        DEFAULT_PIXELS_PER_METER
    }
}

/// Physics world that manages rapier2d simulation
pub struct PhysicsWorld {
    /// Rapier rigid body set
    rigid_body_set: RigidBodySet,
    /// Rapier collider set
    collider_set: ColliderSet,
    /// Physics pipeline
    physics_pipeline: PhysicsPipeline,
    /// Island manager
    island_manager: IslandManager,
    /// Broad phase
    broad_phase: DefaultBroadPhase,
    /// Narrow phase
    narrow_phase: NarrowPhase,
    /// Impulse joint set
    impulse_joint_set: ImpulseJointSet,
    /// Multibody joint set
    multibody_joint_set: MultibodyJointSet,
    /// CCD solver
    ccd_solver: CCDSolver,
    /// Integration parameters
    integration_parameters: IntegrationParameters,
    /// Configuration
    config: PhysicsConfig,
    /// Mapping from ECS entity to rapier rigid body handle
    entity_to_body: HashMap<EntityId, RigidBodyHandle>,
    /// Mapping from rapier rigid body handle to ECS entity
    body_to_entity: HashMap<RigidBodyHandle, EntityId>,
    /// Mapping from ECS entity to rapier collider handle
    entity_to_collider: HashMap<EntityId, ColliderHandle>,
    /// Mapping from rapier collider handle to ECS entity
    collider_to_entity: HashMap<ColliderHandle, EntityId>,
    /// Collision events accumulated since the last clear_collision_events()
    collision_events: Vec<CollisionData>,
    /// Active collision pairs from the previous step (for detecting start/stop)
    previous_collisions: HashSet<CollisionPair>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new(PhysicsConfig::default())
    }
}

impl PhysicsWorld {
    /// Create a new physics world
    pub fn new(mut config: PhysicsConfig) -> Self {
        // Re-validate at entry: the config struct has public fields, so an
        // invalid scale can arrive without going through `with_scale`.
        config.pixels_per_meter = sanitize_pixels_per_meter(config.pixels_per_meter);

        let integration_parameters = IntegrationParameters {
            // Rapier requires at least one solver iteration; floor at 1
            // instead of silently substituting an unrelated value.
            num_solver_iterations: NonZeroUsize::new(config.solver_iterations.max(1))
                .unwrap_or(NonZeroUsize::MIN),
            num_additional_friction_iterations: config.friction_iterations,
            ..IntegrationParameters::default()
        };

        Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            integration_parameters,
            config,
            entity_to_body: HashMap::new(),
            body_to_entity: HashMap::new(),
            entity_to_collider: HashMap::new(),
            collider_to_entity: HashMap::new(),
            collision_events: Vec::new(),
            previous_collisions: HashSet::new(),
        }
    }

    /// Clear all physics bodies, colliders, and entity mappings.
    ///
    /// Preserves configuration (gravity, solver iterations, scale) but resets
    /// all simulation state. Used when the editor restores a world snapshot
    /// and physics needs to re-sync from ECS on the next update.
    pub fn clear(&mut self) {
        self.rigid_body_set = RigidBodySet::new();
        self.collider_set = ColliderSet::new();
        self.island_manager = IslandManager::new();
        self.broad_phase = DefaultBroadPhase::new();
        self.narrow_phase = NarrowPhase::new();
        self.impulse_joint_set = ImpulseJointSet::new();
        self.multibody_joint_set = MultibodyJointSet::new();
        self.ccd_solver = CCDSolver::new();
        self.entity_to_body.clear();
        self.body_to_entity.clear();
        self.entity_to_collider.clear();
        self.collider_to_entity.clear();
        self.collision_events.clear();
        self.previous_collisions.clear();
    }

    /// Get the physics configuration
    pub fn config(&self) -> &PhysicsConfig {
        &self.config
    }

    /// Get gravity
    pub(crate) fn gravity(&self) -> Vec2 {
        self.config.gravity
    }

    /// Convert pixels to meters (vector)
    pub(crate) fn pixels_to_meters(&self, pixels: Vec2) -> Vec2 {
        pixels / self.config.pixels_per_meter
    }

    /// Convert a scalar from pixels to meters
    pub(crate) fn pixels_to_meters_scalar(&self, pixels: f32) -> f32 {
        pixels / self.config.pixels_per_meter
    }

    /// Convert meters to pixels (vector)
    pub(crate) fn meters_to_pixels(&self, meters: Vec2) -> Vec2 {
        meters * self.config.pixels_per_meter
    }

    /// Convert a scalar from meters to pixels
    pub(crate) fn meters_to_pixels_scalar(&self, meters: f32) -> f32 {
        meters * self.config.pixels_per_meter
    }
}
