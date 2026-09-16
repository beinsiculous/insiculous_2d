//! Collider scene persistence: valid leaves survive nesting, while a
//! collider with no leaves fails with a dedicated load error.

use ecs::sprite_components::{Name, Transform2D};
use ecs::World;
use glam::Vec2;
use physics::{Collider, ColliderShape};

use crate::scene_data::SceneLoadError;
use crate::scene_loader::SceneLoader;
use crate::test_support::{roundtrip, save_to_ron, StubResolver};

#[test]
fn capsule_and_compound_colliders_round_trip_through_save_and_load() {
    let bare = ColliderShape::capsule(Vec2::new(-8.0, -16.0), Vec2::new(-8.0, 16.0), 5.0);
    let jaw = ColliderShape::compound(vec![
        ColliderShape::capsule(Vec2::ZERO, Vec2::new(-30.0, 40.0), 6.0),
        ColliderShape::capsule(Vec2::ZERO, Vec2::new(30.0, 40.0), 6.0),
    ]);
    let mut world = World::new();
    let nested = ColliderShape::Compound(vec![
        ColliderShape::Compound(vec![]),
        ColliderShape::Compound(vec![jaw.clone(), ColliderShape::Compound(vec![])]),
    ]);
    for (name, shape) in [("bare", bare.clone()), ("jaw", jaw.clone()), ("nested", nested)] {
        let entity = world.create_entity();
        world.add_component(&entity, Name::new(name)).ok();
        world
            .add_component(&entity, Collider::new(shape).with_offset(Vec2::new(3.0, -4.0)))
            .ok();
    }

    let (loaded, instance) = roundtrip(&world);

    let loaded_bare = loaded
        .get::<Collider>(instance.named_entities["bare"])
        .expect("the capsule collider survives");
    assert_eq!(loaded_bare.shape, bare, "a capsule comes back with both endpoints and its radius");
    assert_eq!(loaded_bare.offset, Vec2::new(3.0, -4.0));
    let loaded_jaw = loaded
        .get::<Collider>(instance.named_entities["jaw"])
        .expect("the compound collider survives");
    assert_eq!(
        loaded_jaw.shape, jaw,
        "a compound comes back part for part, each part a capsule"
    );
    let loaded_nested = loaded.get::<Collider>(instance.named_entities["nested"])
        .expect("nonempty leaves survive empty siblings at any depth");
    assert_eq!(loaded_nested.shape, jaw, "loading preserves the complete collider's leaves");
}

#[test]
fn a_saved_empty_compound_collider_fails_with_a_dedicated_error() {
    // The writer records what the world holds, an empty compound included;
    // the loader is where the defect is caught, so a scene file that names a
    // jaw with no arms fails loudly instead of colliding with nothing.
    for shape in [
        ColliderShape::Compound(vec![]),
        ColliderShape::Compound(vec![ColliderShape::Compound(vec![])]),
    ] {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(&entity, Transform2D::new(Vec2::ZERO)).ok();
        world
            .add_component(&entity, Collider::new(shape))
            .ok();

        let ron_text = save_to_ron(&world);
        let parsed = SceneLoader::parse(&ron_text).expect("the empty compound is writable and parseable");

        let mut loaded = World::new();
        let error = SceneLoader::instantiate(&parsed, &mut loaded, &mut StubResolver::default())
            .expect_err("an empty compound refuses to load");
        assert!(
            matches!(error, SceneLoadError::EmptyCompoundCollider),
            "the refusal is its own named error, got {error:?}"
        );
        assert!(
            error.to_string().contains("no parts"),
            "and it says what is wrong: {error}"
        );
    }
}
