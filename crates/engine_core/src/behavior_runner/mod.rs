//! Behavior runner for processing entity behaviors
//!
//! This module provides the `BehaviorRunner` which processes all entities
//! that have `Behavior` components, handling input-driven movement,
//! AI behaviors, and other game logic.
//!
//! The logic is split across child modules: [`handlers`] holds the
//! player/AI behavior handlers, [`camera`] holds `CameraFollow`.

mod camera;
mod handlers;
use handlers::HandlerFrame;

use std::collections::HashMap;

use glam::Vec2;

use ecs::behavior::{Behavior, BehaviorState, EntityTag};
use ecs::sprite_components::Transform2D;
use ecs::{EntityId, World};
use input::{GameAction, InputHandler, InputMapping};
use physics::{PhysicsSystem, RigidBody, RigidBodyType};

/// Event emitted when a collectible entity is picked up.
/// Read these via `world.read_events::<EntityCollected>()` to update
/// score, play sounds, spawn particles, etc.
#[derive(Debug, Clone)]
pub struct EntityCollected {
    /// The collectible entity that was collected (may already be despawned)
    pub entity: EntityId,
    /// The score value of the collectible
    pub score_value: u32,
    /// The collector entity's tag
    pub collector_tag: String,
}

/// Commands collected while iterating behaviors, applied after the loop to
/// avoid borrow conflicts with the world.
#[derive(Default)]
pub(super) struct BehaviorCommands {
    /// Entities to despawn after processing
    to_despawn: Vec<EntityId>,
    /// Velocity commands (applied via physics or direct transform)
    velocities: Vec<(EntityId, Vec2)>,
    /// Absolute position sets (direct transform write, bypasses physics —
    /// used by CameraFollow, whose entities carry no RigidBody)
    positions: Vec<(EntityId, Vec2)>,
    /// Impulse commands, applied AFTER velocity commands
    impulses: Vec<(EntityId, Vec2)>,
    /// Tag assignments
    tags: Vec<(EntityId, String)>,
    /// Collection events to emit
    collected: Vec<EntityCollected>,
}

/// Processes behavior components for all entities.
///
/// The `BehaviorRunner` iterates over all entities with `Behavior` components
/// and executes the appropriate behavior logic. This should be called from
/// `Game::update()` where input is available.
///
/// # Example
///
/// ```
/// use engine_core::prelude::*;
///
/// struct MyGame {
///     behaviors: BehaviorRunner,
///     physics: Option<PhysicsSystem>,
/// }
///
/// impl Game for MyGame {
///     fn update(&mut self, ctx: &mut GameContext) {
///         self.behaviors.update(
///             ctx.world,
///             ctx.input,
///             ctx.delta_time,
///             self.physics.as_mut(),
///         );
///     }
/// }
/// ```
pub struct BehaviorRunner {
    /// Named entity lookup (populated from SceneInstance)
    named_entities: HashMap<String, EntityId>,
    /// Action bindings for input-driven behaviors (engine default preset)
    actions: InputMapping<GameAction>,
}

impl Default for BehaviorRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl BehaviorRunner {
    /// Create a new behavior runner with the engine's default action bindings
    pub fn new() -> Self {
        Self {
            named_entities: HashMap::new(),
            actions: InputMapping::with_default_bindings(),
        }
    }

    /// Get a mutable reference to the action bindings used by input-driven
    /// behaviors (e.g. to rebind `PlayerControlled` movement keys)
    pub fn actions_mut(&mut self) -> &mut InputMapping<GameAction> {
        &mut self.actions
    }

    /// Set named entities from a SceneInstance
    ///
    /// This allows behaviors like `FollowEntity` to reference entities by name.
    pub fn set_named_entities(&mut self, named: HashMap<String, EntityId>) {
        self.named_entities = named;
    }

    /// Update all entities with behaviors
    ///
    /// This processes all entities that have a `Behavior` component,
    /// executing the appropriate behavior logic for each.
    ///
    /// Should be called from `Game::update()` before physics simulation.
    pub fn update(
        &mut self,
        world: &mut World,
        input: &InputHandler,
        delta_time: f32,
        physics: Option<&mut PhysicsSystem>,
    ) {
        // Commands are collected during iteration and applied afterwards
        // (avoids borrow conflicts with the world).
        let mut commands = BehaviorCommands::default();
        let frame = HandlerFrame { input, delta_time, physics: physics.as_deref() };

        // Process all entities with behaviors directly - avoid cloning
        for entity in world.entities() {
            // Get behavior component by reference to avoid cloning
            let Some(behavior) = world.get::<Behavior>(entity) else { continue };

            // Clone state only when needed (much smaller than Behavior)
            let mut state = world
                .get::<BehaviorState>(entity)
                .cloned()
                .unwrap_or_default();

            match behavior {
                Behavior::PlayerPlatformer { .. } => {
                    self.update_player_platformer(entity, &frame, behavior, &mut state, &mut commands);
                    Self::update_state(world, entity, state);
                }

                Behavior::PlayerTopDown { move_speed, tag } => {
                    self.update_player_top_down(entity, input, *move_speed, tag, &mut commands);
                }

                Behavior::ChaseTagged { .. } => {
                    Self::update_chase_tagged(world, entity, &frame, behavior, &mut state, &mut commands);
                    Self::update_state(world, entity, state);
                }

                Behavior::Patrol { .. } => {
                    Self::update_patrol(world, entity, &frame, behavior, &mut state, &mut commands);
                    Self::update_state(world, entity, state);
                }

                Behavior::FollowEntity { target_name, follow_distance, follow_speed } => {
                    self.update_follow_entity(
                        world, entity, target_name, *follow_distance, *follow_speed, &mut commands,
                    );
                }

                Behavior::FollowTagged { target_tag, follow_distance, follow_speed } => {
                    Self::update_follow_tagged(
                        world, entity, target_tag, *follow_distance, *follow_speed, &mut commands,
                    );
                }

                Behavior::Collectible { score_value, despawn_on_collect, collector_tag } => {
                    Self::update_collectible(
                        world, entity, *score_value, *despawn_on_collect, collector_tag, &mut commands,
                    );
                }

                Behavior::CameraFollow {
                    target_tag, lerp_speed, offset, dead_zone, look_ahead, look_ahead_lerp,
                } => {
                    let params = camera::CameraFollowParams {
                        target_tag,
                        lerp_speed: *lerp_speed,
                        offset: *offset,
                        dead_zone: *dead_zone,
                        look_ahead: *look_ahead,
                        look_ahead_lerp: *look_ahead_lerp,
                    };
                    if let Some(new_pos) = self.update_camera_follow(
                        world, entity, input, delta_time, &params, &mut state,
                    ) {
                        commands.positions.push((entity, new_pos));
                    }
                    Self::update_state(world, entity, state);
                }
            }
        }

        Self::apply_commands(world, physics, delta_time, commands);
    }

    /// Apply the commands collected during behavior iteration: tags first,
    /// then velocities, then impulses, then events and despawns.
    fn apply_commands(
        world: &mut World,
        mut physics: Option<&mut PhysicsSystem>,
        delta_time: f32,
        commands: BehaviorCommands,
    ) {
        // Apply tag assignments
        for (entity, tag) in commands.tags {
            Self::add_entity_tag(world, entity, &tag);
        }

        // Apply velocity commands (either via physics or direct transform)
        if let Some(ref mut physics) = physics {
            for (entity, vel) in commands.velocities {
                // Check if entity is kinematic - kinematic bodies need position-based movement
                let is_kinematic = world
                    .get::<RigidBody>(entity)
                    .map(|rb| rb.body_type == RigidBodyType::Kinematic)
                    .unwrap_or(false);

                if is_kinematic {
                    // Kinematic bodies: use set_kinematic_target for proper physics interaction
                    if let Some(current_pos) = Self::get_position(world, entity) {
                        let new_pos = current_pos + vel * delta_time;
                        physics.physics_world_mut().set_kinematic_target(entity, new_pos, 0.0);
                    }
                } else {
                    // Dynamic bodies: set velocity and let physics handle movement
                    physics.set_velocity(entity, vel, 0.0);
                }
            }
        } else {
            // Fallback: direct transform modification (no physics)
            for (entity, vel) in commands.velocities {
                if let Some(transform) = world.get_mut::<Transform2D>(entity) {
                    transform.position += vel * delta_time;
                }
            }
        }

        // Apply absolute position sets (CameraFollow) — direct transform
        // writes, independent of physics.
        for (entity, pos) in commands.positions {
            if let Some(transform) = world.get_mut::<Transform2D>(entity) {
                transform.position = pos;
            }
        }

        // Apply impulses AFTER velocity commands (so jumps aren't overwritten).
        // This is one of the few places true impulse semantics matter (a jump
        // adds to existing horizontal velocity rather than clobbering it), so
        // we reach down to PhysicsWorld::apply_impulse directly — there is no
        // PhysicsSystem-level impulse API (games use set_velocity as the
        // universal "move this body" call).
        if let Some(ref mut physics) = physics {
            for (entity, impulse) in commands.impulses {
                physics.physics_world_mut().apply_impulse(entity, impulse);
            }
        }

        // Emit collection events before despawning
        for event in commands.collected {
            world.emit_event(event);
        }

        // Remove collected entities
        for entity in commands.to_despawn {
            let _ = world.remove_entity(&entity);
        }
    }

    /// Find the position of the nearest entity with a specific tag (excluding self)
    pub(super) fn find_nearest_tagged_position(world: &World, exclude: EntityId, tag: &str) -> Option<Vec2> {
        let exclude_pos = Self::get_position(world, exclude)?;

        world.entities().into_iter()
            .filter(|e| *e != exclude)
            .filter(|e| world.get::<EntityTag>(*e).map(|t| t.matches(tag)).unwrap_or(false))
            .filter_map(|e| Self::get_position(world, e))
            .min_by(|a, b| {
                let dist_a = (*a - exclude_pos).length_squared();
                let dist_b = (*b - exclude_pos).length_squared();
                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Check if any entity with a specific tag overlaps with the given entity
    pub(super) fn check_tagged_overlap(world: &World, entity: EntityId, tag: &str, radius: f32) -> bool {
        let Some(entity_pos) = Self::get_position(world, entity) else { return false };

        world.entities().into_iter()
            .filter(|e| *e != entity)
            .filter(|e| world.get::<EntityTag>(*e).map(|t| t.matches(tag)).unwrap_or(false))
            .filter_map(|e| Self::get_position(world, e))
            .any(|pos| (pos - entity_pos).length() < radius)
    }

    /// Get entity position (common operation)
    pub(super) fn get_position(world: &World, entity: EntityId) -> Option<Vec2> {
        world.get::<Transform2D>(entity).map(|t| t.position)
    }

    /// Add or update entity tag
    fn add_entity_tag(world: &mut World, entity: EntityId, tag: &str) {
        if let Some(existing) = world.get::<EntityTag>(entity) {
            if !existing.matches(tag) {
                // Update tag if different
                let _ = world.add_component(&entity, EntityTag::new(tag));
            }
        } else {
            let _ = world.add_component(&entity, EntityTag::new(tag));
        }
    }

    /// Update or add BehaviorState for an entity
    fn update_state(world: &mut World, entity: EntityId, state: BehaviorState) {
        if let Some(existing) = world.get_mut::<BehaviorState>(entity) {
            *existing = state;
        } else {
            let _ = world.add_component(&entity, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecs::behavior::{BehaviorPhase, PatrolTarget};

    fn phase_of(world: &World, entity: EntityId) -> BehaviorPhase {
        *world
            .get::<BehaviorState>(entity)
            .expect("stateful behavior should have a BehaviorState")
            .phase
            .current()
    }

    #[test]
    fn test_patrol_arrival_enters_waiting_then_reverses_direction() {
        let mut world = World::new();
        let mut runner = BehaviorRunner::new();
        let input = InputHandler::new();
        let dt = 0.016;

        // Entity starts exactly at point A, so the first update arrives
        // immediately (< 5.0 arrival threshold) and begins the wait.
        let patroller = world.create_entity();
        world.add_component(&patroller, Transform2D::new(Vec2::ZERO)).ok();
        world
            .add_component(
                &patroller,
                Behavior::Patrol { point_a: (0.0, 0.0), point_b: (100.0, 0.0), speed: 50.0, wait_time: 0.1 },
            )
            .ok();

        runner.update(&mut world, &input, dt, None);
        assert_eq!(
            phase_of(&world, patroller),
            BehaviorPhase::Waiting { then_toward: PatrolTarget::B },
            "arriving at A should enter Waiting headed for B"
        );

        // Wait out wait_time (0.1s) via the FSM's elapsed clock: 7 more
        // updates at 16ms = 0.112s in the Waiting state.
        for _ in 0..7 {
            runner.update(&mut world, &input, dt, None);
        }
        assert_eq!(
            phase_of(&world, patroller),
            BehaviorPhase::Patrolling { toward: PatrolTarget::B },
            "after wait_time elapses the patrol should head for B"
        );

        // Without physics the velocity command writes the transform: the
        // patroller is on its way to B and stays in the Patrolling phase.
        runner.update(&mut world, &input, dt, None);
        assert_eq!(phase_of(&world, patroller), BehaviorPhase::Patrolling { toward: PatrolTarget::B });
        assert!(world.get::<Transform2D>(patroller).expect("transform").position.x > 0.0);
    }

    #[test]
    fn test_chase_enters_and_leaves_chasing_phase_on_range() {
        let mut world = World::new();
        let mut runner = BehaviorRunner::new();
        let input = InputHandler::new();
        let dt = 0.016;

        let chaser = world.create_entity();
        world.add_component(&chaser, Transform2D::new(Vec2::ZERO)).ok();
        world
            .add_component(
                &chaser,
                Behavior::ChaseTagged {
                    target_tag: "player".to_string(),
                    detection_range: 50.0,
                    chase_speed: 100.0,
                    lose_interest_range: 80.0,
                },
            )
            .ok();

        // No target anywhere: Idle.
        runner.update(&mut world, &input, dt, None);
        assert_eq!(phase_of(&world, chaser), BehaviorPhase::Idle);

        // Target within detection range -> Chasing.
        let target = world.create_entity();
        world.add_component(&target, Transform2D::new(Vec2::new(30.0, 0.0))).ok();
        world.add_component(&target, EntityTag::new("player")).ok();
        runner.update(&mut world, &input, dt, None);
        assert_eq!(phase_of(&world, chaser), BehaviorPhase::Chasing);

        // Inside lose-interest range the chase persists (hysteresis band).
        world.get_mut::<Transform2D>(target).expect("target").position = Vec2::new(70.0, 0.0);
        runner.update(&mut world, &input, dt, None);
        assert_eq!(
            phase_of(&world, chaser),
            BehaviorPhase::Chasing,
            "chase persists between detection and lose-interest ranges"
        );

        // Beyond lose-interest range -> back to Idle.
        world.get_mut::<Transform2D>(target).expect("target").position = Vec2::new(200.0, 0.0);
        runner.update(&mut world, &input, dt, None);
        assert_eq!(phase_of(&world, chaser), BehaviorPhase::Idle);
    }
}
