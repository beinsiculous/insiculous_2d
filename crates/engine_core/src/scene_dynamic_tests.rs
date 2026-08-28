//! Scene round-trip tests for `ComponentData::Dynamic` (issue #43): the
//! dynamic registry tier reaching RON scene files — including the audio
//! components, which the serializer silently DROPPED before this.

use ecs::audio_components::{AudioListener, AudioSource};
use ecs::sprite_components::Transform2D;
use ecs::World;
use glam::Vec2;

use crate::scene_data::ComponentData;
use crate::scene_loader::SceneLoader;
use crate::scene_serializer::{serialize_to_ron, world_to_scene_data};
use crate::texture_ref::TextureResolver;
use crate::SceneLoadError;

fn test_texture_path(handle: u32) -> String {
    if handle == 0 {
        "#white".to_string()
    } else {
        format!("#texture_{}", handle)
    }
}

struct StubResolver;
impl TextureResolver for StubResolver {
    fn resolve_texture(
        &mut self,
        _texture_ref: &str,
    ) -> Result<renderer::TextureHandle, SceneLoadError> {
        Ok(renderer::TextureHandle { id: 0 })
    }
}

/// world → RON string → parsed → fresh world.
fn roundtrip(world: &World) -> World {
    let scene = world_to_scene_data(world, "DynRoundTrip", None, &test_texture_path);
    let ron_string = serialize_to_ron(&scene).expect("serialize");
    let parsed = SceneLoader::parse(&ron_string).expect("parse");
    let mut loaded = World::new();
    SceneLoader::instantiate(&parsed, &mut loaded, &mut StubResolver).expect("instantiate");
    loaded
}

ecs::define_component! {
    /// Stand-in for a game-registered component (unique name — the global
    /// registry is process-wide across tests).
    pub struct SceneDynSpikeStats {
        pub score: f32 = 0.0,
        pub kills: u32 = 0,
        pub label: String = String::new(),
        pub aim: (f32, f32) = (0.0, 0.0),
    }
}

#[test]
fn test_dynamic_payload_survives_ron_round_trip() {
    // THE load-bearing risk of #43: serde_json::Value → RON → Value must
    // preserve numbers (int and float), strings, and nesting exactly.
    ecs::register_components(|r| r.register::<SceneDynSpikeStats>());

    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(Vec2::ZERO)).ok();
    world
        .add_component(
            &entity,
            SceneDynSpikeStats {
                score: 1234.5,
                kills: 42,
                label: "boss #1 — \"final\"".to_string(),
                aim: (-0.25, 1.0),
            },
        )
        .ok();

    let loaded = roundtrip(&world);
    let entity = loaded.entities()[0];
    let stats = loaded
        .get::<SceneDynSpikeStats>(entity)
        .expect("dynamic component restored from RON");
    assert_eq!(stats.score, 1234.5);
    assert_eq!(stats.kills, 42);
    assert_eq!(stats.label, "boss #1 — \"final\"");
    assert_eq!(stats.aim, (-0.25, 1.0));
}

#[test]
fn test_audio_components_survive_save_load() {
    // Regression for the silent drop: AudioSource/AudioListener were
    // editable and snapshot-captured but never written to scene files.
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(Vec2::new(5.0, 6.0))).ok();
    let source = AudioSource {
        sound_id: 7,
        volume: 0.4,
        spatial: true,
        max_distance: 640.0,
        ..AudioSource::default()
    };
    world.add_component(&entity, source).ok();
    world.add_component(&entity, AudioListener::default()).ok();

    let loaded = roundtrip(&world);
    let entity = loaded.entities()[0];
    let source = loaded.get::<AudioSource>(entity).expect("AudioSource saved");
    assert_eq!(source.sound_id, 7);
    assert!((source.volume - 0.4).abs() < f32::EPSILON);
    assert!(source.spatial);
    assert!((source.max_distance - 640.0).abs() < f32::EPSILON);
    assert!(loaded.get::<AudioListener>(entity).is_some(), "AudioListener saved");
}

#[test]
fn test_transient_components_are_not_saved() {
    use ecs::audio_components::PlaySoundEffect;
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(Vec2::ZERO)).ok();
    world.add_component(&entity, PlaySoundEffect::new(3)).ok();

    let scene = world_to_scene_data(&world, "Transient", None, &test_texture_path);
    let dynamic_types: Vec<_> = scene.entities[0]
        .components
        .iter()
        .filter_map(|c| match c {
            ComponentData::Dynamic { component_type, .. } => Some(component_type.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !dynamic_types.contains(&"PlaySoundEffect".to_string()),
        "one-shot requests must not persist: {dynamic_types:?}"
    );
}

#[test]
fn test_dynamic_emissions_are_name_sorted() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(Vec2::ZERO)).ok();
    world.add_component(&entity, AudioSource::default()).ok();
    world.add_component(&entity, AudioListener::default()).ok();

    let scene = world_to_scene_data(&world, "Sorted", None, &test_texture_path);
    let dynamic_types: Vec<_> = scene.entities[0]
        .components
        .iter()
        .filter_map(|c| match c {
            ComponentData::Dynamic { component_type, .. } => Some(component_type.clone()),
            _ => None,
        })
        .collect();
    let mut sorted = dynamic_types.clone();
    sorted.sort();
    assert_eq!(dynamic_types, sorted, "scene diffs must be stable");
    assert!(dynamic_types.contains(&"AudioListener".to_string()));
    assert!(dynamic_types.contains(&"AudioSource".to_string()));
}

#[test]
fn test_unknown_dynamic_component_fails_the_load_loudly() {
    // kimi R2-F2: silently dropping (then resaving without) a game
    // component would corrupt scenes — an unregistered name refuses the
    // whole load with a clear message instead.
    let ron = r#"SceneData(
        name: "Unknown",
        entities: [
            EntityData(
                components: [
                    Dynamic(type: "NeverRegisteredComponent", data: {"x": 1}),
                ],
            ),
        ],
    )"#;
    let parsed = SceneLoader::parse(ron).expect("parse is schema-level, fine");
    let mut world = World::new();
    let err = SceneLoader::instantiate(&parsed, &mut world, &mut StubResolver)
        .expect_err("unregistered dynamic component must refuse the load");
    let message = format!("{err}");
    assert!(
        message.contains("NeverRegisteredComponent"),
        "error names the component: {message}"
    );
}
