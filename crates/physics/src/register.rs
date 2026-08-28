//! Dynamic-registry wiring for the physics components (issue #43, ecs GPP-16).
//!
//! physics is downstream of ecs, so it cannot appear in the registry's
//! builtin list — the engine calls [`register_components`] at startup
//! instead (idempotent; headless scene tests get it via `SceneLoader`).

use ecs::component_registry::{ComponentMeta, ComponentRegistry};

use crate::components::{Collider, RigidBody};

impl ComponentMeta for RigidBody {
    fn type_name() -> &'static str {
        "RigidBody"
    }

    fn field_names() -> &'static [&'static str] {
        // `handle` is #[serde(skip)] runtime state — not a data field.
        &[
            "body_type",
            "velocity",
            "angular_velocity",
            "gravity_scale",
            "linear_damping",
            "angular_damping",
            "can_rotate",
            "ccd_enabled",
        ]
    }
}

impl ComponentMeta for Collider {
    fn type_name() -> &'static str {
        "Collider"
    }

    fn field_names() -> &'static [&'static str] {
        // `handle` is #[serde(skip)] runtime state — not a data field.
        &[
            "shape",
            "offset",
            "is_sensor",
            "friction",
            "restitution",
            "collision_groups",
            "collision_filter",
        ]
    }
}

/// Register the physics components into a dynamic registry.
pub fn register_components(registry: &mut ComponentRegistry) {
    registry.register::<RigidBody>();
    registry.register::<Collider>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecs::World;

    #[test]
    fn test_physics_components_round_trip_through_the_dynamic_tier() {
        let mut registry = ComponentRegistry::new();
        register_components(&mut registry);
        let mut world = World::new();
        let entity = world.create_entity();

        registry
            .insert_default(&mut world, entity, "RigidBody")
            .expect("insert RigidBody");
        registry
            .insert_default(&mut world, entity, "Collider")
            .expect("insert Collider");
        assert!(world.get::<RigidBody>(entity).is_some());

        // The runtime rapier handle is #[serde(skip)]: extraction must not
        // leak it, and re-insertion must round-trip the data fields.
        let value = registry
            .extract_component(&world, entity, "Collider")
            .expect("known")
            .expect("present");
        assert!(value.get("handle").is_none());
        let other = world.create_entity();
        registry
            .insert_component(&mut world, other, "Collider", value)
            .expect("re-insert");
        assert_eq!(
            world.get::<Collider>(entity).map(|c| c.friction),
            world.get::<Collider>(other).map(|c| c.friction)
        );
    }
}
