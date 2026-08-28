//! Commands for property edits: gizmo drags and inspector field writes.

use std::any::Any;

use ecs::audio_components::AudioSource;
use ecs::behavior::Behavior;
use ecs::sprite_components::{Name, Sprite};
use ecs::ui_components::{UiButton, UiLabel, UiPanel};
use ecs::{EntityId, World};
use physics::components::{Collider, RigidBody};

use super::EditorCommand;

// ---------------------------------------------------------------------------
// TransformGizmoCommand
// ---------------------------------------------------------------------------

/// Command for a transform gizmo drag operation.
///
/// Supports merging: consecutive gizmo drags on the same entity collapse
/// into a single undo entry.
pub struct TransformGizmoCommand {
    entity: EntityId,
    initial: common::Transform2D,
    final_val: common::Transform2D,
}

impl TransformGizmoCommand {
    pub fn new(entity: EntityId, initial: common::Transform2D, final_val: common::Transform2D) -> Self {
        Self { entity, initial, final_val }
    }
}

impl EditorCommand for TransformGizmoCommand {
    fn execute(&mut self, world: &mut World) {
        if let Some(t) = world.get_mut::<common::Transform2D>(self.entity) {
            *t = self.final_val;
        }
    }

    fn undo(&mut self, world: &mut World) {
        if let Some(t) = world.get_mut::<common::Transform2D>(self.entity) {
            *t = self.initial;
        }
    }

    fn display_name(&self) -> &str {
        "Transform Gizmo"
    }

    fn try_merge(&mut self, other: &dyn EditorCommand) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<TransformGizmoCommand>() {
            if self.entity == other.entity {
                self.final_val = other.final_val;
                return true;
            }
        }
        false
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// ---------------------------------------------------------------------------
// Set*Commands (inspector property edits)
// ---------------------------------------------------------------------------

/// Generates a `Set*Command` for an inspector property edit on one component
/// type. All five commands share the same shape: store old/new values plus a
/// `field_hint`, write the value on execute/undo, and merge consecutive edits
/// to the same field on the same entity into one undo entry.
macro_rules! impl_set_component_command {
    ($(#[$attr:meta])* $name:ident, $ty:ty, $display:expr) => {
        $(#[$attr])*
        pub struct $name {
            entity: EntityId,
            old: $ty,
            new: $ty,
            field_hint: &'static str,
        }

        impl $name {
            pub fn new(entity: EntityId, old: $ty, new: $ty, field_hint: &'static str) -> Self {
                Self { entity, old, new, field_hint }
            }
        }

        impl EditorCommand for $name {
            fn execute(&mut self, world: &mut World) {
                if let Some(c) = world.get_mut::<$ty>(self.entity) {
                    *c = Clone::clone(&self.new);
                }
            }

            fn undo(&mut self, world: &mut World) {
                if let Some(c) = world.get_mut::<$ty>(self.entity) {
                    *c = Clone::clone(&self.old);
                }
            }

            fn display_name(&self) -> &str { $display }

            fn try_merge(&mut self, other: &dyn EditorCommand) -> bool {
                if let Some(other) = other.as_any().downcast_ref::<$name>() {
                    if self.entity == other.entity && self.field_hint == other.field_hint {
                        self.new = Clone::clone(&other.new);
                        return true;
                    }
                }
                false
            }

            fn as_any(&self) -> &dyn Any { self }
            fn as_any_mut(&mut self) -> &mut dyn Any { self }
        }
    };
}

impl_set_component_command!(
    /// Command for an inspector property edit on a Transform2D.
    SetTransformCommand, common::Transform2D, "Set Transform");
impl_set_component_command!(
    /// Command for an inspector property edit on a Sprite.
    SetSpriteCommand, Sprite, "Set Sprite");
impl_set_component_command!(
    /// Command for an inspector property edit on a RigidBody.
    SetRigidBodyCommand, RigidBody, "Set RigidBody");
impl_set_component_command!(
    /// Command for an inspector property edit on a Collider.
    SetColliderCommand, Collider, "Set Collider");
impl_set_component_command!(
    /// Command for an inspector property edit on an AudioSource.
    SetAudioSourceCommand, AudioSource, "Set AudioSource");
impl_set_component_command!(
    /// Command for an inspector property edit on a Behavior.
    SetBehaviorCommand, Behavior, "Set Behavior");
impl_set_component_command!(
    /// Command for an inspector property edit on a UiLabel.
    SetUiLabelCommand, UiLabel, "Set UiLabel");
impl_set_component_command!(
    /// Command for an inspector property edit on a UiPanel.
    SetUiPanelCommand, UiPanel, "Set UiPanel");
impl_set_component_command!(
    /// Command for an inspector property edit on a UiButton.
    SetUiButtonCommand, UiButton, "Set UiButton");
impl_set_component_command!(
    /// Command for an inspector property edit on a Name. Like every
    /// macro-generated Set command it writes through `get_mut`, so it
    /// requires the component to already exist (a silent no-op otherwise —
    /// the inspector only renders `edit_name` for entities that have one).
    /// To assign a Name to an entity WITHOUT one, use
    /// [`RenameEntityCommand`], which also undoes back to "no Name".
    SetNameCommand, Name, "Set Name");

// ---------------------------------------------------------------------------
// RenameEntityCommand
// ---------------------------------------------------------------------------

/// Rename an entity from the hierarchy (F2): assigns or replaces its `Name`,
/// including entities that have none yet. Undo restores the prior state —
/// the old name, or no `Name` component at all.
pub struct RenameEntityCommand {
    entity: EntityId,
    old: Option<Name>,
    new: Name,
}

impl RenameEntityCommand {
    /// Capture the entity's current `Name` (if any) and the replacement.
    pub fn new(world: &World, entity: EntityId, new: Name) -> Self {
        Self { entity, old: world.get::<Name>(entity).cloned(), new }
    }
}

impl EditorCommand for RenameEntityCommand {
    fn execute(&mut self, world: &mut World) {
        world.add_component(&self.entity, self.new.clone()).ok();
    }

    fn undo(&mut self, world: &mut World) {
        match &self.old {
            Some(name) => {
                world.add_component(&self.entity, name.clone()).ok();
            }
            None => {
                world.remove_component::<Name>(&self.entity).ok();
            }
        }
    }

    fn display_name(&self) -> &str {
        "Rename Entity"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

