use glam::Vec2;
use input::InputHandler;

use crate::{Color, DrawCommand, Rect, UIContext};
use crate::test_support::{frame, input_with_mouse, press_at, release, FIXTURE_FONT, WINDOW};

#[test]
fn points_become_physical_rects_clips_and_strokes() {
    let mut ui = UIContext::new();
    ui.set_pixels_per_point(2.0);
    frame(&mut ui, &InputHandler::new(), |ui| {
        let bounds = Rect::new(10.0, 10.0, 10.0, 10.0);
        ui.push_clip_rect(bounds);
        ui.rect(bounds, Color::WHITE);
        ui.rect_border(bounds, Color::WHITE, 1.0, 3.0);
        ui.pop_clip_rect();
    });
    let physical = Rect::new(20.0, 20.0, 20.0, 20.0);
    assert!(matches!(ui.draw_list().commands(), [
        DrawCommand::PushClipRect { bounds: clip },
        DrawCommand::Rect { bounds, .. },
        DrawCommand::RectBorder { bounds: border, width: 2.0, corner_radius: 6.0, .. },
        DrawCommand::PopClipRect,
    ] if *clip == physical && *bounds == physical && *border == physical));
}

#[test]
fn a_scaled_label_rasterizes_at_the_physical_font_size() {
    let mut ui = UIContext::new();
    ui.load_font(FIXTURE_FONT).unwrap();
    frame(&mut ui, &InputHandler::new(), |ui| {
        ui.label_styled("Height", Vec2::splat(40.0), Color::WHITE, 28.0);
        ui.set_pixels_per_point(2.0);
        ui.label_styled("Height", Vec2::splat(20.0), Color::WHITE, 14.0);
    });
    let text: Vec<_> = ui.draw_list().commands().iter().filter_map(|command| match command {
        DrawCommand::Text { data, .. } => Some(data),
        _ => None,
    }).collect();
    assert_eq!(text.len(), 2);
    assert_eq!(text[0].position, text[1].position);
    assert_eq!(text[1].font_size, 28.0);
    for (unscaled, scaled) in text[0].glyphs.iter().zip(&text[1].glyphs) {
        assert_eq!((scaled.width, scaled.height), (unscaled.width, unscaled.height));
        assert_eq!(scaled.bitmap, unscaled.bitmap);
    }
}

#[test]
fn a_long_label_measures_in_points_and_covers_its_physical_advance() {
    let mut ui = UIContext::new();
    ui.load_font(FIXTURE_FONT).unwrap();
    let text = "Editor UI ".repeat(6);
    assert_eq!(text.chars().count(), 60);
    let original = ui.measure_text_styled(&text, 14.0);
    for scale in [1.25, 2.0] {
        ui.set_pixels_per_point(scale);
        let measured = ui.measure_text_styled(&text, 14.0);
        assert!((measured.x / original.x - 1.0).abs() < 0.01);
        frame(&mut ui, &InputHandler::new(), |ui| {
            ui.label_styled(&text, Vec2::ZERO, Color::WHITE, 14.0);
        });
        let DrawCommand::Text { data, .. } = &ui.draw_list().commands()[0] else {
            panic!("the label must rasterize");
        };
        assert!(measured.x * scale >= data.width);
        assert!(measured.y * scale >= data.height);
        assert!(measured.x * scale - data.width < 1.0);
    }
}

#[test]
fn a_points_button_only_clicks_at_its_scaled_physical_position() {
    let mut ui = UIContext::new();
    ui.set_pixels_per_point(2.0);
    let mut input = InputHandler::new();
    let bounds = Rect::new(100.0, 100.0, 30.0, 20.0);
    for (position, clicked) in [(bounds.center(), false), (bounds.center() * 2.0, true)] {
        press_at(&mut ui, &mut input, position, |ui| ui.button("button", "Hit", bounds));
        assert_eq!(release(&mut ui, &mut input, |ui| ui.button("button", "Hit", bounds)), clicked);
    }
}

#[test]
fn a_physical_game_pointer_is_blocked_by_scaled_editor_chrome() {
    let mut ui = UIContext::new();
    frame(&mut ui, &InputHandler::new(), |ui| {
        ui.set_pixels_per_point(2.0);
        ui.begin_overlay(Rect::new(100.0, 100.0, 40.0, 30.0));
        ui.end_overlay();
        ui.set_pixels_per_point(1.0);
        assert!(ui.is_input_blocked_at(Vec2::new(230.0, 230.0)));
        assert!(!ui.is_input_blocked_at(Vec2::new(115.0, 115.0)));
    });
}

#[test]
fn a_tooltip_keeps_its_offered_scale_when_the_frame_ends_at_another_scale() {
    let mut ui = UIContext::new();
    let bounds = Rect::new(100.0, 100.0, 40.0, 30.0);
    let input = input_with_mouse(bounds.center() * 2.0, false);
    for _ in 0..8 {
        ui.begin_frame_dt(&input, WINDOW, 0.1);
        ui.set_pixels_per_point(2.0);
        ui.tooltip(bounds, "Scale");
        ui.set_pixels_per_point(1.0);
        ui.end_frame();
    }
    assert!(ui.draw_list().commands().iter().any(|command| matches!(command,
        DrawCommand::TextPlaceholder { text, font_size, position, .. }
        if text == "Scale" && *font_size == ui.theme().tooltip.font_size * 2.0
            && position.x > 240.0
    )));
    assert_eq!(ui.pixels_per_point(), 1.0);
}

#[test]
fn a_scaled_field_scrubs_by_points_and_places_its_text_inside_its_box() {
    let mut ui = UIContext::new();
    ui.set_pixels_per_point(2.0);
    let mut input = InputHandler::new();
    let bounds = Rect::new(100.0, 100.0, 100.0, 30.0);
    let options = crate::FloatFieldOpts::range(0.0, 100.0);
    press_at(&mut ui, &mut input, bounds.center() * 2.0, |ui| {
        ui.float_input("number", 10.0, options, bounds)
    });
    let result = crate::test_support::move_to(&mut ui, &mut input,
        bounds.center() * 2.0 + Vec2::new(20.0, 0.0), |ui| {
            ui.float_input("number", 10.0, options, bounds)
        });
    assert!(result.scrubbing);
    assert_eq!(result.value, 20.0);
    assert!(ui.draw_list().commands().iter().any(|command| matches!(command,
        DrawCommand::TextPlaceholder { position, .. }
        if position.x >= 200.0 && position.x < 400.0 && position.y > 200.0 && position.y < 260.0
    )));
}
