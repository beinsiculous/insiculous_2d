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
/// processes them, wheel notches accumulate, trackpad pixels normalize to
/// notches at 100 px per notch, and the frame end clears the delta.
#[test]
fn test_wheel_notches_and_trackpad_pixels_accumulate_as_notches_and_clear_each_frame() {
    let mut input = InputHandler::new();

    input.handle_window_event(&wheel_event(MouseScrollDelta::LineDelta(0.0, 1.0)));
    input.handle_window_event(&wheel_event(MouseScrollDelta::LineDelta(0.0, 0.5)));
    assert_eq!(input.mouse_wheel_delta(), 0.0, "window events queue until processed");
    input.process_queued_events();
    assert_eq!(input.mouse_wheel_delta(), 1.5);
    input.end_frame();
    assert_eq!(input.mouse_wheel_delta(), 0.0);

    let one_notch_of_pixels = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 100.0));
    input.handle_window_event(&wheel_event(one_notch_of_pixels));
    input.process_queued_events();
    assert_eq!(input.mouse_wheel_delta(), 1.0, "100 px is one notch");
    input.end_frame();

    let half_a_notch = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 50.0));
    input.handle_window_event(&wheel_event(half_a_notch));
    input.process_queued_events();
    assert_eq!(input.mouse_wheel_delta(), 0.5, "a trackpad streams fractions of a notch");
    input.end_frame();

    input.handle_window_event(&wheel_event(MouseScrollDelta::LineDelta(0.0, -2.0)));
    input.process_queued_events();
    assert_eq!(input.mouse_wheel_delta(), -2.0);
}

/// The winit boundary for a scaled canvas: the browser reports the pointer in
/// the pixels of the box the page shows the canvas in, and a game whose
/// surface keeps its configured size while the page scales the canvas reads
/// the pointer in the surface's pixels only through the scale.
#[test]
fn test_pointer_scale_maps_the_shown_box_pixels_onto_the_surface() {
    let mut input = InputHandler::new();
    let cursor_at = |x: f64, y: f64| WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(x, y),
    };

    input.handle_window_event(&cursor_at(162.0, 121.5));
    input.process_queued_events();
    assert_eq!(input.mouse_position(), MousePosition { x: 162.0, y: 121.5 }, "1:1 by default");

    // An 800×600 surface shown in a 324×243 box: the box's centre is the surface's centre.
    input.set_pointer_scale(800.0 / 324.0, 600.0 / 243.0);
    input.handle_window_event(&cursor_at(162.0, 121.5));
    input.process_queued_events();
    let position = input.mouse_position();
    assert!((position.x - 400.0).abs() < 1e-3 && (position.y - 300.0).abs() < 1e-3, "got {position:?}");
    // The move after a scale change is measured from nothing: the old position
    // was in the old scale, and a delta against it would be a spike.
    assert_eq!(input.mouse_movement_delta(), (0.0, 0.0), "no spike across the scale change");
    input.end_frame();
    input.handle_window_event(&cursor_at(163.0, 121.5));
    input.process_queued_events();
    let (delta_x, delta_y) = input.mouse_movement_delta();
    assert!((delta_x - 800.0 / 324.0).abs() < 1e-3 && delta_y.abs() < 1e-3, "got ({delta_x}, {delta_y})");
}
