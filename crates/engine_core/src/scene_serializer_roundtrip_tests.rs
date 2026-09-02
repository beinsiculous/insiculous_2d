//! Round-trip tests for `scene_serializer` (world → RON → world), hoisted
//! to a sibling file for size — per-component extraction tests live in
//! `scene_serializer_tests.rs`.

use crate::scene_data::*;
use crate::scene_serializer::*;
use ecs::sprite_components::{Name, Sprite, Transform2D};
use ecs::World;
use glam::{Vec2, Vec4};

/// Stub texture path function for testing
fn test_texture_path(handle: u32) -> String {
    if handle == 0 {
        "#white".to_string()
    } else {
        format!("#texture_{}", handle)
    }
}

struct StubResolver;
impl crate::texture_ref::TextureResolver for StubResolver {
    fn resolve_texture(
        &mut self,
        _texture_ref: &str,
    ) -> Result<renderer::TextureHandle, SceneLoadError> {
        Ok(renderer::TextureHandle { id: 0 })
    }
}

/// Serialize a single-entity world to RON, reload it into a fresh world,
/// and return the loaded entity.
fn roundtrip_single_entity(world: &World) -> (World, ecs::EntityId) {
    let scene = world_to_scene_data(world, "UiRoundTrip", None, &test_texture_path);
    let ron_string = serialize_to_ron(&scene).expect("serialize");
    let parsed = crate::scene_loader::SceneLoader::parse(&ron_string).expect("parse");
    let mut loaded_world = World::new();
    let instance =
        crate::scene_loader::SceneLoader::instantiate(&parsed, &mut loaded_world, &mut StubResolver)
            .expect("instantiate");
    assert_eq!(instance.entity_count, 1);
    let entity = instance.entities[0];
    (loaded_world, entity)
}

#[test]
fn test_roundtrip_serialize_deserialize() {
    let mut world = World::new();

    // Create an entity with multiple components
    let entity = world.create_entity();
    world.add_component(&entity, Name::new("RoundTrip")).ok();
    world
        .add_component(
            &entity,
            Transform2D {
                position: Vec2::new(100.0, 200.0),
                rotation: 0.5,
                scale: Vec2::new(2.0, 2.0),
            },
        )
        .ok();
    world
        .add_component(
            &entity,
            Sprite {
                texture_handle: 0,
                offset: Vec2::new(5.0, 10.0),
                rotation: 0.1,
                scale: Vec2::new(1.5, 1.5),
                color: Vec4::new(0.5, 0.6, 0.7, 1.0),
                depth: 5.0,
                visible: true,
                emissive: 0.0,
                tex_region: [0.0, 0.0, 1.0, 1.0],
            },
        )
        .ok();

    let scene = world_to_scene_data(&world, "RoundTrip", None, &test_texture_path);
    let ron_string = serialize_to_ron(&scene).expect("Serialize should succeed");
    let parsed: SceneData = ron::from_str(&ron_string).expect("Parse should succeed");

    assert_eq!(parsed.name, "RoundTrip");
    assert_eq!(parsed.entities.len(), 1);
    assert_eq!(parsed.entities[0].name, Some("RoundTrip".to_string()));
    assert_eq!(parsed.entities[0].components.len(), 2);

    // Verify Transform2D
    match &parsed.entities[0].components[0] {
        ComponentData::Transform2D {
            position,
            rotation,
            scale,
        } => {
            assert_eq!(*position, (100.0, 200.0));
            assert_eq!(*rotation, 0.5);
            assert_eq!(*scale, (2.0, 2.0));
        }
        other => panic!("Expected Transform2D, got {:?}", other),
    }

    // Verify Sprite
    match &parsed.entities[0].components[1] {
        ComponentData::Sprite {
            texture,
            offset,
            color,
            depth,
            ..
        } => {
            assert_eq!(texture, "#white");
            assert_eq!(*offset, (5.0, 10.0));
            assert_eq!(*color, (0.5, 0.6, 0.7, 1.0));
            assert_eq!(*depth, 5.0);
        }
        other => panic!("Expected Sprite, got {:?}", other),
    }
}

// === UI element round-trips (world → RON → world) ===

#[test]
fn test_ui_label_roundtrip() {
    let mut world = World::new();
    let entity = world.create_entity();
    world
        .add_component(
            &entity,
            ecs::UiLabel {
                text: "@hud.score".into(),
                anchor: ecs::UiAnchor::TopRight,
                offset: Vec2::new(-12.0, 8.0),
                font_size: 22.0,
                color: Vec4::new(0.9, 0.8, 0.2, 1.0),
                visible: true,
            },
        )
        .ok();

    let (loaded, e) = roundtrip_single_entity(&world);
    let label = loaded.get::<ecs::UiLabel>(e).expect("UiLabel survives round-trip");
    assert_eq!(label.text, "@hud.score");
    assert_eq!(label.anchor, ecs::UiAnchor::TopRight);
    assert_eq!(label.offset, Vec2::new(-12.0, 8.0));
    assert_eq!(label.font_size, 22.0);
    assert_eq!(label.color, Vec4::new(0.9, 0.8, 0.2, 1.0));
    assert!(label.visible);
}

#[test]
fn test_ui_panel_roundtrip() {
    let mut world = World::new();
    let entity = world.create_entity();
    world
        .add_component(
            &entity,
            ecs::UiPanel {
                anchor: ecs::UiAnchor::BottomCenter,
                offset: Vec2::new(0.0, -20.0),
                size: Vec2::new(300.0, 80.0),
                background: Vec4::new(0.0, 0.1, 0.2, 0.9),
                border: Vec4::new(0.0, 1.0, 1.0, 1.0),
                border_width: 2.0,
                visible: false,
            },
        )
        .ok();

    let (loaded, e) = roundtrip_single_entity(&world);
    let panel = loaded.get::<ecs::UiPanel>(e).expect("UiPanel survives round-trip");
    assert_eq!(panel.anchor, ecs::UiAnchor::BottomCenter);
    assert_eq!(panel.size, Vec2::new(300.0, 80.0));
    assert_eq!(panel.background, Vec4::new(0.0, 0.1, 0.2, 0.9));
    assert_eq!(panel.border_width, 2.0);
    assert!(!panel.visible);
}

#[test]
fn test_ui_button_roundtrip() {
    let mut world = World::new();
    let entity = world.create_entity();
    world
        .add_component(
            &entity,
            ecs::UiButton {
                text: "@menu.play".into(),
                id: "play".into(),
                anchor: ecs::UiAnchor::Center,
                offset: Vec2::new(0.0, 40.0),
                size: Vec2::new(160.0, 40.0),
                visible: true,
            },
        )
        .ok();

    let (loaded, e) = roundtrip_single_entity(&world);
    let button = loaded.get::<ecs::UiButton>(e).expect("UiButton survives round-trip");
    assert_eq!(button.text, "@menu.play");
    assert_eq!(button.id, "play");
    assert_eq!(button.anchor, ecs::UiAnchor::Center);
    assert_eq!(button.offset, Vec2::new(0.0, 40.0));
    assert_eq!(button.size, Vec2::new(160.0, 40.0));
}

#[test]
fn test_grid_backdrop_round_trips_every_field_and_parses_bare() {
    // #46: the component survives save → load field for field ...
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(Vec2::new(10.0, 20.0))).ok();
    let authored = ecs::GridBackdrop {
        topology: ecs::GridTopology::Square,
        cols: 7,
        rows: 5,
        spacing: 12.5,
        color: Vec4::new(0.1, 0.2, 0.3, 0.4),
        emissive: 1.5,
        visible: false,
        stiffness: 11.0,
        damping: 0.2,
        rest_pull: 3.0,
        rest_alpha_fraction: 0.5,
        activity_attack: 0.1,
        activity_release: 0.9,
        activity_displacement_ref: 2.0,
        activity_velocity_ref: 20.0,
    };
    world.add_component(&entity, authored.clone()).ok();

    let (loaded_world, loaded) = roundtrip_single_entity(&world);
    assert_eq!(loaded_world.get::<ecs::GridBackdrop>(loaded), Some(&authored));
    assert_eq!(loaded_world.get::<Transform2D>(loaded).unwrap().position, Vec2::new(10.0, 20.0));

    // ... and `GridBackdrop()` alone means the playfield preset.
    let scene = crate::scene_loader::SceneLoader::parse(
        "SceneData(name: \"g\", entities: [EntityData(name: Some(\"backdrop\"), components: [GridBackdrop()])])",
    )
    .expect("bare GridBackdrop parses");
    let mut bare_world = World::new();
    let instance = crate::scene_loader::SceneLoader::instantiate(&scene, &mut bare_world, &mut StubResolver)
        .expect("instantiate");
    let backdrop = bare_world.get::<ecs::GridBackdrop>(instance.entities[0]).expect("component attached");
    assert_eq!(backdrop, &ecs::GridBackdrop::default());
}
