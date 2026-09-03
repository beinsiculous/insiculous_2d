//! Body and collider management: add/remove and per-body accessors.

use glam::Vec2;
use rapier2d::prelude::*;

use ecs::EntityId;

use crate::components::{Collider, ColliderShape, RigidBody, RigidBodyType};

use super::PhysicsWorld;

impl PhysicsWorld {
    /// Add a rigid body for an entity
    pub fn add_rigid_body(&mut self, entity: EntityId, body: &mut RigidBody, position: Vec2, rotation: f32) {
        // Remove existing body if any
        self.remove_rigid_body(entity);

        // Convert position from pixels to meters
        let pos = self.pixels_to_meters(position);
        let vel = self.pixels_to_meters(body.velocity);

        // Per-body-type builder setup; shared translation/rotation applied once below.
        let builder = match body.body_type {
            RigidBodyType::Dynamic => {
                let mut builder = RigidBodyBuilder::dynamic()
                    .linvel(vector![vel.x, vel.y])
                    .angvel(body.angular_velocity)
                    .gravity_scale(body.gravity_scale)
                    .linear_damping(body.linear_damping)
                    .angular_damping(body.angular_damping)
                    .ccd_enabled(body.ccd_enabled);
                if !body.can_rotate {
                    builder = builder.lock_rotations();
                }
                builder
            }
            RigidBodyType::Static => RigidBodyBuilder::fixed(),
            // Note: no linvel/angvel here — rapier ignores velocities on
            // position-based kinematic bodies (move them via
            // `set_kinematic_target` instead).
            RigidBodyType::Kinematic => {
                RigidBodyBuilder::kinematic_position_based().ccd_enabled(body.ccd_enabled)
            }
        };
        let rigid_body = builder
            .translation(vector![pos.x, pos.y])
            .rotation(rotation)
            .build();

        let handle = self.rigid_body_set.insert(rigid_body);
        body.handle = Some(handle);

        self.entity_to_body.insert(entity, handle);
        self.body_to_entity.insert(handle, entity);

        log::trace!("Added rigid body for entity {:?}", entity);
    }

    /// Add a collider for an entity
    pub fn add_collider(
        &mut self,
        entity: EntityId,
        collider: &mut Collider,
        rigid_body: Option<&RigidBody>,
    ) {
        // Remove existing collider if any
        self.remove_collider(entity);

        // Create rapier shape (converting from pixels to meters)
        let shape: SharedShape = match &collider.shape {
            ColliderShape::Box { half_extents } => {
                let he = self.pixels_to_meters(*half_extents);
                SharedShape::cuboid(he.x, he.y)
            }
            ColliderShape::Circle { radius } => {
                SharedShape::ball(self.pixels_to_meters_scalar(*radius))
            }
            ColliderShape::CapsuleY { half_height, radius } => SharedShape::capsule_y(
                self.pixels_to_meters_scalar(*half_height),
                self.pixels_to_meters_scalar(*radius),
            ),
            ColliderShape::CapsuleX { half_height, radius } => SharedShape::capsule_x(
                self.pixels_to_meters_scalar(*half_height),
                self.pixels_to_meters_scalar(*radius),
            ),
        };

        // Build collider
        let offset = self.pixels_to_meters(collider.offset);
        let mut builder = ColliderBuilder::new(shape)
            .translation(vector![offset.x, offset.y])
            .friction(collider.friction)
            .restitution(collider.restitution)
            .sensor(collider.is_sensor)
            .active_events(ActiveEvents::COLLISION_EVENTS);

        // Set collision groups using InteractionGroups
        let groups = InteractionGroups::new(
            Group::from_bits_truncate(collider.collision_groups),
            Group::from_bits_truncate(collider.collision_filter),
        );
        builder = builder.collision_groups(groups);

        let rapier_collider = builder.build();

        // Insert collider, attached to rigid body if available
        let handle = if let Some(body) = rigid_body {
            if let Some(body_handle) = body.handle {
                self.collider_set.insert_with_parent(
                    rapier_collider,
                    body_handle,
                    &mut self.rigid_body_set,
                )
            } else {
                self.collider_set.insert(rapier_collider)
            }
        } else {
            self.collider_set.insert(rapier_collider)
        };

        collider.handle = Some(handle);
        self.entity_to_collider.insert(entity, handle);
        self.collider_to_entity.insert(handle, entity);

        log::trace!("Added collider for entity {:?}", entity);
    }

    /// Remove a rigid body for an entity
    pub fn remove_rigid_body(&mut self, entity: EntityId) {
        if let Some(handle) = self.entity_to_body.remove(&entity) {
            self.body_to_entity.remove(&handle);
            self.rigid_body_set.remove(
                handle,
                &mut self.island_manager,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                true, // remove colliders attached to this body
            );
            log::trace!("Removed rigid body for entity {:?}", entity);
        }
    }

    /// Remove a collider for an entity
    pub fn remove_collider(&mut self, entity: EntityId) {
        if let Some(handle) = self.entity_to_collider.remove(&entity) {
            self.collider_to_entity.remove(&handle);
            self.collider_set.remove(
                handle,
                &mut self.island_manager,
                &mut self.rigid_body_set,
                true,
            );
            log::trace!("Removed collider for entity {:?}", entity);
        }
    }

    /// Remove all physics objects for an entity
    pub fn remove_entity(&mut self, entity: EntityId) {
        self.remove_rigid_body(entity);
        self.remove_collider(entity);
    }

    /// All entities that currently have a rigid body or collider here.
    ///
    /// Used by `PhysicsSystem` to garbage-collect physics state for entities
    /// that were removed from the ECS without going through `destroy_entity`.
    pub(crate) fn tracked_entities(&self) -> Vec<EntityId> {
        let mut entities: Vec<EntityId> = self.entity_to_body.keys().copied().collect();
        for entity in self.entity_to_collider.keys() {
            if !self.entity_to_body.contains_key(entity) {
                entities.push(*entity);
            }
        }
        entities
    }

    /// Look up the rapier body for an entity (entity map → body set).
    fn body(&self, entity: EntityId) -> Option<&rapier2d::dynamics::RigidBody> {
        let handle = self.entity_to_body.get(&entity)?;
        self.rigid_body_set.get(*handle)
    }

    /// Look up the rapier body for an entity, mutably (entity map → body set).
    fn body_mut(&mut self, entity: EntityId) -> Option<&mut rapier2d::dynamics::RigidBody> {
        let handle = *self.entity_to_body.get(&entity)?;
        self.rigid_body_set.get_mut(handle)
    }

    /// Get the position and rotation of a rigid body
    pub fn get_body_transform(&self, entity: EntityId) -> Option<(Vec2, f32)> {
        let body = self.body(entity)?;
        let translation = body.translation();
        let rotation = body.rotation().angle();
        let pos_meters = Vec2::new(translation.x, translation.y);

        Some((self.meters_to_pixels(pos_meters), rotation))
    }

    /// Get the velocity of a rigid body
    pub fn get_body_velocity(&self, entity: EntityId) -> Option<(Vec2, f32)> {
        let body = self.body(entity)?;
        let linvel = body.linvel();
        let angvel = body.angvel();
        let vel_meters = Vec2::new(linvel.x, linvel.y);

        Some((self.meters_to_pixels(vel_meters), angvel))
    }

    /// Set the position and rotation of a rigid body
    pub(crate) fn set_body_transform(&mut self, entity: EntityId, position: Vec2, rotation: f32) {
        let pos = self.pixels_to_meters(position);

        if let Some(body) = self.body_mut(entity) {
            body.set_translation(vector![pos.x, pos.y], true);
            body.set_rotation(nalgebra::UnitComplex::new(rotation), true);
        }
    }

    /// Set the next kinematic position (for kinematic bodies)
    ///
    /// This is the proper way to move kinematic_position_based bodies.
    /// The body will move to this position during the next physics step,
    /// properly interacting with other bodies along the way.
    pub fn set_kinematic_target(&mut self, entity: EntityId, position: Vec2, rotation: f32) {
        let pos = self.pixels_to_meters(position);

        if let Some(body) = self.body_mut(entity) {
            body.set_next_kinematic_translation(vector![pos.x, pos.y]);
            body.set_next_kinematic_rotation(nalgebra::UnitComplex::new(rotation));
        }
    }

    /// Set the velocity of a rigid body.
    ///
    /// Note: if the entity hasn't been synced to Rapier yet (e.g., just
    /// spawned this frame), this will silently no-op. Callers that need
    /// deferred-safe behavior should use [`PhysicsSystem::set_velocity`]
    /// instead, which buffers until the body is live.
    ///
    /// [`PhysicsSystem::set_velocity`]: crate::PhysicsSystem::set_velocity
    pub fn set_velocity(&mut self, entity: EntityId, linear: Vec2, angular: f32) {
        let vel = self.pixels_to_meters(linear);

        if let Some(body) = self.body_mut(entity) {
            body.set_linvel(vector![vel.x, vel.y], true);
            body.set_angvel(angular, true);
        }
    }

    /// Apply an impulse to a rigid body
    pub fn apply_impulse(&mut self, entity: EntityId, impulse: Vec2) {
        let imp = self.pixels_to_meters(impulse);

        if let Some(body) = self.body_mut(entity) {
            body.apply_impulse(vector![imp.x, imp.y], true);
        }
    }

    /// Check if an entity has a rigid body
    pub fn has_rigid_body(&self, entity: EntityId) -> bool {
        self.entity_to_body.contains_key(&entity)
    }

    /// Check if an entity has a collider
    pub fn has_collider(&self, entity: EntityId) -> bool {
        self.entity_to_collider.contains_key(&entity)
    }

    /// Get the number of rigid bodies
    #[cfg(test)]
    pub fn rigid_body_count(&self) -> usize {
        self.rigid_body_set.len()
    }

    /// Get the number of colliders
    #[cfg(test)]
    pub fn collider_count(&self) -> usize {
        self.collider_set.len()
    }
}
