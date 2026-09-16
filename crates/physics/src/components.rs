//! Physics components for ECS integration
//!
//! These components wrap rapier2d concepts and can be attached to ECS entities.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Body type for physics simulation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum RigidBodyType {
    /// A dynamic body affected by forces and collisions
    #[default]
    Dynamic,
    /// A static body that never moves
    Static,
    /// A kinematic body controlled directly by the user
    Kinematic,
}

impl RigidBodyType {
    /// All body types in cycle order (drives the editor's variant selector).
    pub const ALL: [RigidBodyType; 3] =
        [RigidBodyType::Dynamic, RigidBodyType::Static, RigidBodyType::Kinematic];

    /// Display label for menus and the inspector.
    pub fn label(self) -> &'static str {
        match self {
            RigidBodyType::Dynamic => "Dynamic",
            RigidBodyType::Static => "Static",
            RigidBodyType::Kinematic => "Kinematic",
        }
    }

    /// Index of this type within [`Self::ALL`].
    pub fn index(self) -> usize {
        match self {
            RigidBodyType::Dynamic => 0,
            RigidBodyType::Static => 1,
            RigidBodyType::Kinematic => 2,
        }
    }
}


/// Rigid body component for physics simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RigidBody {
    /// Type of rigid body
    pub body_type: RigidBodyType,
    /// Linear velocity in units per second
    pub velocity: Vec2,
    /// Angular velocity in radians per second
    pub angular_velocity: f32,
    /// Gravity scale (1.0 = normal gravity, 0.0 = no gravity)
    pub gravity_scale: f32,
    /// Linear damping (velocity decay per second)
    pub linear_damping: f32,
    /// Angular damping (angular velocity decay per second)
    pub angular_damping: f32,
    /// Whether this body can rotate
    pub can_rotate: bool,
    /// Enable Continuous Collision Detection (prevents tunneling through thin objects)
    pub ccd_enabled: bool,
    /// Handle to the rapier rigid body (set by PhysicsWorld)
    #[serde(skip)]
    pub(crate) handle: Option<rapier2d::dynamics::RigidBodyHandle>,
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            body_type: RigidBodyType::Dynamic,
            velocity: Vec2::ZERO,
            angular_velocity: 0.0,
            gravity_scale: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            can_rotate: true,
            ccd_enabled: false,
            handle: None,
        }
    }
}

impl RigidBody {
    /// Create a new dynamic rigid body
    pub fn new_dynamic() -> Self {
        Self::default()
    }

    /// Create a new static rigid body
    pub fn new_static() -> Self {
        Self {
            body_type: RigidBodyType::Static,
            ..Default::default()
        }
    }

    /// Create a new kinematic rigid body
    pub fn new_kinematic() -> Self {
        Self {
            body_type: RigidBodyType::Kinematic,
            ..Default::default()
        }
    }

    /// Set the body type
    pub fn with_body_type(mut self, body_type: RigidBodyType) -> Self {
        self.body_type = body_type;
        self
    }

    /// Set initial velocity
    pub fn with_velocity(mut self, velocity: Vec2) -> Self {
        self.velocity = velocity;
        self
    }

    /// Set initial angular velocity
    pub fn with_angular_velocity(mut self, angular_velocity: f32) -> Self {
        self.angular_velocity = angular_velocity;
        self
    }

    /// Set gravity scale
    pub fn with_gravity_scale(mut self, scale: f32) -> Self {
        self.gravity_scale = scale;
        self
    }

    /// Set linear damping
    pub fn with_linear_damping(mut self, damping: f32) -> Self {
        self.linear_damping = damping;
        self
    }

    /// Set angular damping
    pub fn with_angular_damping(mut self, damping: f32) -> Self {
        self.angular_damping = damping;
        self
    }

    /// Set whether body can rotate
    pub fn with_rotation_locked(mut self, locked: bool) -> Self {
        self.can_rotate = !locked;
        self
    }

    /// Enable Continuous Collision Detection (prevents tunneling through thin objects)
    pub fn with_ccd(mut self, enabled: bool) -> Self {
        self.ccd_enabled = enabled;
        self
    }

    // Note: there are intentionally no `apply_impulse`/`apply_force` methods
    // here. The `velocity` field is only read when the rapier body is first
    // created, so mutating it on a live body silently does nothing. Use
    // `PhysicsSystem::set_velocity` or `PhysicsWorld::apply_impulse` for
    // mass-aware impulses.
}

pub use crate::shapes::ColliderShape;

/// Collider component for collision detection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collider {
    /// Shape of the collider
    pub shape: ColliderShape,
    /// Offset from entity position
    pub offset: Vec2,
    /// Whether this collider is a sensor (triggers events but doesn't cause collision response)
    pub is_sensor: bool,
    /// Friction coefficient (0.0 = no friction, 1.0 = high friction)
    pub friction: f32,
    /// Restitution (bounciness, 0.0 = no bounce, 1.0 = perfect bounce)
    pub restitution: f32,
    /// Collision groups (which groups this collider belongs to)
    pub collision_groups: u32,
    /// Collision filter (which groups this collider can collide with)
    pub collision_filter: u32,
    /// Handle to the rapier collider (set by PhysicsWorld)
    #[serde(skip)]
    pub(crate) handle: Option<rapier2d::geometry::ColliderHandle>,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            shape: ColliderShape::default(),
            offset: Vec2::ZERO,
            is_sensor: false,
            friction: 0.5,
            restitution: 0.0,
            collision_groups: 0xFFFF_FFFF,
            collision_filter: 0xFFFF_FFFF,
            handle: None,
        }
    }
}

impl Collider {
    /// Create a new collider with the given shape
    pub fn new(shape: ColliderShape) -> Self {
        Self {
            shape,
            ..Default::default()
        }
    }

    /// Create a box collider
    pub fn box_collider(width: f32, height: f32) -> Self {
        Self::new(ColliderShape::box_shape(width, height))
    }

    /// Create a circle collider
    pub fn circle_collider(radius: f32) -> Self {
        Self::new(ColliderShape::circle(radius))
    }

    /// Set the offset
    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    /// Set as sensor (no collision response, just detection)
    pub fn as_sensor(mut self) -> Self {
        self.is_sensor = true;
        self
    }

    /// Set friction
    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction.clamp(0.0, 1.0);
        self
    }

    /// Set restitution (bounciness)
    pub fn with_restitution(mut self, restitution: f32) -> Self {
        self.restitution = restitution.clamp(0.0, 1.0);
        self
    }
}

/// Collision event data
#[derive(Debug, Clone)]
pub struct CollisionEvent {
    /// First entity in the collision
    pub entity_a: ecs::EntityId,
    /// Second entity in the collision
    pub entity_b: ecs::EntityId,
    /// Whether the collision started this frame
    pub started: bool,
    /// Whether the collision ended this frame
    pub stopped: bool,
}

impl CollisionEvent {
    /// Check if both entities are involved in this collision (order-independent).
    pub fn involves(&self, a: ecs::EntityId, b: ecs::EntityId) -> bool {
        (self.entity_a == a && self.entity_b == b)
            || (self.entity_a == b && self.entity_b == a)
    }

    /// Get the other entity in the collision, if the given entity is involved.
    ///
    /// Returns `None` if the given entity is not part of this collision.
    pub fn other(&self, entity: ecs::EntityId) -> Option<ecs::EntityId> {
        if self.entity_a == entity {
            Some(self.entity_b)
        } else if self.entity_b == entity {
            Some(self.entity_a)
        } else {
            None
        }
    }
}

/// Contact point data for detailed collision information
#[derive(Debug, Clone)]
pub struct ContactPoint {
    /// Contact point in world space
    pub point: Vec2,
    /// Contact normal (pointing from entity_a to entity_b)
    pub normal: Vec2,
    /// Penetration depth
    pub depth: f32,
}

/// Detailed collision data with contact points
#[derive(Debug, Clone)]
pub struct CollisionData {
    /// Basic collision event
    pub event: CollisionEvent,
    /// Contact points (may be empty for sensors)
    pub contacts: Vec<ContactPoint>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event_between(a: ecs::EntityId, b: ecs::EntityId) -> CollisionEvent {
        CollisionEvent { entity_a: a, entity_b: b, started: true, stopped: false }
    }

    #[test]
    fn test_collision_event_membership_is_order_independent() {
        let a = ecs::EntityId::new();
        let b = ecs::EntityId::new();
        let stranger = ecs::EntityId::new();
        let event = event_between(a, b);

        assert!(event.involves(a, b) && event.involves(b, a), "both orders name the same pair");
        assert!(!event.involves(a, stranger) && !event.involves(stranger, b));
    }

    #[test]
    fn test_collision_event_other_returns_the_partner_or_none() {
        let a = ecs::EntityId::new();
        let b = ecs::EntityId::new();
        let stranger = ecs::EntityId::new();
        let event = event_between(a, b);

        assert_eq!(event.other(a), Some(b));
        assert_eq!(event.other(b), Some(a));
        assert_eq!(event.other(stranger), None);
    }

    #[test]
    fn test_body_type_selector_indices_round_trip_through_their_table() {
        // The inspector cycles by index into ALL: an index that disagrees
        // with its table jumps the selector to the wrong variant.
        for (index, body_type) in RigidBodyType::ALL.iter().enumerate() {
            assert_eq!(body_type.index(), index, "{} out of order", body_type.label());
            assert_eq!(RigidBodyType::ALL[body_type.index()], *body_type);
        }
    }
}
