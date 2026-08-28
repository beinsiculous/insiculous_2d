//! Commands for adding and removing components on entities.

use std::any::Any;

use ecs::{EntityId, World};

use crate::stored_component::{ComponentKind, StoredComponent};

use super::EditorCommand;

// ---------------------------------------------------------------------------
// AddComponentCommand
// ---------------------------------------------------------------------------

/// Command for adding a default component to an entity.
pub struct AddComponentCommand {
    entity: EntityId,
    kind: ComponentKind,
    /// Captured on undo so that redo can restore modifications made between add and undo.
    captured: Option<StoredComponent>,
}

impl AddComponentCommand {
    pub fn new(entity: EntityId, kind: ComponentKind) -> Self {
        Self {
            entity,
            kind,
            captured: None,
        }
    }
}

impl EditorCommand for AddComponentCommand {
    fn execute(&mut self, world: &mut World) {
        if let Some(ref stored) = self.captured {
            // Redo — restore the captured value.
            stored.apply_to(world, self.entity);
        } else {
            // First execute — add default.
            self.kind.add_default(world, self.entity);
        }
    }

    fn undo(&mut self, world: &mut World) {
        // Capture the component before removing it.
        self.captured = self.kind.capture(world, self.entity);
        self.kind.remove(world, self.entity);
    }

    fn display_name(&self) -> &str {
        "Add Component"
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// ---------------------------------------------------------------------------
// RemoveComponentCommand
// ---------------------------------------------------------------------------

/// Command for removing a component from an entity.
pub struct RemoveComponentCommand {
    entity: EntityId,
    /// Primary stored component.
    stored: Option<StoredComponent>,
    /// Extra component captured during RigidBody cascade (the Collider).
    cascade_stored: Option<StoredComponent>,
    kind: ComponentKind,
}

impl RemoveComponentCommand {
    pub fn new(entity: EntityId, kind: ComponentKind) -> Self {
        Self {
            entity,
            stored: None,
            cascade_stored: None,
            kind,
        }
    }
}

impl EditorCommand for RemoveComponentCommand {
    fn execute(&mut self, world: &mut World) {
        // Capture before removal.
        self.stored = self.kind.capture(world, self.entity);

        // Handle RigidBody → Collider cascade (a collider without a rigid
        // body is meaningless in the physics system).
        if self.kind == ComponentKind::RigidBody {
            self.cascade_stored = ComponentKind::Collider.capture(world, self.entity);
            ComponentKind::Collider.remove(world, self.entity);
        }

        self.kind.remove(world, self.entity);
    }

    fn undo(&mut self, world: &mut World) {
        // Restore primary component.
        if let Some(ref stored) = self.stored {
            stored.apply_to(world, self.entity);
        }
        // Restore cascaded component.
        if let Some(ref stored) = self.cascade_stored {
            stored.apply_to(world, self.entity);
        }
    }

    fn display_name(&self) -> &str {
        "Remove Component"
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// ---------------------------------------------------------------------------
// SetComponentValueCommand
// ---------------------------------------------------------------------------

/// Generic whole-component write used by the command API's `set` verb: the
/// old and new values are captured as [`StoredComponent`]s, so ANY registry
/// component works — including the ones without a typed `Set*Command`
/// (Camera, SpriteAnimation, Tilemap, AudioListener). Never merges: each
/// API `set` line is one discrete undo entry.
pub struct SetComponentValueCommand {
    entity: EntityId,
    old: StoredComponent,
    new: StoredComponent,
    name: String,
}

impl SetComponentValueCommand {
    /// `old` and `new` must store the same component type.
    pub fn new(entity: EntityId, old: StoredComponent, new: StoredComponent) -> Self {
        debug_assert_eq!(old.type_name(), new.type_name());
        let name = format!("Set {} (API)", new.type_name());
        Self { entity, old, new, name }
    }
}

impl EditorCommand for SetComponentValueCommand {
    fn execute(&mut self, world: &mut World) {
        self.new.apply_to(world, self.entity);
    }

    fn undo(&mut self, world: &mut World) {
        self.old.apply_to(world, self.entity);
    }

    fn display_name(&self) -> &str {
        &self.name
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
