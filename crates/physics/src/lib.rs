//! 2D Physics system for the insiculous_2d game engine
//!
//! This crate provides physics simulation using rapier2d, integrated with the ECS.
//!
//! # Features
//!
//! - Rigid body dynamics (dynamic, static, kinematic bodies)
//! - Collision detection and response
//! - Multiple collider shapes (box, circle, capsule)
//! - Collision events and callbacks
//! - Raycasting
//! - Fixed timestep simulation
//!
//! # Usage
//!
//! ```rust
//! use physics::{PhysicsSystem, RigidBody, Collider};
//! use ecs::sprite_components::Transform2D;
//! use ecs::{System, World};
//! use glam::Vec2;
//!
//! // Create the ECS world and the physics system
//! let mut world = World::new();
//! let mut physics_system = PhysicsSystem::new();
//!
//! // Create an entity with physics components
//! let entity = world.create_entity();
//! world.add_component(&entity, Transform2D::new(Vec2::new(0.0, 100.0))).unwrap();
//! world.add_component(&entity, RigidBody::new_dynamic()).unwrap();
//! world.add_component(&entity, Collider::box_collider(32.0, 32.0)).unwrap();
//!
//! // Step the simulation each frame (the game loop normally does this)
//! physics_system.update(&mut world, 1.0 / 60.0);
//!
//! // Gravity pulled the dynamic body down
//! let transform = world.get::<Transform2D>(entity).unwrap();
//! assert!(transform.position.y < 100.0);
//! ```

pub mod components;
pub mod presets;
pub mod register;
pub mod physics_system;
pub mod physics_world;

pub mod prelude;

#[cfg(test)]
pub(crate) mod test_support;

// Re-export main types
pub use components::{
    Collider, ColliderShape, CollisionData, CollisionEvent, ContactPoint, RigidBody,
    RigidBodyType,
};
pub use physics_system::PhysicsSystem;
pub use physics_world::{PhysicsConfig, PhysicsWorld};
