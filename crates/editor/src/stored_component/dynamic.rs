//! The editor's bridge to the ecs dynamic component registry (issue #43).
//!
//! Everything the typed `editor_component_registry!` macro does NOT cover
//! falls through to here by name: game-registered components (and the audio
//! components' future siblings) get capture/restore, add/remove, snapshot
//! and clipboard survival, and command-API access — with a read-only
//! serde inspection in place of typed field editors.

use std::any::TypeId;

use ecs::{EntityId, World};

use super::registered_typed_component_type_ids;
use super::StoredComponent;

/// Registered dynamic type names NOT covered by the typed registry, sorted.
///
/// Includes transient types (they are live components an entity carries —
/// snapshot/clipboard must preserve them; the SCENE serializer separately
/// skips non-persisted names).
pub fn dynamic_component_names() -> Vec<String> {
    let typed: Vec<TypeId> = registered_typed_component_type_ids();
    ecs::with_global_registry(|registry| {
        let mut names: Vec<String> = registry
            .type_names()
            .filter(|name| {
                registry
                    .get_type_id(name)
                    .map(|id| !typed.contains(&id))
                    .unwrap_or(false)
            })
            .map(|name| name.to_string())
            .collect();
        names.sort_unstable();
        names
    })
}

/// The `TypeId`s of every dynamic (non-typed) registered component — the
/// snapshot known-set addition that stops `WorldSnapshot` destroying
/// game-registered components on Stop.
pub fn dynamic_component_type_ids() -> Vec<TypeId> {
    let typed: Vec<TypeId> = registered_typed_component_type_ids();
    ecs::with_global_registry(|registry| {
        registry
            .type_names()
            .filter_map(|name| registry.get_type_id(name))
            .filter(|id| !typed.contains(id))
            .collect()
    })
}

/// Append a `StoredComponent::Dynamic` for every dynamic component present
/// on the entity (sorted by name — deterministic capture order).
pub fn capture_dynamic_components(
    world: &World,
    entity: EntityId,
    components: &mut Vec<StoredComponent>,
) {
    for name in dynamic_component_names() {
        let extracted = ecs::with_global_registry(|registry| {
            registry.extract_component(world, entity, &name)
        });
        match extracted {
            Ok(Some(value)) => components.push(StoredComponent::Dynamic { name, value }),
            Ok(None) => {}
            Err(e) => log::error!("dynamic capture of '{name}' failed: {e}"),
        }
    }
}

/// Restore one dynamic stored value onto an entity (the `apply_to` arm).
pub fn apply_dynamic(world: &mut World, entity: EntityId, name: &str, value: &serde_json::Value) {
    let result = ecs::with_global_registry(|registry| {
        registry.insert_component(world, entity, name, value.clone())
    });
    if let Err(e) = result {
        // Restore paths (undo, snapshot) never fail loudly mid-operation;
        // a registry that no longer knows the name is a programmer error.
        log::error!("failed to restore dynamic component '{name}': {e}");
    }
}

/// Whether `name` is a dynamic-tier component (registered, not typed).
pub fn is_dynamic_component(name: &str) -> bool {
    dynamic_component_names().iter().any(|n| n == name)
}

/// Build a `StoredComponent::Dynamic` from JSON, validating the payload
/// against the registered type (the command-API `set` fallthrough).
pub fn stored_dynamic_from_json(
    name: &str,
    value: serde_json::Value,
) -> Result<StoredComponent, String> {
    // NOTE: unknown_component_error itself reads the registry — build it
    // OUTSIDE any with_global_registry closure (re-entrancy panics).
    if !ecs::with_global_registry(|r| r.is_registered(name)) {
        return Err(unknown_component_error(name));
    }
    // Validate by constructing the concrete type once; store the JSON.
    ecs::with_global_registry(|r| r.create_component(name, value.clone()))
        .map_err(|e| format!("bad {name} value: {e}"))?;
    Ok(StoredComponent::Dynamic {
        name: name.to_string(),
        value,
    })
}

/// Capture one dynamic component by name (`Ok(None)` = not present).
pub fn capture_dynamic_by_name(
    world: &World,
    entity: EntityId,
    name: &str,
) -> Result<Option<StoredComponent>, String> {
    if !ecs::with_global_registry(|r| r.is_registered(name)) {
        return Err(unknown_component_error(name));
    }
    let value = ecs::with_global_registry(|r| r.extract_component(world, entity, name))?;
    Ok(value.map(|value| StoredComponent::Dynamic {
        name: name.to_string(),
        value,
    }))
}

/// Attach a default-constructed dynamic component.
pub fn add_dynamic_default(world: &mut World, entity: EntityId, name: &str) -> Result<(), String> {
    ecs::with_global_registry(|registry| registry.insert_default(world, entity, name))
}

/// Remove a dynamic component (returns whether anything was removed).
pub fn remove_dynamic(world: &mut World, entity: EntityId, name: &str) -> bool {
    ecs::with_global_registry(|registry| registry.remove_component(world, entity, name))
}

/// Whether the entity carries the named dynamic component.
pub fn has_dynamic(world: &World, entity: EntityId, name: &str) -> bool {
    ecs::with_global_registry(|registry| registry.has_component(world, entity, name))
}

/// Serialize one present dynamic component (for `capture_all_values`).
pub fn dynamic_value(world: &World, entity: EntityId, name: &str) -> Option<serde_json::Value> {
    ecs::with_global_registry(|registry| {
        registry
            .extract_component(world, entity, name)
            .ok()
            .flatten()
    })
}

/// Dynamic components present on the entity, sorted by name.
pub fn dynamic_components_on(world: &World, entity: EntityId) -> Vec<String> {
    dynamic_component_names()
        .into_iter()
        .filter(|name| has_dynamic(world, entity, name))
        .collect()
}

/// Dynamic components NOT on the entity — the add-popup "Game" section.
pub fn available_dynamic_components(world: &World, entity: EntityId) -> Vec<String> {
    dynamic_component_names()
        .into_iter()
        .filter(|name| !has_dynamic(world, entity, name))
        .collect()
}

/// The "unknown component" error. `settable_component_names` already lists
/// typed AND dynamic names (kimi #43 F3 — no double-append).
pub fn unknown_component_error(name: &str) -> String {
    let known: Vec<String> = super::settable_component_names()
        .iter()
        .map(|n| n.to_string())
        .collect();
    format!("unknown component \"{name}\" — known: {}", known.join(", "))
}
