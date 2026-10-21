//! Scene-authored sound through `GameHarness`: a `SoundEmitter` named in a scene, played
//! by the frame tail after the hierarchy, re-mixed from the main camera, and ended with
//! its instance or its entity. The harness's audio manager is the disabled one: a loop
//! is reported playing until it is stopped, a one-shot ends at the next frame's update,
//! and a song plays on the virtual clock.

use std::path::Path;
use std::time::Duration;

use ecs::blackboard::Blackboard;
use ecs::script::{ScriptRef, ScriptValue, Scripts};
use ecs::{EntityId, Name, SoundEmitter, SoundRef, Spatial, Transform2D};
use engine_core::prelude::{request_transform_hierarchy_reset, Game, GameConfig, GameContext, SceneLoader};
use engine_core::scene_data::SceneLoadError;
use engine_core::test_support::{GameHarness, StubResolver};
use glam::Vec2;
use tempfile::TempDir;

const FRAME: f32 = 1.0 / 60.0;
const SOUNDS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/assets/sounds");
const SONG_FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../music_midi/fixtures");

/// The cast fixture's flute track as the one part, the sine bank's one preset on it.
const SIDECAR: &str = r#"SongFile(
    version: 1,
    soundfont: "soundfonts/sine.sf2",
    parts: [(name: "melody", track: Name("channel 2"))],
    instruments: [(name: "sine", source: SoundFont(path: "soundfonts/sine.sf2", bank: 0, preset: 0))],
    bindings: [(action: "tone", part: "melody", instrument: "sine")],
)"#;

/// A camera at the origin and a coin four world units to its right, both spatial and
/// one-shot, eight units of reach: the coin is heard at half volume, halfway right.
const COIN_SCENE: &str = r#"SceneData(name: "coins", entities: [
    EntityData(name: Some("camera"), components: [
        Transform2D(position: (0.0, 0.0)),
        Camera2D(is_main_camera: true),
    ]),
    EntityData(name: Some("coin"), components: [
        Transform2D(position: (320.0, 0.0)),
        SoundEmitter(sound: Path("sounds/beep_e.wav"), volume: 0.8, play_on_spawn: true,
            spatial: Some(Spatial(max_distance: 8.0))),
    ]),
])"#;

/// Plays its own emitter when the flag is up; `despawn` also despawns it in the hook.
const PLAY: &str = r#"
// @param despawn: bool = false
fn update(me, view, params, cmd, dt) {
    if view.blackboard_bool("play", false) {
        cmd.play_sound(me);
        if params.despawn { cmd.despawn(me); }
        cmd.set_blackboard_bool("play", false);
    }
}
"#;

/// Stops its own emitter and triggers its binding in the order `stop_first` says.
const STOP_AND_TRIGGER: &str = r#"
// @param stop_first: bool = true
fn update(me, view, params, cmd, dt) {
    if view.blackboard_bool("play", false) {
        if params.stop_first {
            cmd.stop_sound(me);
            cmd.trigger(me, "tone");
        } else {
            cmd.trigger(me, "tone");
            cmd.stop_sound(me);
        }
        cmd.set_blackboard_bool("play", false);
    }
}
"#;

/// Stops the emitter on the entity named `target` when the flag is up.
const STOP_TARGET: &str = r#"
// @param target: str = ""
fn update(me, view, params, cmd, dt) {
    if view.blackboard_bool("play", false) {
        cmd.stop_sound(params.target);
        cmd.set_blackboard_bool("play", false);
    }
}
"#;

type FrameHook = Box<dyn FnOnce(&mut GameContext)>;

/// Loads its scene and steps the scripts as a game does, holds the time scale where a
/// test puts it, then runs a test's one-frame hook.
struct Stage {
    scene: Option<String>,
    frozen: bool,
    hook: Option<FrameHook>,
}

impl Game for Stage {
    fn init(&mut self, ctx: &mut GameContext) {
        let asset_base = ctx.assets.base_path().to_string();
        ctx.scripts.reset(ctx.world, &asset_base);
        if let Some(scene) = &self.scene {
            SceneLoader::load_and_instantiate(scene, ctx.world, ctx.assets).expect("the scene loads");
        }
    }

    fn update(&mut self, ctx: &mut GameContext) {
        ctx.time_scale = if self.frozen { 0.0 } else { 1.0 };
        ctx.scripts.early_update(ctx.world, ctx.input, ctx.players, ctx.delta_time, None);
        ctx.scripts.update(ctx.world, ctx.input, ctx.players, ctx.delta_time, &[], None);
        if let Some(hook) = self.hook.take() {
            hook(ctx);
        }
    }
}

/// The sounds, the song and the scripts under one asset base.
struct Fixture {
    directory: TempDir,
    harness: GameHarness<Stage>,
}

impl Fixture {
    /// A stage over `scene` (written under the asset base), `frozen` from its first frame,
    /// which has run.
    fn with_scene(scene: Option<&str>, frozen: bool) -> Self {
        let directory = tempfile::tempdir().expect("tempdir");
        let copy = |from: &Path, to: &str| {
            let target = directory.path().join(to);
            std::fs::create_dir_all(target.parent().expect("a parent")).expect("directories");
            std::fs::copy(from, target).expect("fixture copies");
        };
        copy(&Path::new(SOUNDS).join("beep_e.wav"), "sounds/beep_e.wav");
        copy(&Path::new(SONG_FIXTURES).join("sine.sf2"), "soundfonts/sine.sf2");
        copy(&Path::new(SONG_FIXTURES).join("dansenapolitaine-cast.mid"), "songs/song.mid");
        let write = |to: &str, text: &str| {
            let target = directory.path().join(to);
            std::fs::create_dir_all(target.parent().expect("a parent")).expect("directories");
            std::fs::write(target, text).expect("writes");
        };
        write("songs/song.mid.ron", SIDECAR);
        write("scripts/play.rhai", PLAY);
        write("scripts/stop_and_trigger.rhai", STOP_AND_TRIGGER);
        write("scripts/stop_target.rhai", STOP_TARGET);
        let scene = scene.map(|text| {
            write("scenes/scene.ron", text);
            directory.path().join("scenes/scene.ron").to_string_lossy().to_string()
        });
        let config = GameConfig::new("Sound emitters")
            .with_asset_base_path(directory.path().to_string_lossy().to_string());
        let stage = Stage { scene, frozen, hook: None };
        let mut fixture = Self { directory, harness: GameHarness::new(stage, config) };
        fixture.harness.step(FRAME, &[]);
        fixture
    }

    fn empty() -> Self {
        Self::with_scene(None, false)
    }

    fn play_song(&mut self) {
        let path = self.directory.path().join("songs/song.mid");
        self.harness.audio_mut().play_song(path).expect("the song loads");
    }

    fn spawn(&mut self, name: &str, emitter: SoundEmitter) -> EntityId {
        let world = self.harness.world_mut();
        let entity = world.create_entity();
        world.add_component(&entity, Name(name.to_string())).ok();
        world.add_component(&entity, Transform2D::new(Vec2::ZERO)).ok();
        world.add_component(&entity, emitter).ok();
        entity
    }

    fn attach_script(&mut self, entity: EntityId, script: &str, params: &[(&str, ScriptValue)]) {
        let mut script_ref = ScriptRef::new(script);
        script_ref.source_path = format!("scripts/{script}.rhai");
        for (key, value) in params {
            script_ref.params.insert(key.to_string(), value.clone());
        }
        self.harness.world_mut().add_component(&entity, Scripts(vec![script_ref])).ok();
    }

    /// One frame whose scripts see the flag up.
    fn script_frame(&mut self) {
        let world = self.harness.world_mut();
        world.resource_mut::<Blackboard>().expect("init installed it").set("play", ScriptValue::Bool(true));
        self.harness.step(FRAME, &[]);
    }

    /// One frame in which the game, after its scripts, runs `hook`.
    fn frame_with(&mut self, hook: impl FnOnce(&mut GameContext) + 'static) {
        self.harness.game_mut().hook = Some(Box::new(hook));
        self.harness.step(FRAME, &[]);
    }

    /// Frames with wall time between them, so the song's virtual clock drains what was sent.
    fn settle(&mut self) {
        for _ in 0..6 {
            std::thread::sleep(Duration::from_millis(3));
            self.harness.step(FRAME, &[]);
        }
    }

    fn is_playing(&self, entity: EntityId) -> bool {
        let playing = self.harness.sound_emitters().playing(entity);
        playing.is_some_and(|playback| self.harness.audio().is_playing(playback))
    }
}

fn path_emitter(looping: bool) -> SoundEmitter {
    SoundEmitter { sound: SoundRef::Path("sounds/beep_e.wav".into()), looping, ..SoundEmitter::default() }
}

fn binding_emitter() -> SoundEmitter {
    SoundEmitter { sound: SoundRef::Binding("tone".into()), ..SoundEmitter::default() }
}

#[test]
fn a_scene_naming_a_sound_by_path_plays_it_on_the_first_frame_mixed_from_the_camera() {
    let fixture = Fixture::with_scene(Some(COIN_SCENE), false);
    let coin = fixture.harness.world().entities().into_iter().find(|entity| {
        fixture.harness.world().get::<Name>(*entity).is_some_and(|name| name.as_str() == "coin")
    });
    let coin = coin.expect("the scene named a coin");
    let emitters = fixture.harness.sound_emitters();
    assert_eq!(emitters.plays(coin), 1);
    assert!(emitters.playing(coin).is_some(), "its instance is tracked");
    assert_eq!(emitters.last_mix(coin), Some((0.4, 0.5)), "half of 0.8, halfway right");
}

#[test]
fn a_missing_file_fails_once_and_never_panics() {
    let mut fixture = Fixture::empty();
    let ghost = fixture.spawn(
        "ghost",
        SoundEmitter { sound: SoundRef::Path("sounds/missing.wav".into()), play_on_spawn: true, ..SoundEmitter::default() },
    );
    for _ in 0..10 {
        fixture.harness.step(FRAME, &[]);
    }
    let emitters = fixture.harness.sound_emitters();
    assert_eq!(emitters.failed_loads(), 1);
    assert_eq!((emitters.plays(ghost), emitters.playing(ghost)), (0, None));
}

#[test]
fn a_one_shots_entry_is_gone_the_frame_after_it_ends_and_its_play_still_counts() {
    let mut fixture = Fixture::empty();
    let player = fixture.spawn("player", path_emitter(false));
    fixture.frame_with(move |ctx| ctx.emitters.play(player, ctx.world));
    assert!(fixture.is_playing(player));

    fixture.harness.step(FRAME, &[]);
    assert_eq!(fixture.harness.sound_emitters().playing(player), None, "the ended one-shot is forgotten");
    assert_eq!(fixture.harness.sound_emitters().plays(player), 1);
}

#[test]
fn playing_a_one_shot_twice_starts_two_instances_and_a_running_loop_only_one() {
    let mut fixture = Fixture::empty();
    let player = fixture.spawn("player", path_emitter(false));
    let engine = fixture.spawn("engine", path_emitter(true));
    for _ in 0..2 {
        fixture.frame_with(move |ctx| {
            ctx.emitters.play(player, ctx.world);
            ctx.emitters.play(engine, ctx.world);
        });
    }
    let emitters = fixture.harness.sound_emitters();
    assert_eq!(emitters.plays(player), 2, "every jump is heard");
    assert_eq!(emitters.plays(engine), 1, "a loop already running is left alone");
    assert!(fixture.is_playing(engine));
}

#[test]
fn a_looping_emitters_entity_destroyed_in_update_is_stopped_in_that_frames_tail() {
    let mut fixture = Fixture::empty();
    let engine = fixture.spawn("engine", path_emitter(true));
    fixture.frame_with(move |ctx| ctx.emitters.play(engine, ctx.world));
    let playback = fixture.harness.sound_emitters().playing(engine).expect("the loop plays");

    fixture.frame_with(move |ctx| {
        ctx.world.remove_entity(&engine).ok();
    });
    assert_eq!(fixture.harness.sound_emitters().playing(engine), None);
    assert!(!fixture.harness.audio().is_playing(playback), "the loop was stopped, not left running");
}

#[test]
fn removing_a_looping_emitters_component_stops_the_loop_and_leaves_the_entity() {
    let mut fixture = Fixture::empty();
    let engine = fixture.spawn("engine", path_emitter(true));
    fixture.frame_with(move |ctx| ctx.emitters.play(engine, ctx.world));
    let playback = fixture.harness.sound_emitters().playing(engine).expect("the loop plays");

    fixture.harness.world_mut().remove_component::<SoundEmitter>(&engine).ok();
    fixture.harness.step(FRAME, &[]);
    assert!(fixture.harness.world().validate_entity(&engine).is_ok());
    assert!(!fixture.harness.audio().is_playing(playback));
    assert_eq!(fixture.harness.sound_emitters().playing(engine), None);
}

#[test]
fn a_scripted_play_sound_plays_the_entitys_emitter() {
    let mut fixture = Fixture::empty();
    let bell = fixture.spawn("bell", path_emitter(true));
    fixture.attach_script(bell, "play", &[]);
    fixture.script_frame();
    assert!(fixture.is_playing(bell));
    assert_eq!(fixture.harness.sound_emitters().plays(bell), 1);
}

#[test]
fn a_one_shot_played_by_a_script_that_despawns_its_entity_still_sounds_and_a_loop_is_refused() {
    let mut fixture = Fixture::empty();
    let coin = fixture.spawn("coin", path_emitter(false));
    fixture.attach_script(coin, "play", &[("despawn", ScriptValue::Bool(true))]);
    let engine = fixture.spawn("engine", path_emitter(true));
    fixture.attach_script(engine, "play", &[("despawn", ScriptValue::Bool(true))]);

    fixture.script_frame();
    let emitters = fixture.harness.sound_emitters();
    assert_eq!(emitters.detached_plays(), 1, "the coin's one-shot plays where the coin was");
    assert_eq!(emitters.refused(), 1, "a loop belongs to a living owner");
    assert_eq!((emitters.playing(coin), emitters.playing(engine)), (None, None));
}

#[test]
fn stop_then_trigger_on_a_binding_emitter_leaves_the_new_activation_and_the_reverse_ends_it() {
    let mut fixture = Fixture::empty();
    fixture.play_song();
    let stop_first = fixture.spawn("stop_first", binding_emitter());
    fixture.attach_script(stop_first, "stop_and_trigger", &[]);
    let trigger_first = fixture.spawn("trigger_first", binding_emitter());
    fixture.attach_script(trigger_first, "stop_and_trigger", &[("stop_first", ScriptValue::Bool(false))]);
    fixture.frame_with(move |ctx| {
        ctx.emitters.play(stop_first, ctx.world);
        ctx.emitters.play(trigger_first, ctx.world);
    });
    let before = fixture.harness.sound_sources().source_of(stop_first).expect("the play activated it");
    assert!(fixture.harness.sound_sources().source_of(trigger_first).is_some());

    fixture.script_frame();
    let after = fixture.harness.sound_sources().source_of(stop_first).expect("the trigger re-activated it");
    assert!(after.epoch > before.epoch, "the stop ended the old activation first");
    assert_eq!(fixture.harness.sound_sources().source_of(trigger_first), None, "the stop came last");
}

#[test]
fn a_frozen_engine_starts_nothing_and_the_spawn_sound_fires_on_the_first_live_frame() {
    let mut fixture = Fixture::with_scene(Some(COIN_SCENE), true);
    for _ in 0..3 {
        fixture.harness.step(FRAME, &[]);
    }
    let coin = fixture.harness.world().entities().into_iter().find(|entity| {
        fixture.harness.world().get::<SoundEmitter>(*entity).is_some()
    });
    let coin = coin.expect("the scene has its coin");
    assert_eq!(fixture.harness.sound_emitters().plays(coin), 0, "Editing records no first sight");

    fixture.harness.game_mut().frozen = false;
    fixture.harness.step(FRAME, &[]);
    assert_eq!(fixture.harness.sound_emitters().plays(coin), 1);
}

#[test]
fn a_loop_whose_entity_goes_while_the_engine_is_frozen_is_stopped_in_that_tail() {
    let mut fixture = Fixture::empty();
    let engine = fixture.spawn("engine", path_emitter(true));
    fixture.frame_with(move |ctx| ctx.emitters.play(engine, ctx.world));
    let playback = fixture.harness.sound_emitters().playing(engine).expect("the loop plays");

    fixture.harness.game_mut().frozen = true;
    fixture.frame_with(move |ctx| {
        ctx.world.remove_entity(&engine).ok();
    });
    assert!(!fixture.harness.audio().is_playing(playback), "a paused game's eviction still runs");
    assert_eq!(fixture.harness.sound_emitters().playing(engine), None);
}

#[test]
fn a_stop_in_a_binding_emitters_spawn_frame_wins_over_play_on_spawn() {
    let mut fixture = Fixture::empty();
    fixture.play_song();
    let drum = fixture.spawn("drum", SoundEmitter { play_on_spawn: true, ..binding_emitter() });
    fixture.attach_script(drum, "stop_target", &[("target", ScriptValue::Str("drum".into()))]);

    fixture.script_frame();
    assert_eq!(fixture.harness.sound_sources().source_of(drum), None, "the stop was not undone by first sight");
    assert_eq!(fixture.harness.sound_emitters().plays(drum), 0);
}

#[test]
fn re_adding_a_play_on_spawn_emitter_fires_again() {
    let mut fixture = Fixture::empty();
    let spawner = SoundEmitter { play_on_spawn: true, ..path_emitter(false) };
    let coin = fixture.spawn("coin", spawner.clone());
    fixture.harness.step(FRAME, &[]);
    assert_eq!(fixture.harness.sound_emitters().plays(coin), 1);

    fixture.harness.world_mut().remove_component::<SoundEmitter>(&coin).ok();
    fixture.harness.step(FRAME, &[]);
    fixture.harness.world_mut().add_component(&coin, spawner).ok();
    fixture.harness.step(FRAME, &[]);
    assert_eq!(fixture.harness.sound_emitters().plays(coin), 2, "a re-added emitter is seen anew");
}

#[test]
fn a_one_shot_played_from_rust_in_the_update_that_despawns_its_entity_still_sounds() {
    let mut fixture = Fixture::empty();
    let coin = fixture.spawn("coin", path_emitter(false));
    fixture.frame_with(move |ctx| {
        ctx.emitters.play(coin, ctx.world);
        ctx.world.remove_entity(&coin).ok();
    });
    let emitters = fixture.harness.sound_emitters();
    assert_eq!(emitters.detached_plays(), 1, "the pickup's sound plays where the coin was");
    assert_eq!(emitters.refused(), 0);
    assert_eq!(emitters.released(), 1, "a detached one-shot is owned until it ends");

    fixture.frame_with(|ctx| request_transform_hierarchy_reset(ctx.world));
    assert_eq!(fixture.harness.sound_emitters().released(), 0, "and a world replacement stops it");
}

#[test]
fn a_restart_that_fails_to_load_releases_the_previous_instance_without_tracking_it_twice() {
    let mut fixture = Fixture::empty();
    let player = fixture.spawn("player", path_emitter(false));
    fixture.frame_with(move |ctx| ctx.emitters.play(player, ctx.world));
    assert!(fixture.is_playing(player));

    fixture.frame_with(move |ctx| {
        if let Some(emitter) = ctx.world.get_mut::<SoundEmitter>(player) {
            emitter.sound = SoundRef::Path("sounds/missing.wav".into());
        }
        ctx.emitters.play(player, ctx.world);
    });
    let emitters = fixture.harness.sound_emitters();
    assert_eq!(emitters.failed_loads(), 1);
    assert_eq!(emitters.playing(player), None, "the entry left with the instance it released");
    assert!(emitters.released() <= 1, "the released instance is owned once, never twice");
}

#[test]
fn a_world_replacement_stops_a_released_one_shot_too() {
    let mut fixture = Fixture::empty();
    let player = fixture.spawn("player", path_emitter(false));
    fixture.frame_with(move |ctx| {
        ctx.emitters.play(player, ctx.world);
        ctx.emitters.play(player, ctx.world);
    });
    let emitters = fixture.harness.sound_emitters();
    assert_eq!((emitters.plays(player), emitters.released()), (2, 1), "the first instance is released, still owned");

    fixture.frame_with(|ctx| request_transform_hierarchy_reset(ctx.world));
    let emitters = fixture.harness.sound_emitters();
    assert_eq!((emitters.released(), emitters.plays(player)), (0, 0), "the replacement stops and forgets it");
}

#[test]
fn a_world_replacement_stops_every_instance_and_forgets_the_table() {
    let mut fixture = Fixture::empty();
    let engine = fixture.spawn("engine", path_emitter(true));
    fixture.frame_with(move |ctx| ctx.emitters.play(engine, ctx.world));
    let playback = fixture.harness.sound_emitters().playing(engine).expect("the loop plays");

    fixture.frame_with(|ctx| request_transform_hierarchy_reset(ctx.world));
    assert!(!fixture.harness.audio().is_playing(playback));
    assert_eq!(fixture.harness.sound_emitters().playing(engine), None);
}

#[test]
fn stopping_one_of_two_binding_emitters_on_one_slot_ends_its_source_alone() {
    let mut fixture = Fixture::empty();
    fixture.play_song();
    let drum = fixture.spawn("drum", binding_emitter());
    let bell = fixture.spawn("bell", binding_emitter());
    let conductor = fixture.spawn("conductor", SoundEmitter::default());
    fixture.attach_script(conductor, "stop_target", &[("target", ScriptValue::Str("drum".into()))]);
    fixture.frame_with(move |ctx| {
        ctx.emitters.play(drum, ctx.world);
        ctx.emitters.play(bell, ctx.world);
    });
    fixture.settle();
    let snapshot = |fixture: &Fixture| fixture.harness.audio().song_snapshot().expect("a song").conductor;
    assert_eq!(snapshot(&fixture).live_sources, 2);

    fixture.script_frame();
    fixture.settle();
    assert_eq!(fixture.harness.sound_sources().source_of(drum), None);
    assert!(fixture.harness.sound_sources().source_of(bell).is_some(), "the bell plays on");
    assert_eq!(snapshot(&fixture).live_sources, 1, "the song still counts the bell's notes");
}

#[test]
fn a_binding_emitter_that_is_spatial_or_loops_is_refused_at_load_naming_the_entity() {
    for (field, problem) in [("spatial: Some(Spatial(max_distance: 8.0))", "spatial"), ("looping: true", "loop")] {
        let scene = format!(
            r#"SceneData(name: "s", entities: [EntityData(name: Some("drum"), components: [
                SoundEmitter(sound: Binding("tone"), {field}),
            ])])"#
        );
        let data = SceneLoader::parse(&scene).expect("the text parses");
        let mut world = ecs::World::new();
        match SceneLoader::instantiate(&data, &mut world, &mut StubResolver::default()) {
            Err(SceneLoadError::ComponentError(message)) => {
                assert!(message.contains("entity 'drum'") && message.contains(problem), "{message}");
            }
            other => panic!("expected a component error, got {:?}", other.map(|instance| instance.entities)),
        }
    }
}

#[test]
fn a_negative_volume_is_refused_and_a_zero_reach_is_floored_at_load() {
    let refused = r#"SceneData(name: "s", entities: [EntityData(name: Some("loud"), components: [
        SoundEmitter(volume: -1.0),
    ])])"#;
    let data = SceneLoader::parse(refused).expect("the text parses");
    assert!(matches!(
        SceneLoader::instantiate(&data, &mut ecs::World::new(), &mut StubResolver::default()),
        Err(SceneLoadError::ComponentError(message)) if message.contains("entity 'loud'")
    ));

    let floored = r#"SceneData(name: "s", entities: [EntityData(components: [
        SoundEmitter(spatial: Some(Spatial(max_distance: 0.0))),
    ])])"#;
    let (world, instance) = engine_core::test_support::load_ron(floored);
    let emitter = world.get::<SoundEmitter>(instance.entities[0]).expect("the emitter loads");
    assert_eq!(emitter.spatial, Some(Spatial { max_distance: Spatial::MIN_MAX_DISTANCE }));
}
