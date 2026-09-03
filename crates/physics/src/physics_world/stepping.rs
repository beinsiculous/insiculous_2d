//! Simulation stepping and collision event extraction.

use glam::Vec2;
use rapier2d::prelude::*;

use ecs::EntityId;

use crate::components::{CollisionData, CollisionEvent, ContactPoint};

use super::PhysicsWorld;

/// A canonical collision pair (entity IDs always in consistent order for comparison)
/// This ensures (A, B) and (B, A) are treated as the same collision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct CollisionPair(EntityId, EntityId);

impl CollisionPair {
    /// Create a new collision pair with canonical ordering (smaller ID first)
    pub(super) fn new(a: EntityId, b: EntityId) -> Self {
        // Deterministic ordering via lexicographic (value, generation)
        // comparison. Tuples avoid the bit-packing overflow that occurred
        // when entity values exceeded 2^32.
        let a_key = (a.value(), a.generation());
        let b_key = (b.value(), b.generation());
        if a_key <= b_key {
            Self(a, b)
        } else {
            Self(b, a)
        }
    }

    /// Get the entities in canonical order
    pub(super) fn entities(&self) -> (EntityId, EntityId) {
        (self.0, self.1)
    }
}

fn push_collision(
    events: &mut Vec<CollisionData>,
    entity_a: EntityId,
    entity_b: EntityId,
    started: bool,
    stopped: bool,
    contacts: Vec<ContactPoint>,
) {
    events.push(CollisionData {
        event: CollisionEvent {
            entity_a,
            entity_b,
            started,
            stopped,
        },
        contacts,
    });
}

impl PhysicsWorld {
    /// Step the physics simulation.
    ///
    /// Collision events produced by this step are APPENDED to the event
    /// buffer — they are not cleared here. This lets a fixed-timestep driver
    /// run multiple sub-steps per frame without losing events from earlier
    /// sub-steps. Callers are responsible for calling
    /// [`clear_collision_events`](Self::clear_collision_events) once per
    /// frame, before the first step.
    pub fn step(&mut self, delta_time: f32) {
        self.integration_parameters.dt = delta_time;

        let gravity_meters = self.pixels_to_meters(self.config.gravity);
        let gravity = vector![gravity_meters.x, gravity_meters.y];

        // Step physics
        self.physics_pipeline.step(
            &gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            None,
            &(),
            &(),
        );

        // Build current collision set and process collision events
        self.current_collisions.clear();

        // Process collision events from narrow phase
        for contact_pair in self.narrow_phase.contact_pairs() {
            let collider1 = contact_pair.collider1;
            let collider2 = contact_pair.collider2;

            if let (Some(&entity_a), Some(&entity_b)) = (
                self.collider_to_entity.get(&collider1),
                self.collider_to_entity.get(&collider2),
            ) {
                let has_contact = contact_pair.has_any_active_contact;

                if has_contact {
                    let pair = CollisionPair::new(entity_a, entity_b);
                    self.current_collisions.insert(pair);

                    // Check if this is a new collision (started)
                    let started = !self.previous_collisions.contains(&pair);
                    let contacts = self.get_contact_points_from_pair(contact_pair);

                    push_collision(&mut self.collision_events, entity_a, entity_b, started, false, contacts);
                }
            }
        }

        // Process sensor intersection events from narrow phase.
        // Sensors don't generate contact pairs — they generate intersection pairs instead.
        for (collider1, collider2, intersecting) in self.narrow_phase.intersection_pairs() {
            if let (Some(&entity_a), Some(&entity_b)) = (
                self.collider_to_entity.get(&collider1),
                self.collider_to_entity.get(&collider2),
            ) {
                let pair = CollisionPair::new(entity_a, entity_b);

                if intersecting {
                    self.current_collisions.insert(pair);

                    let started = !self.previous_collisions.contains(&pair);
                    push_collision(&mut self.collision_events, entity_a, entity_b, started, false, Vec::new());
                }
            }
        }

        // Find collisions that ended (were in previous but not in current)
        for pair in &self.previous_collisions {
            if !self.current_collisions.contains(pair) {
                let (entity_a, entity_b) = pair.entities();
                push_collision(&mut self.collision_events, entity_a, entity_b, false, true, Vec::new());
            }
        }

        // Update previous collisions for next step, reusing the allocation
        std::mem::swap(&mut self.previous_collisions, &mut self.current_collisions);
    }

    /// Get contact points from a contact pair, in world space (pixels).
    ///
    /// Rapier reports manifold points/normals in collider1's local frame, so
    /// they are transformed through collider1's world isometry before the
    /// meters-to-pixels conversion.
    fn get_contact_points_from_pair(&self, contact_pair: &ContactPair) -> Vec<ContactPoint> {
        let mut contacts = Vec::new();

        let Some(collider1) = self.collider_set.get(contact_pair.collider1) else {
            return contacts;
        };
        let pos1 = collider1.position();

        for manifold in &contact_pair.manifolds {
            let world_normal = pos1.rotation * manifold.local_n1;
            for point in &manifold.points {
                let world_point = pos1 * point.local_p1;
                let point_meters = Vec2::new(world_point.x, world_point.y);
                contacts.push(ContactPoint {
                    point: self.meters_to_pixels(point_meters),
                    normal: Vec2::new(world_normal.x, world_normal.y),
                    depth: self.meters_to_pixels_scalar(point.dist),
                });
            }
        }

        contacts
    }

    /// Get collision events accumulated since the last
    /// [`clear_collision_events`](Self::clear_collision_events).
    ///
    /// Borrow-based accessor used by the physics driver itself (event-bus
    /// emission). Game code should prefer
    /// `PhysicsSystem::take_collision_events`, which hands out an owned `Vec`
    /// and empties the buffer — no borrow held, no snapshot clone needed.
    pub fn collision_events(&self) -> &[CollisionData] {
        &self.collision_events
    }

    /// Take ownership of the collision event buffer, leaving it empty.
    ///
    /// The drain-style counterpart to [`collision_events`](Self::collision_events):
    /// consuming the buffer means no borrow of the physics world is held while
    /// reacting to events, so callers can freely mutate physics/world state.
    pub fn take_collision_events(&mut self) -> Vec<CollisionData> {
        std::mem::take(&mut self.collision_events)
    }

    /// Clear the collision event buffer.
    ///
    /// Call once per frame before stepping. [`step`](Self::step) appends to
    /// the buffer so multiple sub-steps in one frame all contribute events.
    pub fn clear_collision_events(&mut self) {
        self.collision_events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collision_pair_is_the_same_pair_in_either_order() {
        // started/ongoing dedup looks the pair up across steps; a pair that
        // depended on rapier's collider order would re-start every frame.
        let a = EntityId::new();
        let b = EntityId::new();
        assert_eq!(CollisionPair::new(a, b), CollisionPair::new(b, a));
        assert_eq!(CollisionPair::new(a, b).entities(), CollisionPair::new(b, a).entities());
    }
}
