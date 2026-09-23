//! A game's update loop driven headlessly through `test_support::GameHarness`:
//! the real runner and the real frame, with no window and no GPU.

use std::fs;

use ecs::EntityId;
use engine_core::assets::sprite_sheet::SpriteSheet;
use engine_core::prelude::{Game, GameConfig, GameContext, KeyCode, Lifetime};
use engine_core::test_support::GameHarness;
use input::InputEvent;

const SIDECAR: &str = r#"SheetFile(
    version: 1,
    cell: (16, 16),
    clips: [
        ("walk", (frames: [0, 1, 2, 3], fps: 8.0)),
        ("idle", (frames: [4], fps: 1.0)),
    ],
)"#;

const FRAME: f32 = 1.0 / 60.0;

/// Loads a sheet and spawns a short-lived entity in `init`, then records
/// what each `update` saw of the space key and, on its second frame, sets
/// the title and asks to exit.
#[derive(Default)]
struct Probe {
    init_calls: usize,
    frames: usize,
    sheet: Option<SpriteSheet>,
    spark: Option<EntityId>,
    space_just_pressed_by_frame: Vec<bool>,
}

impl Game for Probe {
    fn init(&mut self, ctx: &mut GameContext) {
        self.init_calls += 1;
        self.sheet = Some(ctx.assets.load_sprite_sheet("deion.png").expect("the sheet loads headlessly"));
        let spark = ctx.world.create_entity();
        // One and a half frames: alive after the first tail, gone after the second.
        ctx.world.add_component(&spark, Lifetime::new(1.5 * FRAME)).ok();
        self.spark = Some(spark);
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.frames += 1;
        self.space_just_pressed_by_frame.push(ctx.input.is_key_just_pressed(KeyCode::Space));
        if self.frames == 2 {
            ctx.set_window_title("Probe, frame two");
            ctx.request_exit();
        }
    }
}

#[test]
fn a_headless_game_runs_init_once_loads_its_art_and_gets_the_engines_frame() {
    let asset_root = tempfile::tempdir().expect("tempdir");
    image::RgbaImage::new(64, 32).save(asset_root.path().join("deion.png")).expect("write png");
    fs::write(asset_root.path().join("deion.sheet.ron"), SIDECAR).expect("write sidecar");
    let config = GameConfig::new("Probe")
        .with_asset_base_path(asset_root.path().to_string_lossy().to_string());
    let mut harness = GameHarness::new(Probe::default(), config);

    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::Space)]);
    let spark = harness.game().spark.expect("init spawned the spark");
    let spark_alive_after_first_frame = harness.world().entities().contains(&spark);
    harness.step(FRAME, &[]);

    let game = harness.game();
    assert_eq!((game.init_calls, game.frames), (1, 2), "init runs once, on the first step");
    let sheet = game.sheet.as_ref().expect("init stored the sheet");
    assert!(
        sheet.texture.id > 0 && (sheet.grid.cols, sheet.grid.rows) == (4, 2),
        "a real handle and the sidecar's grid over the 64x32 PNG: handle {}, grid {}x{}",
        sheet.texture.id,
        sheet.grid.cols,
        sheet.grid.rows
    );
    assert!(
        spark_alive_after_first_frame && !harness.world().entities().contains(&spark),
        "the engine's tail expires the lifetime on the second frame, not the first"
    );
    assert_eq!(
        game.space_just_pressed_by_frame,
        vec![true, false],
        "a press is just-pressed on the frame it arrives and held, not just-pressed, after"
    );
    assert_eq!(harness.window_title(), "Probe, frame two");
    assert!(harness.exit_requested());
}

/// Records what reaches it outside a frame: key handlers and resizes, and
/// whether `init` had run when the first key arrived.
#[derive(Default)]
struct Listener {
    initialized: bool,
    init_preceded_first_key: Option<bool>,
    first_update_saw_inits_title: Option<bool>,
    keys: Vec<(KeyCode, bool)>,
    resizes: Vec<(u32, u32)>,
}

impl Game for Listener {
    fn init(&mut self, ctx: &mut GameContext) {
        self.initialized = true;
        ctx.set_window_title("from init");
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.first_update_saw_inits_title.get_or_insert(ctx.window_title_requested());
    }

    fn on_key_pressed(&mut self, key: KeyCode, _ctx: &mut GameContext) {
        self.init_preceded_first_key.get_or_insert(self.initialized);
        self.keys.push((key, true));
    }

    fn on_key_released(&mut self, key: KeyCode, _ctx: &mut GameContext) {
        self.keys.push((key, false));
    }

    fn on_resize(&mut self, width: u32, height: u32) {
        self.resizes.push((width, height));
    }
}

#[test]
fn the_harness_delivers_what_the_window_loop_delivers_outside_the_frame() {
    let asset_root = tempfile::tempdir().expect("tempdir");
    let config = GameConfig::new("Listener")
        .with_asset_base_path(asset_root.path().to_string_lossy().to_string());
    let mut harness = GameHarness::new(Listener::default(), config);

    harness.step(
        FRAME,
        &[InputEvent::KeyPressed(KeyCode::KeyW), InputEvent::KeyReleased(KeyCode::KeyW)],
    );
    assert_eq!(
        harness.game().keys,
        vec![(KeyCode::KeyW, true), (KeyCode::KeyW, false)],
        "a key event reaches the game's handler, press and release, in order"
    );
    assert_eq!(
        harness.game().init_preceded_first_key,
        Some(true),
        "a key on the first step finds init already run, as in a window"
    );
    assert_eq!(
        harness.game().first_update_saw_inits_title,
        Some(true),
        "the first update sees what init requested even when a key ran init first"
    );

    // The same contract when the frame runs init itself, with no key ahead of it.
    let config = GameConfig::new("Listener, frame first")
        .with_asset_base_path(asset_root.path().to_string_lossy().to_string());
    let mut frame_first = GameHarness::new(Listener::default(), config);
    frame_first.step(FRAME, &[]);
    assert_eq!(
        frame_first.game().first_update_saw_inits_title,
        Some(true),
        "the first update sees what init requested when the frame ran init"
    );

    harness.set_window_size(glam::Vec2::new(320.0, 200.0));
    assert_eq!(harness.game().resizes, vec![(320, 200)], "a resize reaches on_resize");

    let title = harness.context(|_game, ctx| {
        ctx.set_window_title("from a lent context");
        ctx.window_title_requested()
    });
    assert!(title && harness.window_title() == "from a lent context", "a title set through context reads back at once");

    let (loaded, count_after_load, unloaded, count_after_unload) = harness.context(|_game, ctx| {
        let handle = ctx.assets.create_solid_color(2, 2, [255, 0, 0, 255]).expect("a solid colour loads");
        let loaded = ctx.assets.has_texture(handle);
        let count_after_load = ctx.assets.texture_count();
        let unloaded = ctx.assets.unload_texture(handle);
        (loaded, count_after_load, unloaded, ctx.assets.texture_count())
    });
    assert_eq!(
        (loaded, count_after_load, unloaded, count_after_unload),
        (true, 1, true, 0),
        "a headless load is a load: present, counted, and unloadable once"
    );
}
