//! The startup cards through `test_support::GameHarness`: the real runner and
//! frame holding a game behind the cards a config names, with no window and no
//! GPU. The cards are small PNGs the test writes itself.

use engine_core::prelude::{AxisDirection, Game, GameConfig, GameContext, GamepadAxis, KeyCode};
use engine_core::test_support::GameHarness;
use input::InputEvent;
use ui::DrawCommand;

/// An eighth of a second: exact in binary, so sixteen frames are exactly 2.0 s.
const FRAME: f32 = 0.125;
/// A card's life untouched. The clock runs before the frame draws, so the frame
/// on which a card reaches 2.0 s already draws the next card, at alpha 0: the
/// first card is drawn on frames 0 to 14, the second on 15 to 30.
const FRAMES_PER_CARD: usize = 16;
const WINDOW: (u32, u32) = (800, 600);

/// Records what reaches the game: `init`, every `update`, every key handler
/// call, and the keys each `update` saw go down.
#[derive(Default)]
struct Recorder {
    init_calls: usize,
    updates: usize,
    key_handler_calls: usize,
    keys_seen_by_update: Vec<Vec<KeyCode>>,
    stick_crossings_seen_by_update: usize,
}

impl Game for Recorder {
    fn init(&mut self, _ctx: &mut GameContext) {
        self.init_calls += 1;
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.updates += 1;
        self.keys_seen_by_update.push(ctx.input.keyboard().just_pressed_keys().to_vec());
        let crossed = ctx.input.gamepads().iter().any(|(_, pad)| {
            pad.axis_just_activated(GamepadAxis::LeftStickY, AxisDirection::Positive, 0.5)
        });
        self.stick_crossings_seen_by_update += usize::from(crossed);
    }

    fn on_key_pressed(&mut self, _key: KeyCode, _ctx: &mut GameContext) {
        self.key_handler_calls += 1;
    }
}

/// Two cards in a temp asset directory, each a flat colour whose top-left pixel
/// differs from the rest, so the backdrop read is the corner and not the body.
struct Cards {
    directory: tempfile::TempDir,
}

const STUDIO_CORNER: [u8; 4] = [20, 16, 31, 255];
const ENGINE_CORNER: [u8; 4] = [74, 68, 88, 255];

impl Cards {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(directory.path().join("sprites")).expect("sprites directory");
        for (name, corner) in [("studio.png", STUDIO_CORNER), ("engine.png", ENGINE_CORNER)] {
            let mut card = image::RgbaImage::from_pixel(320, 192, image::Rgba([200, 200, 200, 255]));
            card.put_pixel(0, 0, image::Rgba(corner));
            card.save(directory.path().join("sprites").join(name)).expect("write card");
        }
        Self { directory }
    }

    fn config(&self, cards: &[&str]) -> GameConfig {
        GameConfig::new("Cards")
            .with_size(WINDOW.0, WINDOW.1)
            .with_asset_base_path(self.directory.path().to_string_lossy().to_string())
            .with_startup_splashes(cards.iter().copied())
    }

    fn harness(&self) -> GameHarness<Recorder> {
        GameHarness::with_startup_splashes(
            Recorder::default(),
            self.config(&["sprites/studio.png", "sprites/engine.png"]),
        )
    }
}

fn press(key: KeyCode) -> InputEvent {
    InputEvent::KeyPressed(key)
}

fn release(key: KeyCode) -> InputEvent {
    InputEvent::KeyReleased(key)
}

/// The corner colour of the card the last frame drew, read from its margins.
fn card_backdrop(harness: &GameHarness<Recorder>) -> Option<[u8; 3]> {
    harness.ui_commands().iter().skip(1).find_map(|command| match command {
        DrawCommand::Rect { color, .. } => {
            let [r, g, b, _] = color.to_rgba8();
            Some([r, g, b])
        }
        _ => None,
    })
}

#[test]
fn the_game_is_held_through_both_cards_and_starts_the_frame_after_the_handoff() {
    let cards = Cards::new();
    let mut harness = cards.harness();

    for frame in 0..FRAMES_PER_CARD - 1 {
        harness.step(FRAME, &[]);
        assert_eq!(card_backdrop(&harness), Some([20, 16, 31]), "frame {frame}: the studio card");
    }
    for frame in FRAMES_PER_CARD - 1..2 * FRAMES_PER_CARD - 1 {
        assert!(harness.startup_holds_game(), "frame {frame}");
        harness.step(FRAME, &[]);
        assert_eq!(card_backdrop(&harness), Some([74, 68, 88]), "frame {frame}: the engine card");
    }
    harness.step(FRAME, &[]);
    assert_eq!(harness.game().init_calls, 0, "no init through both cards");
    assert!(harness.startup_holds_game(), "the handoff frame still holds the game");
    let handoff_frame: Vec<&DrawCommand> = harness.ui_commands().iter().collect();
    assert!(
        matches!(handoff_frame.as_slice(), [DrawCommand::Rect { color, .. }] if color.to_rgba8() == [0, 0, 0, 255]),
        "the handoff frame is black: {handoff_frame:?}"
    );

    harness.step(FRAME, &[]);
    assert!(!harness.startup_holds_game());
    assert_eq!((harness.game().init_calls, harness.game().updates), (1, 1), "init and update, on the next frame");
}

#[test]
fn a_press_after_a_cards_fade_in_skips_it_and_the_title_never_sees_the_press() {
    let cards = Cards::new();
    let mut harness = cards.harness();
    harness.steps(3, FRAME);

    harness.step(FRAME, &[press(KeyCode::Space)]);
    assert_eq!(card_backdrop(&harness), Some([74, 68, 88]), "the skip frame draws the next card's first frame");
    harness.step(FRAME, &[release(KeyCode::Space)]);
    harness.steps(2, FRAME);

    harness.step(FRAME, &[press(KeyCode::Enter)]);
    assert!(harness.startup_holds_game(), "the last skip starts the handoff");
    assert_eq!(harness.game().key_handler_calls, 0, "no key reached the game's handler");

    // The game's first frames: Enter is still held, its press long gone; a new
    // press inside the quiet period is swallowed, one after it is seen.
    harness.step(FRAME, &[]);
    harness.step(FRAME, &[release(KeyCode::Enter), press(KeyCode::KeyA)]);
    harness.step(FRAME, &[press(KeyCode::KeyB)]);
    let game = harness.game();
    assert_eq!(game.init_calls, 1);
    assert_eq!(
        game.keys_seen_by_update,
        vec![vec![], vec![], vec![KeyCode::KeyB]],
        "the skip and the mashed press never reach the title; the press after the quiet period does"
    );
    assert_eq!(game.key_handler_calls, 1, "nor its key handler: only the press after the quiet period");
}

#[test]
fn a_stick_nudged_in_the_quiet_period_moves_nothing_and_one_after_it_does() {
    let cards = Cards::new();
    let mut harness = cards.harness();
    harness.steps(3, FRAME);
    harness.step(FRAME, &[press(KeyCode::Space)]);
    harness.steps(3, FRAME);
    harness.step(FRAME, &[press(KeyCode::Enter)]);
    harness.step(FRAME, &[]);
    assert!(!harness.startup_holds_game(), "the game runs, its quiet period begun");

    let stick = |value| InputEvent::GamepadAxisUpdated(0, GamepadAxis::LeftStickY, value);
    harness.step(FRAME, &[stick(1.0)]);
    harness.step(FRAME, &[stick(0.0)]);
    assert_eq!(harness.game().stick_crossings_seen_by_update, 0, "a nudge in the quiet period");
    harness.step(FRAME, &[stick(1.0)]);
    assert_eq!(harness.game().stick_crossings_seen_by_update, 1, "and one after it");
}

#[test]
fn a_press_during_a_cards_fade_in_is_ignored_and_a_queued_boot_press_cannot_skip_the_first_card() {
    let cards = Cards::new();
    let mut harness = cards.harness();
    harness.step(FRAME, &[press(KeyCode::Space)]);
    harness.step(FRAME, &[release(KeyCode::Space), press(KeyCode::Enter)]);
    for frame in 2..FRAMES_PER_CARD - 1 {
        harness.step(FRAME, &[]);
        assert_eq!(card_backdrop(&harness), Some([20, 16, 31]), "frame {frame}: the studio card plays on");
    }
    harness.step(FRAME, &[]);
    assert_eq!(card_backdrop(&harness), Some([74, 68, 88]), "and ends on its own at 2.0 s");
}

#[test]
fn a_card_frame_draws_the_black_base_then_the_backdrop_around_the_card_then_the_card() {
    let cards = Cards::new();
    let mut harness = cards.harness();
    harness.step(FRAME, &[]);

    let commands = harness.ui_commands();
    let (base, rest) = commands.split_first().expect("the frame draws");
    assert!(matches!(base, DrawCommand::Rect { bounds, color, .. }
        if (bounds.width, bounds.height) == (800.0, 600.0) && color.to_rgba8() == [0, 0, 0, 255]));
    let Some((DrawCommand::Image { bounds: card, tint, .. }, margins)) = rest.split_last() else {
        panic!("the card is drawn last: {commands:?}");
    };
    assert_eq!((card.x, card.y, card.width, card.height), (80.0, 108.0, 640.0, 384.0), "2x, centred");
    assert_eq!(margins.len(), 4);
    for margin in margins {
        let DrawCommand::Rect { bounds, color, .. } = margin else { panic!("a margin is a rectangle") };
        let overlaps = bounds.x < card.x + card.width
            && card.x < bounds.x + bounds.width
            && bounds.y < card.y + card.height
            && card.y < bounds.y + bounds.height;
        assert!(!overlaps, "nothing fades over the card: {bounds:?}");
        assert_eq!(color.a, tint.a, "the backdrop and the card share one alpha");
    }
    let depths: Vec<f32> = commands.iter().map(DrawCommand::depth).collect();
    assert!(depths.windows(2).all(|pair| pair[0] < pair[1]), "drawn in rising depth: {depths:?}");
    assert_eq!(tint.a, FRAME / 0.25, "one frame into the fade-in");
}

#[test]
fn no_cards_or_only_unreadable_ones_start_the_game_on_the_first_frame() {
    let cards = Cards::new();
    for names in [&[][..], &["sprites/missing.png"][..]] {
        let mut harness = GameHarness::with_startup_splashes(Recorder::default(), cards.config(names));
        harness.step(FRAME, &[]);
        assert!(!harness.startup_holds_game(), "{names:?}");
        assert_eq!(harness.game().init_calls, 1, "{names:?}: init on the first frame, as ever");
    }
}

#[test]
fn a_harness_built_with_new_drops_the_configs_cards() {
    let cards = Cards::new();
    let mut harness = GameHarness::new(Recorder::default(), cards.config(&["sprites/studio.png"]));
    harness.step(FRAME, &[]);
    assert_eq!(harness.game().init_calls, 1);
}

#[test]
#[should_panic(expected = "a startup splash is still running")]
fn a_context_while_a_card_shows_is_refused() {
    let cards = Cards::new();
    let mut harness = cards.harness();
    harness.step(FRAME, &[]);
    harness.context(|game, _| game.init_calls);
}
