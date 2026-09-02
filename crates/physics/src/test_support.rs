//! Shared fixtures for the crate's unit tests.
//!
//! The integration tests under `tests/` cannot see this module; they carry
//! their own copy of `spawn_body` in `tests/common/mod.rs`.

use glam::Vec2;

use ecs::sprite_components::Transform2D;
use ecs::{EntityId, World};

use crate::components::{Collider, RigidBody};
use crate::physics_system::PhysicsSystem;
use crate::physics_world::PhysicsConfig;

/// Spawn an entity carrying the three physics components at `position`.
pub(crate) fn spawn_body(
    world: &mut World,
    position: Vec2,
    body: RigidBody,
    collider: Collider,
) -> EntityId {
    world
        .spawn()
        .with(Transform2D::new(position))
        .with(body)
        .with(collider)
        .id()
}

/// A physics system with gravity switched off, so bodies only move when a
/// test moves them.
pub(crate) fn no_gravity_system() -> PhysicsSystem {
    PhysicsSystem::with_config(PhysicsConfig::new(Vec2::ZERO))
}
