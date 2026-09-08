//! The save pipeline's contracts: what `world_to_scene_data` writes, and
//! that a saved world reads back field for field. The dynamic-registry and
//! Scripts round-trips live in `dynamic_and_scripts_tests.rs`.

use ecs::sprite_components::{Name, Sprite, Transform2D};
use ecs::{World, WorldHierarchyExt};
use glam::{Vec2, Vec4};
use physics::components::{Collider, ColliderShape, RigidBody};

use super::{serialize_to_ron, world_to_scene_data};
use crate::scene_data::*;
use crate::scene_loader::SceneLoader;
use crate::test_support::{load_ron, roundtrip, save_to_ron, test_texture_path};

/// A component as serde sees it — the field-for-field comparison for types
/// without `PartialEq`.
fn as_json<T: serde::Serialize>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).expect("component serializes")
}

/// The components of `world`'s only root entity, as the serializer writes
/// them.
fn saved_components(world: &World) -> Vec<ComponentData> {
    let scene = world_to_scene_data(world, "Save", None, &test_texture_path);
    assert_eq!(scene.entities.len(), 1, "one root entity");
    scene.entities[0].components.clone()
}

#[test]
fn world_round_trips_through_ron_field_for_field() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Name::new("RoundTrip")).ok();
    world
        .add_component(
            &entity,
            Transform2D { position: Vec2::new(100.0, 200.0), rotation: 0.5, scale: Vec2::new(2.0, 2.0) },
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
    world.add_component(&entity, ecs::behavior::EntityTag::new("enemy")).ok();
    let label = ecs::UiLabel {
        text: "@hud.score".into(),
        anchor: ecs::UiAnchor::TopRight,
        offset: Vec2::new(-12.0, 8.0),
        font_size: 22.0,
        color: Vec4::new(0.9, 0.8, 0.2, 1.0),
        visible: true,
    };
    let panel = ecs::UiPanel {
        anchor: ecs::UiAnchor::BottomCenter,
        offset: Vec2::new(0.0, -20.0),
        size: Vec2::new(300.0, 80.0),
        background: Vec4::new(0.0, 0.1, 0.2, 0.9),
        border: Vec4::new(0.0, 1.0, 1.0, 1.0),
        border_width: 2.0,
        visible: false,
    };
    let button = ecs::UiButton {
        text: "@menu.play".into(),
        id: "play".into(),
        anchor: ecs::UiAnchor::Center,
        offset: Vec2::new(0.0, 40.0),
        size: Vec2::new(160.0, 40.0),
        visible: true,
    };
    world.add_component(&entity, label.clone()).ok();
    world.add_component(&entity, panel.clone()).ok();
    world.add_component(&entity, button.clone()).ok();
    let camera = ecs::sprite_components::Camera {
        position: Vec2::new(50.0, 60.0),
        rotation: 0.1,
        zoom: 2.0,
        viewport_size: Vec2::new(1920.0, 1080.0),
        is_main_camera: true,
        near: -1000.0,
        far: 1000.0,
    };
    world.add_component(&entity, camera).ok();
    let mut tilemap = ecs::Tilemap::new(3, 2, 40.0);
    tilemap.tileset = 5;
    tilemap.tile_uv_size = Vec2::new(0.25, 0.25);
    tilemap.depth = -2.0;
    tilemap.set_tile(1, 0, 2);
    world.add_component(&entity, tilemap).ok();
    let patrol = ecs::behavior::Behavior::Patrol {
        point_a: (-30.0, 0.0),
        point_b: (30.0, 15.0),
        speed: 75.0,
        wait_time: 1.25,
    };
    world.add_component(&entity, patrol.clone()).ok();

    // The name is EntityData.name on the wire, never a component row, and
    // the tileset handle goes through the texture-path function.
    let scene = world_to_scene_data(&world, "RoundTrip", None, &test_texture_path);
    assert_eq!(scene.entities[0].name.as_deref(), Some("RoundTrip"));
    assert!(
        scene.entities[0]
            .components
            .iter()
            .any(|c| matches!(c, ComponentData::Tilemap { tileset, .. } if tileset == "#texture_5")),
        "tileset handle 5 is written as its path: {:?}",
        scene.entities[0].components
    );
    assert!(
        !scene.entities[0].components.iter().any(|c| matches!(c, ComponentData::Dynamic { component_type, .. } if component_type == "Name")),
        "Name is not a component row: {:?}",
        scene.entities[0].components
    );

    let (loaded, instance) = roundtrip(&world);
    let entity = instance.named_entities["RoundTrip"];
    let transform = loaded.get::<Transform2D>(entity).expect("Transform2D survives");
    assert_eq!(transform.position, Vec2::new(100.0, 200.0));
    assert_eq!(transform.rotation, 0.5);
    assert_eq!(transform.scale, Vec2::new(2.0, 2.0));
    let sprite = loaded.get::<Sprite>(entity).expect("Sprite survives");
    assert_eq!(sprite.texture_handle, 0, "#white resolves to handle 0");
    assert_eq!(sprite.offset, Vec2::new(5.0, 10.0));
    assert_eq!(sprite.color, Vec4::new(0.5, 0.6, 0.7, 1.0));
    assert_eq!(sprite.depth, 5.0);
    assert!(loaded.get::<ecs::behavior::EntityTag>(entity).expect("EntityTag survives").matches("enemy"));
    assert_eq!(as_json(&loaded.get::<ecs::UiLabel>(entity)), as_json(&Some(&label)));
    assert_eq!(as_json(&loaded.get::<ecs::UiPanel>(entity)), as_json(&Some(&panel)));
    assert_eq!(as_json(&loaded.get::<ecs::UiButton>(entity)), as_json(&Some(&button)));
    let camera = loaded.get::<ecs::sprite_components::Camera>(entity).expect("Camera survives");
    assert_eq!(camera.position, Vec2::new(50.0, 60.0));
    assert_eq!(camera.rotation, 0.1);
    assert_eq!(camera.zoom, 2.0);
    assert_eq!(camera.viewport_size, Vec2::new(1920.0, 1080.0));
    assert!(camera.is_main_camera);
    let tilemap = loaded.get::<ecs::Tilemap>(entity).expect("Tilemap survives");
    assert_eq!((tilemap.width, tilemap.height), (3, 2));
    assert_eq!(tilemap.tile_size, 40.0);
    assert_eq!(tilemap.tiles, vec![0, 2, 0, 0, 0, 0]);
    assert_eq!(tilemap.tile_uv_size, Vec2::new(0.25, 0.25));
    assert_eq!(tilemap.depth, -2.0);
    assert_eq!(tilemap.tileset, 0, "#texture_5 resolves through the stub to the white handle");
    assert_eq!(
        as_json(&loaded.get::<ecs::behavior::Behavior>(entity)),
        as_json(&Some(&patrol)),
        "Patrol round-trips every field"
    );
}

#[test]
fn physics_settings_survive_serialize_then_parse() {
    let world = World::new();
    let settings = PhysicsSettings { gravity: (0.0, -500.0), pixels_per_meter: 50.0, timestep: 1.0 / 120.0 };

    let scene = world_to_scene_data(&world, "Physics", Some(settings), &test_texture_path);
    let parsed = SceneLoader::parse(&serialize_to_ron(&scene).expect("serialize")).expect("parse");

    let physics = parsed.physics.expect("physics block survives");
    assert_eq!(physics.gravity, (0.0, -500.0));
    assert_eq!(physics.pixels_per_meter, 50.0);
    assert_eq!(physics.timestep, 1.0 / 120.0);
}

#[test]
fn save_scene_to_file_writes_a_parseable_file_and_reports_an_unwritable_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Name::new("saved")).ok();
    world.add_component(&entity, Transform2D::new(Vec2::new(1.0, 2.0))).ok();
    let scene = world_to_scene_data(&world, "FileTest", None, &test_texture_path);

    let path = dir.path().join("level.scene.ron");
    super::save_scene_to_file(&scene, &path).expect("writes the file");

    let written = std::fs::read_to_string(&path).expect("file exists");
    let parsed = SceneLoader::parse(&written).expect("the written file parses");
    assert_eq!(parsed.name, "FileTest");
    assert_eq!(parsed.entities[0].name.as_deref(), Some("saved"));

    // An unwritable path (parent is a regular file, so directory creation fails) is an error, not a panic.
    let unwritable_parent = dir.path().join("file_parent");
    std::fs::write(&unwritable_parent, b"not a directory").expect("fixture file");
    let unwritable = unwritable_parent.join("level.scene.ron");
    let err = super::save_scene_to_file(&scene, &unwritable).expect_err("parent is a file");
    assert!(err.contains("Failed to write scene file"), "{err}");
    assert!(!unwritable.exists());
}

#[test]
fn editor_settings_round_trip_and_pre_editor_scenes_read_none() {
    let scene = SceneData {
        name: "Test".to_string(),
        editor: Some(EditorSettings { camera_position: (150.0, -200.0), camera_zoom: 1.5 }),
        ..Default::default()
    };

    let ron_text = serialize_to_ron(&scene).expect("serialize");
    let parsed = SceneLoader::parse(&ron_text).expect("parse");
    assert_eq!(
        parsed.editor,
        Some(EditorSettings { camera_position: (150.0, -200.0), camera_zoom: 1.5 })
    );

    // A scene written before the editor block existed still loads.
    let legacy = SceneLoader::parse(r#"SceneData(name: "Old Scene", entities: [])"#)
        .expect("pre-editor scene parses");
    assert_eq!(legacy.editor, None);
}

#[test]
fn loaded_names_survive_load_then_save_then_save() {
    // The loader attaches a Name component for every named entity, which is
    // what the serializer reads back into EntityData.name — so a scene that
    // was loaded, saved and saved again keeps its names the whole way.
    let (world, instance) = load_ron(
        r#"SceneData(name: "Named", entities: [EntityData(name: Some("hero"), components: [Transform2D(position: (1.0, 2.0))])])"#,
    );
    let hero = instance.named_entities["hero"];
    assert_eq!(world.get::<Name>(hero).map(|n| n.0.as_str()), Some("hero"));

    let (reloaded, _) = roundtrip(&world);
    let saved_again = SceneLoader::parse(&save_to_ron(&reloaded)).expect("second save parses");

    assert_eq!(saved_again.entities.len(), 1);
    assert_eq!(saved_again.entities[0].name.as_deref(), Some("hero"));
}

#[test]
fn sprite_extraction_writes_all_nine_fields() {
    let mut world = World::new();
    let entity = world.create_entity();
    world
        .add_component(
            &entity,
            Sprite {
                texture_handle: 5,
                offset: Vec2::new(1.0, 2.0),
                rotation: 0.5,
                scale: Vec2::new(3.0, 4.0),
                color: Vec4::new(1.0, 0.0, 0.0, 1.0),
                depth: 10.0,
                visible: false,
                emissive: 0.9,
                tex_region: [0.25, 0.5, 0.25, 0.5],
            },
        )
        .ok();

    let components = saved_components(&world);

    assert_eq!(components.len(), 1);
    let ComponentData::Sprite { texture, offset, rotation, scale, color, depth, emissive, tex_region, visible } =
        &components[0]
    else {
        panic!("expected Sprite, got {:?}", components[0]);
    };
    assert_eq!(
        (texture.as_str(), *offset, *rotation, *scale, *color, *depth, *emissive, *tex_region, *visible),
        ("#texture_5", (1.0, 2.0), 0.5, (3.0, 4.0), (1.0, 0.0, 0.0, 1.0), 10.0, 0.9, (0.25, 0.5, 0.25, 0.5), false)
    );
}

#[test]
fn rigid_body_extraction_covers_every_body_type() {
    let dynamic = RigidBody::new_dynamic()
        .with_velocity(Vec2::new(10.0, 20.0))
        .with_angular_velocity(0.5)
        .with_gravity_scale(0.8)
        .with_linear_damping(5.0)
        .with_angular_damping(1.0)
        .with_rotation_locked(true)
        .with_ccd(true);
    let cases = [
        (dynamic, RigidBodyTypeData::Dynamic),
        (RigidBody::new_static(), RigidBodyTypeData::Static),
        (RigidBody::new_kinematic(), RigidBodyTypeData::Kinematic),
    ];

    for (body, expected_type) in cases {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(&entity, body).ok();

        let components = saved_components(&world);

        let ComponentData::RigidBody {
            body_type,
            velocity,
            angular_velocity,
            gravity_scale,
            linear_damping,
            angular_damping,
            can_rotate,
            ccd_enabled,
        } = &components[0]
        else {
            panic!("expected RigidBody, got {:?}", components[0]);
        };
        assert_eq!(*body_type, expected_type);
        if expected_type == RigidBodyTypeData::Dynamic {
            assert_eq!(*velocity, (10.0, 20.0));
            assert_eq!(*angular_velocity, 0.5);
            assert_eq!(*gravity_scale, 0.8);
            assert_eq!(*linear_damping, 5.0);
            assert_eq!(*angular_damping, 1.0);
            assert!(!*can_rotate, "rotation lock writes can_rotate: false");
            assert!(*ccd_enabled);
        }
    }
}

#[test]
fn collider_extraction_covers_circle_and_box_shapes() {
    let circle = Collider::new(ColliderShape::Circle { radius: 25.0 })
        .with_offset(Vec2::new(5.0, 10.0))
        .as_sensor()
        .with_friction(0.3)
        .with_restitution(0.7);
    let square = Collider::new(ColliderShape::Box { half_extents: Vec2::new(40.0, 20.0) });
    fn is_circle_25(shape: &ColliderShapeData) -> bool {
        matches!(shape, ColliderShapeData::Circle { radius } if *radius == 25.0)
    }
    fn is_box_40_20(shape: &ColliderShapeData) -> bool {
        matches!(shape, ColliderShapeData::Box { half_extents } if *half_extents == (40.0, 20.0))
    }
    type ShapeCheck = fn(&ColliderShapeData) -> bool;
    let cases: [(Collider, ShapeCheck, (f32, f32), bool); 2] = [
        (circle, is_circle_25, (5.0, 10.0), true),
        (square, is_box_40_20, (0.0, 0.0), false),
    ];

    for (collider, shape_matches, expected_offset, expected_sensor) in cases {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(&entity, collider).ok();

        let components = saved_components(&world);

        let ComponentData::Collider { shape, offset, is_sensor, friction, restitution } = &components[0]
        else {
            panic!("expected Collider, got {:?}", components[0]);
        };
        assert!(shape_matches(shape), "shape written as {shape:?}");
        assert_eq!(*offset, expected_offset);
        assert_eq!(*is_sensor, expected_sensor);
        if expected_sensor {
            assert_eq!(*friction, 0.3);
            assert_eq!(*restitution, 0.7);
        }
    }
}

#[test]
fn hierarchy_saves_roots_at_top_level_with_children_nested() {
    let mut world = World::new();
    let grandparent = world.create_entity();
    let parent = world.create_entity();
    let child = world.create_entity();
    world.add_component(&grandparent, Name::new("GP")).ok();
    world.add_component(&parent, Name::new("P")).ok();
    world.add_component(&child, Name::new("C")).ok();
    world.add_component(&child, Transform2D::new(Vec2::new(30.0, 40.0))).ok();
    world.set_parent(parent, grandparent).expect("parent under grandparent");
    world.set_parent(child, parent).expect("child under parent");

    let scene = world_to_scene_data(&world, "Hierarchy", None, &test_texture_path);

    assert_eq!(scene.entities.len(), 1, "only roots at the top level");
    let root = &scene.entities[0];
    assert_eq!(root.name.as_deref(), Some("GP"));
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].name.as_deref(), Some("P"));
    let leaf = &root.children[0].children[0];
    assert_eq!(leaf.name.as_deref(), Some("C"));
    assert_eq!(leaf.components.len(), 1, "a nested child keeps its own components");
    let ComponentData::Transform2D { position, .. } = &leaf.components[0] else {
        panic!("expected Transform2D, got {:?}", leaf.components[0]);
    };
    assert_eq!(*position, (30.0, 40.0));
}

#[test]
fn derived_global_transform_never_reaches_the_wire() {
    // The save pipeline is a whitelist: computed state is not authored.
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, ecs::hierarchy::GlobalTransform2D::default()).ok();

    let components = saved_components(&world);

    assert_eq!(components.len(), 0, "wrote {components:?}");
}

#[test]
fn every_root_entity_is_saved_in_a_stable_order() {
    let mut world = World::new();
    for name in ["First", "Second", "Third"] {
        let entity = world.create_entity();
        world.add_component(&entity, Name::new(name)).ok();
    }

    let names = |world: &World| -> Vec<Option<String>> {
        world_to_scene_data(world, "Multi", None, &test_texture_path)
            .entities
            .iter()
            .map(|e| e.name.clone())
            .collect()
    };
    let first_save = names(&world);

    assert_eq!(
        first_save,
        vec![
            Some("First".to_string()),
            Some("Second".to_string()),
            Some("Third".to_string())
        ]
    );
    assert_eq!(names(&world), first_save, "a second save diffs clean against the first");
}

#[test]
fn grid_backdrop_round_trips_every_field_and_parses_bare() {
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

    let (loaded, instance) = roundtrip(&world);
    let loaded_entity = instance.entities[0];
    assert_eq!(loaded.get::<ecs::GridBackdrop>(loaded_entity), Some(&authored));
    assert_eq!(
        loaded.get::<Transform2D>(loaded_entity).map(|t| t.position),
        Some(Vec2::new(10.0, 20.0))
    );

    // `GridBackdrop()` alone means the playfield preset.
    let (bare_world, bare) = load_ron(
        r#"SceneData(name: "g", entities: [EntityData(name: Some("backdrop"), components: [GridBackdrop()])])"#,
    );
    assert_eq!(
        bare_world.get::<ecs::GridBackdrop>(bare.entities[0]),
        Some(&ecs::GridBackdrop::default())
    );
}
