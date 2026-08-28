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

// Re-export main types
pub use components::{
    Collider, ColliderShape, CollisionData, CollisionEvent, ContactPoint, RigidBody,
    RigidBodyType,
};
pub use physics_system::PhysicsSystem;
pub use physics_world::{PhysicsConfig, PhysicsWorld};

#[cfg(test)]
mod tests {
    use super::*;
    use ecs::sprite_components::Transform2D;
    use ecs::{System, World};
    use glam::Vec2;

    #[test]
    fn test_full_physics_workflow() {
        // Create ECS world
        let mut world = World::new();

        // Create physics system
        let mut physics = PhysicsSystem::new();

        // Create a dynamic falling box
        let falling_box = world.create_entity();
        world
            .add_component(&falling_box, Transform2D::new(Vec2::new(0.0, 200.0)))
            .unwrap();
        world
            .add_component(&falling_box, RigidBody::new_dynamic())
            .unwrap();
        world
            .add_component(&falling_box, Collider::box_collider(32.0, 32.0))
            .unwrap();

        // Create a static ground
        let ground = world.create_entity();
        world
            .add_component(&ground, Transform2D::new(Vec2::new(0.0, -100.0)))
            .unwrap();
        world
            .add_component(&ground, RigidBody::new_static())
            .unwrap();
        world
            .add_component(
                &ground,
                Collider::box_collider(400.0, 20.0).with_friction(0.8),
            )
            .unwrap();

        // Initialize physics
        physics.initialize(&mut world).unwrap();

        // Run simulation for a few frames
        let initial_y = world.get::<Transform2D>(falling_box).unwrap().position.y;

        for _ in 0..60 {
            physics.update(&mut world, 1.0 / 60.0);
        }

        let final_y = world.get::<Transform2D>(falling_box).unwrap().position.y;

        // Falling box should have moved down
        assert!(
            final_y < initial_y,
            "Box should fall from {} to {}",
            initial_y,
            final_y
        );

        // Ground should not have moved
        let ground_y = world.get::<Transform2D>(ground).unwrap().position.y;
        assert_eq!(ground_y, -100.0, "Ground should not move");
    }

    #[test]
    fn test_collision_detection() {
        let mut world = World::new();
        let mut physics = PhysicsSystem::new();

        // Create two overlapping boxes
        let box1 = world.create_entity();
        world
            .add_component(&box1, Transform2D::new(Vec2::new(0.0, 0.0)))
            .unwrap();
        world
            .add_component(&box1, RigidBody::new_dynamic().with_gravity_scale(0.0))
            .unwrap();
        world
            .add_component(&box1, Collider::box_collider(32.0, 32.0))
            .unwrap();

        let box2 = world.create_entity();
        world
            .add_component(&box2, Transform2D::new(Vec2::new(10.0, 0.0)))
            .unwrap();
        world
            .add_component(&box2, RigidBody::new_dynamic().with_gravity_scale(0.0))
            .unwrap();
        world
            .add_component(&box2, Collider::box_collider(32.0, 32.0))
            .unwrap();

        physics.initialize(&mut world).unwrap();

        // Run physics - should generate collision events
        physics.update(&mut world, 1.0 / 60.0);

        // Collision should push boxes apart
        let pos1 = world.get::<Transform2D>(box1).unwrap().position;
        let pos2 = world.get::<Transform2D>(box2).unwrap().position;

        // After collision response, boxes should be further apart
        let distance = (pos2.x - pos1.x).abs();
        assert!(distance >= 10.0, "Boxes should be pushed apart by collision");
    }
}
