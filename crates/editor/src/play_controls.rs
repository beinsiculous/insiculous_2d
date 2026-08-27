//! Play / Pause / Stop controls for the editor toolbar.
//!
//! Renders context-sensitive buttons next to the tool toolbar and returns
//! the action the user clicked, if any.

use glam::Vec2;
use ui::{Rect, UIContext};

use crate::play_state::EditorPlayState;
use crate::theme::EditorTheme;

/// Action returned when the user clicks a play control button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayControlAction {
    /// Start or resume the game simulation.
    Play,
    /// Pause the running game simulation.
    Pause,
    /// Stop the game and restore the pre-play snapshot.
    Stop,
}

/// Play control widget rendered to the right of the tool toolbar.
#[derive(Debug, Clone)]
pub struct PlayControls {
    /// Position (set each frame based on toolbar bounds).
    pub position: Vec2,
    /// Button size (matches toolbar button size).
    pub button_size: f32,
    /// Spacing between buttons.
    pub spacing: f32,
}

impl Default for PlayControls {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayControls {
    /// Create new play controls with default sizing.
    pub fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            button_size: 40.0,
            spacing: 4.0,
        }
    }

    /// Width of the first button in the given state ("Resume" needs extra
    /// room for its longer label). Shared by `render` and `chrome_bounds` so
    /// the consume-only chrome rect can never drift from the real layout.
    fn first_button_width(&self, state: EditorPlayState) -> f32 {
        match state {
            EditorPlayState::Paused => self.button_size + 10.0,
            _ => self.button_size,
        }
    }

    /// Whether the given state shows a second (Stop) button.
    fn has_stop_button(state: EditorPlayState) -> bool {
        !matches!(state, EditorPlayState::Editing)
    }

    /// X of the second (Stop) button for states that show one.
    fn stop_x(&self, state: EditorPlayState) -> f32 {
        self.position.x + self.first_button_width(state) + self.spacing
    }

    /// Full chrome footprint for the given state: from the separator line to
    /// the rightmost button. Everything inside consumes mouse gestures so
    /// clicks on control chrome never fall through to viewport picking.
    pub fn chrome_bounds(&self, state: EditorPlayState) -> Rect {
        let right = if Self::has_stop_button(state) {
            self.stop_x(state) + self.button_size
        } else {
            self.position.x + self.first_button_width(state)
        };
        let left = self.position.x - self.spacing * 2.0 - 1.0; // covers the separator line
        Rect::new(left, self.position.y, right - left, self.button_size)
    }

    /// Render play controls and return the clicked action, if any.
    ///
    /// Button layout varies by state:
    /// - **Editing:** `[Play]`
    /// - **Playing:** `[Pause] [Stop]`
    /// - **Paused:**  `[Resume] [Stop]`
    pub fn render(
        &self,
        ui: &mut UIContext,
        state: EditorPlayState,
        theme: &EditorTheme,
    ) -> Option<PlayControlAction> {
        let mut action = None;
        let x = self.position.x;
        let y = self.position.y;

        // Visual separator line between toolbar and play controls
        let sep_x = x - self.spacing * 2.0;
        ui.line(
            Vec2::new(sep_x, y + 4.0),
            Vec2::new(sep_x, y + self.button_size - 4.0),
            theme.separator,
            1.0,
        );

        match state {
            EditorPlayState::Editing => {
                let btn = Rect::new(x, y, self.first_button_width(state), self.button_size);
                ui.rect_rounded(btn, theme.play_button_bg, 4.0);
                if ui.button("play_ctrl_play", "Play", btn) {
                    action = Some(PlayControlAction::Play);
                }
            }
            EditorPlayState::Playing => {
                let pause_btn = Rect::new(x, y, self.first_button_width(state), self.button_size);
                if ui.button("play_ctrl_pause", "Pause", pause_btn) {
                    action = Some(PlayControlAction::Pause);
                }

                let stop_btn = Rect::new(self.stop_x(state), y, self.button_size, self.button_size);
                ui.rect_rounded(stop_btn, theme.stop_button_bg, 4.0);
                if ui.button("play_ctrl_stop", "Stop", stop_btn) {
                    action = Some(PlayControlAction::Stop);
                }
            }
            EditorPlayState::Paused => {
                let resume_btn = Rect::new(x, y, self.first_button_width(state), self.button_size);
                ui.rect_rounded(resume_btn, theme.play_button_bg, 4.0);
                if ui.button("play_ctrl_resume", "Resume", resume_btn) {
                    action = Some(PlayControlAction::Play);
                }

                let stop_btn = Rect::new(self.stop_x(state), y, self.button_size, self.button_size);
                ui.rect_rounded(stop_btn, theme.stop_button_bg, 4.0);
                if ui.button("play_ctrl_stop2", "Stop", stop_btn) {
                    action = Some(PlayControlAction::Stop);
                }
            }
        }

        // Consume-only: presses on the separator/gaps between buttons claim
        // the mouse gesture too. Registered AFTER the buttons so they win
        // the active-widget slot.
        ui.interact("play_ctrl_chrome", self.chrome_bounds(state), true);

        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_play_controls_default() {
        let controls = PlayControls::new();
        assert_eq!(controls.button_size, 40.0);
        assert_eq!(controls.spacing, 4.0);
    }

    #[test]
    fn test_play_controls_chrome_press_claims_mouse_gesture() {
        use input::prelude::MouseButton;
        let mut controls = PlayControls::new();
        controls.position = Vec2::new(300.0, 20.0);
        let theme = EditorTheme::default();
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();

        // Press on the separator line left of the buttons — chrome, no button
        input.mouse_mut().update_position(292.0, 40.0);
        input.mouse_mut().handle_button_press(MouseButton::Left);
        ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
        let action = controls.render(&mut ui, EditorPlayState::Editing, &theme);
        assert!(action.is_none());
        assert!(
            ui.wants_mouse(),
            "a press on play-control chrome must not fall through to viewport picking"
        );
        ui.end_frame();
    }

    #[test]
    fn test_play_controls_chrome_bounds_span_separator_to_last_button() {
        let mut controls = PlayControls::new();
        controls.position = Vec2::new(300.0, 20.0);

        let editing = controls.chrome_bounds(EditorPlayState::Editing);
        assert!(editing.contains(Vec2::new(292.0, 40.0)), "separator is chrome");
        assert!(editing.contains(Vec2::new(339.0, 40.0)), "Play button is chrome");
        assert!(!editing.contains(Vec2::new(360.0, 40.0)), "viewport right of Play stays pickable");

        // Paused is the widest layout: Resume (+10) + spacing + Stop
        let paused = controls.chrome_bounds(EditorPlayState::Paused);
        assert!(paused.contains(Vec2::new(300.0 + 40.0 + 10.0 + 4.0 + 39.0, 40.0)), "Stop is chrome");
    }

    #[test]
    fn test_play_control_action_eq() {
        assert_eq!(PlayControlAction::Play, PlayControlAction::Play);
        assert_ne!(PlayControlAction::Play, PlayControlAction::Pause);
        assert_ne!(PlayControlAction::Pause, PlayControlAction::Stop);
    }
}
