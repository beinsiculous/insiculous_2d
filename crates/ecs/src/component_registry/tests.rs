// NOTE: the global registry is PROCESS-WIDE and never reset (there is no
// prod reset, so tests get none either — kimi #43 F4 adjudication). Every
// test-registered type across all crates must use a globally unique name;
// a collision panics loudly by design.
use super::*;
use crate::World;

define_component! {
    /// Test component for unit tests
    pub struct TestComponent {
        pub value: f32 = 1.0,
        pub name: String = String::new(),
    }
}

define_component! {
    /// A second test type for collision/registration tests
    pub struct GameHealth {
        pub value: f32 = 100.0,
        pub max: f32 = 100.0,
    }
}

#[test]
fn test_define_component_creates_struct() {
    let component = TestComponent::default();
    assert_eq!(component.value, 1.0);
    assert_eq!(component.name, "");
}

#[test]
fn test_component_meta_type_name_and_fields() {
    assert_eq!(TestComponent::type_name(), "TestComponent");
    assert_eq!(TestComponent::field_names(), &["value", "name"]);
}

#[test]
fn test_registry_register_and_lookup() {
    let mut registry = ComponentRegistry::new();
    registry.register::<TestComponent>();

    assert!(registry.is_registered("TestComponent"));
    assert!(!registry.is_registered("NonExistent"));
    assert_eq!(
        registry.name_for(std::any::TypeId::of::<TestComponent>()),
        Some("TestComponent")
    );
}

#[test]
fn test_component_factory_creates_from_json() {
    use serde_json::json;

    let mut registry = ComponentRegistry::new();
    registry.register::<TestComponent>();

    let component = registry
        .create_component("TestComponent", json!({"value": 42.0, "name": "factory_test"}))
        .expect("factory should build from JSON");
    let test_component = component.downcast_ref::<TestComponent>().expect("downcast");
    assert_eq!(test_component.value, 42.0);
    assert_eq!(test_component.name, "factory_test");
}

#[test]
fn test_component_factory_unknown_type() {
    let registry = ComponentRegistry::new();

    let result = registry.create_component("NonExistent", serde_json::json!({}));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown component type"));
}

#[test]
fn test_insert_extract_remove_round_trip_on_a_world() {
    // The heart of #43: a component reaches a World and comes back out
    // through nothing but its registered name.
    use serde_json::json;

    let mut registry = ComponentRegistry::new();
    registry.register::<TestComponent>();
    let mut world = World::new();
    let entity = world.create_entity();

    registry
        .insert_component(&mut world, entity, "TestComponent", json!({"value": 7.5, "name": "dyn"}))
        .expect("insert");
    assert!(registry.has_component(&world, entity, "TestComponent"));
    assert_eq!(world.get::<TestComponent>(entity).map(|c| c.value), Some(7.5));

    let value = registry
        .extract_component(&world, entity, "TestComponent")
        .expect("known type")
        .expect("present on entity");
    assert_eq!(value["value"], 7.5);
    assert_eq!(value["name"], "dyn");

    assert!(registry.remove_component(&mut world, entity, "TestComponent"));
    assert!(!registry.has_component(&world, entity, "TestComponent"));
    assert_eq!(
        registry.extract_component(&world, entity, "TestComponent"),
        Ok(None)
    );
}

#[test]
fn test_insert_default_attaches_default_value() {
    let mut registry = ComponentRegistry::new();
    registry.register::<GameHealth>();
    let mut world = World::new();
    let entity = world.create_entity();

    registry
        .insert_default(&mut world, entity, "GameHealth")
        .expect("insert default");
    let health = world.get::<GameHealth>(entity).expect("attached");
    assert_eq!(health.value, 100.0);
    assert_eq!(health.max, 100.0);
}

#[test]
fn test_insert_rejects_malformed_json() {
    let mut registry = ComponentRegistry::new();
    registry.register::<TestComponent>();
    let mut world = World::new();
    let entity = world.create_entity();

    let result = registry.insert_component(
        &mut world,
        entity,
        "TestComponent",
        serde_json::json!({"value": "not a number"}),
    );
    assert!(result.is_err());
    assert!(!registry.has_component(&world, entity, "TestComponent"));
}

#[test]
fn test_reregistration_is_a_noop() {
    let mut registry = ComponentRegistry::new();
    registry.register::<TestComponent>();
    registry.register::<TestComponent>(); // must not panic
    assert!(registry.is_registered("TestComponent"));
}

#[test]
#[should_panic(expected = "component name collision")]
fn test_same_name_different_type_registration_panics() {
    // Two different Rust types under one name would deserialize saved
    // scenes into the wrong type — fail fast at registration (kimi R2-F3).
    mod imposter {
        crate::define_component! {
            /// Same NAME as the outer TestComponent, different type.
            pub struct TestComponent {
                pub other: bool = false,
            }
        }
    }
    let mut registry = ComponentRegistry::new();
    registry.register::<TestComponent>();
    registry.register::<imposter::TestComponent>();
}

#[test]
fn test_transient_types_are_excluded_from_persistent_names() {
    let mut registry = ComponentRegistry::new();
    registry.register::<GameHealth>();
    registry.register_transient::<TestComponent>();

    let names = registry.persistent_names();
    assert!(names.contains(&"GameHealth"));
    assert!(!names.contains(&"TestComponent"));
    assert!(registry.is_persisted("GameHealth"));
    assert!(!registry.is_persisted("TestComponent"));
}

#[test]
fn test_persistent_names_are_sorted_for_stable_scene_diffs() {
    let mut registry = ComponentRegistry::new();
    registry.register::<TestComponent>();
    registry.register::<GameHealth>();
    let names = registry.persistent_names();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
}

#[test]
fn test_late_registration_into_global_is_visible() {
    // GPP-16: the global registry accepts registrations AFTER init.
    register_components(|r| r.register::<GameHealth>());
    assert!(with_global_registry(|r| r.is_registered("GameHealth")));

    // A game component round-trips through the GLOBAL registry too.
    let mut world = World::new();
    let entity = world.create_entity();
    with_global_registry(|r| {
        r.insert_component(
            &mut world,
            entity,
            "GameHealth",
            serde_json::json!({"value": 25.0, "max": 50.0}),
        )
    })
    .expect("insert via global");
    assert_eq!(world.get::<GameHealth>(entity).map(|h| h.max), Some(50.0));
}

#[test]
fn test_global_registry_has_builtin_components() {
    with_global_registry(|registry| {
        for name in [
            "Transform2D",
            "Sprite",
            "SpriteAnimation",
            "Camera",
            "Name",
            "AudioSource",
            "AudioListener",
            "Tilemap",
            "PlaySoundEffect",
            "UiLabel",
            "UiPanel",
            "UiButton",
        ] {
            assert!(registry.is_registered(name), "missing builtin {name}");
        }
        // PlaySoundEffect is a one-shot request: editable, never saved.
        assert!(!registry.is_persisted("PlaySoundEffect"));
        assert!(registry.is_persisted("AudioSource"));
        // Behavior/EntityTag/GlobalTransform2D are registered so a GAME
        // reusing those names panics at startup instead of the scene
        // serializer's skip arms silently eating its data (kimi #43 F1).
        assert!(registry.is_registered("Behavior"));
        assert!(registry.is_registered("EntityTag"));
        assert!(registry.is_registered("GlobalTransform2D"));
        assert!(!registry.is_persisted("GlobalTransform2D"), "system-computed, never saved");
    });
}

#[test]
fn test_global_registry_creates_audio_source_from_json() {
    use crate::audio_components::AudioSource;
    use serde_json::json;

    let json = json!({
        "sound_id": 7,
        "volume": 0.5,
        "pitch": 1.25,
        "looping": true,
        "play_on_spawn": false,
        "playing": false,
        "spatial": true,
        "max_distance": 800.0,
        "reference_distance": 80.0,
        "rolloff_factor": 2.0
    });

    let component = with_global_registry(|r| r.create_component("AudioSource", json))
        .expect("AudioSource should be creatable from JSON");
    let source = component
        .downcast_ref::<AudioSource>()
        .expect("created component should downcast to AudioSource");

    assert_eq!(source.sound_id, 7);
    assert!((source.volume - 0.5).abs() < f32::EPSILON);
    assert!(source.spatial);
}

#[test]
fn test_reentrant_global_access_panics_with_clear_message() {
    // kimi R2-F9: a nested lock acquisition must panic loudly instead of
    // deadlocking the RwLock.
    let result = std::panic::catch_unwind(|| {
        with_global_registry(|_| {
            with_global_registry(|r| r.is_registered("Transform2D"))
        })
    });
    let err = result.expect_err("nested access must panic");
    let message = err
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_default();
    assert!(
        message.contains("re-entrant global component-registry access"),
        "unexpected panic message: {message}"
    );
}

#[test]
fn test_global_registry_recovers_from_a_poisoned_lock() {
    // kimi R2-F1: a panic inside a registry closure must not brick every
    // later scene load in the process. Poison the lock deliberately...
    let _ = std::panic::catch_unwind(|| {
        register_components(|_| panic!("boom during registration"));
    });
    // ...and the registry still answers.
    assert!(with_global_registry(|r| r.is_registered("Transform2D")));
    register_components(|r| r.register::<TestComponent>());
    assert!(with_global_registry(|r| r.is_registered("TestComponent")));
}

#[test]
fn test_grid_backdrop_is_a_registered_builtin() {
    // #46: create-by-name, dynamic tooling and the editor popup all key on this.
    assert!(with_global_registry(|r| r.is_registered("GridBackdrop")));
}
