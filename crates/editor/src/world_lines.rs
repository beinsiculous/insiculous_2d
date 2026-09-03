//! Screen-projected world line drawing with non-finite coordinates guarded.

use glam::Vec2;
use ui::{Color, UIContext};

use crate::viewport::SceneViewport;

/// Draw a single line segment specified in world coordinates, projected to
/// screen space through `viewport`.
///
/// Skips the line if either projected screen coordinate is non-finite (e.g. from
/// an extreme camera state).
pub fn draw_world_line(
    ui: &mut UIContext,
    viewport: &SceneViewport,
    start: Vec2,
    end: Vec2,
    color: Color,
    width: f32,
) {
    let screen_start = viewport.world_to_screen(start);
    let screen_end = viewport.world_to_screen(end);
    if !screen_start.is_finite() || !screen_end.is_finite() {
        return;
    }
    ui.line(screen_start, screen_end, color, width);
}

/// Draw multiple line segments in world coordinates with a uniform color and stroke width.
pub fn draw_world_segments(
    ui: &mut UIContext,
    viewport: &SceneViewport,
    segments: impl IntoIterator<Item = (Vec2, Vec2)>,
    color: Color,
    width: f32,
) {
    for (start, end) in segments {
        draw_world_line(ui, viewport, start, end, color, width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_viewport;
    use ui::DrawCommand;

    fn line_count(ui: &UIContext) -> usize {
        ui.draw_list()
            .commands()
            .iter()
            .filter(|c| matches!(c, DrawCommand::Line { .. }))
            .count()
    }

    #[test]
    fn test_non_finite_endpoint_emits_no_line_draw_command() {
        let mut ui = UIContext::new();
        let viewport = test_viewport();
        let color = Color::WHITE;
        let width = 1.0;

        // Normal finite segment: emits a line draw command
        draw_world_line(
            &mut ui,
            &viewport,
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 10.0),
            color,
            width,
        );
        assert_eq!(line_count(&ui), 1);

        // Non-finite start: emits nothing
        draw_world_line(
            &mut ui,
            &viewport,
            Vec2::new(f32::NAN, 0.0),
            Vec2::new(10.0, 10.0),
            color,
            width,
        );
        assert_eq!(line_count(&ui), 1);

        // Non-finite end: emits nothing
        draw_world_line(
            &mut ui,
            &viewport,
            Vec2::new(0.0, 0.0),
            Vec2::new(f32::INFINITY, 10.0),
            color,
            width,
        );
        assert_eq!(line_count(&ui), 1);
    }
}
