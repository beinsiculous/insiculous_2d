//! Commands for entity lifecycle: create, delete, and grouped (macro) actions.

use std::any::Any;

use ecs::{EntityId, World, WorldHierarchyExt};

use crate::stored_component::{capture_all_components, restore_components, StoredComponent};

use super::{entity_is_alive, EditorCommand, Rebase};

// ---------------------------------------------------------------------------
// CreateEntityCommand
// ---------------------------------------------------------------------------

/// Command for creating a new entity.
///
/// On first execute the entity is already created by the caller — the command
/// captures its ID and all component data. On undo the entity is removed.
/// On redo the entity is recreated **with the same EntityId** (ids are never
/// recycled, so the slot is free), keeping selections and other history
/// commands that reference it valid across undo/redo cycles.
pub struct CreateEntityCommand {
    entity: EntityId,
    components: Vec<StoredComponent>,
    captured: bool,
    /// The components as they were when the entity was created — what
    /// Keep replays, so a creation made while Paused does not carry the
    /// state the simulation gave it after a Resume.
    creation_components: Vec<StoredComponent>,
    /// Whether the entity was still in the world at the last undo. A
    /// creation made while Paused that the simulation later destroyed
    /// comes back `false`, and Keep drops it rather than resurrecting an
    /// empty phantom under its id.
    present_at_undo: bool,
}

impl CreateEntityCommand {
    /// Create from an entity that was already added to the world.
    pub fn already_created(world: &World, entity: EntityId) -> Self {
        let components = capture_all_components(world, entity);
        Self {
            entity,
            creation_components: components.clone(),
            components,
            captured: true,
            present_at_undo: true,
        }
    }
}

impl EditorCommand for CreateEntityCommand {
    fn execute(&mut self, world: &mut World) {
        if self.captured {
            // First execute — entity already exists. Nothing to do.
            self.captured = false;
        } else {
            // Redo — resurrect the entity under its ORIGINAL id:
            // ids are never recycled, so the slot is guaranteed free, and
            // selections / later commands referencing it stay valid.
            world.create_entity_with_id(self.entity);
            restore_components(world, self.entity, &self.components);
        }
    }

    fn undo(&mut self, world: &mut World) {
        // Capture latest component state before removing.
        self.present_at_undo = entity_is_alive(world, self.entity);
        self.components = capture_all_components(world, self.entity);
        world.remove_entity(&self.entity).ok();
        // Any execute after an undo is a redo and must recreate — also for
        // commands pushed via push_already_executed, where execute() was
        // never called and the flag would otherwise still be set.
        self.captured = false;
    }

    fn rebase_onto(&mut self, _world: &mut World) -> Rebase {
        if !self.present_at_undo {
            return Rebase::DROP;
        }
        self.components = self.creation_components.clone();
        Rebase::Apply
    }

    fn display_name(&self) -> &str {
        "Create Entity"
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// ---------------------------------------------------------------------------
// DeleteEntityCommand
// ---------------------------------------------------------------------------

/// Command for deleting an entity.
pub struct DeleteEntityCommand {
    entity: EntityId,
    components: Vec<StoredComponent>,
    parent: Option<EntityId>,
    children: Vec<EntityId>,
}

impl DeleteEntityCommand {
    /// Create a delete command. Components are captured when executed.
    pub fn new(entity: EntityId) -> Self {
        Self {
            entity,
            components: Vec::new(),
            parent: None,
            children: Vec::new(),
        }
    }
}

impl EditorCommand for DeleteEntityCommand {
    fn execute(&mut self, world: &mut World) {
        // Capture component data and hierarchy before removal.
        self.components = capture_all_components(world, self.entity);
        self.parent = world.get_parent(self.entity);
        self.children = world
            .get_children(self.entity)
            .map(|c| c.to_vec())
            .unwrap_or_default();

        // Reparent children to grandparent (or make roots).
        for &child in &self.children {
            if let Some(parent) = self.parent {
                world.set_parent(child, parent).ok();
            } else {
                world.remove_parent(child).ok();
            }
        }

        world.remove_parent(self.entity).ok();
        world.remove_entity(&self.entity).ok();
    }

    fn undo(&mut self, world: &mut World) {
        // Resurrect the entity under its ORIGINAL id: ids are never
        // recycled, so the slot is guaranteed free, and selections / later
        // commands referencing it stay valid across the undo.
        world.create_entity_with_id(self.entity);
        restore_components(world, self.entity, &self.components);

        // Restore hierarchy.
        if let Some(parent) = self.parent {
            world.set_parent(self.entity, parent).ok();
        }
        for &child in &self.children {
            world.set_parent(child, self.entity).ok();
        }
    }

    fn rebase_onto(&mut self, world: &mut World) -> Rebase {
        // Deleting an entity the authored scene never had would let the
        // later undo create a phantom under its id.
        if entity_is_alive(world, self.entity) {
            Rebase::Apply
        } else {
            Rebase::DROP
        }
    }

    fn display_name(&self) -> &str {
        "Delete Entity"
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// ---------------------------------------------------------------------------
// MacroCommand
// ---------------------------------------------------------------------------

/// Groups multiple commands into a single undoable action.
pub struct MacroCommand {
    name: String,
    commands: Vec<Box<dyn EditorCommand>>,
    /// Set by [`EditorCommand::rebase_onto`], which has to RUN each child
    /// to rebase the next one against its result. The re-execute that
    /// follows would otherwise apply the whole batch twice.
    applied_during_rebase: bool,
}

impl MacroCommand {
    pub fn new(name: impl Into<String>, commands: Vec<Box<dyn EditorCommand>>) -> Self {
        Self {
            name: name.into(),
            commands,
            applied_during_rebase: false,
        }
    }
}

impl EditorCommand for MacroCommand {
    fn execute(&mut self, world: &mut World) {
        if std::mem::take(&mut self.applied_during_rebase) {
            return;
        }
        for cmd in &mut self.commands {
            cmd.execute(world);
        }
    }

    fn undo(&mut self, world: &mut World) {
        for cmd in self.commands.iter_mut().rev() {
            cmd.undo(world);
        }
    }

    fn rebase_onto(&mut self, world: &mut World) -> Rebase {
        // Child by child, each against the world the one before it left:
        // rebasing every child against the authored value would let a
        // second whole-component write erase the first edit, and would
        // rebase a create-then-edit batch's edit before its target exists.
        let mut kept: Vec<Box<dyn EditorCommand>> = Vec::with_capacity(self.commands.len());
        let mut dropped = 0;
        for mut command in std::mem::take(&mut self.commands) {
            match command.rebase_onto(world) {
                Rebase::Apply => {
                    command.execute(world);
                    kept.push(command);
                }
                Rebase::Partial { dropped: inner } => {
                    dropped += inner;
                    command.execute(world);
                    kept.push(command);
                }
                Rebase::Drop { entries } => dropped += entries,
            }
        }
        self.commands = kept;
        self.applied_during_rebase = true;
        if self.commands.is_empty() {
            // Every child went: the count travels with the drop, or three
            // lost edits would read as one on the status line.
            Rebase::Drop { entries: dropped.max(1) }
        } else if dropped > 0 {
            Rebase::Partial { dropped }
        } else {
            Rebase::Apply
        }
    }

    fn display_name(&self) -> &str {
        &self.name
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

