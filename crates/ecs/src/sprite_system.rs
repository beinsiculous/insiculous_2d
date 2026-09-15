//! Sprite animation system for ECS integration

use crate::{
    sprite_components::{Sprite, SpriteAnimation},
    EntityId, System, World,
};

/// Write the region of `entity`'s current animation frame onto its [`Sprite`],
/// when it carries both and the frame resolves.
///
/// [`SpriteAnimationSystem`] does this after every advance; the clip state
/// machine does it the moment it selects a clip, so the frame that enters a
/// state shows the new clip's first frame rather than the previous clip's last.
pub fn sync_sprite_region(world: &mut World, entity: EntityId) {
    let Some(region) = world
        .get::<SpriteAnimation>(entity)
        .and_then(SpriteAnimation::current_uv)
    else {
        return;
    };
    if let Some(sprite) = world.get_mut::<Sprite>(entity) {
        sprite.tex_region = region;
    }
}

/// Advances every [`SpriteAnimation`] and writes the resulting cell region
/// onto the entity's [`Sprite`].
///
/// This is the link that makes animation visible: without it a component can
/// hold clips but nothing ever reaches `Sprite.tex_region`. Entities that have
/// an animation but no sprite are skipped, and an animation whose current
/// frame does not resolve (nothing playing, empty clip, index past the grid)
/// leaves the sprite's region untouched.
pub struct SpriteAnimationSystem;

impl System for SpriteAnimationSystem {
    fn update(&mut self, world: &mut World, delta_time: f32) {
        for entity_id in world.entities() {
            // Advance first, then hand the resolved region to the sprite in a
            // second lookup — two components on one entity cannot be borrowed
            // mutably at once.
            match world.get_mut::<SpriteAnimation>(entity_id) {
                Some(animation) => animation.update(delta_time),
                None => continue,
            }
            sync_sprite_region(world, entity_id);
        }
    }

    fn name(&self) -> &str {
        "SpriteAnimationSystem"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite_components::{AnimationClip, SheetGrid};
    use crate::EcsError;

    /// A 4x1 sheet with a two-frame clip selected and playing.
    fn animated_entity(world: &mut World) -> Result<crate::EntityId, EcsError> {
        let entity = world.create_entity();
        let mut animation = SpriteAnimation::new(SheetGrid::new(4, 1))
            .with_clip("walk", AnimationClip::new(vec![0, 1], 10.0))
            .with_clip("bad", AnimationClip::new(vec![99], 10.0));
        assert!(animation.play("walk"));
        world.add_component(&entity, animation)?;
        world.add_component(&entity, Sprite::new(0))?;
        Ok(entity)
    }

    fn region(world: &World, entity: crate::EntityId) -> [f32; 4] {
        world.get::<Sprite>(entity).expect("sprite").tex_region
    }

    #[test]
    fn test_system_writes_current_frame_region_to_sprite() -> Result<(), EcsError> {
        let mut world = World::new();
        let entity = animated_entity(&mut world)?;

        SpriteAnimationSystem.update(&mut world, 0.0);
        assert_eq!(region(&world, entity), [0.0, 0.0, 0.25, 1.0]);

        // One full frame at 10 fps advances to cell 1.
        SpriteAnimationSystem.update(&mut world, 0.1);
        assert_eq!(region(&world, entity), [0.25, 0.0, 0.25, 1.0]);

        // A frame that does not resolve (index past the 4-cell grid) leaves
        // the sprite's region alone instead of writing garbage.
        assert!(world.get_mut::<SpriteAnimation>(entity).expect("animation").play("bad"));
        SpriteAnimationSystem.update(&mut world, 0.1);
        assert_eq!(region(&world, entity), [0.25, 0.0, 0.25, 1.0]);
        Ok(())
    }

    #[test]
    fn test_system_with_zero_delta_freezes_the_frame() -> Result<(), EcsError> {
        let mut world = World::new();
        let entity = animated_entity(&mut world)?;

        // dt 0 is how a paused game reaches the system (time_scale 0.0).
        for _ in 0..100 {
            SpriteAnimationSystem.update(&mut world, 0.0);
        }

        assert_eq!(world.get::<SpriteAnimation>(entity).expect("animation").current_frame, 0);
        assert_eq!(region(&world, entity), [0.0, 0.0, 0.25, 1.0]);
        Ok(())
    }
}
