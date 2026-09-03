// NOTE: the global registry is PROCESS-WIDE and never reset (there is no
// prod reset, so tests get none either). Every
// test-registered type across all crates must use a globally unique name;
// a collision panics loudly by design.
use super::*;
use crate::World;
use serde_json::json;

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
fn test_insert_extract_remove_round_trip_on_a_world() -> Result<(), String> {
    // The heart of #43: a component reaches a World and comes back out
    // through nothing but its registered name.
    let mut registry = ComponentRegistry::new();
    registry.register::<TestComponent>();
    assert_eq!(
        registry.name_for(std::any::TypeId::of::<TestComponent>()),
        Some("TestComponent"),
        "the name maps back from the concrete TypeId"
    );
    let mut world = World::new();
    let entity = world.create_entity();

    registry.insert_component(&mut world, entity, "TestComponent", json!({"value": 7.5, "name": "dyn"}))?;
    assert!(registry.has_component(&world, entity, "TestComponent"));
    assert_eq!(world.get::<TestComponent>(entity).map(|c| c.value), Some(7.5));

    let extracted = registry
        .extract_component(&world, entity, "TestComponent")?
        .expect("present on the entity");
    assert_eq!(extracted, json!({"value": 7.5, "name": "dyn"}));

    assert!(registry.remove_component(&mut world, entity, "TestComponent"));
    assert!(!registry.has_component(&world, entity, "TestComponent"));
    assert_eq!(registry.extract_component(&world, entity, "TestComponent"), Ok(None));

    // insert_default is the Add Component popup's path: the type's Default lands.
    registry.insert_default(&mut world, entity, "TestComponent")?;
    assert_eq!(
        registry.extract_component(&world, entity, "TestComponent")?,
        Some(json!({"value": 1.0, "name": ""}))
    );
    Ok(())
}

#[test]
fn test_dynamic_operations_reject_unknown_names_and_malformed_json() {
    let mut registry = ComponentRegistry::new();
    registry.register::<TestComponent>();
    let mut world = World::new();
    let entity = world.create_entity();

    // An unknown name is a typed refusal that names the type, never a panic.
    let unknown = registry.create_component("NonExistent", json!({}));
    assert_eq!(unknown.err().as_deref(), Some("Unknown component type: NonExistent"));
    assert!(registry.insert_component(&mut world, entity, "NonExistent", json!({})).is_err());
    assert!(!registry.has_component(&world, entity, "NonExistent"));
    assert!(!registry.remove_component(&mut world, entity, "NonExistent"));

    // Malformed JSON is refused and leaves NOTHING attached.
    let malformed = registry.insert_component(&mut world, entity, "TestComponent", json!({"value": "not a number"}));
    assert!(malformed.is_err());
    assert!(!registry.has_component(&world, entity, "TestComponent"));
    assert!(world.get::<TestComponent>(entity).is_none());
}

#[test]
#[should_panic(expected = "component name collision")]
fn test_same_name_different_type_registration_panics() {
    // Two different Rust types under one name would deserialize saved
    // scenes into the wrong type — fail fast at registration.
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
    registry.register::<TestComponent>(); // the SAME type again is a harmless no-op
    assert!(registry.is_registered("TestComponent"));

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
    assert!(registry.is_registered("TestComponent"), "transient still means editable");
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
fn test_late_registration_into_global_is_visible() -> Result<(), String> {
    // The global registry accepts registrations after init, which
    // is how games register their own components in main().
    register_components(|r| r.register::<GameHealth>());
    assert!(with_global_registry(|r| r.is_registered("GameHealth")));

    // A game component round-trips through the GLOBAL registry too.
    let mut world = World::new();
    let entity = world.create_entity();
    with_global_registry(|r| {
        r.insert_component(&mut world, entity, "GameHealth", json!({"value": 25.0, "max": 50.0}))
    })?;
    assert_eq!(world.get::<GameHealth>(entity).map(|h| h.max), Some(50.0));
    Ok(())
}

#[test]
fn test_global_registry_has_builtin_components() {
    // (name, persisted): the persisted flag is asserted BOTH ways so a
    // builtin silently flipping between saved and transient fails here.
    let roster = [
        ("Transform2D", true),
        ("Sprite", true),
        ("SpriteAnimation", true),
        ("Camera", true),
        ("Name", true),
        ("Tilemap", true),
        ("AudioSource", true),
        ("AudioListener", true),
        ("UiLabel", true),
        ("UiPanel", true),
        ("UiButton", true),
        ("Scripts", true),
        ("GridBackdrop", true),
        // Behavior/EntityTag are registered so a GAME reusing those names
        // panics at startup instead of the scene serializer's skip arms
        // silently eating its data.
        ("Behavior", true),
        ("EntityTag", true),
        // PlaySoundEffect is a one-shot request: editable, never saved.
        ("PlaySoundEffect", false),
        // System-computed, never saved.
        ("GlobalTransform2D", false),
    ];

    with_global_registry(|registry| {
        let persistent = registry.persistent_names();
        for (name, persisted) in roster {
            assert!(registry.is_registered(name), "missing builtin {name}");
            assert_eq!(registry.is_persisted(name), persisted, "{name} persisted flag");
            assert_eq!(persistent.contains(&name), persisted, "{name} in persistent_names");
        }
    });
}

#[test]
fn test_every_persistent_builtin_round_trips_through_its_json_wire() -> Result<(), String> {
    // The inspector and the command API move components as JSON by name:
    // every persisted type's default must extract and re-insert unchanged.
    let mut world = World::new();
    let source = world.create_entity();
    let copy = world.create_entity();

    with_global_registry(|registry| {
        for name in registry.persistent_names() {
            registry.insert_default(&mut world, source, name)?;
            let extracted = registry
                .extract_component(&world, source, name)?
                .ok_or_else(|| format!("{name} vanished after insert_default"))?;

            registry.insert_component(&mut world, copy, name, extracted.clone())?;
            let re_extracted = registry.extract_component(&world, copy, name)?;

            assert_eq!(re_extracted, Some(extracted), "{name} must survive extract -> insert");
        }
        Ok(())
    })
}

#[test]
fn test_reentrant_global_access_panics_with_clear_message() {
    // A nested lock acquisition must panic loudly instead of
    // deadlocking the RwLock.
    let result = std::panic::catch_unwind(|| {
        with_global_registry(|_| with_global_registry(|r| r.is_registered("Transform2D")))
    });

    let err = result.expect_err("nested access must panic");
    let message = err.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(
        message.contains("re-entrant global component-registry access"),
        "unexpected panic message: {message}"
    );
}

#[test]
fn test_global_registry_recovers_from_a_poisoned_lock() {
    // A panic inside a registry closure must not brick every
    // later scene load in the process. Poison the lock deliberately...
    let _ = std::panic::catch_unwind(|| {
        register_components(|_| panic!("boom during registration"));
    });

    // ...and the registry still answers, and still accepts registrations.
    assert!(with_global_registry(|r| r.is_registered("Transform2D")));
    register_components(|r| r.register::<TestComponent>());
    assert!(with_global_registry(|r| r.is_registered("TestComponent")));
}
