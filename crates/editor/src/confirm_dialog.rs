//! A modal confirm dialog on the [`ui::UiLayer::Modal`] band: an
//! input-blocking scrim, a centered panel, and up
//! to three buttons. Deliberately GENERIC — the unsaved-changes prompt is
//! the first consumer; scripting Stage 5's build prompts reuse it.

use glam::Vec2;
use ui::{Rect, UIContext, UiLayer};

use crate::layout::BUTTON_HEIGHT;
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

/// A modal of two or three buttons. Construct per frame (plain data) and call
/// [`render`](Self::render) while the decision is pending.
#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    /// The alternative's label; `None` draws no alternative button, so the dialog can
    /// never offer a choice that does not exist.
    pub alt_label: Option<String>,
    pub cancel_label: String,
}

const DIALOG_WIDTH: f32 = 420.0;
const DIALOG_HEIGHT: f32 = 132.0;
/// A modal has to read as a separate object over the whole window, so its
/// inner margin is wider than the [`crate::layout::PADDING`] panels use.
const PADDING: f32 = 14.0;

impl ConfirmDialog {
    /// The standard unsaved-changes prompt.
    pub fn unsaved_changes(action: &str) -> Self {
        Self {
            title: "Unsaved changes".to_string(),
            message: format!("Save the current scene before {action}?"),
            confirm_label: "Save".to_string(),
            alt_label: Some("Discard".to_string()),
            cancel_label: "Cancel".to_string(),
        }
    }

    /// The unsaved-changes prompt for the song, a document of its own: Save / Discard /
    /// Cancel.
    pub fn unsaved_song(action: &str) -> Self {
        Self {
            title: "Unsaved song".to_string(),
            message: format!("Save the current song before {action}?"),
            confirm_label: "Save".to_string(),
            alt_label: Some("Discard".to_string()),
            cancel_label: "Cancel".to_string(),
        }
    }

    /// The Play prompt over a dirty song. Play never offers to discard — a click on Play
    /// must not destroy work — so the alternative plays the saved song, and only when a
    /// saved file exists: a never-saved song is offered Save / Cancel.
    pub fn play_with_unsaved_song(saved_exists: bool) -> Self {
        let (message, alt_label) = if saved_exists {
            (
                "The song has unsaved edits. Save them before Play, or play the saved song?",
                Some("Play saved".to_string()),
            )
        } else {
            ("The song has never been saved. Save it before Play?", None)
        };
        Self {
            title: "Unsaved song".to_string(),
            message: message.to_string(),
            confirm_label: "Save".to_string(),
            alt_label,
            cancel_label: "Cancel".to_string(),
        }
    }

    /// The Stop prompt for edits recorded while the session was paused.
    pub fn keep_paused_edits(count: usize) -> Self {
        Self {
            title: "Edits made while paused".to_string(),
            message: format!("Keep the {count} edit(s) you made while paused?"),
            confirm_label: "Keep".to_string(),
            alt_label: Some("Discard".to_string()),
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
        let button_width = 88.0;
        let gap = 10.0;
        let cancel = Rect::new(panel.x + panel.width - PADDING - button_width, y, button_width, BUTTON_HEIGHT);
        let alt = Rect::new(cancel.x - gap - button_width, y, button_width, BUTTON_HEIGHT);
        let confirm = Rect::new(alt.x - gap - button_width, y, button_width, BUTTON_HEIGHT);
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
        if let Some(alt_label) = &self.alt_label {
            if ui.button("confirm_dialog_alt", alt_label, alt) {
                choice = Some(ConfirmChoice::Alt);
            }
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
    use crate::test_support::{click_through, frame, WINDOW};

    /// Click `point` on a fresh dialog and return the release frame's choice.
    fn click_at(point: Vec2) -> Option<ConfirmChoice> {
        let dialog = ConfirmDialog::unsaved_changes("opening another scene");
        let theme = EditorTheme::default();
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();
        let (_, choice) = click_through(&mut ui, &mut input, point, |ui| dialog.render(ui, WINDOW, &theme));
        choice
    }

    /// Each of the three buttons returns its own choice, on the release frame.
    #[test]
    fn test_each_button_returns_its_choice() {
        let (confirm, alt, cancel) = ConfirmDialog::button_rects(WINDOW);
        assert_eq!(click_at(confirm.center()), Some(ConfirmChoice::Confirm));
        assert_eq!(click_at(alt.center()), Some(ConfirmChoice::Alt));
        assert_eq!(click_at(cancel.center()), Some(ConfirmChoice::Cancel));
    }

    /// A never-saved song's Play prompt has no "Play saved": the alternative's place is
    /// not a button, and a saved song's prompt offers it.
    #[test]
    fn test_play_saved_is_offered_only_when_a_saved_song_exists() {
        let (_, alt, _) = ConfirmDialog::button_rects(WINDOW);
        let theme = EditorTheme::default();
        for (saved_exists, expected) in [(true, Some(ConfirmChoice::Alt)), (false, None)] {
            let dialog = ConfirmDialog::play_with_unsaved_song(saved_exists);
            assert_eq!(dialog.alt_label.is_some(), saved_exists);
            let mut ui = UIContext::new();
            let mut input = input::InputHandler::new();
            let (_, choice) = click_through(&mut ui, &mut input, alt.center(), |ui| dialog.render(ui, WINDOW, &theme));
            assert_eq!(choice, expected, "saved_exists {saved_exists}");
        }
    }

    /// The scrim is not a choice, and while the dialog shows the whole
    /// window is input-blocked so nothing underneath can be clicked.
    #[test]
    fn test_scrim_click_is_not_a_choice_and_blocks_input() {
        assert_eq!(click_at(Vec2::new(20.0, 20.0)), None, "a scrim click chooses nothing");

        let dialog = ConfirmDialog::unsaved_changes("creating a new scene");
        let theme = EditorTheme::default();
        let mut ui = UIContext::new();
        let input = input::InputHandler::new();
        let corners_blocked = frame(&mut ui, &input, |ui| {
            dialog.render(ui, WINDOW, &theme);
            [Vec2::new(5.0, WINDOW.y - 5.0), Vec2::new(WINDOW.x - 5.0, 5.0), ConfirmDialog::panel_rect(WINDOW).center()]
                .map(|point| ui.is_input_blocked_at(point))
        });
        assert_eq!(corners_blocked, [true, true, true], "the scrim swallows clicks anywhere in the window");
    }
}
