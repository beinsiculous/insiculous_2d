//! Overlay/layer behavior of UIContext — back-compat contract for
//! `begin_overlay` after the UiLayer bands (issue #29).

use super::*;
use crate::{Color, Rect, UiLayer};
#[test]
fn test_begin_overlay_maps_to_floating_and_blocks() {
    // Back-compat contract: begin_overlay = Floating layer + input
    // blocking, exactly as before the UiLayer bands existed.
    let mut ui = UIContext::new();
    let rect = Rect::new(10.0, 10.0, 100.0, 100.0);

    ui.begin_overlay(rect);
    assert_eq!(ui.draw_list().current_layer(), UiLayer::Floating);
    ui.rect(Rect::new(20.0, 20.0, 10.0, 10.0), Color::RED);
    ui.end_overlay();
    assert_eq!(ui.draw_list().current_layer(), UiLayer::Content);

    assert!(ui.is_input_blocked_at(glam::Vec2::new(50.0, 50.0)));
    assert!(!ui.is_input_blocked_at(glam::Vec2::new(500.0, 500.0)));

    // The overlay command reaches the flushed stream at end_frame.
    ui.end_frame();
    assert!(ui.draw_list().commands().iter().any(|c| c.depth() >= UiLayer::Floating.depth_base()));
}
