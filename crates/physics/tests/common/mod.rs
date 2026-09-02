//! Shared setup for the physics crate's integration tests.

use ecs::sprite_components::Transform2D;
use ecs::{EntityId, World};
use glam::Vec2;
use physics::{Collider, RigidBody};

/// Spawn an entity carrying the three physics components at `position`.
pub fn spawn_body(world: &mut World, position: Vec2, body: RigidBody, collider: Collider) -> EntityId {
    world
        .spawn()
        .with(Transform2D::new(position))
        .with(body)
        .with(collider)
        .id()
}
