//! Transform hierarchy propagation system
//!
//! This module provides a system that propagates transforms through the entity hierarchy,
//! updating GlobalTransform2D components based on the local Transform2D and parent transforms.
//!
//! Propagation is dirty-flagged: the system caches the last-propagated local
//! transform and parent link per entity (value comparison, not mutation
//! hooks), so a frame where nothing moved recomputes nothing. Any writer is
//! detected automatically — `get_mut` edits, snapshot restores, reparenting —
//! because dirtiness is derived from current values. `GlobalTransform2D` is
//! system-owned: manual writes to it are NOT change-tracked and will be
//! overwritten the next time the owning entity is dirty.

use std::collections::HashMap;

use crate::entity::EntityId;
use crate::hierarchy::{GlobalTransform2D, Parent};
use crate::hierarchy_extension::WorldHierarchyExt;
use crate::sprite_components::Transform2D;
use crate::system::System;
use crate::world::World;

/// One DFS traversal frame: `(entity, parent (id, global) — None for roots,
/// ancestor_dirty)`.
type TraversalFrame = (EntityId, Option<(EntityId, GlobalTransform2D)>, bool);

/// Last-propagated state for one entity (the dirty-flag baseline).
struct CachedNode {
    /// Local transform as of the last propagation.
    local: Transform2D,
    /// Parent link as of the last propagation (None = root).
    parent: Option<EntityId>,
    /// Frame stamp of the last visit — entries with a stale stamp belong to
    /// removed entities and get pruned.
    stamp: u64,
}

/// System that propagates transforms through the entity hierarchy
///
/// This system updates GlobalTransform2D components for all entities with Transform2D.
/// For root entities (no Parent component), GlobalTransform2D equals Transform2D.
/// For child entities, GlobalTransform2D is computed by multiplying the parent's
/// GlobalTransform2D with the child's local Transform2D.
///
/// # Usage
///
/// Add this system to your world and ensure entities have:
/// - `Transform2D` - local transform relative to parent (or world if no parent)
/// - `GlobalTransform2D` - will be computed automatically
/// - `Parent` (optional) - if present, transform is relative to parent
///
/// # Example
///
/// ```
/// use ecs::{GlobalTransform2D, Transform2D, TransformHierarchySystem, World, WorldHierarchyExt};
/// use glam::Vec2;
///
/// let mut world = World::new();
/// world.add_system(TransformHierarchySystem::new());
///
/// // Create parent
/// let parent = world.create_entity();
/// world.add_component(&parent, Transform2D::new(Vec2::new(100.0, 0.0))).ok();
/// world.add_component(&parent, GlobalTransform2D::default()).ok();
///
/// // Create child
/// let child = world.create_entity();
/// world.add_component(&child, Transform2D::new(Vec2::new(50.0, 0.0))).ok();
/// world.add_component(&child, GlobalTransform2D::default()).ok();
/// world.set_parent(child, parent).ok();
///
/// // After update, child's GlobalTransform2D.position will be (150.0, 0.0)
/// # world.initialize().unwrap();
/// # world.start().unwrap();
/// world.update(0.016).unwrap();
/// let global = world.get::<GlobalTransform2D>(child).unwrap();
/// assert_eq!(global.position, Vec2::new(150.0, 0.0));
/// ```
pub struct TransformHierarchySystem {
    /// Whether the system is enabled
    enabled: bool,
    /// Per-entity dirty-flag baseline (last-propagated local + parent link).
    cache: HashMap<EntityId, CachedNode>,
    /// Reusable DFS stack, kept across frames to avoid per-frame allocations.
    stack: Vec<TraversalFrame>,
    /// Monotonic update counter used to stamp cache entries.
    frame: u64,
    /// How many entities had their global transform recomputed last update.
    recomputed_last_update: usize,
    /// How many entities were visited (checked) last update.
    visited_last_update: usize,
}

impl TransformHierarchySystem {
    /// Create a new transform hierarchy system
    pub fn new() -> Self {
        Self {
            enabled: true,
            cache: HashMap::new(),
            stack: Vec::new(),
            frame: 0,
            recomputed_last_update: 0,
            visited_last_update: 0,
        }
    }

    /// Enable or disable the system
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if the system is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Forget all cached propagation state, forcing a full recompute on the
    /// next update.
    ///
    /// The analogue of `PhysicsSystem::clear()`: call after wholesale world
    /// replacement (e.g. the editor restoring a `WorldSnapshot`) so no stale
    /// baseline survives. Not needed for ordinary edits — those are detected
    /// by value comparison automatically.
    pub fn reset(&mut self) {
        self.cache.clear();
        self.stack.clear();
        self.recomputed_last_update = 0;
        self.visited_last_update = 0;
    }

    /// How many entities had their `GlobalTransform2D` recomputed during the
    /// most recent update. A fully clean frame reports 0.
    pub fn recomputed_last_update(&self) -> usize {
        self.recomputed_last_update
    }

    /// How many entities were visited (dirty-checked) during the most recent
    /// update.
    pub fn visited_last_update(&self) -> usize {
        self.visited_last_update
    }

    /// Number of entities currently tracked in the propagation cache.
    pub fn tracked_entity_count(&self) -> usize {
        self.cache.len()
    }
}

impl Default for TransformHierarchySystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for TransformHierarchySystem {
    fn initialize(&mut self, _world: &mut World) -> Result<(), String> {
        log::debug!("TransformHierarchySystem initialized");
        Ok(())
    }

    fn update(&mut self, world: &mut World, _delta_time: f32) {
        if !self.enabled {
            return;
        }

        self.frame += 1;
        let mut recomputed = 0usize;
        let mut visited = 0usize;
        // Cache entries stamped or (re)inserted this frame. If the cache
        // holds more entries than this afterwards, entities were removed and
        // the stale entries get pruned.
        let mut live_entries = 0usize;

        // Seed the reusable stack with root entities (single pass — no
        // get_root_entities()/entities() Vec allocations).
        self.stack.clear();
        self.stack.extend(
            world
                .entity_ids()
                .filter(|&e| world.get::<Parent>(e).is_none())
                .map(|e| (e, None, false)),
        );

        while let Some((entity, parent, ancestor_dirty)) = self.stack.pop() {
            visited += 1;

            let Some(local) = world.get::<Transform2D>(entity).copied() else {
                // No local transform: nothing to propagate for this entity.
                // Drop any stale baseline (dirtying the subtree for this
                // transition frame) and let children propagate against this
                // entity's stored global, preserving pre-dirty-flag behavior.
                let was_cached = self.cache.remove(&entity).is_some();
                let node_global = world
                    .get::<GlobalTransform2D>(entity)
                    .copied()
                    .unwrap_or_default();
                if let Some(children) = world.get_children(entity) {
                    let inherit_dirty = ancestor_dirty || was_cached;
                    self.stack.extend(
                        children
                            .iter()
                            .map(|&c| (c, Some((entity, node_global)), inherit_dirty)),
                    );
                }
                continue;
            };

            let parent_id = parent.map(|(id, _)| id);

            // Dirty check. The cache stamp must be refreshed on EVERY visit
            // (not short-circuited past), or pruning would evict live
            // entities that happened to be clean under a dirty ancestor.
            let mut dirty = ancestor_dirty;
            match self.cache.get_mut(&entity) {
                Some(cached) => {
                    cached.stamp = self.frame;
                    live_entries += 1;
                    if cached.local != local || cached.parent != parent_id {
                        dirty = true;
                    }
                }
                None => dirty = true,
            }
            if !dirty && world.get::<GlobalTransform2D>(entity).is_none() {
                // Someone removed the (system-owned) global — restore it.
                dirty = true;
            }

            let node_global = match parent {
                None => GlobalTransform2D::from_transform(&local),
                Some((_, parent_global)) => parent_global.mul_transform(&local),
            };

            if dirty {
                recomputed += 1;
                Self::set_global_transform(world, entity, node_global);
                if self
                    .cache
                    .insert(
                        entity,
                        CachedNode { local, parent: parent_id, stamp: self.frame },
                    )
                    .is_none()
                {
                    live_entries += 1;
                }
            }

            if let Some(children) = world.get_children(entity) {
                self.stack.extend(
                    children
                        .iter()
                        .map(|&c| (c, Some((entity, node_global)), dirty)),
                );
            }
        }

        // Prune baselines of removed entities (only when something vanished).
        if self.cache.len() > live_entries {
            let frame = self.frame;
            self.cache.retain(|_, c| c.stamp == frame);
        }

        self.recomputed_last_update = recomputed;
        self.visited_last_update = visited;
    }

    fn shutdown(&mut self, _world: &mut World) -> Result<(), String> {
        log::debug!("TransformHierarchySystem shut down");
        Ok(())
    }

    fn name(&self) -> &str {
        "TransformHierarchySystem"
    }
}

impl TransformHierarchySystem {
    /// Set or add a GlobalTransform2D component on an entity.
    fn set_global_transform(world: &mut World, entity: EntityId, global: GlobalTransform2D) {
        if let Some(existing) = world.get_mut::<GlobalTransform2D>(entity) {
            *existing = global;
        } else {
            world.add_component(&entity, global).ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn test_scaled_parent_transform_propagation() -> Result<(), crate::EcsError> {
        let mut world = World::new();
        let parent = world.create_entity();
        world.add_component(&parent, Transform2D::new(Vec2::ZERO).with_scale(Vec2::new(2.0, 2.0)))?;
        world.add_component(&parent, GlobalTransform2D::default())?;
        let child = world.create_entity();
        world.add_component(&child, Transform2D::new(Vec2::new(10.0, 10.0)))?;
        world.add_component(&child, GlobalTransform2D::default())?;
        world.set_parent(child, parent)?;

        TransformHierarchySystem::new().update(&mut world, 0.016);

        // The parent's scale multiplies the child's local position and scale.
        let child_global = world.get::<GlobalTransform2D>(child).expect("system writes the global");
        assert!(child_global.position.abs_diff_eq(Vec2::new(20.0, 20.0), 1e-3), "{:?}", child_global);
        assert!(child_global.scale.abs_diff_eq(Vec2::new(2.0, 2.0), 1e-3), "{:?}", child_global);
        Ok(())
    }
}
