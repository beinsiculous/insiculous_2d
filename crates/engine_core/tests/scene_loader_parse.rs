//! Parse-level contracts of `SceneLoader` (public API only — tests that need
//! the private merge live inline in `scene_loader.rs`): the serde defaults
//! that keep old scene files loading, the tilemap component, the bundled
//! example scenes, and the legacy camera-follow shape.

use ecs::Tilemap;
use engine_core::scene_data::{BehaviorData, ComponentData};
use engine_core::scene_loader::SceneLoader;
use engine_core::test_support::load_ron;
use renderer::texture::TextureHandle;

/// The one Sprite row of a one-entity scene whose sprite carries `fields`.
fn parse_sprite(fields: &str) -> ComponentData {
    let text = format!(
        r##"SceneData(name: "Sprite", entities: [EntityData(name: Some("player"), components: [Sprite(texture: "#white"{fields}), EntityTag(tag: "enemy")])])"##
    );
    let scene = SceneLoader::parse(&text).expect("scene parses");
    assert_eq!(scene.name, "Sprite");
    assert_eq!(scene.entities[0].name.as_deref(), Some("player"));
    assert!(matches!(&scene.entities[0].components[1], ComponentData::EntityTag { tag } if tag == "enemy"));
    scene.entities[0].components[0].clone()
}

#[test]
fn sprite_fields_default_when_omitted_and_parse_when_explicit() {
    // Scenes written before emissive / tex_region / visible existed must load
    // unchanged: unlit, full texture, visible. A plain `#[serde(default)]`
    // on tex_region or visible would render nothing / hide every old sprite.
    let cases = [
        ("", (0.0, (0.0, 0.0, 1.0, 1.0), true)),
        (", emissive: 0.9, tex_region: (0.25, 0.5, 0.25, 0.5), visible: false", (0.9, (0.25, 0.5, 0.25, 0.5), false)),
    ];

    for (fields, expected) in cases {
        let sprite = parse_sprite(fields);

        let ComponentData::Sprite { emissive, tex_region, visible, .. } = sprite else {
            panic!("expected Sprite, got {sprite:?}");
        };
        assert_eq!((emissive, tex_region, visible), expected, "fields: {fields:?}");
    }
}

#[test]
fn tilemap_parses_and_instantiates_with_a_resolved_tileset() {
    let (world, instance) = load_ron(
        r##"SceneData(
            name: "Tilemap Test",
            entities: [EntityData(
                name: Some("level"),
                components: [
                    Transform2D(position: (-160.0, 120.0)),
                    Tilemap(tileset: "#white", width: 3, height: 2, tile_size: 40.0, tiles: [1, 0, 2, 0, 3, 0], tile_uv_size: (0.25, 0.25)),
                ],
            )],
        )"##,
    );

    let tilemap = world.get::<Tilemap>(instance.entities[0]).expect("Tilemap attached");
    assert_eq!((tilemap.width, tilemap.height), (3, 2));
    assert_eq!(tilemap.tileset, TextureHandle::WHITE.id, "the tileset reference resolved to a handle");
    assert_eq!(tilemap.tiles, vec![1, 0, 2, 0, 3, 0]);
    assert_eq!(tilemap.depth, -1.0, "depth defaults behind sprites");
    assert_eq!(tilemap.sprite_instances().count(), 3, "empty tiles draw nothing");
}

#[test]
fn bundled_example_scenes_parse_and_hello_world_follows_its_player() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/assets/scenes/");
    let mut scenes = Vec::new();
    for name in ["hello_world.scene.ron", "behavior_demo.scene.ron"] {
        let text = std::fs::read_to_string(format!("{dir}{name}")).unwrap_or_else(|e| panic!("read {name}: {e}"));
        let scene = SceneLoader::parse(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"));
        assert!(!scene.entities.is_empty(), "{name} has entities");
        scenes.push(scene);
    }

    // hello_world doubles as the editor demo's level: its main camera follows
    // the "player" tag the Player prefab carries.
    let hello_world = &scenes[0];
    let camera = hello_world
        .entities
        .iter()
        .find(|e| e.name.as_deref() == Some("camera"))
        .expect("camera entity present");
    assert!(camera.components.iter().any(|c| matches!(c, ComponentData::Camera2D { is_main_camera: true, .. })));
    assert!(camera.components.iter().any(|c| matches!(
        c,
        ComponentData::Behavior(BehaviorData::CameraFollow { target_tag, .. }) if target_tag == "player"
    )));
    assert!(hello_world.prefabs["Player"]
        .components
        .iter()
        .any(|c| matches!(c, ComponentData::EntityTag { tag } if tag == "player")));
}

#[test]
fn legacy_camera_follow_scene_without_look_ahead_still_parses() {
    // Scenes authored before look-ahead existed keep loading, with the
    // feature defaulted off.
    let scene = SceneLoader::parse(
        r#"SceneData(
            name: "Legacy",
            entities: [EntityData(
                name: Some("camera"),
                components: [
                    Transform2D(position: (0.0, 0.0)),
                    Behavior(CameraFollow(target_tag: "player", lerp_speed: 0.12, offset: (0.0, 60.0), dead_zone: Some((160.0, 100.0)))),
                ],
            )],
        )"#,
    )
    .expect("legacy scene parses");

    let behavior = scene.entities[0]
        .components
        .iter()
        .find_map(|c| match c {
            ComponentData::Behavior(b) => Some(b),
            _ => None,
        })
        .expect("camera behavior present");
    let BehaviorData::CameraFollow { look_ahead, look_ahead_lerp, .. } = behavior else {
        panic!("expected CameraFollow, got {behavior:?}");
    };
    assert_eq!(*look_ahead, (0.0, 0.0), "look-ahead defaults to disabled");
    assert_eq!(*look_ahead_lerp, 0.08);
}

#[test]
fn prefab_entity_instantiates_with_its_overrides_merged_over_the_prefab() {
    // The prefab wire: a prefab table, an entity referencing one by name,
    // and an override layer replacing the prefab's component of the same
    // type while the rest of the prefab still applies.
    let (world, instance) = load_ron(
        r##"SceneData(
            name: "Prefab Test",
            prefabs: {
                "Enemy": PrefabData(
                    components: [
                        Transform2D(position: (0.0, 0.0), scale: (2.0, 2.0)),
                        Sprite(texture: "#white", color: (1.0, 0.0, 0.0, 1.0)),
                        EntityTag(tag: "enemy"),
                    ],
                ),
            },
            entities: [
                EntityData(
                    name: Some("enemy1"),
                    prefab: Some("Enemy"),
                    overrides: [Transform2D(position: (500.0, 100.0))],
                ),
            ],
        )"##,
    );

    let enemy = instance.named_entities["enemy1"];
    let transform = world.get::<ecs::sprite_components::Transform2D>(enemy).expect("Transform2D");
    assert_eq!(transform.position, glam::Vec2::new(500.0, 100.0), "the override replaces the prefab's transform");
    assert_eq!(transform.scale, glam::Vec2::ONE, "replaced whole, not merged field by field");
    let sprite = world.get::<ecs::sprite_components::Sprite>(enemy).expect("prefab Sprite still applies");
    assert_eq!(sprite.color, glam::Vec4::new(1.0, 0.0, 0.0, 1.0));
    assert_eq!(sprite.texture_handle, TextureHandle::WHITE.id);
    assert!(world.get::<ecs::behavior::EntityTag>(enemy).expect("prefab EntityTag still applies").matches("enemy"));
}
