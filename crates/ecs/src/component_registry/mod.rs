//! Component registry for unified component definitions.
//!
//! This is THE dynamic component tier (widened ARCH-006, issue #43): a
//! name-keyed table of fn pointers — monomorphized at [`ComponentRegistry::register`]
//! time — that can create, insert, extract, remove, and default-construct a
//! component on a [`World`](crate::World) without the caller knowing its
//! concrete type. Scene save/load, the editor's snapshot/clipboard/inspector,
//! and the command API all reach game-registered components through it.
//!
//! Downstream crates (physics, games) register their components via
//! [`register_components`] at startup; the editor's typed
//! `editor_component_registry!` remains a widget overlay on top of this tier.
//!
//! Components with load-time resolve logic (texture references, `.sheet.ron`
//! sidecars, physics shape enums) keep their concrete `ComponentData` wire
//! variants — a pure serde path cannot express that resolution. Dynamic
//! components must NOT store raw `EntityId` references: entity ids are not
//! stable across save/load; entity references belong to the concrete wire
//! tier (`Behavior`'s `target_name`, `Scripts`' name-keyed Entity params).

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use crate::entity::EntityId;
use crate::world::World;

/// Factory function type for creating components from JSON
pub type ComponentFactoryFn = fn(serde_json::Value) -> Result<Box<dyn Any + Send + Sync>, String>;

/// Metadata about a component type for editor inspection and serialization
pub trait ComponentMeta: Send + Sync + 'static {
    /// The component's display name (e.g., "Transform2D")
    fn type_name() -> &'static str
    where
        Self: Sized;

    /// Field names for editor inspection
    fn field_names() -> &'static [&'static str]
    where
        Self: Sized;
}

/// One registered component type: everything the dynamic tier can do with
/// it, captured as monomorphized fn pointers at registration time. No
/// type-erased `World` storage is needed — each pointer downcasts (or
/// deserializes) to the concrete `T` and uses the ordinary generic API.
struct ComponentEntry {
    type_id: TypeId,
    /// Written to scene files on save? Transient components (one-shot
    /// requests like `PlaySoundEffect`) are editable but never persisted.
    persist: bool,
    create: ComponentFactoryFn,
    insert: fn(&mut World, EntityId, serde_json::Value) -> Result<(), String>,
    extract: fn(&World, EntityId) -> Option<Result<serde_json::Value, String>>,
    remove: fn(&mut World, EntityId) -> bool,
    has: fn(&World, EntityId) -> bool,
    default_value: fn() -> serde_json::Value,
}

/// Runtime registry for component type lookup by name
pub struct ComponentRegistry {
    entries: HashMap<&'static str, ComponentEntry>,
}

impl ComponentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Register a persisted component type (saved to scene files as
    /// `ComponentData::Dynamic` unless it has a concrete wire variant).
    ///
    /// Re-registering the same type under the same name is a no-op;
    /// registering a DIFFERENT type under an existing name panics — a
    /// name collision would deserialize saved data into the wrong type,
    /// silently corrupting scenes (fail fast at startup instead).
    pub fn register<T>(&mut self)
    where
        T: ComponentMeta
            + serde::Serialize
            + serde::de::DeserializeOwned
            + Default
            + Send
            + Sync
            + 'static,
    {
        self.register_inner::<T>(true);
    }

    /// Register a component type that is editable/inspectable but never
    /// written to scene files (one-shot requests like `PlaySoundEffect`).
    pub fn register_transient<T>(&mut self)
    where
        T: ComponentMeta
            + serde::Serialize
            + serde::de::DeserializeOwned
            + Default
            + Send
            + Sync
            + 'static,
    {
        self.register_inner::<T>(false);
    }

    fn register_inner<T>(&mut self, persist: bool)
    where
        T: ComponentMeta
            + serde::Serialize
            + serde::de::DeserializeOwned
            + Default
            + Send
            + Sync
            + 'static,
    {
        let name = T::type_name();
        if let Some(existing) = self.entries.get(name) {
            if existing.type_id == TypeId::of::<T>() {
                return; // idempotent re-registration
            }
            panic!(
                "component name collision: '{name}' is already registered for a \
                 different type — two crates registering different Rust types \
                 under one name would corrupt saved scenes on load"
            );
        }
        self.entries.insert(
            name,
            ComponentEntry {
                type_id: TypeId::of::<T>(),
                persist,
                create: |json| {
                    serde_json::from_value::<T>(json)
                        .map(|c| Box::new(c) as Box<dyn Any + Send + Sync>)
                        .map_err(|e| e.to_string())
                },
                insert: |world, entity, json| {
                    let component =
                        serde_json::from_value::<T>(json).map_err(|e| e.to_string())?;
                    world
                        .add_component(&entity, component)
                        .map_err(|e| e.to_string())
                },
                extract: |world, entity| {
                    world
                        .get::<T>(entity)
                        .map(|c| serde_json::to_value(c).map_err(|e| e.to_string()))
                },
                remove: |world, entity| world.remove_component::<T>(&entity).is_ok(),
                has: |world, entity| world.get::<T>(entity).is_some(),
                default_value: || {
                    serde_json::to_value(T::default()).unwrap_or(serde_json::Value::Null)
                },
            },
        );
    }

    /// Check if a component type is registered
    pub fn is_registered(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Get TypeId for a component name
    pub fn get_type_id(&self, name: &str) -> Option<TypeId> {
        self.entries.get(name).map(|e| e.type_id)
    }

    /// The registered name for a TypeId, if any.
    pub fn name_for(&self, type_id: TypeId) -> Option<&'static str> {
        self.entries
            .iter()
            .find(|(_, e)| e.type_id == type_id)
            .map(|(name, _)| *name)
    }

    /// Get all registered type names
    pub fn type_names(&self) -> impl Iterator<Item = &&'static str> {
        self.entries.keys()
    }

    /// Names of all PERSISTED registered types, sorted (stable scene diffs).
    pub fn persistent_names(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, e)| e.persist)
            .map(|(name, _)| *name)
            .collect();
        names.sort_unstable();
        names
    }

    /// Whether a registered type is written to scene files on save.
    pub fn is_persisted(&self, name: &str) -> bool {
        self.entries.get(name).map(|e| e.persist).unwrap_or(false)
    }

    /// Create a component by name from JSON
    pub fn create_component(
        &self,
        name: &str,
        json: serde_json::Value,
    ) -> Result<Box<dyn Any + Send + Sync>, String> {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| format!("Unknown component type: {}", name))?;
        (entry.create)(json)
    }

    /// Deserialize + attach a component by name onto an entity.
    pub fn insert_component(
        &self,
        world: &mut World,
        entity: EntityId,
        name: &str,
        json: serde_json::Value,
    ) -> Result<(), String> {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| format!("Unknown component type: {}", name))?;
        (entry.insert)(world, entity, json)
    }

    /// Attach a default-constructed component by name onto an entity.
    pub fn insert_default(
        &self,
        world: &mut World,
        entity: EntityId,
        name: &str,
    ) -> Result<(), String> {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| format!("Unknown component type: {}", name))?;
        (entry.insert)(world, entity, (entry.default_value)())
    }

    /// Serialize an entity's component by name. `Ok(None)` = the entity
    /// doesn't carry it.
    pub fn extract_component(
        &self,
        world: &World,
        entity: EntityId,
        name: &str,
    ) -> Result<Option<serde_json::Value>, String> {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| format!("Unknown component type: {}", name))?;
        (entry.extract)(world, entity).transpose()
    }

    /// Whether the entity carries the named component (false for unknown names).
    pub fn has_component(&self, world: &World, entity: EntityId, name: &str) -> bool {
        self.entries
            .get(name)
            .map(|e| (e.has)(world, entity))
            .unwrap_or(false)
    }

    /// Remove the named component from an entity. Returns whether anything
    /// was removed (false for unknown names too).
    pub fn remove_component(&self, world: &mut World, entity: EntityId, name: &str) -> bool {
        self.entries
            .get(name)
            .map(|e| (e.remove)(world, entity))
            .unwrap_or(false)
    }

    /// The default value of a registered type as JSON (for editor add flows).
    pub fn default_value(&self, name: &str) -> Option<serde_json::Value> {
        self.entries.get(name).map(|e| (e.default_value)())
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global registry of component types, behind an RwLock so downstream
/// crates (physics, games) can register at startup — ecs GPP-16.
static COMPONENT_REGISTRY: OnceLock<RwLock<ComponentRegistry>> = OnceLock::new();

thread_local! {
    /// Re-entrancy guard: `std::sync::RwLock` deadlocks on same-thread
    /// read→write (and may on read→read); catch it with a clear panic
    /// instead of a hang (kimi round-2 F9/F1).
    static REGISTRY_LOCK_HELD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

struct ReentrancyGuard;

impl ReentrancyGuard {
    fn acquire(context: &str) -> Self {
        REGISTRY_LOCK_HELD.with(|held| {
            if held.get() {
                panic!(
                    "re-entrant global component-registry access ({context}): never call \
                     register_components or with_global_registry from inside a registry \
                     closure — the RwLock would deadlock"
                );
            }
            held.set(true);
        });
        ReentrancyGuard
    }
}

impl Drop for ReentrancyGuard {
    fn drop(&mut self) {
        REGISTRY_LOCK_HELD.with(|held| held.set(false));
    }
}

fn global() -> &'static RwLock<ComponentRegistry> {
    COMPONENT_REGISTRY.get_or_init(|| {
        let mut registry = ComponentRegistry::new();

        // Register built-in ECS components
        use crate::audio_components::{AudioListener, AudioSource, PlaySoundEffect};
        use crate::sprite_components::{Camera, Name, Sprite, SpriteAnimation, Transform2D};
        use crate::tilemap::Tilemap;
        use crate::ui_components::{UiButton, UiLabel, UiPanel};
        registry.register::<Transform2D>();
        registry.register::<Sprite>();
        registry.register::<SpriteAnimation>();
        registry.register::<Camera>();
        registry.register::<Name>();
        registry.register::<Tilemap>();
        registry.register::<AudioSource>();
        registry.register::<AudioListener>();
        // One-shot request, editable but never saved to a scene file.
        registry.register_transient::<PlaySoundEffect>();
        registry.register::<UiLabel>();
        registry.register::<UiPanel>();
        registry.register::<UiButton>();
        // Registered so a GAME reusing these names hits the collision panic
        // at startup instead of the scene serializer's concrete/skip arms
        // silently eating its data (kimi #43 F1). Behavior/EntityTag persist
        // through their concrete wire arms; GlobalTransform2D is
        // system-computed and never persisted.
        registry.register::<crate::behavior::Behavior>();
        registry.register::<crate::behavior::EntityTag>();
        registry.register_transient::<crate::hierarchy::GlobalTransform2D>();
        // The scripting seam Stage 1 (#44): inert data, persisted through
        // its concrete ComponentData::Scripts wire arm (name-mapped Entity
        // params), so the serializer's skip list covers it like Behavior.
        registry.register::<crate::script::Scripts>();

        RwLock::new(registry)
    })
}

/// Run `f` with read access to the global registry.
///
/// NEVER call [`register_components`] (or this function) from inside the
/// closure — same-thread lock re-entry panics (by design, instead of a
/// silent RwLock deadlock). Lock poisoning is recovered: registration is
/// idempotent inserts, so state after a panicking closure stays usable.
pub fn with_global_registry<R>(f: impl FnOnce(&ComponentRegistry) -> R) -> R {
    let _guard = ReentrancyGuard::acquire("with_global_registry");
    let lock = global();
    let registry = lock.read().unwrap_or_else(|poison| poison.into_inner());
    f(&registry)
}

/// Register component types into the global registry — callable at any
/// time (games call it in `main()` before `run_game`; the engine calls it
/// for its own types). Idempotent per type; a same-name/different-type
/// registration panics (see [`ComponentRegistry::register`]).
pub fn register_components(f: impl FnOnce(&mut ComponentRegistry)) {
    let _guard = ReentrancyGuard::acquire("register_components");
    let lock = global();
    let mut registry = lock.write().unwrap_or_else(|poison| poison.into_inner());
    f(&mut registry);
}

/// Simple macro for defining components with standard derives
#[macro_export]
macro_rules! define_component {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                pub $field:ident : $type:ty = $default:expr
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct $name {
            $(
                $(#[$field_meta])*
                pub $field: $type,
            )*
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $( $field: $default, )*
                }
            }
        }

        impl $crate::component_registry::ComponentMeta for $name {
            fn type_name() -> &'static str {
                stringify!($name)
            }

            fn field_names() -> &'static [&'static str] {
                &[ $( stringify!($field), )* ]
            }
        }
    };
}

#[cfg(test)]
mod tests;
