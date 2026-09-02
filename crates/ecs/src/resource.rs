//! Typed global resources for cross-system game state.
//!
//! Resources are singleton values stored in the World, accessed by type.
//! Use them for game-wide state like score, settings, or repositories
//! that multiple systems need to read/write.
//!
//! # Example
//! ```
//! use ecs::World;
//!
//! struct Score { value: u32 }
//!
//! let mut world = World::new();
//! world.insert_resource(Score { value: 0 });
//!
//! // Read
//! let score = world.resource::<Score>().unwrap();
//! assert_eq!(score.value, 0);
//!
//! // Write
//! world.resource_mut::<Score>().unwrap().value += 10;
//! assert_eq!(world.resource::<Score>().unwrap().value, 10);
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Storage for typed singleton resources.
///
/// Each resource type can have at most one instance. Resources are
/// accessed by their concrete type via `TypeId`.
pub struct ResourceStorage {
    resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl ResourceStorage {
    /// Create an empty resource storage.
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    /// Insert a resource, replacing any previous value of the same type.
    /// Returns the previous value if one existed.
    pub fn insert<T: Send + Sync + 'static>(&mut self, resource: T) -> Option<T> {
        let previous = self.resources.insert(
            TypeId::of::<T>(),
            Box::new(resource),
        );
        previous.and_then(|boxed| boxed.downcast::<T>().ok().map(|b| *b))
    }

    /// Get an immutable reference to a resource by type.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.resources
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }

    /// Get a mutable reference to a resource by type.
    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_mut::<T>())
    }

    /// Remove a resource by type, returning it if it existed.
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.resources
            .remove(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast::<T>().ok().map(|b| *b))
    }

    /// Check if a resource of the given type exists.
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.resources.contains_key(&TypeId::of::<T>())
    }

    /// Remove all resources.
    pub fn clear(&mut self) {
        self.resources.clear();
    }

    /// Get the number of stored resources.
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Check if storage is empty.
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }
}

impl Default for ResourceStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Score {
        value: u32,
    }

    #[derive(Debug, PartialEq)]
    struct Lives {
        count: i32,
    }

    #[test]
    fn test_insert_replaces_previous() {
        let mut storage = ResourceStorage::new();
        storage.insert(Score { value: 10 });

        let previous = storage.insert(Score { value: 20 });

        assert_eq!(previous, Some(Score { value: 10 }), "the replaced value comes back");
        assert_eq!(storage.get::<Score>(), Some(&Score { value: 20 }));

        // Mutation through get_mut is visible on the next get.
        storage.get_mut::<Score>().expect("present").value += 5;
        assert_eq!(storage.get::<Score>(), Some(&Score { value: 25 }));
    }

    #[test]
    fn test_resources_are_keyed_by_type_and_coexist() {
        let mut storage = ResourceStorage::new();
        storage.insert(Score { value: 100 });
        storage.insert(Lives { count: 3 });

        assert_eq!(storage.get::<Score>(), Some(&Score { value: 100 }));
        assert_eq!(storage.get::<Lives>(), Some(&Lives { count: 3 }));
        assert_eq!(storage.len(), 2);

        storage.clear();
        assert!(storage.is_empty());
        assert_eq!(storage.get::<Score>(), None);
        assert_eq!(storage.get::<Lives>(), None);
    }

    #[test]
    fn test_remove_returns_the_resource_and_none_when_absent() {
        let mut storage = ResourceStorage::new();
        storage.insert(Score { value: 42 });

        assert_eq!(storage.remove::<Score>(), Some(Score { value: 42 }));
        assert_eq!(storage.get::<Score>(), None, "removed means gone");
        assert_eq!(storage.remove::<Score>(), None, "a second remove finds nothing");
        assert_eq!(storage.remove::<Lives>(), None, "a type never inserted is None");
    }
}
