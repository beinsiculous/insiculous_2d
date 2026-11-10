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

struct DialogLayout {
    panel: Rect,
    message_lines: Vec<String>,
    confirm: Rect,
    alt: Rect,
    cancel: Rect,
}

impl ConfirmDialog {
    /// Confirm removal of the named asset family, naming the files it removes — a
    /// companion and a recovered copy go with their primary, and the person confirms that
    /// exact set — and the `.ron` and `.rhai` files that reference it, which keep the name.
    pub fn delete_asset(name: &str, files: &[String], references: &[String]) -> Self {
        let listed = files.iter().take(4).cloned().collect::<Vec<_>>().join(", ");
        let more = if files.len() > 4 { ", …" } else { "" };
        let mut message = format!("Delete {name} ({} files: {listed}{more})? This cannot be undone.", files.len());
        if !references.is_empty() {
            let shown = references.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            let more = if references.len() > 3 { ", …" } else { "" };
            message.push_str(&format!(" {} files name it and will keep the name: {shown}{more}.", references.len()));
        }
        Self {
            title: "Delete asset".into(),
            message,
            confirm_label: "Delete".into(), alt_label: None, cancel_label: "Cancel".into(),
        }
    }

    /// Warn about the old path's textual references before a file family moves.
    pub fn rename_despite_references(count: usize, names: &[String]) -> Self {
        let mut shown = names.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
        if count > names.len().min(3) { shown.push_str(", …"); }
        Self {
            title: "References to the old path".into(),
            message: format!("{count} files name the old path: {shown}. Scanned every .ron and .rhai under the asset root. References will keep the old path."),
            confirm_label: "Rename anyway".into(), alt_label: None, cancel_label: "Cancel".into(),
        }
    }

    /// Save a dirty open document before moving the file that is its Save target.
    pub fn save_and_rename(document: &str) -> Self {
        Self {
            title: "Save before renaming".into(),
            message: format!("{document} has unsaved edits. Save them before renaming?"),
            confirm_label: "Save and rename".into(), alt_label: None, cancel_label: "Cancel".into(),
        }
    }

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

    /// The prompt before a Save As replaces another song's file: Overwrite / Cancel, with
    /// no alternative — there is nothing else to do with that path. The message names the
    /// sample folder: the save prunes it to this song's samples, as it would its own.
    pub fn overwrite_song(path: &std::path::Path) -> Self {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        Self {
            title: "Replace song".to_string(),
            message: format!("{name} is another song. Replace its .mid, .mid.ron and sample folder with this one's?"),
            confirm_label: "Overwrite".to_string(),
            alt_label: None,
            cancel_label: "Cancel".to_string(),
        }
    }

    /// The prompt before a save replaces the song's sidecar that changed on disk since the
    /// song read it: Overwrite / Cancel, with no alternative. Overwrite discards what the
    /// other program wrote there, which the message says, since a button is too narrow to.
    pub fn map_changed_on_disk(sidecar: &std::path::Path) -> Self {
        let name = sidecar
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| sidecar.display().to_string());
        Self {
            title: "Song map changed on disk".to_string(),
            message: format!(
                "The map changed on disk since you loaded it: {name}. Overwrite it with this song's map, discarding the change?"
            ),
            confirm_label: "Overwrite".to_string(),
            alt_label: None,
            cancel_label: "Cancel".to_string(),
        }
    }

    /// The prompt when another program rewrote the open scene while it had unsaved edits:
    /// Reload / Keep mine, with no alternative. The message says what Reload costs, since a
    /// button is too narrow to.
    pub fn scene_changed_on_disk(path: &std::path::Path) -> Self {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        Self {
            title: "Scene changed on disk".to_string(),
            message: format!("{name} changed on disk. Reload it and discard your edits?"),
            confirm_label: "Reload".to_string(),
            alt_label: None,
            cancel_label: "Keep mine".to_string(),
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

    fn layout(&self, ui: &UIContext, window_size: Vec2, theme: &EditorTheme) -> DialogLayout {
        let mut panel = Self::panel_rect(window_size);
        panel.width = panel.width.min((window_size.x - 2.0 * PADDING).max(120.0));
        panel.x = (window_size.x - panel.width) / 2.0;
        let line_height = theme.fonts.body + 4.0;
        let mut message_lines = crate::prompt_dialog::wrap_error(&self.message, panel.width - 2.0 * PADDING,
            |text| ui.measure_text_styled(text, theme.fonts.body).x);
        // A message longer than the window would push the buttons under its edge; the lines
        // that fit are kept and the last of them says the rest was cut.
        let lines_that_fit = (((window_size.y - 2.0 * PADDING - DIALOG_HEIGHT) / line_height).max(0.0) as usize) + 1;
        if message_lines.len() > lines_that_fit {
            message_lines.truncate(lines_that_fit);
            if let Some(last) = message_lines.last_mut() { last.push('…'); }
        }
        panel.height += message_lines.len().saturating_sub(1) as f32 * line_height;
        panel.y = ((window_size.y - panel.height) / 2.0).max(0.0);
        let button_width = |label: &str| 88.0_f32.max(ui.measure_text(label).x + 20.0);
        // No alternative, no slot for one: a prompt without it keeps its confirm beside Cancel.
        // A row wider than the panel — a long label in a narrow window at 200 % — is shrunk to
        // fit, every button by the same factor, so each stays inside the panel and clickable.
        let mut widths = [button_width(&self.confirm_label), self.alt_label.as_deref().map_or(0.0, button_width), button_width(&self.cancel_label)];
        let gaps = 10.0 * if self.alt_label.is_some() { 2.0 } else { 1.0 };
        let available = panel.width - 2.0 * PADDING - gaps;
        let total: f32 = widths.iter().sum();
        if total > available && total > 0.0 {
            for width in &mut widths { *width = (*width * available / total).floor(); }
        }
        let [confirm_width, alt_width, cancel_width] = widths;
        let row_y = panel.bottom() - PADDING - BUTTON_HEIGHT;
        let cancel = Rect::new(panel.right() - PADDING - cancel_width, row_y, cancel_width, BUTTON_HEIGHT);
        // Without an alternative its empty rect sits in the one gap, so a press there is nothing.
        let (alt_x, confirm_right) = if self.alt_label.is_some() {
            (cancel.x - 10.0 - alt_width, cancel.x - 20.0 - alt_width)
        } else {
            (cancel.x - 5.0, cancel.x - 10.0)
        };
        let alt = Rect::new(alt_x, row_y, alt_width, BUTTON_HEIGHT);
        let confirm = Rect::new(confirm_right - confirm_width, row_y, confirm_width, BUTTON_HEIGHT);
        DialogLayout { panel, message_lines, confirm, alt, cancel }
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
        let DialogLayout { panel, message_lines, confirm, alt, cancel } = self.layout(ui, window_size, theme);
        let line_height = theme.fonts.body + 4.0;
        ui.panel_styled(panel, theme.surface_4, theme.popup_border, 1.0);

        ui.label_styled(
            &self.title,
            Vec2::new(panel.x + PADDING, panel.y + PADDING + theme.fonts.heading * 0.75),
            theme.text_primary,
            theme.fonts.heading,
        );
        for (index, line) in message_lines.iter().enumerate() {
            ui.label_styled(line,
                Vec2::new(panel.x + PADDING, panel.y + PADDING + 34.0 + theme.fonts.body * 0.75 + index as f32 * line_height),
                theme.text_primary, theme.fonts.body);
        }

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

    /// Each of the three buttons returns its own choice, on the release frame — measured
    /// from the layout the render draws, since a label's width and a message's lines move
    /// the buttons.
    #[test]
    fn test_each_button_returns_its_choice() {
        let dialog = ConfirmDialog::unsaved_changes("opening another scene");
        let layout = dialog.layout(&UIContext::new(), WINDOW, &EditorTheme::default());
        assert_eq!(click_at(layout.confirm.center()), Some(ConfirmChoice::Confirm));
        assert_eq!(click_at(layout.alt.center()), Some(ConfirmChoice::Alt));
        assert_eq!(click_at(layout.cancel.center()), Some(ConfirmChoice::Cancel));
    }

    /// The overwrite prompt offers Overwrite and Cancel and no third button: there is
    /// nothing else to do with another song's path. It names the sample folder, which the
    /// save prunes to this song's samples — the other song's audio goes with its files.
    #[test]
    fn test_the_overwrite_prompt_names_the_file_and_its_sample_folder_and_offers_no_alternative() {
        let dialog = ConfirmDialog::overwrite_song(std::path::Path::new("/project/music/theme.mid"));
        assert_eq!(dialog.confirm_label, "Overwrite");
        assert_eq!(dialog.alt_label, None);
        assert!(dialog.message.contains("theme.mid"), "{}", dialog.message);
        assert!(dialog.message.contains("sample folder"), "{}", dialog.message);
    }

    /// A never-saved song's Play prompt has no "Play saved": the alternative's place is
    /// not a button, and a saved song's prompt offers it.
    #[test]
    fn test_play_saved_is_offered_only_when_a_saved_song_exists() {
        let theme = EditorTheme::default();
        for (saved_exists, expected) in [(true, Some(ConfirmChoice::Alt)), (false, None)] {
            let dialog = ConfirmDialog::play_with_unsaved_song(saved_exists);
            assert_eq!(dialog.alt_label.is_some(), saved_exists);
            let mut ui = UIContext::new();
            let mut input = input::InputHandler::new();
            let alt = dialog.layout(&ui, WINDOW, &theme).alt;
            let (_, choice) = click_through(&mut ui, &mut input, alt.center(), |ui| dialog.render(ui, WINDOW, &theme));
            assert_eq!(choice, expected, "saved_exists {saved_exists}");
        }
    }

    #[test]
    fn a_narrow_window_at_scale_keeps_the_buttons_inside_the_panel_and_the_panel_inside_the_window() {
        let theme = EditorTheme::default();
        let ui = UIContext::new();
        let names: Vec<String> = (0..3).map(|index| format!("scenes/chapter_{index}/a_very_long_level_name_{index}.scene.ron")).collect();
        for (dialog, window) in [
            (ConfirmDialog::save_and_rename("scenes/chapters/a_very_long_level_name.scene.ron"), Vec2::new(320.0, 240.0)),
            (ConfirmDialog::rename_despite_references(3, &names), Vec2::new(320.0, 160.0)),
        ] {
            let layout = dialog.layout(&ui, window, &theme);
            assert!(layout.panel.y >= 0.0 && layout.panel.bottom() <= window.y, "{:?} in {window}", layout.panel);
            assert!(layout.confirm.x >= layout.panel.x + PADDING && layout.cancel.right() <= layout.panel.right() - PADDING, "{:?} / {:?}", layout.confirm, layout.cancel);
            assert!(layout.confirm.right() <= layout.cancel.x, "the confirm sits beside Cancel with no phantom slot between");
            assert!(layout.confirm.bottom() <= layout.panel.bottom());
        }
    }

    #[test]
    fn reference_and_save_confirmations_wrap_names_and_keep_the_full_action_clickable() {
        let names = vec!["scenes/nested/level_one.scene.ron".into(), "scripts/hero.rhai".into()];
        let theme = EditorTheme::default();
        for dialog in [ConfirmDialog::rename_despite_references(2, &names), ConfirmDialog::save_and_rename("scenes/level_one.scene.ron")] {
            let mut ui = UIContext::new();
            let mut input = input::InputHandler::new();
            let layout = dialog.layout(&ui, WINDOW, &theme);
            assert!(layout.message_lines.len() > 1);
            assert!(layout.message_lines.iter().all(|line| ui.measure_text_styled(line, theme.fonts.body).x <= layout.panel.width - 2.0 * PADDING));
            assert!(layout.confirm.width > ui.measure_text(&dialog.confirm_label).x);
            assert!(layout.confirm.x >= layout.panel.x + PADDING);
            let (_, choice) = click_through(&mut ui, &mut input, layout.confirm.center(), |ui| dialog.render(ui, WINDOW, &theme));
            assert_eq!(choice, Some(ConfirmChoice::Confirm));
            let text = crate::test_support::drawn_text(&ui);
            assert!(text.contains(&dialog.confirm_label));
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
