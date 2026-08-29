//! A modal confirm dialog on the [`ui::UiLayer::Modal`] band (issue #52,
//! audit §1.4/§6.7(7)): an input-blocking scrim, a centered panel, and up
//! to three buttons. Deliberately GENERIC — the unsaved-changes prompt is
//! the first consumer; scripting Stage 5's build prompts reuse it.

use glam::Vec2;
use ui::{Rect, UIContext, UiLayer};

use crate::theme::EditorTheme;

/// What the user chose this frame, if anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmChoice {
    /// The primary action (e.g. "Save").
    Confirm,
    /// The alternative action (e.g. "Discard").
    Alt,
    /// Dismiss without acting (e.g. "Cancel"). Escape maps here too —
    /// the HOST handles the key (its shortcut dispatch owns Escape).
    Cancel,
}

/// A three-button modal. Construct per frame (plain data) and call
/// [`render`](Self::render) while the decision is pending.
#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub alt_label: String,
    pub cancel_label: String,
}

const DIALOG_WIDTH: f32 = 420.0;
const DIALOG_HEIGHT: f32 = 132.0;
const BUTTON_HEIGHT: f32 = 26.0;
const PADDING: f32 = 14.0;

impl ConfirmDialog {
    /// The standard unsaved-changes prompt.
    pub fn unsaved_changes(action: &str) -> Self {
        Self {
            title: "Unsaved changes".to_string(),
            message: format!("Save the current scene before {action}?"),
            confirm_label: "Save".to_string(),
            alt_label: "Discard".to_string(),
            cancel_label: "Cancel".to_string(),
        }
    }

    /// The centered panel rect for a window of `window_size`.
    pub fn panel_rect(window_size: Vec2) -> Rect {
        Rect::new(
            (window_size.x - DIALOG_WIDTH) * 0.5,
            (window_size.y - DIALOG_HEIGHT) * 0.5,
            DIALOG_WIDTH,
            DIALOG_HEIGHT,
        )
    }

    /// Button rects `(confirm, alt, cancel)`, right-aligned in the panel —
    /// pure geometry, shared by render and the hit tests.
    pub fn button_rects(window_size: Vec2) -> (Rect, Rect, Rect) {
        let panel = Self::panel_rect(window_size);
        let y = panel.y + panel.height - PADDING - BUTTON_HEIGHT;
        let w = 88.0;
        let gap = 10.0;
        let cancel = Rect::new(panel.x + panel.width - PADDING - w, y, w, BUTTON_HEIGHT);
        let alt = Rect::new(cancel.x - gap - w, y, w, BUTTON_HEIGHT);
        let confirm = Rect::new(alt.x - gap - w, y, w, BUTTON_HEIGHT);
        (confirm, alt, cancel)
    }

    /// Draw the scrim + panel + buttons and return this frame's choice.
    ///
    /// The FULL-WINDOW blocking rect is the input scrim: widgets under it
    /// go inert for the whole frame, which is why the host renders the
    /// dialog EARLY in the frame (the drag-ghost pattern — a blocking rect
    /// only protects widgets registered after it).
    pub fn render(
        &self,
        ui: &mut UIContext,
        window_size: Vec2,
        theme: &EditorTheme,
    ) -> Option<ConfirmChoice> {
        let full_window = Rect::new(0.0, 0.0, window_size.x, window_size.y);
        ui.begin_overlay_in(UiLayer::Modal, full_window);

        // Dim scrim, then the panel (surface_4 = the floating-surface tone,
        // popup_border keeps the ≥3:1 WCAG contract).
        ui.rect(full_window, ui::Color::new(0.0, 0.0, 0.0, 0.45));
        let panel = Self::panel_rect(window_size);
        ui.panel_styled(panel, theme.surface_4, theme.popup_border, 1.0);

        ui.label_styled(
            &self.title,
            Vec2::new(panel.x + PADDING, panel.y + PADDING + theme.fonts.heading * 0.75),
            theme.text_primary,
            theme.fonts.heading,
        );
        ui.label_styled(
            &self.message,
            Vec2::new(panel.x + PADDING, panel.y + PADDING + 34.0 + theme.fonts.body * 0.75),
            theme.text_primary,
            theme.fonts.body,
        );

        let (confirm, alt, cancel) = Self::button_rects(window_size);
        let mut choice = None;
        if ui.button("confirm_dialog_confirm", &self.confirm_label, confirm) {
            choice = Some(ConfirmChoice::Confirm);
        }
        if ui.button("confirm_dialog_alt", &self.alt_label, alt) {
            choice = Some(ConfirmChoice::Alt);
        }
        if ui.button("confirm_dialog_cancel", &self.cancel_label, cancel) {
            choice = Some(ConfirmChoice::Cancel);
        }

        ui.end_overlay();
        choice
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use input::prelude::MouseButton;

    const WINDOW: Vec2 = Vec2::new(1280.0, 720.0);

    fn click_at(point: Vec2) -> Option<ConfirmChoice> {
        let dialog = ConfirmDialog::unsaved_changes("opening another scene");
        let theme = EditorTheme::default();
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();

        // Press frame.
        input.mouse_mut().update_position(point.x, point.y);
        input.mouse_mut().handle_button_press(MouseButton::Left);
        ui.begin_frame(&input, WINDOW);
        dialog.render(&mut ui, WINDOW, &theme);
        ui.end_frame();
        // Release frame — clicks fire here.
        input.mouse_mut().handle_button_release(MouseButton::Left);
        ui.begin_frame(&input, WINDOW);
        let choice = dialog.render(&mut ui, WINDOW, &theme);
        ui.end_frame();
        choice
    }

    fn center(r: Rect) -> Vec2 {
        Vec2::new(r.x + r.width * 0.5, r.y + r.height * 0.5)
    }

    #[test]
    fn test_each_button_returns_its_choice() {
        let (confirm, alt, cancel) = ConfirmDialog::button_rects(WINDOW);
        assert_eq!(click_at(center(confirm)), Some(ConfirmChoice::Confirm));
        assert_eq!(click_at(center(alt)), Some(ConfirmChoice::Alt));
        assert_eq!(click_at(center(cancel)), Some(ConfirmChoice::Cancel));
    }

    #[test]
    fn test_scrim_click_is_not_a_choice_and_blocks_input() {
        // A click on the scrim (outside the panel) chooses nothing…
        assert_eq!(click_at(Vec2::new(20.0, 20.0)), None);

        // …and the full window is input-blocked while the dialog shows.
        let dialog = ConfirmDialog::unsaved_changes("creating a new scene");
        let theme = EditorTheme::default();
        let mut ui = UIContext::new();
        let input = input::InputHandler::new();
        ui.begin_frame(&input, WINDOW);
        dialog.render(&mut ui, WINDOW, &theme);
        assert!(
            ui.is_input_blocked_at(Vec2::new(5.0, 700.0)),
            "the scrim swallows clicks anywhere in the window"
        );
        ui.end_frame();
    }
}
