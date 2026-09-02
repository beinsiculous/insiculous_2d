//! Integration tests: the mouse's per-frame delta model and scroll normalization.

use input::prelude::*;
use winit::dpi::PhysicalPosition;
use winit::event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent};

#[test]
fn test_movement_delta_ignores_the_startup_warp_sums_within_a_frame_and_resets_per_frame() {
    let mut mouse = MouseState::new();

    // The first update only establishes the position: its delta against the
    // default (0, 0) would be a spurious startup warp
    mouse.update_position(10.0, 0.0);
    assert_eq!(mouse.position(), MousePosition { x: 10.0, y: 0.0 });
    assert_eq!(mouse.movement_delta(), (0.0, 0.0));

    // Several move events in one frame (high polling rate) sum to the full
    // frame movement: (15,5)-(10,0) + (12,8)-(15,5) = (2,8)
    mouse.update_position(15.0, 5.0);
    mouse.update_position(12.0, 8.0);
    assert_eq!(mouse.position(), MousePosition { x: 12.0, y: 8.0 });
    assert_eq!(mouse.movement_delta(), (2.0, 8.0));

    // End of frame resets the delta even though the mouse stays still ...
    mouse.clear_frame_state();
    assert_eq!(mouse.movement_delta(), (0.0, 0.0));
    mouse.clear_frame_state();
    assert_eq!(mouse.movement_delta(), (0.0, 0.0));

    // ... and the next move is measured from the current position
    mouse.update_position(15.0, 12.0);
    assert_eq!(mouse.movement_delta(), (3.0, 4.0));
}

fn wheel_event(delta: MouseScrollDelta) -> WindowEvent {
    WindowEvent::MouseWheel {
        device_id: DeviceId::dummy(),
        delta,
        phase: TouchPhase::Moved,
    }
}

/// The winit boundary for scrolling: window events queue until the frame
/// processes them, wheel lines accumulate, trackpad pixels normalize to lines
/// at 16 px per line, and the frame end clears the delta.
#[test]
fn test_wheel_lines_and_trackpad_pixels_accumulate_as_lines_and_clear_each_frame() {
    let mut input = InputHandler::new();

    input.handle_window_event(&wheel_event(MouseScrollDelta::LineDelta(0.0, 1.0)));
    input.handle_window_event(&wheel_event(MouseScrollDelta::LineDelta(0.0, 0.5)));
    assert_eq!(input.mouse_wheel_delta(), 0.0, "window events queue until processed");
    input.process_queued_events();
    assert_eq!(input.mouse_wheel_delta(), 1.5);
    input.end_frame();
    assert_eq!(input.mouse_wheel_delta(), 0.0);

    let thirty_two_pixels = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 32.0));
    input.handle_window_event(&wheel_event(thirty_two_pixels));
    input.process_queued_events();
    assert_eq!(input.mouse_wheel_delta(), 2.0, "32 px is two lines");
    input.end_frame();

    input.handle_window_event(&wheel_event(MouseScrollDelta::LineDelta(0.0, -2.0)));
    input.process_queued_events();
    assert_eq!(input.mouse_wheel_delta(), -2.0);
}
