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
use engine_core::scripting::ScriptRunner;
use input::{InputHandler, InputSettings};
use physics::{PhysicsConfig, PhysicsSystem};

mod preview;

pub use preview::{PreviewControls, PreviewHost};

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
    /// Playing frame update order: behaviors -> scripts early_update -> physics step ->
    /// collision event drain -> scripts update -> transform hierarchy.
    pub(crate) fn update_frame(
        &mut self,
        world: &mut World,
        input: &InputHandler,
        players: &InputSettings,
        scripts: &mut ScriptRunner,
        asset_base: &str,
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
            scripts.reset(world, asset_base);
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

        scripts.early_update(
            world,
            input,
            players,
            delta_time,
            self.physics.as_mut(),
        );

        let collisions = if let Some(physics) = &mut self.physics {
            physics.update(world, delta_time);
            physics.take_collision_events()
        } else {
            Vec::new()
        };

        scripts.update(
            world,
            input,
            players,
            delta_time,
            &collisions,
            self.physics.as_mut(),
        );

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
        self.update_frame(
            ctx.world,
            ctx.input,
            &*ctx.players,
            ctx.scripts,
            ctx.assets.base_path(),
            ctx.delta_time,
        );
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
    use ecs::sprite_components::{AnimationClip, SpriteAnimation};
    use ecs::{ClipState, ClipStateMachine, OnFinished, Transform2D};

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

        let players = InputSettings::default_two_player();
        let mut scripts = ScriptRunner::new();
        for _ in 0..10 {
            host.update_frame(&mut world, &input, &players, &mut scripts, "", dt);
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
        let players = InputSettings::default_two_player();
        let mut scripts = ScriptRunner::new();
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

        host.update_frame(&mut world, &input, &players, &mut scripts, "", dt);

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

        host.update_frame(&mut world, &input, &players, &mut scripts, "", dt);

        let late_transform = world.get::<Transform2D>(late_follower).expect("transform");
        assert!(late_transform.position.y > 0.0, "a target named mid-Play resolves on the next frame");

        host.reset_play_state();
        assert!(!host.play_initialized);
    }

    /// One playing frame as the engine really runs it: the host's update (whose
    /// script phase applies its commands at its end), then the frame tail's
    /// systems — `SpriteAnimationSystem`, `LifetimeSystem`, and
    /// `ClipStateMachineSystem` in that order (`engine_core`'s frame_tail).
    fn frame(
        host: &mut ProjectHost,
        world: &mut World,
        input: &InputHandler,
        players: &InputSettings,
        scripts: &mut ScriptRunner,
        asset_base: &str,
        delta_time: f32,
    ) {
        host.update_frame(world, input, players, scripts, asset_base, delta_time);
        ecs::System::update(&mut ecs::SpriteAnimationSystem, world, delta_time);
        ecs::System::update(&mut ecs::LifetimeSystem, world, delta_time);
        ecs::System::update(&mut ecs::ClipStateMachineSystem, world, delta_time);
    }

    #[test]
    fn test_a_script_drives_a_clip_state_chain_through_the_host_without_restarting_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("door.rhai"),
            r#"fn update(me, view, params, cmd, dt) {
                   // Re-assert the state the machine is already in: a
                   // transition to it is a no-op, so the clip must keep
                   // running and reach its end.
                   cmd.set_clip_state(me, view.clip_state(me));
                   cmd.set_velocity(me, vec2(0.0, 0.0));
               }"#,
        )
        .expect("wrote the door script");

        let mut host = ProjectHost::new(PathBuf::from("."));
        let mut world = World::new();
        let input = InputHandler::new();
        let players = InputSettings::default_two_player();
        let mut scripts = ScriptRunner::new();
        let delta_time = 0.1;

        let door = world.spawn().id();
        world
            .add_component(
                &door,
                SpriteAnimation::new(common::SheetGrid::new(4, 1))
                    .with_clip("close", AnimationClip::new(vec![0, 1, 2], 10.0).with_looping(false))
                    .with_clip("open", AnimationClip::new(vec![3, 2, 1], 10.0).with_looping(false))
                    .with_clip("hum", AnimationClip::new(vec![0, 1], 10.0)),
            )
            .expect("add SpriteAnimation");
        world
            .add_component(
                &door,
                ClipStateMachine::new(
                    "closing",
                    vec![
                        ("closing".to_string(), ClipState::new("close", OnFinished::Next("opening".to_string()))),
                        ("opening".to_string(), ClipState::new("open", OnFinished::Next("wide".to_string()))),
                        ("wide".to_string(), ClipState::staying("hum")),
                    ],
                ),
            )
            .expect("add ClipStateMachine");
        let spawn_position = Vec2::new(12.0, -3.0);
        world
            .add_component(&door, Transform2D::new(spawn_position))
            .expect("add Transform2D");
        world.add_component(&door, Name::new("door")).expect("add Name");
        let mut script_ref = ecs::script::ScriptRef::new("door");
        script_ref.source_path = "door.rhai".to_string();
        world
            .add_component(&door, ecs::script::Scripts(vec![script_ref]))
            .expect("add Scripts");

        // Closing -> opening -> wide needs three one-shot clips to run out
        // end to end, which only happens if the per-frame `set_clip_state` of
        // the state it is already in leaves the clip alone.
        for _ in 0..16 {
            frame(&mut host, &mut world, &input, &players, &mut scripts, dir.path().to_str().expect("utf-8 path"), delta_time);
        }

        assert_eq!(
            world.get::<ClipStateMachine>(door).expect("machine").state(),
            "wide",
            "the script's pin never restarted a clip, so each one finished"
        );
        let animation = world.get::<SpriteAnimation>(door).expect("animation");
        assert_eq!(animation.current_clip.as_deref(), Some("hum"));
        assert!(animation.playing, "the last state's looping clip runs on");
        assert_eq!(
            world.get::<Transform2D>(door).expect("transform").position,
            spawn_position,
            "the loop is pinned in place: its physics command asks for no motion"
        );
        assert!(scripts.errors().is_empty(), "{:?}", scripts.errors());
    }

    #[test]
    fn test_physics_builds_only_when_the_scene_declares_physics_settings() {
        let mut host = ProjectHost::new(PathBuf::from("."));
        let mut world = World::new();
        let input = InputHandler::new();
        let players = InputSettings::default_two_player();
        let mut scripts = ScriptRunner::new();
        let dt = 0.016;

        world.insert_resource(PhysicsSettings {
            gravity: (0.0, -420.0),
            pixels_per_meter: 64.0,
            timestep: 1.0 / 60.0,
        });

        assert!(host.physics.is_none());
        host.update_frame(&mut world, &input, &players, &mut scripts, "", dt);
        assert!(host.physics.is_some());

        host.reset_play_state();
        assert!(host.physics.is_none());
    }
}
