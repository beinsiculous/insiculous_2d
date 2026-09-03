//! ECS ↔ rapier synchronization and orphaned-state garbage collection.
//!
//! Sync direction matters: rapier is authoritative for simulated motion —
//! `sync_physics_to_ecs` writes body positions/velocities back into the ECS
//! components every frame. In the other direction, `sync_entity_to_physics`
//! adds missing bodies/colliders AND pushes **external ECS-side edits** into
//! rapier: components are value-compared against the last-pushed
//! baseline, so editing `Transform2D` teleports the live body and editing
//! `Collider` rebuilds its rapier collider. The writeback refreshes the
//! baseline, so rapier-driven motion is never mistaken for an edit.

use std::collections::HashSet;

use glam::Vec2;

use ecs::sprite_components::Transform2D;
use ecs::{EntityId, World};

use crate::components::{Collider, RigidBody, RigidBodyType};

use super::{PhysicsSystem, PushedState};

impl PhysicsSystem {
    /// Garbage-collect physics state for entities that were removed from
    /// the ECS directly (`world.remove_entity`) without going through
    /// `destroy_entity` — otherwise their rapier bodies keep simulating
    /// and colliding invisibly forever. Also prunes pending deferred
    /// velocities/resets for those entities.
    pub(super) fn prune_removed_entities(&mut self, alive: &HashSet<EntityId>) {
        for entity in self.physics_world.tracked_entities() {
            if !alive.contains(&entity) {
                log::debug!(
                    "Removing orphaned physics state for despawned entity {:?}",
                    entity
                );
                self.physics_world.remove_entity(entity);
            }
        }
        self.pending_ops.retain(|(e, _)| alive.contains(e));
        self.baselines.retain(|e, _| alive.contains(e));
    }

    /// Sync a single entity from ECS to physics world.
    ///
    /// Adds missing bodies/colliders, and detects **external ECS-side edits**
    /// on live entities by value-comparing the components against the
    /// last-pushed baseline: a changed `Transform2D` teleports the
    /// body (velocity preserved), a changed `Collider` rebuilds the rapier
    /// collider, a removed `Collider` removes it. The physics writeback in
    /// `sync_physics_to_ecs` refreshes the baseline, so rapier-driven motion
    /// is never mistaken for an edit.
    pub(super) fn sync_entity_to_physics(&mut self, world: &mut World, entity: EntityId) {
        let transform = world.get::<Transform2D>(entity).cloned();

        if let Some(mut rigid_body) = world.get::<RigidBody>(entity).cloned() {
            let (position, rotation) = transform
                .as_ref()
                .map(|t| (t.position, t.rotation))
                .unwrap_or((Vec2::ZERO, 0.0));

            if !self.physics_world.has_rigid_body(entity) {
                // Add rigid body to physics world
                self.physics_world.add_rigid_body(entity, &mut rigid_body, position, rotation);
                // Update the component with the handle
                if let Some(body) = world.get_mut::<RigidBody>(entity) {
                    body.handle = rigid_body.handle;
                }
                // Fresh baseline for the new body (collider recorded below).
                self.baselines
                    .insert(entity, PushedState { position, rotation, collider: None });
            } else {
                match self.baselines.get_mut(&entity) {
                    Some(baseline) => {
                        // External Transform2D edit → teleport the live body.
                        if transform.is_some()
                            && (baseline.position != position || baseline.rotation != rotation)
                        {
                            self.physics_world.set_body_transform(entity, position, rotation);
                            baseline.position = position;
                            baseline.rotation = rotation;
                            self.pushed_edits_last_update += 1;
                        }
                    }
                    // Body exists but was never baselined (defensive) —
                    // adopt the current state without pushing anything.
                    None => {
                        self.baselines
                            .insert(entity, PushedState { position, rotation, collider: None });
                    }
                }
            }

            // Collider: add when missing, rebuild when edited, drop when removed.
            self.sync_collider(world, entity, true);
        } else if world.get::<Collider>(entity).is_some() {
            // Collider without rigid body (static collision geometry).
            // Note: standalone colliders are placed by their offset only, so
            // Transform2D edits don't apply to them (pre-existing behavior);
            // shape/property edits still rebuild.
            self.baselines.entry(entity).or_insert(PushedState {
                position: Vec2::ZERO,
                rotation: 0.0,
                collider: None,
            });
            self.sync_collider(world, entity, false);
        }
    }

    /// Add / rebuild / remove the rapier collider to match the ECS component.
    fn sync_collider(&mut self, world: &mut World, entity: EntityId, attach_to_body: bool) {
        if let Some(mut collider) = world.get::<Collider>(entity).cloned() {
            let needs_add = !self.physics_world.has_collider(entity);
            let edited = self
                .baselines
                .get(&entity)
                .is_some_and(|b| b.collider.as_ref() != Some(&collider));
            if needs_add || edited {
                let body_ref = if attach_to_body { world.get::<RigidBody>(entity) } else { None };
                // add_collider removes any existing collider first (rebuild).
                self.physics_world.add_collider(entity, &mut collider, body_ref);
                if let Some(coll) = world.get_mut::<Collider>(entity) {
                    coll.handle = collider.handle;
                }
                if !needs_add {
                    self.pushed_edits_last_update += 1;
                }
                if let Some(baseline) = self.baselines.get_mut(&entity) {
                    baseline.collider = Some(collider);
                }
            }
        } else if self.physics_world.has_collider(entity) {
            // Collider component removed → drop the stale rapier collider.
            self.physics_world.remove_collider(entity);
            if let Some(baseline) = self.baselines.get_mut(&entity) {
                baseline.collider = None;
            }
            self.pushed_edits_last_update += 1;
        }
    }

    /// Sync physics results back to ECS transforms (and refresh the
    /// external-edit baselines so this writeback isn't detected as an edit).
    pub(super) fn sync_physics_to_ecs(&mut self, world: &mut World) {
        let entities: Vec<EntityId> = world.entities();

        for entity in entities {
            // Get body type first to avoid borrow conflicts
            let body_type = world.get::<RigidBody>(entity).map(|b| b.body_type);

            if let Some(body_type) = body_type {
                // Sync both Dynamic and Kinematic bodies back to ECS
                // Static bodies don't move, so no need to sync them
                if body_type == RigidBodyType::Dynamic || body_type == RigidBodyType::Kinematic {
                    // Get physics transform
                    if let Some((position, rotation)) = self.physics_world.get_body_transform(entity) {
                        // Update ECS transform
                        if let Some(transform) = world.get_mut::<Transform2D>(entity) {
                            transform.position = position;
                            transform.rotation = rotation;
                        }
                        // Keep the baseline in lockstep: this write came FROM
                        // rapier, so it must not read as an external edit.
                        if let Some(baseline) = self.baselines.get_mut(&entity) {
                            baseline.position = position;
                            baseline.rotation = rotation;
                        }
                    }

                    // Update velocity in component (for dynamic bodies)
                    if body_type == RigidBodyType::Dynamic {
                        if let Some((linear_vel, angular_vel)) = self.physics_world.get_body_velocity(entity) {
                            if let Some(rigid_body) = world.get_mut::<RigidBody>(entity) {
                                rigid_body.velocity = linear_vel;
                                rigid_body.angular_velocity = angular_vel;
                            }
                        }
                    }
                }
            }
        }
    }
}
