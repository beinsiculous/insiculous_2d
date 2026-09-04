//! Data-only game host for the standalone editor and web playground.
//!
//! Provides physics and behavior preview during play mode. All real editing
//! (including initial scene load) is handled by `EditorGame` wrapping this;
//! scene loading here would bypass scene_path/physics/dirty tracking and
//! silently break save.
//!
//! No physics block, no physics: a scene that declares no `PhysicsSettings`
//! runs Play without a `PhysicsSystem`, and behaviors then move transforms
//! directly. The editor never invents gravity — with a physics system present,
//! a behavior's velocity on an entity that has no `RigidBody` goes to a rapier
//! body that does not exist and the entity never moves, so an invented default
//! would freeze every body-less behavior scene. A scene that wants simulation
//! declares `physics:`.

use std::collections::HashMap;
use std::path::PathBuf;

use ecs::{Name, World};
use engine_core::prelude::*;
use engine_core::scene_data::PhysicsSettings;
use input::InputHandler;
use physics::{PhysicsConfig, PhysicsSystem};

/// Data-only game host for the editor, running physics and behaviors during play mode.
pub struct ProjectHost {
    project_path: PathBuf,
    physics: Option<PhysicsSystem>,
    behaviors: BehaviorRunner,
    transform_hierarchy: TransformHierarchySystem,
    play_initialized: bool,
}

impl ProjectHost {
    /// Create a new project host rooted at the given project directory.
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            physics: None,
            behaviors: BehaviorRunner::new(),
            transform_hierarchy: TransformHierarchySystem::new(),
            play_initialized: false,
        }
    }

    /// Step one playing frame: update behaviors, physics, and transform hierarchy.
    ///
    /// Playing frame update order: behaviors -> physics step -> transform hierarchy.
    /// Script runner integration fits around physics: behaviors -> scripts early_update ->
    /// physics -> scripts update -> transform hierarchy.
    pub(crate) fn update_frame(
        &mut self,
        world: &mut World,
        input: &InputHandler,
        delta_time: f32,
    ) {
        if !self.play_initialized {
            self.play_initialized = true;

            if let Some(settings) = world.resource::<PhysicsSettings>() {
                let config = PhysicsConfig::new(Vec2::new(
                    settings.gravity.0,
                    settings.gravity.1,
                ))
                .with_scale(settings.pixels_per_meter);
                let mut physics = PhysicsSystem::with_config(config);
                if let Err(error) = physics.initialize(world) {
                    log::warn!("physics preview not initialised: {error}");
                }
                self.physics = Some(physics);
            }
        }

        // Rebuilt every frame, not once per session: the command API can
        // create or rename entities mid-Play, and a FollowEntity target that
        // appears then must resolve now, not after the next Stop.
        let mut named_entities = HashMap::new();
        for entity in world.entities() {
            if let Some(name) = world.get::<Name>(entity) {
                named_entities.insert(name.0.clone(), entity);
            }
        }
        self.behaviors.set_named_entities(named_entities);

        self.behaviors.update(
            world,
            input,
            delta_time,
            self.physics.as_mut(),
        );

        if let Some(physics) = &mut self.physics {
            physics.update(world, delta_time);
        }
        self.transform_hierarchy.update(world, delta_time);
    }

    /// Reset play-mode state when simulation stops.
    pub(crate) fn reset_play_state(&mut self) {
        self.physics = None;
        self.play_initialized = false;
        self.behaviors.set_named_entities(HashMap::new());
    }
}

impl Game for ProjectHost {
    fn init(&mut self, ctx: &mut GameContext) {
        // Project config only: the initial scene is opened by EditorGame
        // through its real load path right after this returns.
        let assets_path = self.project_path.join("assets");
        ctx.assets.set_base_path(assets_path.to_string_lossy());
        self.transform_hierarchy.initialize(ctx.world).ok();
        log::info!("Editor opened project: {}", self.project_path.display());
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.update_frame(ctx.world, ctx.input, ctx.delta_time);
    }

    fn on_play_stopped(&mut self, _ctx: &mut GameContext) {
        // Drop physics and clear named entities so the next Play rebuilds
        // from the current scene settings and restored world state.
        self.reset_play_state();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecs::Transform2D;

    #[test]
    fn test_patrol_entity_advances_over_playing_frames_without_physics() {
        let mut host = ProjectHost::new(PathBuf::from("."));
        let mut world = World::new();
        let input = InputHandler::new();
        let dt = 0.016;

        let entity = world.spawn().id();
        world
            .add_component(&entity, Transform2D::default())
            .expect("add Transform2D");
        world
            .add_component(
                &entity,
                Behavior::Patrol {
                    point_a: (0.0, 0.0),
                    point_b: (100.0, 0.0),
                    speed: 50.0,
                    wait_time: 0.0,
                },
            )
            .expect("add Behavior");

        for _ in 0..10 {
            host.update_frame(&mut world, &input, dt);
        }

        let transform = world.get::<Transform2D>(entity).expect("entity has transform");
        assert!(
            transform.position.x > 0.0,
            "expected patrol entity to advance x > 0.0, got x = {}",
            transform.position.x
        );
    }

    #[test]
    fn test_named_entities_resolve_during_play_and_clear_on_stop() {
        let mut host = ProjectHost::new(PathBuf::from("."));
        let mut world = World::new();
        let input = InputHandler::new();
        let dt = 0.016;

        let target = world.spawn().id();
        world
            .add_component(&target, Name::new("target"))
            .expect("add Name");
        world
            .add_component(
                &target,
                Transform2D {
                    position: Vec2::new(100.0, 0.0),
                    ..Default::default()
                },
            )
            .expect("add Transform2D");

        let follower = world.spawn().id();
        world
            .add_component(&follower, Transform2D::default())
            .expect("add Transform2D");
        world
            .add_component(
                &follower,
                Behavior::FollowEntity {
                    target_name: "target".to_string(),
                    follow_distance: 10.0,
                    follow_speed: 50.0,
                },
            )
            .expect("add Behavior");

        host.update_frame(&mut world, &input, dt);

        let follower_transform = world.get::<Transform2D>(follower).expect("transform");
        assert!(
            follower_transform.position.x > 0.0,
            "follower should advance toward named target"
        );

        // A target that appears mid-Play (the command API can create and
        // rename entities while Playing) must resolve on the next frame, not
        // after the next Stop.
        let late_target = world.spawn().id();
        world.add_component(&late_target, Name::new("late")).expect("add Name");
        world
            .add_component(&late_target, Transform2D { position: Vec2::new(0.0, 100.0), ..Default::default() })
            .expect("add Transform2D");
        let late_follower = world.spawn().id();
        world.add_component(&late_follower, Transform2D::default()).expect("add Transform2D");
        world
            .add_component(
                &late_follower,
                Behavior::FollowEntity { target_name: "late".to_string(), follow_distance: 10.0, follow_speed: 50.0 },
            )
            .expect("add Behavior");

        host.update_frame(&mut world, &input, dt);

        let late_transform = world.get::<Transform2D>(late_follower).expect("transform");
        assert!(late_transform.position.y > 0.0, "a target named mid-Play resolves on the next frame");

        host.reset_play_state();
        assert!(!host.play_initialized);
    }

    #[test]
    fn test_physics_builds_only_when_the_scene_declares_physics_settings() {
        let mut host = ProjectHost::new(PathBuf::from("."));
        let mut world = World::new();
        let input = InputHandler::new();
        let dt = 0.016;

        world.insert_resource(PhysicsSettings {
            gravity: (0.0, -420.0),
            pixels_per_meter: 64.0,
            timestep: 1.0 / 60.0,
        });

        assert!(host.physics.is_none());
        host.update_frame(&mut world, &input, dt);
        assert!(host.physics.is_some());

        host.reset_play_state();
        assert!(host.physics.is_none());
    }
}
