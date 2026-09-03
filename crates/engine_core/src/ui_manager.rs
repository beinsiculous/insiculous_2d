//! UI Manager - extracts UI responsibilities from GameRunner
//!
//! Handles UI frame lifecycle, draw command collection, and event coordination.

use ui::{UIContext, DrawCommand};
use input::InputHandler;

/// Manages UI lifecycle and draw command collection
pub struct UIManager {
    ui_context: UIContext,
}

impl UIManager {
    /// Create a new UI manager
    pub fn new() -> Self {
        Self {
            ui_context: UIContext::new(),
        }
    }

    /// Begin a new UI frame. `dt` (seconds since the last frame) paces
    /// held-key repeat in text inputs.
    pub fn begin_frame(&mut self, input: &InputHandler, window_size: glam::Vec2, dt: f32) {
        self.ui_context.begin_frame_dt(input, window_size, dt);
    }

    /// Get mutable access to the UI context
    pub fn ui_context(&mut self) -> &mut UIContext {
        &mut self.ui_context
    }

    /// End the UI frame and collect draw commands
    pub fn end_frame(&mut self) -> Vec<DrawCommand> {
        self.ui_context.end_frame();
        self.ui_context.draw_list().commands().to_vec()
    }
}

impl Default for UIManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_frame_returns_the_commands_drawn_during_the_frame() {
        let input = InputHandler::new();
        let mut manager = UIManager::new();

        manager.begin_frame(&input, glam::Vec2::new(800.0, 600.0), 1.0 / 60.0);
        manager.ui_context().label("Test", glam::Vec2::new(10.0, 10.0));
        let commands = manager.end_frame();

        // No font is loaded, so the label lands as a placeholder — the
        // draw list the renderer receives must still carry it, where it was put.
        let label = commands.iter().find_map(|command| match command {
            DrawCommand::TextPlaceholder { text, position, .. } => Some((text.as_str(), *position)),
            DrawCommand::Text { data, .. } => Some((data.text.as_str(), data.position)),
            _ => None,
        });
        assert_eq!(label, Some(("Test", glam::Vec2::new(10.0, 10.0))));
    }
}
