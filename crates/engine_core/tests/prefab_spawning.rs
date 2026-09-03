//! Runtime prefab spawning (the Prototype pattern's actual purpose): a loaded
//! scene retains its prefab table and can stamp out new entities from it
//! mid-game with override semantics, and a spawn that fails leaves nothing
//! behind — all headless through the `TextureResolver` seam.

use std::collections::HashMap;

use ecs::behavior::EntityTag;
use ecs::sprite_components::{Sprite, Transform2D};
use ecs::World;
use engine_core::prelude::*;
use engine_core::test_support::StubResolver;
use engine_core::TextureResolver;
use glam::Vec2;
use renderer::TextureHandle;

/// A scene with one "Ball" prefab (transform + sprite + tag) and one
/// pre-placed entity using it.
fn ball_scene() -> SceneData {
    let ball_prefab = PrefabData {
        components: vec![
            ComponentData::Transform2D { position: (0.0, 0.0), rotation: 0.0, scale: (1.0, 1.0) },
            ComponentData::Sprite {
                texture: "#white".to_string(),
                offset: (0.0, 0.0),
                rotation: 0.0,
                scale: (16.0, 16.0),
                color: (1.0, 1.0, 1.0, 1.0),
                depth: 0.0,
                emissive: 0.0,
                tex_region: (0.0, 0.0, 1.0, 1.0),
                visible: true,
            },
            ComponentData::EntityTag { tag: "ball".to_string() },
        ],
    };
    let mut prefabs = HashMap::new();
    prefabs.insert("Ball".to_string(), ball_prefab);

    SceneData {
        name: "prefab test".to_string(),
        physics: None,
        editor: None,
        prefabs,
        entities: vec![EntityData {
            name: Some("first_ball".to_string()),
            prefab: Some("Ball".to_string()),
            overrides: vec![],
            components: vec![],
            parent: None,
            children: vec![],
        }],
    }
}

#[test]
fn spawn_prefab_stamps_a_new_entity_with_overrides_applied() -> Result<(), SceneLoadError> {
    let mut world = World::new();
    let mut resolver = StubResolver::default();
    let instance = SceneLoader::instantiate(&ball_scene(), &mut world, &mut resolver)?;
    let scene_ball = instance.get_entity("first_ball").expect("the placed ball is named");
    let overrides = [ComponentData::Transform2D { position: (200.0, 300.0), rotation: 0.0, scale: (1.0, 1.0) }];

    let spawned = instance.spawn_prefab(&mut world, &mut resolver, "Ball", &overrides)?;

    assert!(instance.has_prefab("Ball") && !instance.has_prefab("Paddle"), "the prefab table is retained");
    assert_ne!(spawned, scene_ball, "a runtime spawn is a NEW entity");
    assert_eq!(
        world.get::<Transform2D>(spawned).map(|t| t.position),
        Some(Vec2::new(200.0, 300.0)),
        "the override replaces the prefab's transform"
    );
    assert!(world.get::<Sprite>(spawned).is_some(), "non-overridden prefab components still apply");
    assert!(world.get::<EntityTag>(spawned).is_some_and(|tag| tag.matches("ball")));
    assert_eq!(instance.entity_count, 1, "the scene's own bookkeeping is untouched — the caller owns the spawn");
    Ok(())
}

#[test]
fn failed_spawn_leaves_no_half_built_entity() -> Result<(), SceneLoadError> {
    /// A resolver that always fails — a missing texture file.
    struct FailingResolver;
    impl TextureResolver for FailingResolver {
        fn resolve_texture(&mut self, texture_ref: &str) -> Result<TextureHandle, SceneLoadError> {
            Err(SceneLoadError::TextureLoadError(texture_ref.to_string()))
        }
    }

    let mut world = World::new();
    let instance = SceneLoader::instantiate(&ball_scene(), &mut world, &mut StubResolver::default())?;
    let before = world.entities().len();

    let unknown = instance.spawn_prefab(&mut world, &mut StubResolver::default(), "Nope", &[]);
    let mid_build = instance.spawn_prefab(&mut world, &mut FailingResolver, "Ball", &[]);

    assert!(matches!(unknown, Err(SceneLoadError::PrefabNotFound(_))), "{unknown:?}");
    assert!(mid_build.is_err(), "a texture failure fails the spawn");
    assert_eq!(world.entities().len(), before, "neither failure leaves entity debris");
    Ok(())
}
