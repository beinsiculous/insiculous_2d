//! Entity clipboard: registry-driven capture and spawn of entity subtrees.
//!
//! Copy/Paste/Cut and Duplicate all flow through one pair of halves —
//! [`capture_entity_tree`] (world → data) and [`SpawnTreeCommand`]
//! (data → world, undoably). The command's undo removes the WHOLE spawned
//! subtree ([`ecs::World::remove_entity`] detaches only the one entity —
//! per-root `CreateEntityCommand` undo used to orphan pasted children), and
//! every redo records the freshly spawned root so a later undo never
//! targets a stale id.
//!
//! Limitation (documented, pre-existing for Duplicate too): components
//! holding raw `EntityId` references are captured verbatim — a pasted copy
//! still points at the ORIGINAL target entity.

use std::any::{Any, TypeId};
use std::collections::HashSet;

use ecs::sprite_components::Name;
use ecs::{EntityId, World, WorldHierarchyExt};
use glam::Vec2;

use crate::commands::EditorCommand;
use crate::stored_component::{
    capture_all_components, registered_component_type_ids, restore_components, StoredComponent,
};

/// One captured entity subtree: registry-known components plus children.
/// Hierarchy links are rebuilt explicitly on spawn, never stored.
#[derive(Debug, Clone)]
pub struct ClipboardEntity {
    /// Registry-known components of this entity
    pub components: Vec<StoredComponent>,
    /// Captured child subtrees, in hierarchy order
    pub children: Vec<ClipboardEntity>,
}

/// Capture `root` and its descendants into clipboard data.
pub fn capture_entity_tree(world: &World, root: EntityId) -> ClipboardEntity {
    ClipboardEntity {
        components: capture_all_components(world, root),
        children: world
            .get_children(root)
            .map(|children| {
                children
                    .iter()
                    .copied()
                    .map(|child| capture_entity_tree(world, child))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Component type NAMES on `root`'s subtree that the editor registry does
/// not know — these are silently absent from a capture. Callers surface a
/// status-bar warning (warn, never block).
pub fn uncaptured_component_names(world: &World, root: EntityId) -> Vec<&'static str> {
    let known: HashSet<TypeId> = registered_component_type_ids().into_iter().collect();
    let mut names: Vec<&'static str> = Vec::new();
    let mut entities = vec![root];
    entities.extend(world.get_descendants(root));
    for entity in entities {
        for (type_id, name) in world.component_types(entity) {
            let is_hierarchy = type_id == TypeId::of::<ecs::hierarchy::Parent>()
                || type_id == TypeId::of::<ecs::hierarchy::Children>();
            if !is_hierarchy && !known.contains(&type_id) && !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names.sort_unstable();
    names
}

/// Spawn a captured subtree into the world with fresh entity ids. The
/// `offset` applies to the ROOT's position only; `name_suffix` (e.g.
/// `" (Copy)"`) is appended to every spawned entity that has a `Name`,
/// matching Duplicate's behavior.
pub fn spawn_entity_tree(
    world: &mut World,
    tree: &ClipboardEntity,
    parent: Option<EntityId>,
    offset: Vec2,
    name_suffix: Option<&str>,
) -> EntityId {
    let mut spawned = Vec::new();
    spawn_tree_inner(world, tree, parent, offset, name_suffix, None, &mut spawned);
    spawned[0]
}

/// The one spawn recursion. `reuse_ids` (preorder: root, then each child
/// subtree) resurrects entities under specific ids via
/// `create_entity_with_id` — how redo and Cut-undo keep selections and
/// later history commands valid. Every spawned id is
/// appended to `spawned` in the same preorder, and `spawned.len()` doubles
/// as the index into `reuse_ids`.
fn spawn_tree_inner(
    world: &mut World,
    tree: &ClipboardEntity,
    parent: Option<EntityId>,
    offset: Vec2,
    name_suffix: Option<&str>,
    reuse_ids: Option<&[EntityId]>,
    spawned: &mut Vec<EntityId>,
) -> EntityId {
    let entity = match reuse_ids {
        Some(ids) => world.create_entity_with_id(ids[spawned.len()]),
        None => world.create_entity(),
    };
    spawned.push(entity);
    restore_components(world, entity, &tree.components);

    if offset != Vec2::ZERO {
        if let Some(transform) = world.get_mut::<common::Transform2D>(entity) {
            transform.position += offset;
        }
    }
    if let Some(suffix) = name_suffix {
        if let Some(name) = world.get::<Name>(entity) {
            let renamed = format!("{}{}", name.as_str(), suffix);
            world.add_component(&entity, Name::new(renamed)).ok();
        }
    }
    if let Some(parent) = parent {
        world.set_parent(entity, parent).ok();
    }

    for child in &tree.children {
        spawn_tree_inner(world, child, Some(entity), Vec2::ZERO, name_suffix, reuse_ids, spawned);
    }
    entity
}

/// The live ids of `root`'s subtree in the same preorder the spawn
/// recursion uses (root first, then each child subtree depth-first).
fn tree_entity_ids(world: &World, root: EntityId) -> Vec<EntityId> {
    let mut ids = vec![root];
    if let Some(children) = world.get_children(root) {
        let children: Vec<EntityId> = children.to_vec();
        for child in children {
            ids.extend(tree_entity_ids(world, child));
        }
    }
    ids
}

/// Undoable spawn of one captured subtree (Paste and Duplicate both push
/// one of these per root). Undo deletes the spawned root's entire subtree
/// (`remove_entity_hierarchy` — depth-safe); redo resurrects every entity
/// under the SAME ids the first execute allocated, so the selection and any
/// later history command referencing the spawned entities stay valid
/// across undo/redo cycles.
pub struct SpawnTreeCommand {
    tree: ClipboardEntity,
    parent: Option<EntityId>,
    offset: Vec2,
    name_suffix: Option<&'static str>,
    /// Preorder ids from the FIRST execute; redo reuses them. Empty while
    /// never executed.
    spawned_ids: Vec<EntityId>,
    /// Whether the spawned subtree is currently alive in the world.
    alive: bool,
    display_name: &'static str,
}

impl SpawnTreeCommand {
    /// A paste-style spawn (no rename).
    pub fn paste(tree: ClipboardEntity, parent: Option<EntityId>, offset: Vec2) -> Self {
        Self {
            tree,
            parent,
            offset,
            name_suffix: None,
            spawned_ids: Vec::new(),
            alive: false,
            display_name: "Paste Entity",
        }
    }

    /// A duplicate-style spawn (" (Copy)" appended to every Name).
    pub fn duplicate(tree: ClipboardEntity, parent: Option<EntityId>, offset: Vec2) -> Self {
        Self {
            tree,
            parent,
            offset,
            name_suffix: Some(" (Copy)"),
            spawned_ids: Vec::new(),
            alive: false,
            display_name: "Duplicate Entity",
        }
    }

    /// The spawned root, stable across undo/redo cycles.
    pub fn spawned_root(&self) -> Option<EntityId> {
        self.spawned_ids.first().copied()
    }
}

impl EditorCommand for SpawnTreeCommand {
    fn execute(&mut self, world: &mut World) {
        if self.alive {
            return;
        }
        if self.spawned_ids.is_empty() {
            // First execute: fresh ids, recorded for every later redo.
            spawn_tree_inner(
                world,
                &self.tree,
                self.parent,
                self.offset,
                self.name_suffix,
                None,
                &mut self.spawned_ids,
            );
        } else {
            // Redo: resurrect under the ORIGINAL ids.
            let ids = std::mem::take(&mut self.spawned_ids);
            let mut respawned = Vec::with_capacity(ids.len());
            spawn_tree_inner(
                world,
                &self.tree,
                self.parent,
                self.offset,
                self.name_suffix,
                Some(&ids),
                &mut respawned,
            );
            self.spawned_ids = ids;
        }
        self.alive = true;
    }

    fn undo(&mut self, world: &mut World) {
        if !self.alive {
            return;
        }
        if let Some(root) = self.spawned_root() {
            // Depth-safe subtree removal: removing a child first would
            // promote grandchildren to roots and orphan them.
            world.remove_entity_hierarchy(&root).ok();
        }
        self.alive = false;
    }

    fn display_name(&self) -> &str {
        self.display_name
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Undoable removal of one live subtree — what Cut pushes per selection
/// root. Delete's reparent-the-children semantics are wrong for Cut: the
/// clipboard holds the WHOLE subtree, so leaving live children behind
/// would duplicate them on paste. Undo resurrects every entity under its
/// ORIGINAL id with the hierarchy (and the root's parent link) intact.
pub struct DeleteTreeCommand {
    tree: ClipboardEntity,
    ids: Vec<EntityId>,
    parent: Option<EntityId>,
}

impl DeleteTreeCommand {
    /// Capture `root`'s live subtree in preparation for removal.
    pub fn new(world: &World, root: EntityId) -> Self {
        Self {
            tree: capture_entity_tree(world, root),
            ids: tree_entity_ids(world, root),
            parent: world.get_parent(root),
        }
    }
}

impl EditorCommand for DeleteTreeCommand {
    fn execute(&mut self, world: &mut World) {
        if let Some(root) = self.ids.first() {
            world.remove_entity_hierarchy(root).ok();
        }
    }

    fn undo(&mut self, world: &mut World) {
        let ids = std::mem::take(&mut self.ids);
        let mut respawned = Vec::with_capacity(ids.len());
        spawn_tree_inner(
            world,
            &self.tree,
            self.parent,
            Vec2::ZERO,
            None,
            Some(&ids),
            &mut respawned,
        );
        self.ids = ids;
    }

    fn display_name(&self) -> &str {
        "Cut Entity"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use common::Transform2D;

    use crate::test_support::named_entity;

    fn name_of(world: &World, entity: EntityId) -> Option<String> {
        world.get::<Name>(entity).map(|n| n.as_str().to_string())
    }

    fn children_of(world: &World, entity: EntityId) -> Vec<EntityId> {
        world.get_children(entity).map(|c| c.to_vec()).unwrap_or_default()
    }

    /// Parent → Child → Grandchild, named A/B/C at x = 1/2/3.
    fn three_deep(world: &mut World) -> (EntityId, EntityId, EntityId) {
        let a = named_entity(world, "A", Vec2::new(1.0, 0.0));
        let b = named_entity(world, "B", Vec2::new(2.0, 0.0));
        let c = named_entity(world, "C", Vec2::new(3.0, 0.0));
        world.set_parent(b, a).ok();
        world.set_parent(c, b).ok();
        (a, b, c)
    }

    #[test]
    fn test_capture_and_spawn_round_trips_a_hierarchy_with_the_offset_on_the_root_only() {
        let mut world = World::new();
        let parent = named_entity(&mut world, "Parent", Vec2::new(10.0, 20.0));
        let child = named_entity(&mut world, "Child", Vec2::new(1.0, 2.0));
        world.set_parent(child, parent).ok();

        let tree = capture_entity_tree(&world, parent);
        let spawned = spawn_entity_tree(&mut world, &tree, None, Vec2::new(5.0, 0.0), None);

        assert_ne!(spawned, parent, "a paste is a fresh entity");
        assert_eq!(
            world.get::<Transform2D>(spawned).map(|t| t.position),
            Some(Vec2::new(15.0, 20.0)),
            "the offset applies to the root"
        );
        let children = children_of(&world, spawned);
        assert_eq!(children.len(), 1, "the child spawns re-parented under the copy");
        assert_eq!(
            world.get::<Transform2D>(children[0]).map(|t| t.position),
            Some(Vec2::new(1.0, 2.0)),
            "child positions are parent-local and un-offset"
        );
        assert_eq!(name_of(&world, children[0]), Some("Child".to_string()));
    }

    #[test]
    fn test_paste_undo_removes_the_whole_subtree_including_grandchildren() {
        // Depth ≥ 2 regression: removing a child before the root promotes
        // its children to roots — a plain descendant loop orphaned the
        // grandchild, and per-root CreateEntityCommand undo orphaned the
        // pasted child.
        let mut world = World::new();
        let (a, _, _) = three_deep(&mut world);
        let baseline = world.entities().len();
        let tree = capture_entity_tree(&world, a);

        let mut cmd = SpawnTreeCommand::paste(tree, None, Vec2::new(20.0, 0.0));
        cmd.execute(&mut world);
        assert_eq!(world.entities().len(), baseline + 3, "root, child and grandchild pasted");

        cmd.undo(&mut world);
        assert_eq!(world.entities().len(), baseline, "no orphaned child or grandchild left behind");
    }

    #[test]
    fn test_paste_redo_resurrects_the_same_ids_for_root_and_children() {
        // The selection and any later history command referencing
        // the pasted entities must stay valid across undo/redo.
        let mut world = World::new();
        let source = named_entity(&mut world, "Parent", Vec2::ZERO);
        let child = named_entity(&mut world, "Child", Vec2::ZERO);
        world.set_parent(child, source).ok();
        let tree = capture_entity_tree(&world, source);

        let mut cmd = SpawnTreeCommand::paste(tree, None, Vec2::ZERO);
        cmd.execute(&mut world);
        let first_root = cmd.spawned_root().expect("spawned");
        let first_child = children_of(&world, first_root)[0];

        cmd.undo(&mut world);
        assert_eq!(name_of(&world, first_root), None, "undo removes the pasted root");

        cmd.execute(&mut world); // redo
        assert_eq!(cmd.spawned_root(), Some(first_root), "redo resurrects the SAME root id");
        assert_eq!(name_of(&world, first_root), Some("Parent".to_string()));
        assert_eq!(name_of(&world, first_child), Some("Child".to_string()), "child id stable too");

        cmd.undo(&mut world);
        assert_eq!(world.entities().len(), 2, "only the originals remain");
    }

    #[test]
    fn test_cut_removes_the_whole_subtree_and_undo_restores_ids_hierarchy_and_values() {
        let mut world = World::new();
        let (a, b, c) = three_deep(&mut world);

        let mut cmd = DeleteTreeCommand::new(&world, a);
        cmd.execute(&mut world);
        assert!(
            world.entities().is_empty(),
            "cut removes the WHOLE subtree — no promoted children (Delete's reparent semantics would duplicate them on paste)"
        );

        cmd.undo(&mut world);
        assert_eq!(world.entities().len(), 3);
        assert_eq!(name_of(&world, a), Some("A".to_string()), "original ids resurrect exactly");
        assert_eq!(world.get_parent(b), Some(a));
        assert_eq!(world.get_parent(c), Some(b));
        assert_eq!(world.get::<Transform2D>(c).map(|t| t.position), Some(Vec2::new(3.0, 0.0)));
    }

    #[test]
    fn test_duplicate_suffixes_every_spawned_name() {
        let mut world = World::new();
        let parent = named_entity(&mut world, "Parent", Vec2::ZERO);
        let child = named_entity(&mut world, "Child", Vec2::ZERO);
        world.set_parent(child, parent).ok();
        let tree = capture_entity_tree(&world, parent);

        let mut cmd = SpawnTreeCommand::duplicate(tree, None, Vec2::ZERO);
        cmd.execute(&mut world);

        let root = cmd.spawned_root().expect("spawned");
        assert_eq!(name_of(&world, root), Some("Parent (Copy)".to_string()));
        assert_eq!(name_of(&world, children_of(&world, root)[0]), Some("Child (Copy)".to_string()));
    }

    #[test]
    fn test_paste_warning_names_unregistered_components_but_never_hierarchy_links() {
        struct EnemyAi;

        let mut world = World::new();
        let parent = named_entity(&mut world, "P", Vec2::ZERO);
        let child = named_entity(&mut world, "C", Vec2::ZERO);
        world.set_parent(child, parent).ok();
        assert!(
            uncaptured_component_names(&world, parent).is_empty(),
            "Parent/Children are rebuilt explicitly, never reported as lost"
        );

        world.add_component(&child, EnemyAi).ok();

        let names = uncaptured_component_names(&world, parent);
        assert_eq!(names.len(), 1, "a component the registry cannot capture is reported once");
        assert!(names[0].contains("EnemyAi"), "reported by type name, found on a descendant: {names:?}");
    }
}
