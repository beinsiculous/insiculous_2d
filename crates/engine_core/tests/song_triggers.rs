//! The host's side of a song through `GameHarness`: the asset base the runner sets,
//! scripts triggering bindings, and the frame tail ending the sources of entities that
//! are gone. The harness's audio manager is the disabled one, so the song plays on its
//! virtual clock, advanced by the wall time between frames.

use std::path::{Path, PathBuf};
use std::time::Duration;

use ecs::blackboard::Blackboard;
use ecs::script::{ScriptRef, ScriptValue, Scripts};
use ecs::{EntityId, Name};
use engine_core::prelude::{
    request_transform_hierarchy_reset, Game, GameConfig, GameContext, SourceId,
};
use engine_core::test_support::GameHarness;
use audio::PositionSnapshot;
use tempfile::TempDir;

const FRAME: f32 = 1.0 / 60.0;
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../music_midi/fixtures");
/// A blackboard flag the scripts trigger on and clear, so a test chooses the frame.
const PLAY_FLAG: &str = "play";

/// The cast fixture's flute track as the one part, the sine bank's one preset on it.
const SIDECAR: &str = r#"SongFile(
    version: 1,
    soundfont: "soundfonts/sine.sf2",
    parts: [(name: "melody", track: Name("channel 2"))],
    instruments: [(name: "sine", source: SoundFont(path: "soundfonts/sine.sf2", bank: 0, preset: 0))],
    bindings: [(action: "tone", part: "melody", instrument: "sine")],
)"#;

/// Triggers `params.binding` on its own entity when the flag is up; `despawn` also
/// despawns the entity in the same hook.
const PING: &str = r#"
// @param binding: str = "tone"
// @param despawn: bool = false
fn update(me, view, params, cmd, dt) {
    if view.blackboard_bool("play", false) {
        cmd.trigger(me, params.binding);
        if params.despawn { cmd.despawn(me); }
        cmd.set_blackboard_bool("play", false);
    }
}
"#;

/// Triggers `tone` on the entity named `params.target` when the flag is up.
const PING_TARGET: &str = r#"
// @param target: str = ""
fn update(me, view, params, cmd, dt) {
    if view.blackboard_bool("play", false) {
        cmd.trigger(params.target, "tone");
        cmd.set_blackboard_bool("play", false);
    }
}
"#;

type FrameHook = Box<dyn FnOnce(&mut GameContext)>;

/// Steps the script runner as a game does, then runs a test's one-frame hook.
#[derive(Default)]
struct Stage {
    hook: Option<FrameHook>,
}

impl Game for Stage {
    fn init(&mut self, ctx: &mut GameContext) {
        let asset_base = ctx.assets.base_path().to_string();
        ctx.scripts.reset(ctx.world, &asset_base);
    }

    fn update(&mut self, ctx: &mut GameContext) {
        ctx.scripts.early_update(ctx.world, ctx.input, ctx.players, ctx.delta_time, None);
        ctx.scripts.update(ctx.world, ctx.input, ctx.players, ctx.delta_time, &[], None);
        if let Some(hook) = self.hook.take() {
            hook(ctx);
        }
    }
}

/// The song, its bank, a recorded track and the scripts, under one asset base.
struct Fixture {
    directory: TempDir,
    harness: GameHarness<Stage>,
}

impl Fixture {
    /// A stage that has run `init`, with the first song playing.
    fn playing() -> Self {
        let directory = tempfile::tempdir().expect("tempdir");
        let copy = |from: &str, to: &str| {
            let target = directory.path().join(to);
            std::fs::create_dir_all(target.parent().expect("a parent")).expect("directories");
            std::fs::copy(Path::new(FIXTURES).join(from), target).expect("fixture copies");
        };
        copy("sine.sf2", "soundfonts/sine.sf2");
        copy("dansenapolitaine-cast.mid", "songs/first.mid");
        copy("dansenapolitaine-cast.mid", "songs/second.mid");
        copy("sine_48000_mono.wav", "music/hum.wav");
        let write = |to: &str, text: &str| {
            std::fs::create_dir_all(directory.path().join(to).parent().expect("a parent"))
                .expect("directories");
            std::fs::write(directory.path().join(to), text).expect("writes");
        };
        write("songs/first.mid.ron", SIDECAR);
        write("songs/second.mid.ron", SIDECAR);
        write("scripts/ping.rhai", PING);
        write("scripts/ping_target.rhai", PING_TARGET);
        let config = GameConfig::new("Song triggers")
            .with_asset_base_path(directory.path().to_string_lossy().to_string());
        let mut fixture = Self { directory, harness: GameHarness::new(Stage::default(), config) };
        fixture.harness.step(FRAME, &[]);
        fixture.play_song("first");
        fixture
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.directory.path().join(relative)
    }

    fn play_song(&mut self, stem: &str) {
        let path = self.path(&format!("songs/{stem}.mid"));
        self.harness.audio_mut().play_song(path).expect("the song loads");
    }

    fn spawn_plain(&mut self, name: &str) -> EntityId {
        let world = self.harness.world_mut();
        let entity = world.create_entity();
        world.add_component(&entity, Name(name.to_string())).ok();
        entity
    }

    fn spawn(&mut self, name: &str, script: &str, params: &[(&str, ScriptValue)]) -> EntityId {
        let entity = self.spawn_plain(name);
        let world = self.harness.world_mut();
        let mut script_ref = ScriptRef::new(script);
        script_ref.source_path = format!("scripts/{script}.rhai");
        for (key, value) in params {
            script_ref.params.insert(key.to_string(), value.clone());
        }
        world.add_component(&entity, Scripts(vec![script_ref])).ok();
        entity
    }

    fn raise_flag(&mut self) {
        let board = self.harness.world_mut().resource_mut::<Blackboard>().expect("init installed it");
        board.set(PLAY_FLAG, ScriptValue::Bool(true));
    }

    /// One frame whose scripts see the flag up.
    fn trigger_frame(&mut self) {
        self.raise_flag();
        self.harness.step(FRAME, &[]);
    }

    /// One frame in which the game, after its scripts, runs `hook`.
    fn frame_with(&mut self, hook: impl FnOnce(&mut GameContext) + 'static) {
        self.harness.game_mut().hook = Some(Box::new(hook));
        self.harness.step(FRAME, &[]);
    }

    /// Frames with wall time between them, so the virtual clock renders blocks and the
    /// conductor drains what was sent: a command waits at most one block for its drain.
    fn settle(&mut self) {
        for _ in 0..6 {
            std::thread::sleep(Duration::from_millis(3));
            self.harness.step(FRAME, &[]);
        }
    }

    fn conductor(&self) -> PositionSnapshot {
        self.harness.audio().song_snapshot().expect("a song is loaded").conductor
    }

    fn source_of(&self, entity: EntityId) -> Option<SourceId> {
        self.harness.sound_sources().source_of(entity)
    }
}

#[test]
fn a_song_plays_with_its_sidecar_paths_relative_to_the_configs_asset_base() {
    let fixture = Fixture::playing();
    assert!(fixture.harness.audio().is_song_playing(), "the bank resolved under the asset base");
}

#[test]
fn a_script_that_triggers_and_despawns_its_entity_plays_the_note_and_leaves_no_source() {
    let mut fixture = Fixture::playing();
    let bat = fixture.spawn("bat", "ping", &[("despawn", ScriptValue::Bool(true))]);
    fixture.trigger_frame();
    assert!(fixture.harness.world().validate_entity(&bat).is_err());
    assert_eq!(fixture.source_of(bat), None, "the tail ended it, with no call from the game");

    fixture.settle();
    let conductor = fixture.conductor();
    assert_eq!((conductor.accepted_triggers, conductor.live_sources), (1, 0));
}

#[test]
fn an_entity_destroyed_from_rust_has_its_source_ended_by_the_next_tail() {
    let mut fixture = Fixture::playing();
    let bat = fixture.spawn("bat", "ping", &[]);
    fixture.trigger_frame();
    fixture.settle();
    assert_eq!(fixture.conductor().live_sources, 1);

    fixture.harness.world_mut().remove_entity(&bat).ok();
    fixture.harness.step(FRAME, &[]);
    assert_eq!(fixture.source_of(bat), None);
    fixture.settle();
    assert_eq!(fixture.conductor().live_sources, 0);
}

#[test]
fn a_world_replacement_flushes_every_source_and_the_next_trigger_activates_anew() {
    let mut fixture = Fixture::playing();
    let bat = fixture.spawn("bat", "ping", &[]);
    fixture.trigger_frame();
    let before = fixture.source_of(bat).expect("the trigger minted a source");
    fixture.settle();

    fixture.frame_with(|ctx| request_transform_hierarchy_reset(ctx.world));
    assert_eq!(fixture.source_of(bat), None, "a flush forgets every source");
    fixture.settle();
    assert_eq!(fixture.conductor().live_sources, 0);

    fixture.trigger_frame();
    let after = fixture.source_of(bat).expect("a trigger re-mints");
    assert!(after.epoch > before.epoch + 1, "above the flush's own epoch");
    fixture.settle();
    let conductor = fixture.conductor();
    assert_eq!((conductor.accepted_triggers, conductor.live_sources), (2, 1));
}

#[test]
fn one_frames_triggers_reach_the_song_in_entity_order_then_script_order() {
    let mut fixture = Fixture::playing();
    let targets: Vec<EntityId> =
        ["a", "b", "c", "d"].iter().map(|name| fixture.spawn_plain(name)).collect();
    let target = |name: &str| [("target", ScriptValue::Str(name.to_string()))];
    let first = fixture.spawn("first", "ping_target", &target("d"));
    let second = fixture.spawn("second", "ping_target", &target("b"));
    // A second script on each: entity order first, then each entity's script order.
    for (entity, name) in [(first, "c"), (second, "a")] {
        let mut script_ref = ScriptRef::new("ping_target");
        script_ref.source_path = "scripts/ping_target.rhai".to_string();
        script_ref.params.insert("target".to_string(), ScriptValue::Str(name.to_string()));
        let world = fixture.harness.world_mut();
        world.get_mut::<Scripts>(entity).expect("scripts").0.push(script_ref);
    }

    fixture.trigger_frame();
    let epochs: Vec<u32> = targets
        .iter()
        .map(|entity| fixture.source_of(*entity).expect("each target triggered").epoch)
        .collect();
    let (a, b, c, d) = (epochs[0], epochs[1], epochs[2], epochs[3]);
    assert!(d < c && c < b && b < a, "submitted d, c, b, a: epochs {epochs:?}");
    fixture.settle();
    assert_eq!(fixture.conductor().accepted_triggers, 4);
}

#[test]
fn muting_the_music_or_the_sfx_bus_keeps_the_clock_and_the_triggers_running() {
    let mut fixture = Fixture::playing();
    fixture.spawn("bat", "ping", &[]);
    for (music_volume, sfx_volume) in [(0.0, 1.0), (1.0, 0.0)] {
        fixture.harness.audio_mut().set_music_volume(music_volume);
        fixture.harness.audio_mut().set_sfx_volume(sfx_volume);
        let before = fixture.conductor();
        fixture.trigger_frame();
        fixture.settle();
        let after = fixture.conductor();
        assert!(after.sample_position > before.sample_position, "the clock runs");
        assert_eq!(after.accepted_triggers, before.accepted_triggers + 1);
    }
}

#[test]
fn a_recorded_sound_plays_beside_a_song_without_touching_it() {
    let mut fixture = Fixture::playing();
    fixture.spawn("bat", "ping", &[]);
    let bytes = std::fs::read(fixture.path("music/hum.wav")).expect("the wav reads");
    let audio = fixture.harness.audio_mut();
    let blip = audio.load_sound_from_bytes(bytes).expect("the wav decodes");
    audio.play(blip).expect("a recorded sound plays");

    fixture.trigger_frame();
    fixture.settle();
    assert!(fixture.harness.audio().is_song_playing());
    assert_eq!(fixture.conductor().accepted_triggers, 1);
}

#[test]
fn a_song_change_keeps_each_live_source_and_the_new_song_admits_it() {
    let mut fixture = Fixture::playing();
    let bat = fixture.spawn("bat", "ping", &[]);
    fixture.trigger_frame();
    let in_first_song = fixture.source_of(bat).expect("the trigger minted a source");

    let hum = fixture.path("music/hum.wav");
    fixture.harness.audio_mut().play_music(hum).expect("the track loads");
    assert!(!fixture.harness.audio().is_song_playing(), "one music slot");
    fixture.trigger_frame();
    assert_eq!(fixture.harness.sound_sources().refused_triggers(), 1, "no song refuses it");

    fixture.play_song("second");
    fixture.trigger_frame();
    assert_eq!(fixture.source_of(bat), Some(in_first_song), "no flush on a song change");
    fixture.settle();
    let conductor = fixture.conductor();
    assert_eq!((conductor.accepted_triggers, conductor.live_sources), (1, 1));
}

#[test]
fn after_a_flush_the_menu_triggers_under_a_fresh_epoch_and_its_old_one_is_refused() {
    let mut fixture = Fixture::playing();
    let old_menu = fixture.harness.context(|_, ctx| {
        let menu = ctx.sources.menu();
        assert!(ctx.sources.trigger(ctx.audio, "tone", menu));
        menu
    });
    fixture.frame_with(|ctx| request_transform_hierarchy_reset(ctx.world));
    fixture.settle();

    let new_menu = fixture.harness.context(|_, ctx| {
        let menu = ctx.sources.menu();
        assert!(ctx.sources.trigger(ctx.audio, "tone", menu));
        assert!(ctx.sources.trigger(ctx.audio, "tone", old_menu), "sent; the conductor refuses it");
        menu
    });
    assert!(new_menu.epoch > old_menu.epoch);
    fixture.settle();
    let conductor = fixture.conductor();
    assert_eq!((conductor.accepted_triggers, conductor.stale_triggers), (2, 1));
}

#[test]
fn a_trigger_queued_in_the_frame_of_a_world_replacement_never_sounds() {
    let mut fixture = Fixture::playing();
    let bat = fixture.spawn("bat", "ping", &[]);
    fixture.raise_flag();
    fixture.frame_with(|ctx| request_transform_hierarchy_reset(ctx.world));
    assert_eq!(fixture.source_of(bat), None, "the trigger was dropped, not minted");
    fixture.settle();
    assert_eq!(fixture.conductor().accepted_triggers, 0);
    assert_eq!(fixture.harness.sound_sources().refused_triggers(), 0, "dropped, not refused");
}

#[test]
fn a_deactivation_a_full_queue_refused_ends_the_old_generation_before_its_re_entry_plays() {
    let mut fixture = Fixture::playing();
    let crowd: Vec<EntityId> = (0..17)
        .map(|index| fixture.spawn_plain(&format!("crowd {index}")))
        .collect();
    // Deactivations go out in entity order, so the last one spawned is the one refused.
    let bat = crowd[16];
    let old_bat = fixture.harness.context(|_, ctx| {
        crowd.iter().for_each(|entity| {
            ctx.sources.entity(*entity);
        });
        ctx.sources.entity(bat)
    });

    // In one frame, before the conductor drains anything: triggers fill the queue to
    // its lifecycle reserve, and seventeen deactivations overflow the reserve by one.
    let storm_crowd = crowd.clone();
    let rejected_before = fixture.conductor().commands_rejected;
    fixture.frame_with(move |ctx| {
        let game = ctx.sources.game();
        while ctx.sources.trigger(ctx.audio, "tone", game) {}
        for entity in storm_crowd {
            ctx.world.remove_entity(&entity).ok();
        }
    });
    assert_eq!(
        fixture.conductor().commands_rejected,
        rejected_before + 2,
        "the storm's last trigger and one deactivation, the bat's, were refused"
    );

    // Before the manager retries the refused deactivation, the id comes back as a newer
    // generation and triggers: refused, since a waiting deactivation goes first.
    let newer = EntityId::with_generation(bat.value(), bat.generation() + 1);
    fixture.harness.world_mut().create_entity_with_id(newer);
    let refused_before = fixture.harness.sound_sources().refused_triggers();
    let new_bat = fixture.harness.context(|_, ctx| {
        let source = ctx.sources.entity(newer);
        assert!(!ctx.sources.trigger(ctx.audio, "tone", source));
        source
    });
    assert_eq!(fixture.harness.sound_sources().refused_triggers(), refused_before + 1);
    fixture.settle();

    let stale_before = fixture.conductor().stale_triggers;
    fixture.harness.context(|_, ctx| {
        assert!(ctx.sources.trigger(ctx.audio, "tone", new_bat));
        assert!(ctx.sources.trigger(ctx.audio, "tone", old_bat));
    });
    fixture.settle();
    let conductor = fixture.conductor();
    assert_eq!(conductor.stale_triggers, stale_before + 1, "the old generation never revives");
    assert!(new_bat.generation > old_bat.generation && new_bat.epoch > old_bat.epoch);
    assert_eq!(fixture.source_of(newer), Some(new_bat));
}
