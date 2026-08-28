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
    /// Toggle whether the viewport follows the game camera (issue #42).
    ToggleCameraFollow,
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

    /// X of the camera-follow toggle (play sessions only, after Stop).
    fn follow_x(&self, state: EditorPlayState) -> f32 {
        self.stop_x(state) + self.button_size + self.spacing
    }

    /// Width of the camera-follow toggle ("Follow" needs the wide label).
    fn follow_width(&self) -> f32 {
        self.button_size + 14.0
    }

    /// Full chrome footprint for the given state: from the separator line to
    /// the rightmost button. Everything inside consumes mouse gestures so
    /// clicks on control chrome never fall through to viewport picking.
    pub fn chrome_bounds(&self, state: EditorPlayState) -> Rect {
        let right = if Self::has_stop_button(state) {
            self.follow_x(state) + self.follow_width()
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
    /// - **Playing:** `[Pause] [Stop] [Follow]`
    /// - **Paused:**  `[Resume] [Stop] [Follow]`
    ///
    /// `camera_follow` renders the Follow toggle highlighted (accent
    /// background) when the viewport is mirroring the game camera.
    pub fn render(
        &self,
        ui: &mut UIContext,
        state: EditorPlayState,
        camera_follow: bool,
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

        // Camera-follow toggle, play sessions only (issue #42): highlighted
        // while the viewport mirrors the game camera; plain while free.
        if Self::has_stop_button(state) {
            let follow_btn =
                Rect::new(self.follow_x(state), y, self.follow_width(), self.button_size);
            if camera_follow {
                ui.rect_rounded(follow_btn, theme.play_button_bg, 4.0);
            }
            if ui.button("play_ctrl_follow", "Follow", follow_btn) {
                action = Some(PlayControlAction::ToggleCameraFollow);
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
        let action = controls.render(&mut ui, EditorPlayState::Editing, true, &theme);
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

        // Paused is the widest layout: Resume (+10) + spacing + Stop + Follow
        let paused = controls.chrome_bounds(EditorPlayState::Paused);
        assert!(paused.contains(Vec2::new(300.0 + 40.0 + 10.0 + 4.0 + 39.0, 40.0)), "Stop is chrome");
        // Follow toggle sits after Stop (issue #42) and is chrome too.
        assert!(
            paused.contains(Vec2::new(300.0 + 50.0 + 4.0 + 40.0 + 4.0 + 50.0, 40.0)),
            "Follow toggle is chrome"
        );
    }

    #[test]
    fn test_follow_toggle_shows_only_during_play_session() {
        let mut controls = PlayControls::new();
        controls.position = Vec2::new(300.0, 20.0);
        let theme = EditorTheme::default();
        let mut input = input::InputHandler::new();

        // Editing: chrome ends at the Play button — no Follow toggle.
        let editing = controls.chrome_bounds(EditorPlayState::Editing);
        let playing = controls.chrome_bounds(EditorPlayState::Playing);
        assert!(playing.width > editing.width + controls.follow_width() - 1.0);

        // Clicking the Follow button while Playing returns the toggle action.
        let follow_center = Vec2::new(
            controls.follow_x(EditorPlayState::Playing) + controls.follow_width() * 0.5,
            20.0 + controls.button_size * 0.5,
        );
        use input::prelude::MouseButton;
        let mut ui = UIContext::new();
        // Frame 1: press.
        input.mouse_mut().update_position(follow_center.x, follow_center.y);
        input.mouse_mut().handle_button_press(MouseButton::Left);
        ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
        controls.render(&mut ui, EditorPlayState::Playing, true, &theme);
        ui.end_frame();
        // Frame 2: release → click fires.
        input.mouse_mut().handle_button_release(MouseButton::Left);
        ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
        let action = controls.render(&mut ui, EditorPlayState::Playing, true, &theme);
        ui.end_frame();
        assert_eq!(action, Some(PlayControlAction::ToggleCameraFollow));
    }

    #[test]
    fn test_play_control_action_eq() {
        assert_eq!(PlayControlAction::Play, PlayControlAction::Play);
        assert_ne!(PlayControlAction::Play, PlayControlAction::Pause);
        assert_ne!(PlayControlAction::Pause, PlayControlAction::Stop);
    }
}
