//! Commands for property edits: gizmo drags and inspector field writes.

use std::any::Any;

use ecs::audio_components::AudioSource;
use ecs::behavior::{Behavior, EntityTag};
use ecs::component_registry::ComponentMeta;
use ecs::sprite_components::{Name, Sprite};
use ecs::ui_components::{UiButton, UiLabel, UiPanel};
use ecs::{EntityId, World};
use physics::components::{Collider, RigidBody};

use super::EditorCommand;

// ---------------------------------------------------------------------------
// SetComponentCommand (inspector property edits & gizmo drags)
// ---------------------------------------------------------------------------

/// Whole-component write for one component type (inspector fields, gizmo drags).
/// Consecutive edits to the same entity AND the same `field_hint` merge into one undo
/// entry. Distinct `T`s are distinct types, so `downcast_ref::<Self>` keeps merge
/// isolation per component exactly as the thirteen macro-generated structs did.
pub struct SetComponentCommand<T: ecs::Component + ComponentMeta + Clone + Send + 'static> {
    entity: EntityId,
    old: T,
    new: T,
    field_hint: &'static str,
    display: String,
}

impl<T: ecs::Component + ComponentMeta + Clone + Send + 'static> SetComponentCommand<T> {
    pub fn new(entity: EntityId, old: T, new: T, field_hint: &'static str) -> Self {
        Self {
            entity,
            old,
            new,
            field_hint,
            display: format!("Set {}", <T as ComponentMeta>::type_name()),
        }
    }
}

impl<T: ecs::Component + ComponentMeta + Clone + Send + 'static> EditorCommand for SetComponentCommand<T> {
    fn execute(&mut self, world: &mut World) {
        if let Some(c) = world.get_mut::<T>(self.entity) {
            *c = self.new.clone();
        }
    }

    fn undo(&mut self, world: &mut World) {
        if let Some(c) = world.get_mut::<T>(self.entity) {
            *c = self.old.clone();
        }
    }

    fn display_name(&self) -> &str {
        &self.display
    }

    fn try_merge(&mut self, other: &dyn EditorCommand) -> bool {
        match other.as_any().downcast_ref::<Self>() {
            Some(o) if o.entity == self.entity && o.field_hint == self.field_hint => {
                self.new = o.new.clone();
                true
            }
            _ => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Gizmo drags are Transform2D sets under this hint: they merge with each other and
/// never with an inspector field edit ("position", "rotation", ...).
pub const GIZMO_FIELD_HINT: &str = "gizmo";

pub type SetTransformCommand = SetComponentCommand<common::Transform2D>;
pub type SetSpriteCommand = SetComponentCommand<Sprite>;
pub type SetRigidBodyCommand = SetComponentCommand<RigidBody>;
pub type SetColliderCommand = SetComponentCommand<Collider>;
pub type SetAudioSourceCommand = SetComponentCommand<AudioSource>;
pub type SetBehaviorCommand = SetComponentCommand<Behavior>;
pub type SetUiLabelCommand = SetComponentCommand<UiLabel>;
pub type SetUiPanelCommand = SetComponentCommand<UiPanel>;
pub type SetUiButtonCommand = SetComponentCommand<UiButton>;
pub type SetEntityTagCommand = SetComponentCommand<EntityTag>;
pub type SetScriptsCommand = SetComponentCommand<ecs::script::Scripts>;
pub type SetGridBackdropCommand = SetComponentCommand<ecs::GridBackdrop>;
pub type SetNameCommand = SetComponentCommand<Name>;


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


// ---------------------------------------------------------------------------
// NudgeCommand
// ---------------------------------------------------------------------------

/// One arrow-key nudge of the selection roots.
///
/// OS key-repeat machine-guns `on_key_pressed`, so consecutive nudges over
/// the SAME entity set merge into one history entry (each keeps the first
/// `old` and adopts the latest `new`); the caller seals the entry with
/// `CommandHistory::break_merge()` on key release, giving one undo step per
/// key hold. Being a distinct type, it can never merge into a preceding
/// gizmo-drag entry.
pub struct NudgeCommand {
    /// Per entity: `(id, old_position, new_position)`
    moves: Vec<(EntityId, glam::Vec2, glam::Vec2)>,
}

impl NudgeCommand {
    pub fn new(moves: Vec<(EntityId, glam::Vec2, glam::Vec2)>) -> Self {
        Self { moves }
    }
}

impl EditorCommand for NudgeCommand {
    fn execute(&mut self, world: &mut World) {
        for (entity, _, new_pos) in &self.moves {
            if let Some(t) = world.get_mut::<common::Transform2D>(*entity) {
                t.position = *new_pos;
            }
        }
    }

    fn undo(&mut self, world: &mut World) {
        for (entity, old_pos, _) in &self.moves {
            if let Some(t) = world.get_mut::<common::Transform2D>(*entity) {
                t.position = *old_pos;
            }
        }
    }

    fn display_name(&self) -> &str {
        "Nudge"
    }

    fn try_merge(&mut self, other: &dyn EditorCommand) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<NudgeCommand>() {
            let same_set = self.moves.len() == other.moves.len()
                && self
                    .moves
                    .iter()
                    .zip(&other.moves)
                    .all(|(a, b)| a.0 == b.0);
            if same_set {
                for (mine, theirs) in self.moves.iter_mut().zip(&other.moves) {
                    mine.2 = theirs.2;
                }
                return true;
            }
        }
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
