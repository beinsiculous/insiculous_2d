//! Physics presets for common game types
//!
//! These presets provide tested, ready-to-use physics configurations
//! so developers don't have to guess at values.

use glam::Vec2;
use crate::components::{Collider, RigidBody};
use crate::physics_world::PhysicsConfig;

/// Preset rigid body configurations
impl RigidBody {
    /// Create a player body optimized for platformer games
    pub fn player_platformer() -> Self {
        Self::new_dynamic()
            .with_linear_damping(5.0) // stops quickly when not moving
            .with_rotation_locked(true)
            .with_ccd(true)
    }
}

/// Preset collider configurations
impl Collider {
    /// Create a player box collider with high friction for the given sprite size
    pub fn player_box(width: f32, height: f32) -> Self {
        Self::box_collider(width, height)
            .with_friction(0.8)
    }

    /// Create a ground/platform collider
    pub fn platform(width: f32, height: f32) -> Self {
        Self::box_collider(width, height)
            .with_friction(0.8)
    }
}

/// Preset physics world configurations
impl PhysicsConfig {
    /// Standard platformer physics
    /// - Gravity: -980 (feels like ~10 m/s^2 with 100 px/m scale)
    /// - High solver iterations for stable stacking
    pub fn platformer() -> Self {
        Self::new(Vec2::new(0.0, -980.0))
            .with_iterations(16, 8)
    }

    /// Top-down game physics (no gravity)
    pub fn top_down() -> Self {
        Self::new(Vec2::ZERO)
            .with_iterations(8, 4)
    }

    /// Space physics (no gravity, low iterations)
    pub fn space() -> Self {
        Self::new(Vec2::ZERO)
            .with_iterations(4, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_values_are_the_tuning_the_examples_and_games_rely_on() {
        // hello_world / editor_demo build their player and ground on these.
        let player = RigidBody::player_platformer();
        assert_eq!(
            (player.linear_damping, player.can_rotate, player.ccd_enabled),
            (5.0, false, true),
            "player_platformer: quick stop, no tumbling, no tunnelling"
        );
        assert_eq!(Collider::player_box(80.0, 80.0).friction, 0.8);
        assert_eq!(Collider::platform(800.0, 40.0).friction, 0.8);

        // The examples and the standalone editor default to platformer();
        // breakout / space_invaders run on top_down(), asteroids on space().
        let configs = [
            (PhysicsConfig::platformer(), Vec2::new(0.0, -980.0), "platformer"),
            (PhysicsConfig::top_down(), Vec2::ZERO, "top_down"),
            (PhysicsConfig::space(), Vec2::ZERO, "space"),
        ];
        for (config, gravity, name) in configs {
            assert_eq!(config.gravity, gravity, "{name} gravity");
            assert_eq!(config.pixels_per_meter, 100.0, "{name} keeps the 100 px/m scale");
        }
    }
}
