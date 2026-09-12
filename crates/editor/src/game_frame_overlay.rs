//! Game-frame camera rect overlay for the scene view.
//!
//! Draws a muted frame outlining the world rectangle shown by the game when run
//! at its configured resolution (`GameConfig.width` × `height`), centered on the
//! main camera at its current zoom level.

use glam::Vec2;
use ui::{Color, Rect, UIContext};

use crate::viewport::SceneViewport;

/// Compute the world-space rectangle of the game frame given the camera position,
/// camera zoom, and configured frame dimensions (in pixels).
///
/// Half-extents are `frame / (2.0 * zoom)`.
pub fn game_frame_rect(position: Vec2, zoom: f32, frame: Vec2) -> Rect {
    let half_width = frame.x / (2.0 * zoom);
    let half_height = frame.y / (2.0 * zoom);
    Rect::new(
        position.x - half_width,
        position.y - half_height,
        half_width * 2.0,
        half_height * 2.0,
    )
}

/// Render the game-frame overlay rectangle as four line segments, projected through
/// `viewport` and clipped to `clip_bounds`.
pub fn render_game_frame_overlay(
    ui: &mut UIContext,
    viewport: &SceneViewport,
    world_rect: Rect,
    color: Color,
    clip_bounds: Rect,
) {
    ui.push_clip_rect(clip_bounds);

    let top_left = Vec2::new(world_rect.x, world_rect.y);
    let top_right = Vec2::new(world_rect.right(), world_rect.y);
    let bottom_right = Vec2::new(world_rect.right(), world_rect.bottom());
    let bottom_left = Vec2::new(world_rect.x, world_rect.bottom());

    crate::world_lines::draw_world_line(ui, viewport, top_left, top_right, color, 1.0);
    crate::world_lines::draw_world_line(ui, viewport, top_right, bottom_right, color, 1.0);
    crate::world_lines::draw_world_line(ui, viewport, bottom_right, bottom_left, color, 1.0);
    crate::world_lines::draw_world_line(ui, viewport, bottom_left, top_left, color, 1.0);

    ui.pop_clip_rect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_viewport;
    use ui::DrawCommand;

    #[test]
    fn test_game_frame_rect_at_zoom_2_is_half_frame_and_centered() {
        let camera_pos = Vec2::new(100.0, 50.0);
        let zoom = 2.0;
        let frame = Vec2::new(800.0, 600.0);

        let rect = game_frame_rect(camera_pos, zoom, frame);

        assert_eq!(rect.width, 400.0);
        assert_eq!(rect.height, 300.0);
        assert_eq!(rect.x, 100.0 - 200.0);
        assert_eq!(rect.y, 50.0 - 150.0);
        assert_eq!(rect.center(), camera_pos);
    }

    #[test]
    fn test_render_game_frame_overlay_draws_four_lines() {
        let mut ui = UIContext::new();
        let viewport = test_viewport();
        let rect = Rect::new(-100.0, -50.0, 200.0, 100.0);
        let color = Color::new(0.5, 0.5, 0.5, 0.5);
        let clip = Rect::new(0.0, 0.0, 800.0, 600.0);

        render_game_frame_overlay(&mut ui, &viewport, rect, color, clip);

        let line_count = ui
            .draw_list()
            .commands()
            .iter()
            .filter(|cmd| matches!(cmd, DrawCommand::Line { .. }))
            .count();
        assert_eq!(line_count, 4);
    }
}
