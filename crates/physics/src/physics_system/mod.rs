//! Physics system for ECS integration
//!
//! This module provides a system that synchronizes ECS components with the physics world.
//!
//! Split by responsibility:
//! - `mod.rs` — struct, builders, pass-through API
//! - `sync.rs` — ECS↔rapier synchronization and orphan garbage collection
//! - `update.rs` — the `System` trait implementation (fixed-timestep driver)
//!
//! # Collision Event Delivery
//!
//! Events reach consumers through [`take_collision_events`](PhysicsSystem::take_collision_events) —
//! for game code: an owned `Vec` drained once per frame, shared by every
//! consumer (gameplay, pickups). Taking ownership means no borrow of the
//! physics system is held while reacting, so handlers can freely call
//! `set_velocity` / `destroy_entity` / etc.
//!
//! # API Design: Pass-Through Methods
//!
//! [`PhysicsSystem`] provides several methods that delegate directly to [`PhysicsWorld`]:
//! - `set_velocity()` — the single, universal "launch / move this body at
//!   velocity V" API. Safe on bodies spawned this frame (defers until synced).
//! - `take_collision_events()`
//! - `destroy_entity()`
//! - `reset_body()`
//!
//! These pass-through methods exist intentionally for **API ergonomics**:
//!
//! ```
//! # use physics::PhysicsSystem;
//! # use ecs::World;
//! # use glam::Vec2;
//! # let mut world = World::new();
//! # let entity = world.create_entity();
//! # let mut physics_system = PhysicsSystem::new();
//! // With pass-through (cleaner):
//! physics_system.set_velocity(entity, Vec2::new(0.0, 100.0), 0.0);
//!
//! // Without pass-through:
//! physics_system.physics_world_mut().set_velocity(entity, Vec2::new(0.0, 100.0), 0.0);
//! ```
//!
//! Note: the legacy `apply_impulse` pass-through was removed — every callsite
//! in the workspace was semantically "start this body at velocity V" rather
//! than a mass-aware momentum delta, and having two functions for the same
//! intent was a footgun (impulse silently no-ops on same-frame spawns).
//! `PhysicsWorld::apply_impulse` remains for the rare case that genuinely
//! needs mass-aware impulse semantics on a live body.
//!
//! Users who need advanced physics operations can still access the underlying
//! [`PhysicsWorld`] via [`physics_world()`](PhysicsSystem::physics_world) and
//! [`physics_world_mut()`](PhysicsSystem::physics_world_mut).

mod sync;
mod update;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use glam::Vec2;

use ecs::{EntityId, World};

use crate::components::{Collider, CollisionData};
use crate::physics_world::{PhysicsConfig, PhysicsWorld};

/// A body operation deferred because the entity wasn't synced into rapier
/// yet (same-frame spawn). Drained in call order during the next `update()`,
/// so the documented "reset then launch" pattern applies the reset first and
/// the launch velocity lands intact.
#[derive(Debug, Clone, Copy)]
enum DeferredBodyOp {
    /// Move the body to a position and zero its velocity (`reset_body`).
    Reset { position: Vec2 },
    /// Set linear + angular velocity (`set_velocity`).
    SetVelocity { linear: Vec2, angular: f32 },
}

/// The state last pushed into (or read back from) rapier for a live entity.
///
/// The GPP-09 dirty-flag baseline: during sync, the current ECS components
/// are value-compared against this; a mismatch means game/editor code edited
/// them externally, and the edit is pushed into rapier (teleport the body /
/// rebuild the collider). The physics writeback updates the baseline too, so
/// rapier-driven motion is never mistaken for an external edit.
struct PushedState {
    /// Body position as last pushed/read back (pixels).
    position: Vec2,
    /// Body rotation as last pushed/read back (radians).
    rotation: f32,
    /// Collider component as last built into rapier (None = no collider).
    collider: Option<Collider>,
}

/// Maximum number of fixed-timestep catch-up steps in a single update.
///
/// Bounds the work done after a stall regardless of how small the
/// configured `fixed_timestep` is; leftover accumulated time is dropped
/// rather than simulated, trading a small slowdown for stability.
const MAX_STEPS_PER_UPDATE: u32 = 8;

/// Physics system that steps the simulation and syncs transforms
pub struct PhysicsSystem {
    /// The physics world
    physics_world: PhysicsWorld,
    /// Accumulated time for fixed timestep
    time_accumulator: f32,
    /// Fixed timestep for physics updates (1/60 second by default)
    fixed_timestep: f32,
    /// Maximum delta time to prevent spiral of death
    max_delta_time: f32,
    /// Body ops deferred for entities not yet synced to Rapier
    /// (applied in call order during the next `update()`)
    pending_ops: Vec<(EntityId, DeferredBodyOp)>,
    /// Per-entity baseline of the state last pushed into rapier — external
    /// ECS-side edits are detected by value comparison against this.
    baselines: HashMap<EntityId, PushedState>,
    /// How many external edits were pushed into rapier during the last update.
    pushed_edits_last_update: usize,
}

impl PhysicsSystem {
    /// Create a new physics system with default configuration
    pub fn new() -> Self {
        Self::with_config(PhysicsConfig::default())
    }

    /// Create a new physics system with custom configuration
    pub fn with_config(config: PhysicsConfig) -> Self {
        Self {
            physics_world: PhysicsWorld::new(config),
            time_accumulator: 0.0,
            fixed_timestep: 1.0 / 60.0,
            max_delta_time: 0.1,
            pending_ops: Vec::new(),
            baselines: HashMap::new(),
            pushed_edits_last_update: 0,
        }
    }

    /// Clear all physics state, forcing re-sync from ECS on next update.
    ///
    /// Preserves configuration (gravity, scale) but resets all rapier
    /// bodies, colliders, and entity mappings. Call this when the editor
    /// restores a world snapshot to ensure physics re-initializes from the
    /// restored ECS component values.
    pub fn clear(&mut self) {
        self.physics_world.clear();
        self.pending_ops.clear();
        self.baselines.clear();
        self.time_accumulator = 0.0;
    }

    /// How many external ECS-side edits (Transform2D teleports, collider
    /// rebuilds, collider removals) were pushed into rapier during the most
    /// recent `update()`. A frame where only rapier moved things reports 0.
    pub fn external_edits_pushed_last_update(&self) -> usize {
        self.pushed_edits_last_update
    }

    /// Remove an entity from both the physics world and the ECS world.
    ///
    /// This is the recommended way to destroy physics entities — it ensures
    /// both systems stay in sync and clears any pending deferred velocities.
    pub fn destroy_entity(&mut self, world: &mut World, entity: EntityId) {
        self.physics_world.remove_entity(entity);
        world.remove_entity(&entity).ok();
        self.pending_ops.retain(|(e, _)| *e != entity);
        self.baselines.remove(&entity);
    }

    /// Get a reference to the physics world
    pub fn physics_world(&self) -> &PhysicsWorld {
        &self.physics_world
    }

    /// Get a mutable reference to the physics world
    pub fn physics_world_mut(&mut self) -> &mut PhysicsWorld {
        &mut self.physics_world
    }

    /// Take the collision events from the last update's physics steps,
    /// leaving the buffer empty.
    ///
    /// Call once per frame after `update()` and share the returned `Vec`
    /// among all consumers (gameplay reactions, pickups, ...). Empty if the
    /// last update ran zero steps; contains the events of every sub-step if
    /// the last update ran several. Because the events are owned, handlers
    /// can freely mutate physics/world state while iterating — no `.to_vec()`
    /// snapshot dance.
    pub fn take_collision_events(&mut self) -> Vec<CollisionData> {
        self.physics_world.take_collision_events()
    }

    /// Set the velocity of a rigid body — the universal "launch / move this
    /// body at velocity V" API.
    ///
    /// Safe to call on entities spawned this same frame: if the body hasn't
    /// been synced into Rapier yet, the velocity is buffered and applied
    /// automatically during the next `update()`. This is the one function
    /// games should reach for when starting bodies moving.
    pub fn set_velocity(&mut self, entity: EntityId, linear: Vec2, angular: f32) {
        if self.physics_world.has_rigid_body(entity) {
            self.physics_world.set_velocity(entity, linear, angular);
        } else {
            self.pending_ops
                .push((entity, DeferredBodyOp::SetVelocity { linear, angular }));
        }
    }

    /// Get the velocity of a rigid body
    pub fn get_body_velocity(&self, entity: EntityId) -> Option<(Vec2, f32)> {
        self.physics_world.get_body_velocity(entity)
    }

    /// Set the next kinematic position (for kinematic bodies)
    pub fn set_kinematic_target(&mut self, entity: EntityId, position: Vec2, rotation: f32) {
        self.physics_world.set_kinematic_target(entity, position, rotation);
    }

    /// Reset a body's position and zero its velocity.
    ///
    /// Safe to call on entities spawned this same frame: if the body hasn't
    /// been synced into Rapier yet, the reset is buffered and applied during
    /// the next `update()` in call order — so `reset_body` followed by
    /// `set_velocity` lands with the launch velocity intact.
    pub fn reset_body(&mut self, entity: EntityId, position: Vec2) {
        if self.physics_world.has_rigid_body(entity) {
            self.physics_world.set_body_transform(entity, position, 0.0);
            self.physics_world.set_velocity(entity, Vec2::ZERO, 0.0);
        } else {
            self.pending_ops.push((entity, DeferredBodyOp::Reset { position }));
        }
    }
}

impl Default for PhysicsSystem {
    fn default() -> Self {
        Self::new()
    }
}
