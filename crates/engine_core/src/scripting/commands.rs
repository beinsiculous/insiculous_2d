//! Commands emitted by scripts and applied at the end of each update phase.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use glam::{Vec2, Vec4};

use common::Transform2D;
use ecs::blackboard::Blackboard;
use ecs::script::ScriptValue;
use ecs::sprite_components::{Name, Sprite};
use ecs::ui_components::UiLabel;
use ecs::{EntityId, World};
use physics::{PhysicsSystem, RigidBody, RigidBodyType};

use super::view::SelfView;

/// Target of a script command: an explicit entity ID or a resolved name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Target {
    /// Target by explicit entity ID.
    Entity(EntityId),
    /// Target by entity name component.
    Named(String),
}

impl From<&SelfView> for Target {
    fn from(me: &SelfView) -> Self {
        Target::Entity(me.entity)
    }
}

impl From<EntityId> for Target {
    fn from(id: EntityId) -> Self {
        Target::Entity(id)
    }
}

impl From<&str> for Target {
    fn from(name: &str) -> Self {
        Target::Named(name.to_string())
    }
}

impl From<String> for Target {
    fn from(name: String) -> Self {
        Target::Named(name)
    }
}

/// One deferred command emitted by a script.
#[derive(Debug, Clone)]
pub enum ScriptCommand {
    ResetBody { target: Target, position: Vec2 },
    SetPosition { target: Target, position: Vec2 },
    SetRotation { target: Target, radians: f32 },
    SetVelocity { target: Target, velocity: Vec2 },
    /// One axis of velocity. Velocity commands for one entity fold in push order into a
    /// single write, so `set_velocity_x` then `set_velocity_y` in one hook lands as both,
    /// and an axis nobody set keeps the value it has when the commands apply.
    SetVelocityX { target: Target, x: f32 },
    SetVelocityY { target: Target, y: f32 },
    SetKinematicTarget { target: Target, position: Vec2 },
    SetSpriteColor { target: Target, color: [f32; 4] },
    SetSpriteVisible { target: Target, visible: bool },
    SetLabelText { target: Target, text: String },
    SetBlackboard { key: String, value: ScriptValue },
    Despawn { target: Target },
}

/// Buffer of commands returned by scripts.
#[derive(Debug, Default, Clone)]
pub struct ScriptCommands {
    pub(crate) commands: Vec<ScriptCommand>,
}

impl ScriptCommands {
    /// Create an empty command buffer.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Reset a physics body's position and clear its velocity.
    pub fn reset_body(&mut self, target: impl Into<Target>, position: Vec2) {
        self.commands.push(ScriptCommand::ResetBody {
            target: target.into(),
            position,
        });
    }

    /// Set an entity's transform position directly.
    pub fn set_position(&mut self, target: impl Into<Target>, position: Vec2) {
        self.commands.push(ScriptCommand::SetPosition {
            target: target.into(),
            position,
        });
    }

    /// Set an entity's transform rotation in radians.
    pub fn set_rotation(&mut self, target: impl Into<Target>, radians: f32) {
        self.commands.push(ScriptCommand::SetRotation {
            target: target.into(),
            radians,
        });
    }

    /// Set linear velocity.
    pub fn set_velocity(&mut self, target: impl Into<Target>, velocity: Vec2) {
        self.commands.push(ScriptCommand::SetVelocity {
            target: target.into(),
            velocity,
        });
    }

    /// Set the horizontal velocity, keeping the vertical one.
    pub fn set_velocity_x(&mut self, target: impl Into<Target>, x: f32) {
        self.commands.push(ScriptCommand::SetVelocityX { target: target.into(), x });
    }

    /// Set the vertical velocity, keeping the horizontal one.
    pub fn set_velocity_y(&mut self, target: impl Into<Target>, y: f32) {
        self.commands.push(ScriptCommand::SetVelocityY { target: target.into(), y });
    }

    /// Set kinematic target position for kinematic bodies.
    pub fn set_kinematic_target(&mut self, target: impl Into<Target>, position: Vec2) {
        self.commands.push(ScriptCommand::SetKinematicTarget {
            target: target.into(),
            position,
        });
    }

    /// Set sprite color tint.
    pub fn set_sprite_color(&mut self, target: impl Into<Target>, color: [f32; 4]) {
        self.commands.push(ScriptCommand::SetSpriteColor {
            target: target.into(),
            color,
        });
    }

    /// Set sprite visibility.
    pub fn set_sprite_visible(&mut self, target: impl Into<Target>, visible: bool) {
        self.commands.push(ScriptCommand::SetSpriteVisible {
            target: target.into(),
            visible,
        });
    }

    /// Set UI label text.
    pub fn set_label_text(&mut self, target: impl Into<Target>, text: impl Into<String>) {
        self.commands.push(ScriptCommand::SetLabelText {
            target: target.into(),
            text: text.into(),
        });
    }

    /// Set a blackboard value.
    pub fn set_blackboard(&mut self, key: impl Into<String>, value: ScriptValue) {
        self.commands.push(ScriptCommand::SetBlackboard {
            key: key.into(),
            value,
        });
    }

    /// Set a boolean blackboard value.
    pub fn set_blackboard_bool(&mut self, key: impl Into<String>, value: bool) {
        self.set_blackboard(key, ScriptValue::Bool(value));
    }

    /// Set an integer blackboard value.
    pub fn set_blackboard_int(&mut self, key: impl Into<String>, value: i32) {
        self.set_blackboard(key, ScriptValue::I32(value));
    }

    /// Set a float blackboard value.
    pub fn set_blackboard_float(&mut self, key: impl Into<String>, value: f32) {
        self.set_blackboard(key, ScriptValue::F32(value));
    }

    /// Set a string blackboard value.
    pub fn set_blackboard_str(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.set_blackboard(key, ScriptValue::Str(value.into()));
    }

    /// Despawn an entity.
    pub fn despawn(&mut self, target: impl Into<Target>) {
        self.commands.push(ScriptCommand::Despawn {
            target: target.into(),
        });
    }

    /// Apply commands to the world and physics systems by kind order.
    pub fn apply(
        self,
        world: &mut World,
        mut physics: Option<&mut PhysicsSystem>,
        delta_time: f32,
        errors: &mut super::runner::ScriptErrors,
    ) {
        let mut name_to_entities: HashMap<String, Vec<EntityId>> = HashMap::new();
        for entity in world.entities() {
            if let Some(name) = world.get::<Name>(entity) {
                name_to_entities
                    .entry(name.as_str().to_string())
                    .or_default()
                    .push(entity);
            }
        }

        let mut resolve_target = |target: &Target| -> Option<EntityId> {
            match target {
                Target::Entity(id) => {
                    if world.validate_entity(id).is_ok() {
                        Some(*id)
                    } else {
                        None
                    }
                }
                Target::Named(name) => match name_to_entities.get(name) {
                    None => {
                        // The name is the dedupe key: one report per missing name per Play.
                        errors.push(super::runner::ScriptError {
                            file: name.clone(),
                            line: 0,
                            kind: super::runner::ScriptErrorKind::MissingTarget,
                            message: format!("Target entity named '{name}' not found"),
                        });
                        None
                    }
                    Some(matches) if matches.len() > 1 => {
                        errors.push(super::runner::ScriptError {
                            file: name.clone(),
                            line: 0,
                            kind: super::runner::ScriptErrorKind::AmbiguousTarget,
                            message: format!(
                                "Target entity named '{name}' is ambiguous ({} entities match)",
                                matches.len()
                            ),
                        });
                        None
                    }
                    Some(matches) => Some(matches[0]),
                },
            }
        };

        let mut resets: Vec<(EntityId, Vec2)> = Vec::new();
        let mut positions: Vec<(EntityId, Vec2)> = Vec::new();
        let mut rotations: Vec<(EntityId, f32)> = Vec::new();
        let mut velocities: BTreeMap<EntityId, Vec2> = BTreeMap::new();
        let mut kinematic_targets: Vec<(EntityId, Vec2)> = Vec::new();
        let mut sprite_colors: Vec<(EntityId, [f32; 4])> = Vec::new();
        let mut sprite_visibilities: Vec<(EntityId, bool)> = Vec::new();
        let mut label_texts: Vec<(EntityId, String)> = Vec::new();
        let mut blackboard_writes: Vec<(String, ScriptValue)> = Vec::new();
        let mut despawns: Vec<EntityId> = Vec::new();

        for cmd in self.commands {
            match cmd {
                ScriptCommand::ResetBody { target, position } => {
                    if let Some(e) = resolve_target(&target) {
                        resets.push((e, position));
                    }
                }
                ScriptCommand::SetPosition { target, position } => {
                    if let Some(e) = resolve_target(&target) {
                        positions.push((e, position));
                    }
                }
                ScriptCommand::SetRotation { target, radians } => {
                    if let Some(e) = resolve_target(&target) {
                        rotations.push((e, radians));
                    }
                }
                ScriptCommand::SetVelocity { target, velocity } => {
                    if let Some(e) = resolve_target(&target) {
                        velocities.insert(e, velocity);
                    }
                }
                ScriptCommand::SetVelocityX { target, x } => {
                    if let Some(e) = resolve_target(&target) {
                        velocities
                            .entry(e)
                            .or_insert_with(|| current_velocity(world, physics.as_deref(), e))
                            .x = x;
                    }
                }
                ScriptCommand::SetVelocityY { target, y } => {
                    if let Some(e) = resolve_target(&target) {
                        velocities
                            .entry(e)
                            .or_insert_with(|| current_velocity(world, physics.as_deref(), e))
                            .y = y;
                    }
                }
                ScriptCommand::SetKinematicTarget { target, position } => {
                    if let Some(e) = resolve_target(&target) {
                        kinematic_targets.push((e, position));
                    }
                }
                ScriptCommand::SetSpriteColor { target, color } => {
                    if let Some(e) = resolve_target(&target) {
                        sprite_colors.push((e, color));
                    }
                }
                ScriptCommand::SetSpriteVisible { target, visible } => {
                    if let Some(e) = resolve_target(&target) {
                        sprite_visibilities.push((e, visible));
                    }
                }
                ScriptCommand::SetLabelText { target, text } => {
                    if let Some(e) = resolve_target(&target) {
                        label_texts.push((e, text));
                    }
                }
                ScriptCommand::SetBlackboard { key, value } => {
                    blackboard_writes.push((key, value));
                }
                ScriptCommand::Despawn { target } => {
                    if let Some(e) = resolve_target(&target) {
                        despawns.push(e);
                    }
                }
            }
        }

        // 1. Resets first
        let mut reset_entities = HashSet::new();
        for (entity, pos) in resets {
            reset_entities.insert(entity);
            if let Some(ref mut phys) = physics {
                phys.reset_body(entity, pos);
            }
            if let Some(transform) = world.get_mut::<Transform2D>(entity) {
                transform.position = pos;
            }
        }

        // 2. Positions and rotations
        for (entity, pos) in positions {
            if let Some(transform) = world.get_mut::<Transform2D>(entity) {
                transform.position = pos;
            }
        }
        for (entity, rad) in rotations {
            if let Some(transform) = world.get_mut::<Transform2D>(entity) {
                transform.rotation = rad;
            }
        }

        // 3. Velocities and kinematic targets (dropping velocity for reset entities)
        if let Some(ref mut phys) = physics {
            for (entity, vel) in velocities {
                if reset_entities.contains(&entity) {
                    continue;
                }
                let is_kinematic = world
                    .get::<RigidBody>(entity)
                    .map(|rb| rb.body_type == RigidBodyType::Kinematic)
                    .unwrap_or(false);

                if is_kinematic {
                    if let Some(transform) = world.get::<Transform2D>(entity) {
                        let new_pos = transform.position + vel * delta_time;
                        phys.physics_world_mut()
                            .set_kinematic_target(entity, new_pos, 0.0);
                    }
                } else {
                    phys.set_velocity(entity, vel, 0.0);
                }
            }
            for (entity, target_pos) in kinematic_targets {
                if world.get::<RigidBody>(entity).is_some() {
                    phys.physics_world_mut()
                        .set_kinematic_target(entity, target_pos, 0.0);
                } else if let Some(transform) = world.get_mut::<Transform2D>(entity) {
                    transform.position = target_pos;
                }
            }
        } else {
            // No physics fallback: integrate velocity into transform directly
            for (entity, vel) in velocities {
                if reset_entities.contains(&entity) {
                    continue;
                }
                if let Some(transform) = world.get_mut::<Transform2D>(entity) {
                    transform.position += vel * delta_time;
                }
            }
            for (entity, target_pos) in kinematic_targets {
                if let Some(transform) = world.get_mut::<Transform2D>(entity) {
                    transform.position = target_pos;
                }
            }
        }

        // 4. Sprite / label / blackboard writes
        for (entity, color) in sprite_colors {
            if let Some(sprite) = world.get_mut::<Sprite>(entity) {
                sprite.color = Vec4::from(color);
            }
        }
        for (entity, visible) in sprite_visibilities {
            if let Some(sprite) = world.get_mut::<Sprite>(entity) {
                sprite.visible = visible;
            }
        }
        for (entity, text) in label_texts {
            if let Some(label) = world.get_mut::<UiLabel>(entity) {
                label.text = text;
            }
        }
        for (key, val) in blackboard_writes {
            if let Some(bb) = world.resource_mut::<Blackboard>() {
                bb.set(key, val);
            } else {
                let mut bb = Blackboard::new();
                bb.set(key, val);
                world.insert_resource(bb);
            }
        }

        // 5. Despawns last
        for entity in despawns {
            let _ = world.remove_entity(&entity);
        }
    }
}

/// An entity's linear velocity as the scripts see it: the live physics body when there is
/// one, else the `RigidBody` component's value, else zero.
pub(super) fn current_velocity(
    world: &World,
    physics: Option<&PhysicsSystem>,
    entity: EntityId,
) -> Vec2 {
    physics
        .and_then(|system| system.physics_world().get_body_velocity(entity).map(|(linear, _)| linear))
        .or_else(|| world.get::<RigidBody>(entity).map(|body| body.velocity))
        .unwrap_or(Vec2::ZERO)
}

/// Shared handle to command buffer passed by value into Rhai scripts.
#[derive(Clone, Default)]
pub struct ScriptCommandsHandle(pub Rc<RefCell<Vec<ScriptCommand>>>);

impl ScriptCommandsHandle {
    /// Create a new shared handle.
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(Vec::new())))
    }

    /// Push a command into the shared buffer.
    pub fn push(&self, cmd: ScriptCommand) {
        self.0.borrow_mut().push(cmd);
    }

    /// Drain all commands from the shared buffer.
    pub fn drain(&self) -> Vec<ScriptCommand> {
        self.0.borrow_mut().drain(..).collect()
    }
}
