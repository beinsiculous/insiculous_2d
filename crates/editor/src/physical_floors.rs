//! Hard physical floors shared between inspector field editors and the command API sanitizer.

use glam::Vec2;
use physics::components::{Collider, ColliderShape};

pub const SCALE_FLOOR: f32 = 0.01;
pub const COLLIDER_EXTENT_FLOOR: f32 = 0.5;
pub const CAPSULE_HALF_HEIGHT_FLOOR: f32 = 0.0;
/// A sound's level: below silence means nothing.
pub const VOLUME_FLOOR: f32 = 0.0;
/// A spatial sound's reach: zero would divide its attenuation by zero.
pub const MAX_DISTANCE_FLOOR: f32 = ecs::Spatial::MIN_MAX_DISTANCE;

/// Clamp Transform2D scale above the physical floor.
pub fn clamp_transform(transform: &mut common::Transform2D) {
    transform.scale = transform.scale.max(Vec2::splat(SCALE_FLOOR));
}

/// Clamp Sprite scale above the physical floor.
pub fn clamp_sprite(sprite: &mut ecs::sprite_components::Sprite) {
    sprite.scale = sprite.scale.max(Vec2::splat(SCALE_FLOOR));
}

/// Clamp a SoundEmitter's volume and spatial reach above their floors (`max` keeps the
/// floor for a NaN as well), and hold a binding emitter to what the scene loader accepts:
/// never spatial, never looping. The inspector's kind cycle and the command API's writes
/// both pass through here, so neither can save a scene the loader refuses.
pub fn clamp_sound_emitter(emitter: &mut ecs::SoundEmitter) {
    emitter.volume = emitter.volume.max(VOLUME_FLOOR);
    if emitter.sound.is_binding() {
        emitter.spatial = None;
        emitter.looping = false;
    }
    if let Some(spatial) = emitter.spatial.as_mut() {
        spatial.max_distance = spatial.max_distance.max(MAX_DISTANCE_FLOOR);
    }
}

/// Clamp Collider shape dimensions above physical minimums (zero-extent breaks physics).
pub fn clamp_collider(collider: &mut Collider) {
    collider.shape = clamp_shape(collider.shape.clone());
}

/// Clamp one shape's dimensions, and a compound's part by part.
///
/// A capsule's endpoints are left alone: a zero-length segment is a ball,
/// which is a capsule rapier builds without complaint, and the endpoints are
/// positions rather than sizes.
fn clamp_shape(shape: ColliderShape) -> ColliderShape {
    match shape {
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
        ColliderShape::Capsule { a, b, radius } => ColliderShape::Capsule {
            a,
            b,
            radius: radius.max(COLLIDER_EXTENT_FLOOR),
        },
        ColliderShape::Compound(parts) => {
            ColliderShape::Compound(parts.into_iter().map(clamp_shape).collect())
        }
    }
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
    fn test_clamp_sound_emitter_holds_a_typed_minus_one_at_the_floors() {
        let mut emitter = ecs::SoundEmitter {
            volume: -1.0,
            spatial: Some(ecs::Spatial { max_distance: -1.0 }),
            ..Default::default()
        };
        clamp_sound_emitter(&mut emitter);
        assert_eq!(emitter.volume, VOLUME_FLOOR);
        assert_eq!(emitter.spatial, Some(ecs::Spatial { max_distance: MAX_DISTANCE_FLOOR }));
    }

    #[test]
    fn test_clamp_sound_emitter_strips_what_a_binding_cannot_have() {
        let mut emitter = ecs::SoundEmitter {
            sound: ecs::SoundRef::Binding("hit".into()),
            looping: true,
            spatial: Some(ecs::Spatial { max_distance: 8.0 }),
            ..Default::default()
        };
        clamp_sound_emitter(&mut emitter);
        assert!(!emitter.looping && emitter.spatial.is_none(), "an API write cannot save what the loader refuses");
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
    fn test_clamp_collider_floors_a_capsule_radius_and_leaves_its_endpoints_alone() {
        // Coincident endpoints are a ball, and rapier builds that capsule
        // happily — the endpoints are positions, not sizes.
        let endpoints = (Vec2::new(-40.0, 12.0), Vec2::new(-40.0, 12.0));
        let mut c = Collider::new(ColliderShape::capsule(endpoints.0, endpoints.1, 0.1));
        clamp_collider(&mut c);
        assert_eq!(
            c.shape,
            ColliderShape::Capsule {
                a: endpoints.0,
                b: endpoints.1,
                radius: COLLIDER_EXTENT_FLOOR,
            }
        );
    }

    #[test]
    fn test_clamp_collider_reaches_every_part_of_a_compound() {
        let mut c = Collider::new(ColliderShape::compound(vec![
            ColliderShape::circle(0.1),
            ColliderShape::capsule(Vec2::new(-30.0, 40.0), Vec2::ZERO, -1.0),
        ]));
        clamp_collider(&mut c);
        assert_eq!(
            c.shape,
            ColliderShape::Compound(vec![
                ColliderShape::Circle { radius: COLLIDER_EXTENT_FLOOR },
                ColliderShape::Capsule {
                    a: Vec2::new(-30.0, 40.0),
                    b: Vec2::ZERO,
                    radius: COLLIDER_EXTENT_FLOOR,
                },
            ])
        );
    }
}
