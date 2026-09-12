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
    /// Toggle whether the viewport follows the game camera.
    ToggleCameraFollow,
}

/// Play control widget rendered in the scene view's toolbar strip.
///
/// The strip places it — at its centre where there is room, from the right
/// edge where there is not — and it reports the width it needs so the strip
/// can lay the tool group out around it.
#[derive(Debug, Clone)]
pub struct PlayControls {
    /// Position of the first button (set each frame by the strip layout).
    pub position: Vec2,
    /// Width of a plain button; the wider labels add to it.
    pub button_size: f32,
    /// Height of every button — the compact strip button.
    pub height: f32,
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
            button_size: 48.0,
            height: 30.0,
            spacing: 4.0,
        }
    }

    /// Width the separator and the gap to the tool group occupy left of
    /// [`position`](Self::position). Part of the group's footprint, so the
    /// strip keeps it clear.
    pub const LEAD_WIDTH: f32 = 9.0;

    /// Content width of the widest state (`Paused`: Resume, Stop, Follow) at
    /// the default sizing — the width the strip's minimum is built from.
    ///
    /// Held in step with [`content_width`](Self::content_width) by
    /// `test_the_widest_state_is_the_constant_the_strip_budgets_for`.
    pub const WIDEST_CONTENT_WIDTH: f32 = 176.0;

    /// Width of the first button in the given state ("Resume" needs extra
    /// room for its longer label). Shared by `render` and `chrome_bounds` so
    /// the consume-only chrome rect can never drift from the real layout.
    fn first_button_width(&self, state: EditorPlayState) -> f32 {
        match state {
            EditorPlayState::Paused => self.button_size + 10.0,
            _ => self.button_size,
        }
    }

    /// Width from [`position`](Self::position) to the right edge of the last
    /// button in `state` — what the strip must leave clear to its right.
    pub fn content_width(&self, state: EditorPlayState) -> f32 {
        if Self::has_stop_button(state) {
            self.follow_x(state) + self.follow_width() - self.position.x
        } else {
            self.first_button_width(state)
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
        let right = self.position.x + self.content_width(state);
        let left = self.position.x - Self::LEAD_WIDTH; // covers the separator line
        Rect::new(left, self.position.y, right - left, self.height)
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
            Vec2::new(sep_x, y + self.height - 4.0),
            theme.separator,
            1.0,
        );

        match state {
            EditorPlayState::Editing => {
                let button = Rect::new(x, y, self.first_button_width(state), self.height);
                ui.rect_rounded(button, theme.play_button_bg, 4.0);
                if ui.button("play_ctrl_play", "Play", button) {
                    action = Some(PlayControlAction::Play);
                }
                ui.tooltip(button, "Run the game (F5)");
            }
            EditorPlayState::Playing => {
                let pause_btn = Rect::new(x, y, self.first_button_width(state), self.height);
                if ui.button("play_ctrl_pause", "Pause", pause_btn) {
                    action = Some(PlayControlAction::Pause);
                }
                ui.tooltip(pause_btn, "Freeze the running game where it is (Ctrl+P)");

                let stop_btn = Rect::new(self.stop_x(state), y, self.button_size, self.height);
                ui.rect_rounded(stop_btn, theme.stop_button_bg, 4.0);
                if ui.button("play_ctrl_stop", "Stop", stop_btn) {
                    action = Some(PlayControlAction::Stop);
                }
                ui.tooltip(stop_btn, "End the session and put the scene back (Ctrl+Shift+P)");
            }
            EditorPlayState::Paused => {
                let resume_btn = Rect::new(x, y, self.first_button_width(state), self.height);
                ui.rect_rounded(resume_btn, theme.play_button_bg, 4.0);
                if ui.button("play_ctrl_resume", "Resume", resume_btn) {
                    action = Some(PlayControlAction::Play);
                }
                ui.tooltip(resume_btn, "Carry on from where the game paused (F5)");

                let stop_btn = Rect::new(self.stop_x(state), y, self.button_size, self.height);
                ui.rect_rounded(stop_btn, theme.stop_button_bg, 4.0);
                if ui.button("play_ctrl_stop2", "Stop", stop_btn) {
                    action = Some(PlayControlAction::Stop);
                }
                ui.tooltip(stop_btn, "End the session and put the scene back (Ctrl+Shift+P)");
            }
        }

        // Camera-follow toggle, play sessions only: highlighted
        // while the viewport mirrors the game camera; plain while free.
        if Self::has_stop_button(state) {
            let follow_btn =
                Rect::new(self.follow_x(state), y, self.follow_width(), self.height);
            if camera_follow {
                ui.rect_rounded(follow_btn, theme.play_button_bg, 4.0);
            }
            if ui.button("play_ctrl_follow", "Follow", follow_btn) {
                action = Some(PlayControlAction::ToggleCameraFollow);
            }
            ui.tooltip(follow_btn, "Keep the viewport on the game camera (Ctrl+Shift+F)");
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
    use crate::test_support::{press_at, release};

    const ORIGIN: Vec2 = Vec2::new(300.0, 20.0);

    /// The strip budgets for the play controls with two constants rather
    /// than a live widget (its minimum width is a `const`). Both must stay
    /// true of the real layout, or the strip reserves the wrong room and the
    /// controls it exists to protect clip.
    #[test]
    fn test_the_widest_state_is_the_constant_the_strip_budgets_for() {
        let controls = controls();
        let widest = [EditorPlayState::Editing, EditorPlayState::Playing, EditorPlayState::Paused]
            .into_iter()
            .map(|state| controls.content_width(state))
            .fold(0.0_f32, f32::max);

        assert_eq!(widest, PlayControls::WIDEST_CONTENT_WIDTH);
        assert_eq!(controls.content_width(EditorPlayState::Paused), widest, "Paused is the widest");
        let chrome = controls.chrome_bounds(EditorPlayState::Paused);
        assert_eq!(
            ORIGIN.x - chrome.x,
            PlayControls::LEAD_WIDTH,
            "the separator's lead is what the strip keeps clear"
        );
    }

    fn controls() -> PlayControls {
        let mut controls = PlayControls::new();
        controls.position = ORIGIN;
        controls
    }

    /// Play-control chrome claims the mouse gesture: a press on the
    /// separator or a gap never falls through to viewport picking. The
    /// chrome spans the separator to the last button of the current state,
    /// and the Follow toggle exists only inside a play session, where a
    /// click on it (fired on the release frame) returns the toggle action.
    #[test]
    fn test_play_controls_chrome_press_claims_mouse_gesture() {
        let controls = controls();
        let theme = EditorTheme::default();
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();

        // Press on the separator line left of the buttons — chrome, no button.
        let separator = Vec2::new(ORIGIN.x - 8.0, ORIGIN.y + controls.height * 0.5);
        let action = press_at(&mut ui, &mut input, separator, |ui| {
            controls.render(ui, EditorPlayState::Editing, true, &theme)
        });
        assert_eq!(action, None);
        assert!(ui.wants_mouse(), "a press on play-control chrome must not fall through to picking");
        release(&mut ui, &mut input, |ui| controls.render(ui, EditorPlayState::Editing, true, &theme));

        // Editing: [Play] only — the viewport right of Play stays pickable.
        let editing = controls.chrome_bounds(EditorPlayState::Editing);
        let button_middle = ORIGIN.y + controls.height * 0.5;
        assert!(editing.contains(separator), "the separator is chrome");
        assert!(editing.contains(Vec2::new(ORIGIN.x + 1.0, button_middle)), "the Play button is chrome");
        assert!(
            !editing.contains(Vec2::new(editing.right() + 1.0, button_middle)),
            "right of Play is the viewport"
        );

        // Paused is the widest layout: [Resume +10] [Stop] [Follow].
        let paused = controls.chrome_bounds(EditorPlayState::Paused);
        let stop_center = Vec2::new(controls.stop_x(EditorPlayState::Paused) + 1.0, button_middle);
        let follow_center = Vec2::new(
            controls.follow_x(EditorPlayState::Paused) + controls.follow_width() * 0.5,
            button_middle,
        );
        assert!(paused.contains(stop_center), "Stop is chrome");
        assert!(paused.contains(follow_center), "the Follow toggle is chrome");
        assert!(!editing.contains(follow_center), "no Follow toggle outside a play session");

        // Clicking Follow while Playing returns the toggle action on release.
        let follow_center = Vec2::new(
            controls.follow_x(EditorPlayState::Playing) + controls.follow_width() * 0.5,
            ORIGIN.y + controls.height * 0.5,
        );
        let pressed = press_at(&mut ui, &mut input, follow_center, |ui| {
            controls.render(ui, EditorPlayState::Playing, true, &theme)
        });
        assert_eq!(pressed, None, "clicks fire on release");
        let clicked = release(&mut ui, &mut input, |ui| controls.render(ui, EditorPlayState::Playing, true, &theme));
        assert_eq!(clicked, Some(PlayControlAction::ToggleCameraFollow));
    }
}
