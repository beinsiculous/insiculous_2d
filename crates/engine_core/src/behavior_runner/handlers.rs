//! Per-variant behavior handlers (player movement, AI, collectibles).
//!
//! Each handler collects [`BehaviorCommands`] instead of touching the world
//! directly; `BehaviorRunner::apply_commands` applies them after iteration.

use glam::Vec2;

use ecs::behavior::{Behavior, BehaviorPhase, BehaviorState, PatrolTarget};
use ecs::{EntityId, World};
use input::{GameAction, InputHandler};
use physics::PhysicsSystem;

use super::{BehaviorCommands, BehaviorRunner, EntityCollected};

/// The read-only per-frame inputs every `&Behavior` handler reads: built once
/// per `BehaviorRunner::update` and shared by every entity in that frame.
pub(super) struct HandlerFrame<'a> {
    pub input: &'a InputHandler,
    pub delta_time: f32,
    pub physics: Option<&'a PhysicsSystem>,
}

impl BehaviorRunner {
    /// `Behavior::PlayerPlatformer` — input-driven horizontal movement plus a
    /// cooldown-gated jump impulse; Y velocity stays with physics (gravity).
    pub(super) fn update_player_platformer(
        &self,
        entity: EntityId,
        frame: &HandlerFrame,
        behavior: &Behavior,
        state: &mut BehaviorState,
        commands: &mut BehaviorCommands,
    ) {
        let Behavior::PlayerPlatformer { move_speed, jump_impulse, jump_cooldown, tag } = behavior
        else {
            log::error!("update_player_platformer dispatched for {behavior:?}");
            return;
        };
        let (move_speed, jump_impulse, jump_cooldown) = (*move_speed, *jump_impulse, *jump_cooldown);
        let HandlerFrame { input, delta_time, physics } = *frame;

        // Update cooldown timer
        if state.timer > 0.0 {
            state.timer -= delta_time;
        }

        // Calculate horizontal velocity only - let physics handle Y (gravity + jumps)
        let mut velocity_x = 0.0;
        if self.actions.is_active(GameAction::MoveLeft, input) { velocity_x = -move_speed; }
        if self.actions.is_active(GameAction::MoveRight, input) { velocity_x = move_speed; }

        // For platformers, only set X velocity - preserve Y for physics
        if let Some(physics) = physics {
            let current_vel = physics.physics_world()
                .get_body_velocity(entity)
                .map(|(v, _)| v)
                .unwrap_or(Vec2::ZERO);
            // Set X to input, keep Y from physics (gravity/jumps)
            let velocity = Vec2::new(velocity_x, current_vel.y);
            commands.velocities.push((entity, velocity));
        }

        // Jump - collect impulse to apply AFTER velocity commands.
        // Action1 (Space / pad-0 A / mouse left in the default preset) so
        // rebinds and gamepad jump work — never a raw key read.
        if self.actions.just_activated(GameAction::Action1, input) && state.timer <= 0.0 {
            commands.impulses.push((entity, Vec2::new(0.0, jump_impulse)));
            state.timer = jump_cooldown;
        }

        commands.tags.push((entity, tag.to_string()));
    }

    /// `Behavior::PlayerTopDown` — input-driven movement on both axes with
    /// normalized diagonals.
    pub(super) fn update_player_top_down(
        &self,
        entity: EntityId,
        input: &InputHandler,
        move_speed: f32,
        tag: &str,
        commands: &mut BehaviorCommands,
    ) {
        // Calculate movement velocity from input
        let mut vel = Vec2::ZERO;
        if self.actions.is_active(GameAction::MoveUp, input) { vel.y += move_speed; }
        if self.actions.is_active(GameAction::MoveDown, input) { vel.y -= move_speed; }
        if self.actions.is_active(GameAction::MoveLeft, input) { vel.x -= move_speed; }
        if self.actions.is_active(GameAction::MoveRight, input) { vel.x += move_speed; }

        // Normalize diagonal movement
        if vel.length_squared() > 0.0 {
            vel = vel.normalize() * move_speed;
        }

        commands.velocities.push((entity, vel));
        commands.tags.push((entity, tag.to_string()));
    }

    /// `Behavior::ChaseTagged` — chase the nearest tagged entity once it is
    /// inside detection range, give up beyond lose-interest range.
    ///
    /// Phase FSM: `Idle` ⇄ `Chasing` — enter on `distance < detection_range`,
    /// leave on `distance > lose_interest_range` or when no target exists.
    pub(super) fn update_chase_tagged(
        world: &World,
        entity: EntityId,
        frame: &HandlerFrame,
        behavior: &Behavior,
        state: &mut BehaviorState,
        commands: &mut BehaviorCommands,
    ) {
        let Behavior::ChaseTagged {
            target_tag,
            detection_range,
            chase_speed,
            lose_interest_range,
        } = behavior
        else {
            log::error!("update_chase_tagged dispatched for {behavior:?}");
            return;
        };
        let (detection_range, chase_speed, lose_interest_range) =
            (*detection_range, *chase_speed, *lose_interest_range);

        state.phase.tick(frame.delta_time);

        if let Some(target_pos) = Self::find_nearest_tagged_position(world, entity, target_tag) {
            if let Some(entity_pos) = Self::get_position(world, entity) {
                let distance = (target_pos - entity_pos).length();

                let chasing = state.phase.is(&BehaviorPhase::Chasing);
                if !chasing && distance < detection_range {
                    state.phase.transition_to(BehaviorPhase::Chasing);
                } else if chasing && distance > lose_interest_range {
                    state.phase.transition_to(BehaviorPhase::Idle);
                }

                if state.phase.is(&BehaviorPhase::Chasing) {
                    let velocity = (target_pos - entity_pos).normalize_or_zero() * chase_speed;
                    commands.velocities.push((entity, velocity));
                } else {
                    commands.velocities.push((entity, Vec2::ZERO));
                }
            }
        } else {
            state.phase.transition_to(BehaviorPhase::Idle);
            commands.velocities.push((entity, Vec2::ZERO));
        }
    }

    /// `Behavior::Patrol` — walk back and forth between two points, pausing
    /// at each end for `wait_time` seconds.
    ///
    /// Phase FSM: `Idle` → `Patrolling { toward }` → (on arrival)
    /// `Waiting { then_toward }` → (after `wait_time`, via the machine's
    /// `elapsed()` clock) → `Patrolling` toward the other endpoint.
    pub(super) fn update_patrol(
        world: &World,
        entity: EntityId,
        frame: &HandlerFrame,
        behavior: &Behavior,
        state: &mut BehaviorState,
        commands: &mut BehaviorCommands,
    ) {
        let Behavior::Patrol { point_a, point_b, speed, wait_time } = behavior
        else {
            log::error!("update_patrol dispatched for {behavior:?}");
            return;
        };
        let point_a = Vec2::new(point_a.0, point_a.1);
        let point_b = Vec2::new(point_b.0, point_b.1);
        let (speed, wait_time) = (*speed, *wait_time);

        state.phase.tick(frame.delta_time);

        if let BehaviorPhase::Waiting { then_toward } = *state.phase.current() {
            if state.phase.elapsed() >= wait_time {
                state.phase.transition_to(BehaviorPhase::Patrolling { toward: then_toward });
            }
            commands.velocities.push((entity, Vec2::ZERO));
        } else if let Some(entity_pos) = Self::get_position(world, entity) {
            // Idle (first update) starts the patrol toward A, matching the
            // pre-FSM default direction.
            let toward = match *state.phase.current() {
                BehaviorPhase::Patrolling { toward } => toward,
                _ => {
                    state.phase.transition_to(BehaviorPhase::Patrolling { toward: PatrolTarget::A });
                    PatrolTarget::A
                }
            };
            let target = match toward {
                PatrolTarget::A => point_a,
                PatrolTarget::B => point_b,
            };

            if (target - entity_pos).length() < 5.0 {
                state.phase.transition_to(BehaviorPhase::Waiting { then_toward: toward.other() });
                commands.velocities.push((entity, Vec2::ZERO));
            } else {
                let velocity = (target - entity_pos).normalize() * speed;
                commands.velocities.push((entity, velocity));
            }
        }
    }

    /// `Behavior::FollowEntity` — move toward a named entity while farther
    /// away than `follow_distance`.
    pub(super) fn update_follow_entity(
        &self,
        world: &World,
        entity: EntityId,
        target_name: &str,
        follow_distance: f32,
        follow_speed: f32,
        commands: &mut BehaviorCommands,
    ) {
        let mut vel = Vec2::ZERO;
        if let Some(&target_entity) = self.named_entities.get(target_name) {
            if let (Some(target_pos), Some(entity_pos)) = (
                Self::get_position(world, target_entity),
                Self::get_position(world, entity),
            ) {
                let to_target = target_pos - entity_pos;
                if to_target.length() > follow_distance {
                    vel = to_target.normalize() * follow_speed;
                }
            }
        }
        commands.velocities.push((entity, vel));
    }

    /// `Behavior::FollowTagged` — move toward the nearest tagged entity while
    /// farther away than `follow_distance`.
    pub(super) fn update_follow_tagged(
        world: &World,
        entity: EntityId,
        target_tag: &str,
        follow_distance: f32,
        follow_speed: f32,
        commands: &mut BehaviorCommands,
    ) {
        let mut vel = Vec2::ZERO;
        if let Some(target_pos) = Self::find_nearest_tagged_position(world, entity, target_tag) {
            if let Some(entity_pos) = Self::get_position(world, entity) {
                let to_target = target_pos - entity_pos;
                if to_target.length() > follow_distance {
                    vel = to_target.normalize() * follow_speed;
                }
            }
        }
        commands.velocities.push((entity, vel));
    }

    /// `Behavior::Collectible` — emit a collection event (and optionally
    /// despawn) when an entity with the collector tag overlaps.
    pub(super) fn update_collectible(
        world: &World,
        entity: EntityId,
        score_value: u32,
        despawn_on_collect: bool,
        collector_tag: &str,
        commands: &mut BehaviorCommands,
    ) {
        if Self::check_tagged_overlap(world, entity, collector_tag, 40.0) {
            log::info!("Collected! +{} points", score_value);
            commands.collected.push(EntityCollected {
                entity,
                score_value,
                collector_tag: collector_tag.to_string(),
            });
            if despawn_on_collect {
                commands.to_despawn.push(entity);
            }
        }
    }
}
