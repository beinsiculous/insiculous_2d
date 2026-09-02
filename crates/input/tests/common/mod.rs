//! Shared setup for the input crate's integration tests.

use input::prelude::*;

/// Queue `events` and process them, as the engine does at the top of a
/// frame. Pair with `input.end_frame()` to close the frame.
pub fn frame(input: &mut InputHandler, events: &[InputEvent]) {
    for event in events {
        input.queue_event(event.clone());
    }
    input.process_queued_events();
}
