//! Hierarchy extension trait for World.
//!
//! This module provides the `WorldHierarchyExt` trait which adds hierarchy management
//! methods to the `World` struct. This follows the extension trait pattern to improve
//! code organization by extracting hierarchy-related functionality into a focused module.
//!
//! # Example
//! ```
//! use ecs::{World, WorldHierarchyExt};
//!
//! let mut world = World::new();
//! let parent = world.create_entity();
//! let child = world.create_entity();
//!
//! // Set up hierarchy
//! world.set_parent(child, parent).unwrap();
//!
//! // Query hierarchy
//! assert_eq!(world.get_parent(child), Some(parent));
//! assert!(world.get_children(parent).unwrap().contains(&child));
//! ```

use crate::entity::EntityId;
use crate::hierarchy::{Children, Parent};
use crate::EcsError;
use crate::World;

/// Extension trait providing hierarchy management methods for World.
///
/// This trait is automatically implemented for `World` and provides methods for:
/// - Setting and removing parent-child relationships
/// - Querying parents, children, ancestors, and descendants
/// - Recursive entity hierarchy removal
///
/// # Design Notes
/// This trait extracts hierarchy management from the core `World` struct,
/// following the Single Responsibility Principle. The `World` struct remains
/// focused on entity and component management, while hierarchy operations
/// are provided through this extension.
pub trait WorldHierarchyExt {
    /// Set an entity's parent, establishing a parent-child relationship.
    ///
    /// This will:
    /// 1. Add a Parent component to the child entity
    /// 2. Add/update a Children component on the parent entity
    /// 3. Remove the child from any previous parent's Children component
    ///
    /// # Errors
    /// - Returns an error if either entity doesn't exist
    /// - Returns an error if setting the parent would create a cycle
    /// - Returns an error if trying to set an entity as its own parent
    ///
    /// # Example
    /// ```
    /// # use ecs::{World, WorldHierarchyExt};
    /// # let mut world = World::new();
    /// let parent = world.create_entity();
    /// let child = world.create_entity();
    /// world.set_parent(child, parent)?;
    /// # assert_eq!(world.get_parent(child), Some(parent));
    /// # Ok::<(), ecs::EcsError>(())
    /// ```
    fn set_parent(&mut self, child: EntityId, parent: EntityId) -> Result<(), EcsError>;

    /// Remove an entity's parent, making it a root entity.
    ///
    /// This will:
    /// 1. Remove the Parent component from the entity
    /// 2. Remove the entity from its parent's Children component
    ///
    /// # Errors
    /// Returns an error if the entity doesn't exist.
    fn remove_parent(&mut self, entity: EntityId) -> Result<(), EcsError>;

    /// Get an entity's parent.
    ///
    /// Returns `None` if the entity has no parent (is a root entity) or doesn't exist.
    fn get_parent(&self, entity: EntityId) -> Option<EntityId>;

    /// Get an entity's children.
    ///
    /// Returns `None` if the entity has no children or doesn't exist.
    fn get_children(&self, entity: EntityId) -> Option<&[EntityId]>;

    /// Get all root entities (entities without a parent).
    fn get_root_entities(&self) -> Vec<EntityId>;

    /// Recursively get all descendants of an entity.
    ///
    /// Returns an empty vector if the entity has no children or doesn't exist.
    fn get_descendants(&self, entity: EntityId) -> Vec<EntityId>;

    /// Get all ancestors of an entity (parent, grandparent, etc.).
    ///
    /// Returns an empty vector if the entity has no parent or doesn't exist.
    fn get_ancestors(&self, entity: EntityId) -> Vec<EntityId>;

    /// Check if an entity is an ancestor of another entity.
    fn is_ancestor_of(&self, potential_ancestor: EntityId, entity: EntityId) -> bool;

    /// Check if an entity is a descendant of another entity.
    fn is_descendant_of(&self, potential_descendant: EntityId, entity: EntityId) -> bool;

    /// Remove an entity and all its descendants from the hierarchy.
    ///
    /// This recursively removes all children and their children, etc.
    ///
    /// # Errors
    /// Returns an error if the entity doesn't exist.
    fn remove_entity_hierarchy(&mut self, entity: &EntityId) -> Result<(), EcsError>;
}

impl WorldHierarchyExt for World {
    fn set_parent(&mut self, child: EntityId, parent: EntityId) -> Result<(), EcsError> {
        // Validate both entities exist
        if self.get_entity(&child).is_err() {
            return Err(EcsError::EntityNotFound(child));
        }
        if self.get_entity(&parent).is_err() {
            return Err(EcsError::EntityNotFound(parent));
        }

        // Prevent self-parenting
        if child == parent {
            return Err(EcsError::SystemError(
                "Cannot set entity as its own parent".to_string(),
            ));
        }

        // Prevent cycles: if `child` is an ancestor of `parent`, setting this parent would create a cycle
        if self.is_ancestor_of(child, parent) {
            return Err(EcsError::SystemError(
                "Cannot set parent: would create a cycle in the hierarchy".to_string(),
            ));
        }

        // Remove from previous parent if any
        if let Some(old_parent) = self.get::<Parent>(child) {
            let old_parent_id = old_parent.entity();
            if old_parent_id != parent {
                // Remove child from old parent's Children component
                if let Some(old_children) = self.get_mut::<Children>(old_parent_id) {
                    old_children.remove(&child);
                }
            }
        }

        // Set the Parent component on the child
        self.add_component(&child, Parent::new(parent)).ok();

        // Add/update Children component on parent
        if let Some(children) = self.get_mut::<Children>(parent) {
            children.add(child);
        } else {
            // Parent doesn't have Children component yet, add it
            self.add_component(&parent, Children::with_children(vec![child]))
                .ok();
        }

        log::trace!(
            "Set parent of entity {} to {}",
            child.value(),
            parent.value()
        );
        Ok(())
    }

    fn remove_parent(&mut self, entity: EntityId) -> Result<(), EcsError> {
        if self.get_entity(&entity).is_err() {
            return Err(EcsError::EntityNotFound(entity));
        }

        // Get and remove the parent reference
        if let Some(parent) = self.get::<Parent>(entity) {
            let parent_id = parent.entity();

            // Remove from parent's Children component
            if let Some(children) = self.get_mut::<Children>(parent_id) {
                children.remove(&entity);
            }
        }

        // Remove the Parent component
        let _ = self.remove_component::<Parent>(&entity);

        log::trace!("Removed parent from entity {}", entity.value());
        Ok(())
    }

    fn get_parent(&self, entity: EntityId) -> Option<EntityId> {
        self.get::<Parent>(entity).map(|p| p.entity())
    }

    fn get_children(&self, entity: EntityId) -> Option<&[EntityId]> {
        self.get::<Children>(entity).map(|c| c.entities())
    }

    fn get_root_entities(&self) -> Vec<EntityId> {
        self.entity_ids()
            .filter(|id| self.get::<Parent>(*id).is_none())
            .collect()
    }

    fn get_descendants(&self, entity: EntityId) -> Vec<EntityId> {
        let mut descendants = Vec::new();
        collect_descendants_recursive(self, entity, &mut descendants);
        descendants
    }

    fn get_ancestors(&self, entity: EntityId) -> Vec<EntityId> {
        let mut ancestors = Vec::new();
        let mut current = entity;

        while let Some(parent) = self.get_parent(current) {
            ancestors.push(parent);
            current = parent;
        }

        ancestors
    }

    fn is_ancestor_of(&self, potential_ancestor: EntityId, entity: EntityId) -> bool {
        // Early-exit upward walk instead of materializing the ancestor list.
        // The step cap guards against malformed cycles (e.g. a manually
        // constructed Parent chain that bypassed set_parent's cycle check).
        let mut current = entity;
        let mut steps = self.entity_count();
        while let Some(parent) = self.get_parent(current) {
            if parent == potential_ancestor {
                return true;
            }
            if steps == 0 {
                log::warn!("is_ancestor_of: hierarchy cycle detected at entity {}", current.value());
                return false;
            }
            steps -= 1;
            current = parent;
        }
        false
    }

    fn is_descendant_of(&self, potential_descendant: EntityId, entity: EntityId) -> bool {
        // A descendant relationship is the ancestor relationship reversed;
        // walk upward from the candidate instead of materializing the subtree.
        self.is_ancestor_of(entity, potential_descendant)
    }

    fn remove_entity_hierarchy(&mut self, entity: &EntityId) -> Result<(), EcsError> {
        // First, collect all descendants (we need to do this before we start removing)
        let descendants = self.get_descendants(*entity);

        // Remove descendants in reverse order (deepest first)
        for descendant in descendants.into_iter().rev() {
            self.remove_entity(&descendant)?;
        }

        // Remove the entity itself (remove_entity unlinks it from its parent)
        self.remove_entity(entity)?;

        Ok(())
    }
}

/// Helper function to recursively collect descendants
fn collect_descendants_recursive(world: &World, entity: EntityId, descendants: &mut Vec<EntityId>) {
    if let Some(children) = world.get_children(entity) {
        // Clone the children slice to avoid borrowing issues
        let children_vec: Vec<EntityId> = children.to_vec();
        for child in children_vec {
            descendants.push(child);
            collect_descendants_recursive(world, child, descendants);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_ancestors_are_ordered_nearest_first() -> Result<(), EcsError> {
        let mut world = World::new();
        let grandparent = world.create_entity();
        let parent = world.create_entity();
        let child = world.create_entity();
        world.set_parent(parent, grandparent)?;
        world.set_parent(child, parent)?;

        let ancestors = world.get_ancestors(child);

        assert_eq!(ancestors, [parent, grandparent]);
        Ok(())
    }

    #[test]
    fn test_reparent_prunes_the_old_parents_child_list() -> Result<(), EcsError> {
        let mut world = World::new();
        let first = world.create_entity();
        let second = world.create_entity();
        let child = world.create_entity();
        world.set_parent(child, first)?;

        world.set_parent(child, second)?;

        assert_eq!(world.get_parent(child), Some(second));
        assert_eq!(world.get_children(second), Some(&[child][..]));
        assert_eq!(
            world.get_children(first).unwrap_or(&[]),
            [],
            "the old parent must not keep the child"
        );

        // remove_parent makes the child a root again.
        world.remove_parent(child)?;
        assert_eq!(world.get_parent(child), None);
        assert_eq!(world.get_children(second).unwrap_or(&[]), []);
        assert!(world.get_root_entities().contains(&child));
        Ok(())
    }

    #[test]
    fn test_get_children_preserves_insertion_order_across_reparenting() -> Result<(), EcsError> {
        // The hierarchy panel lists children in this order, and the scene
        // serializer writes them in it.
        let mut world = World::new();
        let parent = world.create_entity();
        let a = world.create_entity();
        let b = world.create_entity();
        let c = world.create_entity();
        world.set_parent(a, parent)?;
        world.set_parent(b, parent)?;
        world.set_parent(c, parent)?;
        assert_eq!(world.get_children(parent), Some(&[a, b, c][..]));

        world.remove_parent(b)?;
        assert_eq!(world.get_children(parent), Some(&[a, c][..]));

        world.set_parent(b, parent)?;
        assert_eq!(world.get_children(parent), Some(&[a, c, b][..]), "a re-added child goes last");
        Ok(())
    }
}
