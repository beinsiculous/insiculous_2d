//! Behavior components for entity behaviors
//!
//! This module provides behavior components that define how entities respond
//! to input and game events. Behaviors are data-driven and can be defined
//! in scene files.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::state_machine::StateMachine;

/// Behavior component that defines how an entity responds to input and events.
///
/// Each variant represents a different type of behavior with its own configuration.
/// Behaviors are processed by the `BehaviorRunner` in engine_core.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Behavior {
    /// Player-controlled platformer movement (WASD + Space for jump)
    PlayerPlatformer {
        /// Horizontal movement speed in pixels per second
        #[serde(default = "default_move_speed")]
        move_speed: f32,
        /// Upward impulse applied when jumping
        #[serde(default = "default_jump_impulse")]
        jump_impulse: f32,
        /// Cooldown between jumps in seconds
        #[serde(default = "default_jump_cooldown")]
        jump_cooldown: f32,
        /// Tag to identify this entity (for targeting by other behaviors)
        #[serde(default = "default_player_tag")]
        tag: String,
    },

    /// Player-controlled top-down movement (WASD in all directions)
    PlayerTopDown {
        /// Movement speed in pixels per second
        #[serde(default = "default_move_speed")]
        move_speed: f32,
        /// Tag to identify this entity (for targeting by other behaviors)
        #[serde(default = "default_player_tag")]
        tag: String,
    },

    /// Follow another entity by name
    FollowEntity {
        /// Name of the target entity to follow
        target_name: String,
        /// Minimum distance to maintain from target
        #[serde(default = "default_follow_distance")]
        follow_distance: f32,
        /// Movement speed when following
        #[serde(default = "default_follow_speed")]
        follow_speed: f32,
    },

    /// Follow the nearest entity with a specific tag
    FollowTagged {
        /// Tag of entities to follow
        #[serde(default = "default_player_tag")]
        target_tag: String,
        /// Minimum distance to maintain from target
        #[serde(default = "default_follow_distance")]
        follow_distance: f32,
        /// Movement speed when following
        #[serde(default = "default_follow_speed")]
        follow_speed: f32,
    },

    /// Patrol between two world positions
    Patrol {
        /// First patrol point (x, y)
        point_a: (f32, f32),
        /// Second patrol point (x, y)
        point_b: (f32, f32),
        /// Movement speed
        #[serde(default = "default_patrol_speed")]
        speed: f32,
        /// Time to wait at each point before moving
        #[serde(default = "default_wait_time")]
        wait_time: f32,
    },

    /// Collectible item that can be picked up by entities with a specific tag
    Collectible {
        /// Score value when collected
        #[serde(default = "default_score")]
        score_value: u32,
        /// Whether to despawn when collected
        #[serde(default = "default_true")]
        despawn_on_collect: bool,
        /// Tag of entities that can collect this item
        #[serde(default = "default_player_tag")]
        collector_tag: String,
    },

    /// AI that chases entities with a specific tag when in range
    ChaseTagged {
        /// Tag of entities to chase
        #[serde(default = "default_player_tag")]
        target_tag: String,
        /// Distance at which the entity starts chasing
        #[serde(default = "default_detection_range")]
        detection_range: f32,
        /// Movement speed when chasing
        #[serde(default = "default_chase_speed")]
        chase_speed: f32,
        /// Distance at which the entity stops chasing
        #[serde(default = "default_lose_range")]
        lose_interest_range: f32,
    },

    /// Smoothly move this entity toward the nearest entity with a tag.
    ///
    /// Intended for camera entities (pair with a main `Camera` component so
    /// the render camera tracks it), but works on any entity. The entity
    /// should not carry a `RigidBody` — position is written directly.
    CameraFollow {
        /// Tag of the entity to follow
        #[serde(default = "default_player_tag")]
        target_tag: String,
        /// Fraction of the remaining distance covered per frame at 60 FPS,
        /// 0.0–1.0; 1.0 snaps instantly (dt-corrected at other frame rates)
        #[serde(default = "default_lerp_speed")]
        lerp_speed: f32,
        /// Fixed offset from the target position (x, y)
        #[serde(default)]
        offset: (f32, f32),
        /// Optional dead zone (full width, full height) centered on this
        /// entity: no movement while the target stays inside the box
        #[serde(default)]
        dead_zone: Option<(f32, f32)>,
        /// Maximum look-ahead shift (x, y) in pixels while a direction is
        /// held; (0, 0) disables look-ahead. The direction comes from the
        /// `BehaviorRunner`'s `InputMapping<GameAction>` — the same mapping
        /// `PlayerPlatformer` reads, so camera and player always agree.
        ///
        /// The lead is applied to the focus point *before* the dead-zone
        /// clamp, so a stationary player leads by
        /// `max(0, look_ahead - dead_zone_half_extent)` per axis.
        #[serde(default)]
        look_ahead: (f32, f32),
        /// Fraction of the remaining look-ahead distance covered per frame at
        /// 60 FPS (0.0–1.0), dt-corrected like `lerp_speed`. Deliberately
        /// slower than `lerp_speed` so a quick tap glides instead of snapping.
        #[serde(default = "default_look_ahead_lerp")]
        look_ahead_lerp: f32,
    },
}

// Default value functions for serde
fn default_move_speed() -> f32 { 120.0 }
fn default_jump_impulse() -> f32 { 420.0 }
fn default_jump_cooldown() -> f32 { 0.3 }
fn default_follow_distance() -> f32 { 50.0 }
fn default_follow_speed() -> f32 { 100.0 }
fn default_patrol_speed() -> f32 { 80.0 }
fn default_wait_time() -> f32 { 1.0 }
fn default_score() -> u32 { 10 }
fn default_detection_range() -> f32 { 200.0 }
fn default_chase_speed() -> f32 { 80.0 }
fn default_lose_range() -> f32 { 300.0 }
fn default_true() -> bool { true }
fn default_player_tag() -> String { "player".to_string() }
fn default_lerp_speed() -> f32 { 0.1 }
fn default_look_ahead_lerp() -> f32 { 0.08 }

impl Default for Behavior {
    fn default() -> Self {
        Behavior::PlayerPlatformer {
            move_speed: default_move_speed(),
            jump_impulse: default_jump_impulse(),
            jump_cooldown: default_jump_cooldown(),
            tag: default_player_tag(),
        }
    }
}

impl Behavior {
    /// Variant names in declaration order (indices match `variant_index`).
    pub const VARIANT_NAMES: &'static [&'static str] = &[
        "PlayerPlatformer",
        "PlayerTopDown",
        "FollowEntity",
        "FollowTagged",
        "Patrol",
        "Collectible",
        "ChaseTagged",
        "CameraFollow",
    ];

    /// Display name of this behavior's variant
    pub fn variant_name(&self) -> &'static str {
        Self::VARIANT_NAMES[self.variant_index()]
    }

    /// Index of this behavior's variant within `VARIANT_NAMES`
    pub fn variant_index(&self) -> usize {
        match self {
            Behavior::PlayerPlatformer { .. } => 0,
            Behavior::PlayerTopDown { .. } => 1,
            Behavior::FollowEntity { .. } => 2,
            Behavior::FollowTagged { .. } => 3,
            Behavior::Patrol { .. } => 4,
            Behavior::Collectible { .. } => 5,
            Behavior::ChaseTagged { .. } => 6,
            Behavior::CameraFollow { .. } => 7,
        }
    }

    /// Build a behavior of the given variant index with default field values.
    ///
    /// Indices wrap around, so `default_for_variant(i % VARIANT_NAMES.len())`
    /// callers may pass any index produced by cycling forward or backward.
    pub fn default_for_variant(index: usize) -> Behavior {
        match index % Self::VARIANT_NAMES.len() {
            0 => Behavior::default(),
            1 => Behavior::PlayerTopDown {
                move_speed: default_move_speed(),
                tag: default_player_tag(),
            },
            2 => Behavior::FollowEntity {
                target_name: String::new(),
                follow_distance: default_follow_distance(),
                follow_speed: default_follow_speed(),
            },
            3 => Behavior::FollowTagged {
                target_tag: default_player_tag(),
                follow_distance: default_follow_distance(),
                follow_speed: default_follow_speed(),
            },
            4 => Behavior::Patrol {
                point_a: (0.0, 0.0),
                point_b: (100.0, 0.0),
                speed: default_patrol_speed(),
                wait_time: default_wait_time(),
            },
            5 => Behavior::Collectible {
                score_value: default_score(),
                despawn_on_collect: default_true(),
                collector_tag: default_player_tag(),
            },
            6 => Behavior::ChaseTagged {
                target_tag: default_player_tag(),
                detection_range: default_detection_range(),
                chase_speed: default_chase_speed(),
                lose_interest_range: default_lose_range(),
            },
            _ => Behavior::CameraFollow {
                target_tag: default_player_tag(),
                lerp_speed: default_lerp_speed(),
                offset: (0.0, 0.0),
                dead_zone: None,
                look_ahead: (0.0, 0.0),
                look_ahead_lerp: default_look_ahead_lerp(),
            },
        }
    }
}

// Note: Component trait is implemented via blanket impl in component.rs
// for all types that implement Any + Send + Sync

/// Which patrol endpoint a patrolling entity is headed toward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatrolTarget {
    /// Patrol point A.
    A,
    /// Patrol point B.
    B,
}

impl PatrolTarget {
    /// The opposite endpoint.
    pub fn other(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }
}

/// The phase a stateful behavior is currently in.
///
/// An entity has exactly one `Behavior`, so a single phase enum covers all
/// stateful variants — and makes illegal combinations (waiting AND chasing
/// at once) unrepresentable, unlike the boolean flags this replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorPhase {
    /// No phase-driven activity (initial state; for `ChaseTagged` this
    /// means "not chasing").
    Idle,
    /// `Patrol`: moving toward an endpoint.
    Patrolling {
        /// The endpoint being approached.
        toward: PatrolTarget,
    },
    /// `Patrol`: paused at an endpoint; resumes toward `then_toward` once
    /// the wait elapses (tracked via the state machine's `elapsed()`).
    Waiting {
        /// The endpoint to head for after the wait.
        then_toward: PatrolTarget,
    },
    /// `ChaseTagged`: pursuing the target.
    Chasing,
}

/// Runtime state for behaviors (not serialized in scene files).
///
/// This component stores transient state that behaviors need during
/// execution. It's automatically added by the BehaviorRunner. Phase-driven
/// behaviors (patrol, chase) track their mode in a [`StateMachine`] rather
/// than boolean flags; wait durations use the machine's `elapsed()` clock.
#[derive(Debug, Clone)]
pub struct BehaviorState {
    /// Countdown timer for cooldowns (jump cooldown in `PlayerPlatformer`).
    pub timer: f32,
    /// Phase FSM for patrol/chase behaviors.
    pub phase: StateMachine<BehaviorPhase>,
    /// Smoothed input look-ahead offset in pixels (`CameraFollow`).
    pub look_offset: Vec2,
}

impl Default for BehaviorState {
    fn default() -> Self {
        Self {
            timer: 0.0,
            phase: StateMachine::new(BehaviorPhase::Idle),
            look_offset: Vec2::ZERO,
        }
    }
}

/// Tag component for entity identification.
///
/// Used by behaviors to identify and target entities dynamically.
/// For example, player behaviors add an EntityTag("player"), and
/// ChaseTagged behaviors can target any tag like "player", "enemy", "ally", etc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntityTag(pub String);

impl crate::component_registry::ComponentMeta for Behavior {
    fn type_name() -> &'static str {
        "Behavior"
    }

    fn field_names() -> &'static [&'static str] {
        // Enum — per-variant fields; the editor's behavior_editor owns display.
        &[]
    }
}

impl crate::component_registry::ComponentMeta for EntityTag {
    fn type_name() -> &'static str {
        "EntityTag"
    }

    fn field_names() -> &'static [&'static str] {
        &["0"]
    }
}

impl EntityTag {
    /// Create a new entity tag
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

    /// Check if this tag matches a given string
    pub fn matches(&self, tag: &str) -> bool {
        self.0 == tag
    }
}

impl Default for EntityTag {
    fn default() -> Self {
        Self("player".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_every_variant_round_trips_through_ron_including_option_fields() -> TestResult {
        // An authored CameraFollow with every optional field set.
        let authored = Behavior::CameraFollow {
            target_tag: "player".to_string(),
            lerp_speed: 0.5,
            offset: (0.0, 50.0),
            dead_zone: Some((200.0, 120.0)),
            look_ahead: (220.0, 140.0),
            look_ahead_lerp: 0.05,
        };

        let restored: Behavior = ron::from_str(&ron::to_string(&authored)?)?;

        match restored {
            Behavior::CameraFollow {
                target_tag,
                lerp_speed,
                offset,
                dead_zone,
                look_ahead,
                look_ahead_lerp,
            } => {
                assert_eq!(target_tag, "player");
                assert_eq!(lerp_speed, 0.5);
                assert_eq!(offset, (0.0, 50.0));
                assert_eq!(dead_zone, Some((200.0, 120.0)));
                assert_eq!(look_ahead, (220.0, 140.0));
                assert_eq!(look_ahead_lerp, 0.05);
            }
            other => panic!("wrong variant: {other:?}"),
        }

        // The default of every variant survives the same trip unchanged.
        for (index, name) in Behavior::VARIANT_NAMES.iter().enumerate() {
            let serialized = ron::to_string(&Behavior::default_for_variant(index))?;
            let restored: Behavior = ron::from_str(&serialized)?;
            assert_eq!(restored.variant_index(), index, "{name}");
            assert_eq!(ron::to_string(&restored)?, serialized, "{name} must re-serialize identically");
        }
        Ok(())
    }

    #[test]
    fn test_bare_variant_form_parses_to_the_variant_default() -> TestResult {
        // Hand-written scenes write `PlayerPlatformer()` and get the
        // documented defaults, which are also `Behavior::default()`.
        let parsed: Behavior = ron::from_str("PlayerPlatformer()")?;

        assert_eq!(ron::to_string(&parsed)?, ron::to_string(&Behavior::default())?);
        match parsed {
            Behavior::PlayerPlatformer { move_speed, jump_impulse, jump_cooldown, tag } => {
                assert_eq!(move_speed, 120.0);
                assert_eq!(jump_impulse, 420.0);
                assert_eq!(jump_cooldown, 0.3);
                assert_eq!(tag, "player");
            }
            other => panic!("wrong variant: {other:?}"),
        }

        // The runner gives every scene-loaded Behavior entity this state:
        // Idle, just entered, with nothing accumulated.
        let state = BehaviorState::default();
        assert!(state.phase.is(&BehaviorPhase::Idle));
        assert!(state.phase.just_entered());
        assert_eq!(state.timer, 0.0);
        assert_eq!(state.look_offset, Vec2::ZERO);
        Ok(())
    }

    #[test]
    fn test_camera_follow_parses_legacy_four_field_form() -> TestResult {
        // The bare form carries the documented defaults: look-ahead OFF.
        let bare: Behavior = ron::from_str("CameraFollow()")?;
        let Behavior::CameraFollow {
            dead_zone: default_dead_zone,
            look_ahead: default_look_ahead,
            look_ahead_lerp: default_look_ahead_lerp,
            ..
        } = bare
        else {
            panic!("wrong variant: {bare:?}");
        };
        assert_eq!(default_dead_zone, None);
        assert_eq!(default_look_ahead, (0.0, 0.0), "look-ahead defaults to disabled");
        assert_eq!(default_look_ahead_lerp, 0.08);

        // Exactly the shape shipped scene files used before look-ahead
        // existed: it must still parse, keeping its authored fields and
        // landing the missing ones on those defaults.
        let legacy: Behavior = ron::from_str(
            r#"CameraFollow(
                target_tag: "player",
                lerp_speed: 0.12,
                offset: (0.0, 60.0),
                dead_zone: Some((160.0, 100.0)),
            )"#,
        )?;
        match legacy {
            Behavior::CameraFollow { lerp_speed, dead_zone, look_ahead, look_ahead_lerp, .. } => {
                assert_eq!(lerp_speed, 0.12);
                assert_eq!(dead_zone, Some((160.0, 100.0)));
                assert_eq!(look_ahead, default_look_ahead);
                assert_eq!(look_ahead_lerp, default_look_ahead_lerp);
            }
            other => panic!("wrong variant: {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn test_default_for_variant_round_trips_variant_index_and_wraps() {
        let count = Behavior::VARIANT_NAMES.len();
        for index in 0..count {
            let behavior = Behavior::default_for_variant(index);
            assert_eq!(behavior.variant_index(), index);
            assert_eq!(behavior.variant_name(), Behavior::VARIANT_NAMES[index]);
        }

        // The editor's cycle row steps past the end and wraps.
        assert_eq!(Behavior::default_for_variant(count).variant_index(), 0);
        assert_eq!(Behavior::default_for_variant(count + 2).variant_index(), 2);
    }

    #[test]
    fn test_entity_tag_matches_only_its_own_tag() {
        let tag = EntityTag::new("enemy");

        assert!(tag.matches("enemy"));
        assert!(!tag.matches("player"));
        assert!(!tag.matches("Enemy"), "matching is exact, not case-folded");
    }
}
