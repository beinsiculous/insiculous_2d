//! Key handlers set state; they never emit. The window loop runs a handler
//! between frames and the next frame flushes the event bus first, so an
//! event a handler emits never reaches `update`. The harness delivers keys
//! in the window loop's order, which is what this pins.

use engine_core::prelude::{Game, GameConfig, GameContext, KeyCode};
use engine_core::test_support::GameHarness;
use input::InputEvent;

const FRAME: f32 = 1.0 / 60.0;

struct Pinged;

/// Emits `Pinged` from its key handler and from its second `update`, and
/// records how many each `update` could read.
#[derive(Default)]
struct Emitter {
    handler_calls: usize,
    frames: usize,
    pings_read_by_frame: Vec<usize>,
}

impl Game for Emitter {
    fn on_key_pressed(&mut self, _key: KeyCode, ctx: &mut GameContext) {
        self.handler_calls += 1;
        ctx.world.emit_event(Pinged);
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.frames += 1;
        if self.frames == 2 {
            ctx.world.emit_event(Pinged);
        }
        self.pings_read_by_frame.push(ctx.world.read_events::<Pinged>().len());
    }
}

#[test]
fn a_world_event_emitted_in_a_key_handler_is_flushed_before_the_next_update_reads_it() {
    let mut harness = GameHarness::new(Emitter::default(), GameConfig::new("Emitter"));

    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::Space)]);
    harness.step(FRAME, &[]);

    let game = harness.game();
    assert_eq!(game.handler_calls, 1, "the handler ran, and emitted");
    assert_eq!(
        game.pings_read_by_frame,
        vec![0, 1],
        "the handler's event was flushed before update; one emitted from update is read"
    );
}
