//! Component management for the ECS.
//!
//! This module provides HashMap-based per-type component storage.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::entity::EntityId;

/// A trait for components in the ECS
pub trait Component: Any + Send + Sync {
    /// Get the type name of the component
    fn type_name(&self) -> &'static str;

    /// Get self as Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Get self as mutable Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Any + Send + Sync> Component for T {
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// HashMap-based per-type component storage
#[derive(Default)]
pub struct ComponentStore {
    /// The components, indexed by entity ID
    components: HashMap<EntityId, Box<dyn Component>>,
}

impl ComponentStore {
    /// Create a new component storage
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    /// Add a component for an entity
    pub fn add<T: Component>(&mut self, entity_id: EntityId, component: T) {
        self.components.insert(entity_id, Box::new(component));
    }

    /// Remove a component for an entity
    pub fn remove(&mut self, entity_id: &EntityId) -> Option<Box<dyn Component>> {
        self.components.remove(entity_id)
    }

    /// Get a typed reference to a component for an entity
    pub fn get_typed<T: Component>(&self, entity_id: &EntityId) -> Option<&T> {
        // Use .as_ref() to get &dyn Component before calling as_any(),
        // otherwise the blanket impl on Box<dyn Component> would be called
        // instead of going through virtual dispatch to the concrete type.
        self.components
            .get(entity_id)
            .and_then(|c| c.as_ref().as_any().downcast_ref::<T>())
    }

    /// Get a typed mutable reference to a component for an entity
    pub fn get_typed_mut<T: Component>(&mut self, entity_id: &EntityId) -> Option<&mut T> {
        // Use .as_mut() to get &mut dyn Component before calling as_any_mut()
        self.components
            .get_mut(entity_id)
            .and_then(|c| c.as_mut().as_any_mut().downcast_mut::<T>())
    }

    /// Check if an entity has a component stored
    pub fn has_entity(&self, entity_id: &EntityId) -> bool {
        self.components.contains_key(entity_id)
    }

    /// Get the number of components
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Check if there are no components
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Remove all components for an entity
    pub fn remove_all(&mut self, entity_id: &EntityId) {
        self.components.remove(entity_id);
    }

    /// Get the type name of this store's component for an entity, if present
    pub fn stored_type_name(&self, entity_id: &EntityId) -> Option<&'static str> {
        // Use .as_ref() to get &dyn Component before calling type_name(),
        // otherwise the blanket impl on Box<dyn Component> would report the
        // Box's own type name instead of the concrete component's.
        self.components
            .get(entity_id)
            .map(|c| c.as_ref().type_name())
    }
}

/// A registry for all component types
#[derive(Default)]
pub struct ComponentRegistry {
    /// The component storages, indexed by component type ID
    storages: HashMap<TypeId, ComponentStore>,
}

impl ComponentRegistry {
    /// Create a new component registry
    pub fn new() -> Self {
        Self {
            storages: HashMap::new(),
        }
    }

    /// Register a component type
    pub fn register<T: Component>(&mut self) {
        self.storages.entry(TypeId::of::<T>()).or_default();
    }

    /// Add a component for an entity
    pub fn add<T: Component>(&mut self, entity_id: EntityId, component: T) {
        self.storages
            .entry(TypeId::of::<T>())
            .or_default()
            .add(entity_id, component);
    }

    /// Remove a component for an entity
    pub fn remove<T: Component>(&mut self, entity_id: &EntityId) -> Option<Box<dyn Component>> {
        let type_id = TypeId::of::<T>();
        self.storages.get_mut(&type_id)?.remove(entity_id)
    }

    /// Get a typed reference to a component for an entity
    pub fn get_typed<T: Component>(&self, entity_id: &EntityId) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.storages.get(&type_id)?.get_typed::<T>(entity_id)
    }

    /// Get a typed mutable reference to a component for an entity
    pub fn get_typed_mut<T: Component>(&mut self, entity_id: &EntityId) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        self.storages.get_mut(&type_id)?.get_typed_mut::<T>(entity_id)
    }

    /// Check if an entity has a component
    pub fn has<T: Component>(&self, entity_id: &EntityId) -> bool {
        let type_id = TypeId::of::<T>();
        self.storages
            .get(&type_id)
            .is_some_and(|s| s.has_entity(entity_id))
    }

    /// Check if an entity has a component of a specific type ID
    ///
    /// This is used internally by query_entities() to check for components by TypeId.
    pub fn has_type(&self, entity_id: &EntityId, type_id: TypeId) -> bool {
        self.storages
            .get(&type_id)
            .is_some_and(|storage| storage.has_entity(entity_id))
    }

    /// Remove all components for an entity
    pub fn remove_all(&mut self, entity_id: &EntityId) {
        for storage in self.storages.values_mut() {
            storage.remove_all(entity_id);
        }
    }

    /// Get the `(TypeId, type name)` of every component currently attached
    /// to an entity, in unspecified order.
    ///
    /// This is the only type-erased view of an entity's components; callers
    /// that clone or serialize components (snapshots, duplication) use it to
    /// detect types they don't know how to handle.
    pub fn component_types(&self, entity_id: &EntityId) -> Vec<(TypeId, &'static str)> {
        self.storages
            .iter()
            .filter_map(|(type_id, storage)| {
                storage
                    .stored_type_name(entity_id)
                    .map(|name| (*type_id, name))
            })
            .collect()
    }

    /// Initialize the component registry
    pub fn initialize(&mut self) -> Result<(), String> {
        log::debug!("Initializing component registry with {} component types", self.storages.len());
        Ok(())
    }

    /// Shutdown the component registry
    pub fn shutdown(&mut self) -> Result<(), String> {
        log::debug!("Shutting down component registry, removing all components");
        self.storages.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Health(u32);

    #[test]
    fn test_boxed_component_downcasts_only_through_as_ref_as_any() {
        let boxed: Box<dyn Component> = Box::new(Health(7));

        // The right path: `.as_ref()` reaches the concrete type by virtual dispatch.
        assert_eq!(boxed.as_ref().as_any().downcast_ref::<Health>(), Some(&Health(7)));
        assert_eq!(boxed.as_ref().type_name(), std::any::type_name::<Health>());

        // The footgun: `.as_any()` on the Box itself resolves the blanket
        // `Component` impl for `Box<dyn Component>`, so the concrete
        // downcast fails and the type name is the Box's own.
        assert_eq!(boxed.as_any().downcast_ref::<Health>(), None);
        assert_ne!(boxed.type_name(), std::any::type_name::<Health>());
    }
}
