//! Hard physical floors shared between inspector field editors and the command API sanitizer.

use glam::Vec2;
use physics::components::{Collider, ColliderShape};

pub const SCALE_FLOOR: f32 = 0.01;
pub const COLLIDER_EXTENT_FLOOR: f32 = 0.5;
pub const CAPSULE_HALF_HEIGHT_FLOOR: f32 = 0.0;
pub const VOLUME_MIN: f32 = 0.0;
pub const VOLUME_MAX: f32 = 1.0;
pub const PITCH_FLOOR: f32 = 0.1;

/// Clamp Transform2D scale above the physical floor.
pub fn clamp_transform(transform: &mut common::Transform2D) {
    transform.scale = transform.scale.max(Vec2::splat(SCALE_FLOOR));
}

/// Clamp Sprite scale above the physical floor.
pub fn clamp_sprite(sprite: &mut ecs::sprite_components::Sprite) {
    sprite.scale = sprite.scale.max(Vec2::splat(SCALE_FLOOR));
}

/// Clamp Collider shape dimensions above physical minimums (zero-extent breaks physics).
pub fn clamp_collider(collider: &mut Collider) {
    collider.shape = match collider.shape {
        ColliderShape::Box { half_extents } => ColliderShape::Box {
            half_extents: half_extents.max(Vec2::splat(COLLIDER_EXTENT_FLOOR)),
        },
        ColliderShape::Circle { radius } => ColliderShape::Circle {
            radius: radius.max(COLLIDER_EXTENT_FLOOR),
        },
        ColliderShape::CapsuleY { half_height, radius } => ColliderShape::CapsuleY {
            half_height: half_height.max(CAPSULE_HALF_HEIGHT_FLOOR),
            radius: radius.max(COLLIDER_EXTENT_FLOOR),
        },
        ColliderShape::CapsuleX { half_height, radius } => ColliderShape::CapsuleX {
            half_height: half_height.max(CAPSULE_HALF_HEIGHT_FLOOR),
            radius: radius.max(COLLIDER_EXTENT_FLOOR),
        },
    };
}

/// Clamp AudioSource volume to [0, 1] and pitch above the playback floor.
pub fn clamp_audio_source(source: &mut ecs::audio_components::AudioSource) {
    source.volume = source.volume.clamp(VOLUME_MIN, VOLUME_MAX);
    source.pitch = source.pitch.max(PITCH_FLOOR);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_transform_enforces_scale_floor() {
        let mut t = common::Transform2D::new(Vec2::ZERO);
        t.scale = Vec2::new(-5.0, 0.0);
        clamp_transform(&mut t);
        assert_eq!(t.scale, Vec2::splat(SCALE_FLOOR));
    }

    #[test]
    fn test_clamp_sprite_enforces_scale_floor() {
        let mut s = ecs::sprite_components::Sprite::new(0);
        s.scale = Vec2::new(0.0, -1.0);
        clamp_sprite(&mut s);
        assert_eq!(s.scale, Vec2::splat(SCALE_FLOOR));
    }

    #[test]
    fn test_clamp_collider_enforces_extent_floors() {
        let mut c = Collider::new(ColliderShape::Box { half_extents: Vec2::new(-1.0, 0.1) });
        clamp_collider(&mut c);
        assert_eq!(c.shape, ColliderShape::Box { half_extents: Vec2::splat(COLLIDER_EXTENT_FLOOR) });

        let mut c = Collider::new(ColliderShape::Circle { radius: 0.1 });
        clamp_collider(&mut c);
        assert_eq!(c.shape, ColliderShape::Circle { radius: COLLIDER_EXTENT_FLOOR });

        let mut c = Collider::new(ColliderShape::CapsuleY { half_height: -1.0, radius: 0.2 });
        clamp_collider(&mut c);
        assert_eq!(c.shape, ColliderShape::CapsuleY {
            half_height: CAPSULE_HALF_HEIGHT_FLOOR,
            radius: COLLIDER_EXTENT_FLOOR,
        });

        let mut c = Collider::new(ColliderShape::CapsuleX { half_height: -1.0, radius: 0.2 });
        clamp_collider(&mut c);
        assert_eq!(c.shape, ColliderShape::CapsuleX {
            half_height: CAPSULE_HALF_HEIGHT_FLOOR,
            radius: COLLIDER_EXTENT_FLOOR,
        });
    }

    #[test]
    fn test_clamp_audio_source_enforces_volume_and_pitch() {
        let mut a = ecs::audio_components::AudioSource::new(1);
        a.volume = -0.5;
        a.pitch = 0.01;
        clamp_audio_source(&mut a);
        assert_eq!(a.volume, VOLUME_MIN);
        assert_eq!(a.pitch, PITCH_FLOOR);

        a.volume = 1.5;
        clamp_audio_source(&mut a);
        assert_eq!(a.volume, VOLUME_MAX);
    }
}
