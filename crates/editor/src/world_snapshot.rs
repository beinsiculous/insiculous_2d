//! World state snapshot for play-mode save/restore.
//!
//! Captures every component type known to the editor component registry
//! (`editor_component_registry!` in `stored_component/`) plus the hierarchy
//! components into a plain struct, and restores them on stop. This uses
//! typed cloning — no serialization required — so it is fast and does not
//! change the `Component` trait. New registry entries are captured
//! automatically; there is no second list to maintain here.
//!
//! Component types NOT in the registry (e.g. game-defined generics like
//! `HierarchicalStateMachine<S, P>`) cannot be cloned by any engine-side
//! list. Capture detects them via [`ecs::World::component_types`], reports
//! them through [`WorldSnapshot::uncaptured_types`] /
//! [`WorldSnapshot::loss_warning`], and they are lost on restore.

use std::any::TypeId;
use std::collections::HashSet;

use ecs::hierarchy::{Children, Parent};
use ecs::{EntityId, World};

use crate::stored_component::{
    capture_all_components, registered_component_type_ids, restore_components, StoredComponent,
};

/// Snapshot of a single entity's components.
struct EntitySnapshot {
    id: EntityId,
    /// Every registry-known component on the entity, captured via the
    /// editor component registry (the single source of truth).
    components: Vec<StoredComponent>,
    // The registry deliberately excludes hierarchy (commands manage it), so
    // the snapshot carries Parent/Children explicitly.
    parent: Option<Parent>,
    children: Option<Children>,
}

impl EntitySnapshot {
    /// Capture all registry components plus hierarchy from a single entity.
    fn capture(world: &World, id: EntityId) -> Self {
        Self {
            id,
            components: capture_all_components(world, id),
            parent: world.get::<Parent>(id).cloned(),
            children: world.get::<Children>(id).cloned(),
        }
    }

    /// Restore this snapshot's components onto an existing entity in the world.
    fn restore(self, world: &mut World) {
        let id = self.id;
        restore_components(world, id, &self.components);
        if let Some(p) = self.parent {
            world.add_component(&id, p).ok();
        }
        if let Some(c) = self.children {
            world.add_component(&id, c).ok();
        }
    }
}

/// A complete snapshot of all entities and their capturable components.
///
/// Created by `WorldSnapshot::capture()` before entering play mode and
/// consumed by `WorldSnapshot::restore()` when stopping play mode.
pub struct WorldSnapshot {
    snapshots: Vec<EntitySnapshot>,
    /// Full type paths of component types present at capture time that no
    /// registry entry covers — these are lost on restore. Deduped, sorted.
    uncaptured_types: Vec<&'static str>,
}

impl WorldSnapshot {
    /// Capture the current world state.
    ///
    /// Component types outside the editor registry cannot be captured; they
    /// are recorded (see [`Self::uncaptured_types`]) and logged, never a
    /// block — a game-defined generic component can never be registered, and
    /// refusing Play would make such games uneditable.
    pub fn capture(world: &World) -> Self {
        let mut known: HashSet<TypeId> = registered_component_type_ids().into_iter().collect();
        known.insert(TypeId::of::<Parent>());
        known.insert(TypeId::of::<Children>());

        let mut uncaptured: Vec<&'static str> = Vec::new();
        let snapshots = world
            .entities()
            .into_iter()
            .map(|id| {
                for (type_id, name) in world.component_types(id) {
                    if !known.contains(&type_id) && !uncaptured.contains(&name) {
                        uncaptured.push(name);
                    }
                }
                EntitySnapshot::capture(world, id)
            })
            .collect();
        uncaptured.sort_unstable();

        if !uncaptured.is_empty() {
            log::warn!(
                "WorldSnapshot: {} component type(s) not in the editor registry \
                 will be lost on restore: {}",
                uncaptured.len(),
                uncaptured.join(", ")
            );
        }

        Self { snapshots, uncaptured_types: uncaptured }
    }

    /// Restore the captured state, replacing the current world contents.
    ///
    /// Clears all entities and components, then recreates them from the
    /// snapshot. The world is wholesale-replaced, so the caller must reset
    /// any entity-indexed system caches afterwards (the editor's Stop path
    /// resets the transform propagation system).
    pub fn restore(self, world: &mut World) {
        world.clear();

        for snapshot in self.snapshots {
            world.create_entity_with_id(snapshot.id);
            snapshot.restore(world);
        }
    }

    /// Number of entities in the snapshot.
    pub fn entity_count(&self) -> usize {
        self.snapshots.len()
    }

    /// Full type paths of component types that were present at capture time
    /// but could not be captured (not in the editor registry). Restoring
    /// this snapshot loses them.
    pub fn uncaptured_types(&self) -> &[&'static str] {
        &self.uncaptured_types
    }

    /// Player-facing warning to show when entering Play, if anything will be
    /// lost by the Play → Stop round-trip. `None` when capture was complete.
    pub fn loss_warning(&self) -> Option<String> {
        if self.uncaptured_types.is_empty() {
            return None;
        }
        Some(format!(
            "{} component type(s) not in the editor registry will be lost on Stop: {}",
            self.uncaptured_types.len(),
            display_names(&self.uncaptured_types).join(", ")
        ))
    }

    /// Player-facing report to show after this snapshot has been restored,
    /// naming what was actually dropped. `None` when capture was complete.
    pub fn drop_report(&self) -> Option<String> {
        if self.uncaptured_types.is_empty() {
            return None;
        }
        Some(format!(
            "Restored authored scene; dropped {} unregistered component type(s): {}",
            self.uncaptured_types.len(),
            display_names(&self.uncaptured_types).join(", ")
        ))
    }
}

/// Shorten a full type path for display: strip generic arguments, keep the
/// last path segment (`game::ai::Brain<State>` → `Brain`).
fn short_type_name(full: &str) -> &str {
    let base = full.split('<').next().unwrap_or(full);
    base.rsplit("::").next().unwrap_or(base)
}

/// Display names for the status bar: shortened, except that names which
/// collide after shortening fall back to their full paths so distinct types
/// stay distinguishable. Full paths always go to the log regardless.
fn display_names(full_paths: &[&'static str]) -> Vec<String> {
    let shorts: Vec<&str> = full_paths.iter().map(|f| short_type_name(f)).collect();
    shorts
        .iter()
        .enumerate()
        .map(|(i, short)| {
            let collides = shorts.iter().enumerate().any(|(j, other)| j != i && other == short);
            if collides {
                full_paths[i].to_string()
            } else {
                (*short).to_string()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
