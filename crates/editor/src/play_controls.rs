//! Play / Pause / Stop controls for the editor toolbar.
//!
//! Renders context-sensitive buttons next to the tool toolbar and returns
//! the action the user clicked, if any.

use glam::Vec2;
use ui::{Rect, UIContext};

use crate::icons::Icon;
use crate::play_state::EditorPlayState;
use crate::theme::EditorTheme;
use crate::toolbar::{draw_segment, segment_button, tooltip_text, ButtonLook, SEGMENT_DIVIDER_GAP, TOOL_BUTTON_SIZE};
use crate::toolbar_strip::{draw_separator_before, GROUP_GAP};

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

/// Play control widget rendered in the scene view's toolbar strip: the
/// transport as one segment of icon buttons.
///
/// The strip places it — at its centre where there is room, from the right
/// edge where there is not — and it reports the width it needs so the strip
/// can lay the tool group out around it.
#[derive(Debug, Clone)]
pub struct PlayControls {
    /// Position of the first button (set each frame by the strip layout).
    pub position: Vec2,
}

impl Default for PlayControls {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayControls {
    /// Create new play controls; the strip sets the position each frame.
    pub fn new() -> Self {
        Self { position: Vec2::ZERO }
    }

    /// Width left of [`position`](Self::position) that belongs to the group:
    /// the half of the strip's gap its separator line stands in. Part of the
    /// group's footprint, so the strip keeps it clear.
    pub const LEAD_WIDTH: f32 = GROUP_GAP / 2.0;

    /// Content width of the widest states (`Playing` and `Paused`: three
    /// buttons) — the width the strip's minimum is built from.
    ///
    /// Held in step with [`content_width`](Self::content_width) by
    /// `test_the_widest_state_is_the_constant_the_strip_budgets_for`.
    pub const WIDEST_CONTENT_WIDTH: f32 = 3.0 * TOOL_BUTTON_SIZE + 2.0 * SEGMENT_DIVIDER_GAP;

    /// How many buttons `state` shows: Play alone while editing; the
    /// session's Pause or Resume, Stop and Follow during one.
    fn button_count(state: EditorPlayState) -> usize {
        match state {
            EditorPlayState::Editing => 1,
            EditorPlayState::Playing | EditorPlayState::Paused => 3,
        }
    }

    /// Bounds of the `index`th button from [`position`](Self::position).
    /// Shared by `render` and `chrome_bounds` so the consume-only chrome
    /// rect can never drift from the real layout.
    fn button_bounds(&self, index: usize) -> Rect {
        Rect::new(
            self.position.x + index as f32 * (TOOL_BUTTON_SIZE + SEGMENT_DIVIDER_GAP),
            self.position.y,
            TOOL_BUTTON_SIZE,
            TOOL_BUTTON_SIZE,
        )
    }

    /// Width from [`position`](Self::position) to the right edge of the last
    /// button in `state` — what the strip must leave clear to its right.
    pub fn content_width(&self, state: EditorPlayState) -> f32 {
        self.button_bounds(Self::button_count(state) - 1).right() - self.position.x
    }

    /// Full chrome footprint for the given state: from the separator line to
    /// the rightmost button. Everything inside consumes mouse gestures so
    /// clicks on control chrome never fall through to viewport picking.
    pub fn chrome_bounds(&self, state: EditorPlayState) -> Rect {
        let right = self.position.x + self.content_width(state);
        let left = self.position.x - Self::LEAD_WIDTH;
        Rect::new(left, self.position.y, right - left, TOOL_BUTTON_SIZE)
    }

    /// Render play controls and return the clicked action, if any.
    ///
    /// Button layout varies by state:
    /// - **Editing:** `[Play]`
    /// - **Playing:** `[Pause] [Stop] [Follow]`
    /// - **Paused:**  `[Resume] [Stop] [Follow]`
    ///
    /// Play and Resume wear `play_button_bg`, Stop `stop_button_bg`;
    /// `camera_follow` puts the Follow toggle on — filled, ringed, and its
    /// tooltip saying so.
    pub fn render(
        &self,
        ui: &mut UIContext,
        state: EditorPlayState,
        camera_follow: bool,
        theme: &EditorTheme,
    ) -> Option<PlayControlAction> {
        draw_separator_before(ui, theme, self.position);
        let buttons: Vec<Rect> = (0..Self::button_count(state)).map(|index| self.button_bounds(index)).collect();
        draw_segment(ui, theme, &buttons);

        let play_tint = ButtonLook::Tinted(theme.play_button_bg);
        let mut action = None;
        let mut button = |index: usize, id: &str, icon: Icon, look: ButtonLook, tooltip: String, clicked_action| {
            if segment_button(ui, theme, id, icon, self.button_bounds(index), look, &tooltip) {
                action = Some(clicked_action);
            }
        };

        match state {
            EditorPlayState::Editing => {
                let tooltip = tooltip_text("Play", Some("F5"), "Run the game.");
                button(0, "play_ctrl_play", Icon::Play, play_tint, tooltip, PlayControlAction::Play);
            }
            EditorPlayState::Playing => {
                let tooltip = tooltip_text("Pause", Some("Ctrl+P"), "Freeze the running game where it is.");
                button(0, "play_ctrl_pause", Icon::Pause, ButtonLook::Plain, tooltip, PlayControlAction::Pause);
            }
            EditorPlayState::Paused => {
                let tooltip = tooltip_text("Resume", Some("F5"), "Carry on from where the game paused.");
                button(0, "play_ctrl_resume", Icon::Resume, play_tint, tooltip, PlayControlAction::Play);
            }
        }

        if state != EditorPlayState::Editing {
            let tooltip = tooltip_text("Stop", Some("Ctrl+Shift+P"), "End the session and put the scene back.");
            let stop_tint = ButtonLook::Tinted(theme.stop_button_bg);
            let stop_id = if state == EditorPlayState::Playing { "play_ctrl_stop" } else { "play_ctrl_stop2" };
            button(1, stop_id, Icon::Stop, stop_tint, tooltip, PlayControlAction::Stop);

            let (look, hint) = if camera_follow {
                (ButtonLook::On, "On: the viewport follows the game camera. Click to move it freely.")
            } else {
                (ButtonLook::Plain, "Off: click to keep the viewport on the game camera.")
            };
            let tooltip = tooltip_text("Follow", Some("Ctrl+Shift+F"), hint);
            button(2, "play_ctrl_follow", Icon::Follow, look, tooltip, PlayControlAction::ToggleCameraFollow);
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
        assert_eq!(controls.content_width(EditorPlayState::Paused), widest, "a paused session is the widest");
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
        let separator = Vec2::new(ORIGIN.x - PlayControls::LEAD_WIDTH + 0.5, ORIGIN.y + TOOL_BUTTON_SIZE * 0.5);
        let action = press_at(&mut ui, &mut input, separator, |ui| {
            controls.render(ui, EditorPlayState::Editing, true, &theme)
        });
        assert_eq!(action, None);
        assert!(ui.wants_mouse(), "a press on play-control chrome must not fall through to picking");
        release(&mut ui, &mut input, |ui| controls.render(ui, EditorPlayState::Editing, true, &theme));

        // Editing: [Play] only — the viewport right of Play stays pickable.
        let editing = controls.chrome_bounds(EditorPlayState::Editing);
        let button_middle = ORIGIN.y + TOOL_BUTTON_SIZE * 0.5;
        assert!(editing.contains(separator), "the separator is chrome");
        assert!(editing.contains(Vec2::new(ORIGIN.x + 1.0, button_middle)), "the Play button is chrome");
        assert!(
            !editing.contains(Vec2::new(editing.right() + 1.0, button_middle)),
            "right of Play is the viewport"
        );

        // A play session's layout: [Resume or Pause] [Stop] [Follow].
        let paused = controls.chrome_bounds(EditorPlayState::Paused);
        let stop_center = controls.button_bounds(1).center();
        let follow_center = controls.button_bounds(2).center();
        assert!(paused.contains(stop_center), "Stop is chrome");
        assert!(paused.contains(follow_center), "the Follow toggle is chrome");
        assert!(!editing.contains(follow_center), "no Follow toggle outside a play session");

        // Clicking Follow while Playing returns the toggle action on release.
        let pressed = press_at(&mut ui, &mut input, follow_center, |ui| {
            controls.render(ui, EditorPlayState::Playing, true, &theme)
        });
        assert_eq!(pressed, None, "clicks fire on release");
        let clicked = release(&mut ui, &mut input, |ui| controls.render(ui, EditorPlayState::Playing, true, &theme));
        assert_eq!(clicked, Some(PlayControlAction::ToggleCameraFollow));
    }
}
