//! Read-only snapshots handed to scripts each phase.

use std::collections::{BTreeMap, HashSet};

use glam::Vec2;

use common::Transform2D;
use ecs::script::ScriptValue;
use ecs::EntityId;

/// Per-instance view handed to scripts as `me`.
#[derive(Debug, Clone)]
pub struct SelfView {
    /// Entity owning this script instance.
    pub entity: EntityId,
    /// Name component value if attached to this entity.
    pub name: Option<String>,
    /// Pre-step (early update) or post-step (update) transform.
    pub transform: Transform2D,
    /// Current linear velocity.
    pub velocity: Vec2,
}

impl SelfView {
    /// Create a new self view.
    pub fn new(
        entity: EntityId,
        name: Option<String>,
        transform: Transform2D,
        velocity: Vec2,
    ) -> Self {
        Self {
            entity,
            name,
            transform,
            velocity,
        }
    }

    /// Entity ID of this instance.
    pub fn entity(&self) -> EntityId {
        self.entity
    }

    /// Entity name if present.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Entity transform.
    pub fn transform(&self) -> Transform2D {
        self.transform
    }

    /// Entity position.
    pub fn position(&self) -> Vec2 {
        self.transform.position
    }

    /// Entity rotation in radians.
    pub fn rotation(&self) -> f32 {
        self.transform.rotation
    }

    /// Entity linear velocity.
    pub fn velocity(&self) -> Vec2 {
        self.velocity
    }
}

/// Recorded contact/collision between two entities during a physics phase.
#[derive(Debug, Clone)]
pub struct ScriptCollision {
    pub entity_a: EntityId,
    pub entity_b: EntityId,
    pub name_a: Option<String>,
    pub name_b: Option<String>,
    pub started: bool,
    pub stopped: bool,
}

/// Transform and velocity for one named entity in the scene.
#[derive(Debug, Clone, Copy)]
pub struct EntityState {
    pub entity: EntityId,
    pub transform: Transform2D,
    pub velocity: Vec2,
}

/// Phase-shared view accessible by every script running in that phase.
#[derive(Debug, Clone, Default)]
pub struct ScriptView {
    pub delta_time: f32,
    pub frame: u32,
    pub blackboard: BTreeMap<String, ScriptValue>,
    pub entities: BTreeMap<String, EntityState>,
    pub collisions: Vec<ScriptCollision>,
    pub player_axes: Vec<Vec2>,
    pub player_actions_active: Vec<HashSet<String>>,
    pub player_actions_just_activated: Vec<HashSet<String>>,
}

impl ScriptView {
    /// Delta time in seconds for this frame.
    pub fn delta_time(&self) -> f32 {
        self.delta_time
    }

    /// Running frame counter.
    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// Check if a named entity exists in the view.
    pub fn has_entity(&self, name: &str) -> bool {
        self.entities.contains_key(name)
    }

    /// Transform of a named entity.
    pub fn transform(&self, name: &str) -> Option<Transform2D> {
        self.entities.get(name).map(|s| s.transform)
    }

    /// Position of a named entity.
    pub fn position(&self, name: &str) -> Option<Vec2> {
        self.entities.get(name).map(|s| s.transform.position)
    }

    /// Linear velocity of a named entity.
    pub fn velocity(&self, name: &str) -> Option<Vec2> {
        self.entities.get(name).map(|s| s.velocity)
    }

    /// Read a boolean from the blackboard with fallback default.
    pub fn blackboard_bool(&self, key: &str, default: bool) -> bool {
        match self.blackboard.get(key) {
            Some(ScriptValue::Bool(b)) => *b,
            _ => default,
        }
    }

    /// Read an integer from the blackboard with fallback default.
    pub fn blackboard_int(&self, key: &str, default: i32) -> i32 {
        match self.blackboard.get(key) {
            Some(ScriptValue::I32(i)) => *i,
            Some(ScriptValue::F32(f)) => *f as i32,
            _ => default,
        }
    }

    /// Read a float from the blackboard with fallback default.
    pub fn blackboard_float(&self, key: &str, default: f32) -> f32 {
        match self.blackboard.get(key) {
            Some(ScriptValue::F32(f)) => *f,
            Some(ScriptValue::I32(i)) => *i as f32,
            _ => default,
        }
    }

    /// Read a string from the blackboard with fallback default.
    pub fn blackboard_str(&self, key: &str, default: &str) -> String {
        match self.blackboard.get(key) {
            Some(ScriptValue::Str(s)) => s.clone(),
            _ => default.to_string(),
        }
    }

    /// Horizontal movement input for player (0-indexed).
    pub fn move_x(&self, player: usize) -> f32 {
        self.player_axes.get(player).map(|v| v.x).unwrap_or(0.0)
    }

    /// Vertical movement input for player (0-indexed).
    pub fn move_y(&self, player: usize) -> f32 {
        self.player_axes.get(player).map(|v| v.y).unwrap_or(0.0)
    }

    /// Check if action is currently active for player.
    pub fn is_active(&self, player: usize, action: &str) -> bool {
        self.player_actions_active
            .get(player)
            .map(|set| set.contains(&action.to_ascii_lowercase()))
            .unwrap_or(false)
    }

    /// Check if action was just activated this frame for player.
    pub fn just_activated(&self, player: usize, action: &str) -> bool {
        self.player_actions_just_activated
            .get(player)
            .map(|set| set.contains(&action.to_ascii_lowercase()))
            .unwrap_or(false)
    }

    /// Whether a contact between two named entities STARTED this frame. The view carries
    /// start and stop events, never ongoing contact: a ground check polled here is true
    /// for exactly one frame.
    pub fn has_collision(&self, a: &str, b: &str) -> bool {
        self.has_collision_started(a, b)
    }

    /// Check if a collision started between two named entities.
    pub fn has_collision_started(&self, a: &str, b: &str) -> bool {
        self.collisions.iter().any(|c| {
            if !c.started {
                return false;
            }
            match (&c.name_a, &c.name_b) {
                (Some(na), Some(nb)) => (na == a && nb == b) || (na == b && nb == a),
                _ => false,
            }
        })
    }

    /// Check if a collision stopped between two named entities.
    pub fn has_collision_stopped(&self, a: &str, b: &str) -> bool {
        self.collisions.iter().any(|c| {
            if !c.stopped {
                return false;
            }
            match (&c.name_a, &c.name_b) {
                (Some(na), Some(nb)) => (na == a && nb == b) || (na == b && nb == a),
                _ => false,
            }
        })
    }

    /// Check if an instance entity collided with a named entity.
    pub fn has_collision_with(&self, me: &SelfView, other: &str) -> bool {
        self.collisions.iter().any(|c| {
            if !c.started {
                return false;
            }
            if c.entity_a == me.entity && c.name_b.as_deref() == Some(other) {
                return true;
            }
            if c.entity_b == me.entity && c.name_a.as_deref() == Some(other) {
                return true;
            }
            false
        })
    }
}
