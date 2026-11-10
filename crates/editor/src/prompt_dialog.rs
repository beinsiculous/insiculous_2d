//! A name prompt on the modal band, with errors kept beside the editable name.

use glam::Vec2;
use ui::{Rect, UIContext, UiLayer};
use winit::keyboard::KeyCode;
use crate::{EditorTheme, EditResult, FieldId, RowLayout};

// Above any inspector component's stride, so the prompt's field never shares a widget id
// with a component row drawn in the same frame.
const NAME_FIELD: FieldId = FieldId { component_index: 90_000, field_index: 0, subfield_index: 0 };
/// The confirm dialog's dim, so the two modal bands read as one kind of thing.
const SCRIM: ui::Color = ui::Color::new(0.0, 0.0, 0.0, 0.45);
const PADDING: f32 = 14.0;

/// A confirmation is a proposal: the host closes only after its write succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptOutcome { Confirmed(String), Cancelled, Open }

/// A single name field and a named action/Cancel, owning the whole window while open.
#[derive(Debug, Default)]
pub struct PromptDialog {
    title: String,
    confirm_label: String,
    name: String,
    error: Option<String>,
    open: bool,
    focus_name: bool,
    confirm: bool,
    cancel: bool,
}

impl PromptDialog {
    /// Open a fresh prompt, replacing the previous draft and error.
    pub fn open(&mut self, title: &str, initial: &str, confirm_label: &str) {
        *self = Self { title: title.into(), confirm_label: confirm_label.into(), name: initial.into(), open: true, focus_name: true, ..Self::default() };
    }
    /// Whether the prompt owns the frame and the keyboard.
    pub fn is_open(&self) -> bool { self.open }
    /// Close after a successful write or a play transition.
    pub fn close(&mut self) { self.open = false; self.confirm = false; self.cancel = false; }
    /// Keep the current draft open and show a refusal under the field.
    pub fn show_error(&mut self, text: impl Into<String>) { self.error = Some(text.into()); self.open = true; self.focus_name = true; }
    /// The visible refusal, for the host's tests and status reporting.
    pub fn error(&self) -> Option<&str> { self.error.as_deref() }
    /// Claim every key while open; the UI field receives its ordinary text input.
    pub fn consumes_key(&mut self, key: KeyCode) -> bool {
        if !self.open { return false; }
        match key {
            KeyCode::Enter | KeyCode::NumpadEnter => self.confirm = true,
            KeyCode::Escape => self.cancel = true,
            _ => {},
        }
        true
    }
    /// Draw before other widgets, using the UI's points window for centring and scrim.
    pub fn render(&mut self, ui: &mut UIContext, theme: &EditorTheme) -> PromptOutcome {
        if !self.open { return PromptOutcome::Open; }
        let window = ui.window_size();
        let width = 460.0_f32.min((window.x - PADDING * 2.0).max(120.0));
        let error_lines = self.error.as_ref().map(|error| wrap_error(&format!("Error: {error}"), width - PADDING * 2.0, |text| ui.measure_text_styled(text, theme.fonts.small).x)).unwrap_or_default();
        let height = 130.0 + error_lines.len() as f32 * 18.0;
        let panel = Rect::new((window.x - width) / 2.0, (window.y - height) / 2.0, width, height);
        ui.begin_overlay_in(UiLayer::Modal, Rect::new(0.0, 0.0, window.x, window.y));
        ui.rect(Rect::new(0.0, 0.0, window.x, window.y), SCRIM);
        ui.panel_styled(panel, theme.surface_4, theme.popup_border, 1.0);
        ui.label_styled(&self.title, Vec2::new(panel.x + PADDING, panel.y + 25.0), theme.text_primary, theme.fonts.heading);
        if std::mem::take(&mut self.focus_name) { ui.focus_text_input(NAME_FIELD, &self.name); }
        let layout = RowLayout { pos: Vec2::new(panel.x + PADDING, panel.y + 40.0), control_x: panel.x + 65.0, right: panel.right() - PADDING };
        let mut style = theme.editable_field_style();
        style.input_width = width;
        if let EditResult::Changed(name) = crate::text_field::edit_string(ui, NAME_FIELD, "Name", &self.name, layout, &style) { self.name = name; }
        for (index, line) in error_lines.iter().enumerate() {
            ui.label_styled(line, Vec2::new(panel.x + PADDING, panel.y + 82.0 + index as f32 * 18.0), theme.status_error, theme.fonts.small);
        }
        let cancel_bounds = Rect::new(panel.right() - PADDING - 88.0, panel.bottom() - PADDING - 24.0, 88.0, 24.0);
        let confirm_bounds = Rect::new(cancel_bounds.x - 98.0, cancel_bounds.y, 88.0, 24.0);
        let confirmed = std::mem::take(&mut self.confirm) | ui.button("asset_prompt_create", &self.confirm_label, confirm_bounds);
        let cancelled = std::mem::take(&mut self.cancel) | ui.button("asset_prompt_cancel", "Cancel", cancel_bounds);
        ui.end_overlay();
        if cancelled { self.close(); ui.clear_text_focus(); PromptOutcome::Cancelled }
        else if confirmed { PromptOutcome::Confirmed(self.name.clone()) }
        else { PromptOutcome::Open }
    }
}

/// Wrap at spaces to the width, so an error breaks between words; a word wider than the
/// whole line (an overlong file name) is split at characters, since a line that cannot wrap
/// would run out of the dialog and hide the very name to correct.
pub(crate) fn wrap_error(text: &str, width: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if !line.is_empty() && measure(&candidate) > width {
            lines.push(std::mem::take(&mut line));
        }
        if line.is_empty() {
            for character in word.chars() {
                if !line.is_empty() && measure(&format!("{line}{character}")) > width {
                    lines.push(std::mem::take(&mut line));
                }
                line.push(character);
            }
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() { lines.push(line); }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{frame, drawn_text, WINDOW};

    #[test]
    fn a_prompt_confirms_the_edited_name_keeps_errors_open_and_escape_cancels() {
        let mut prompt = PromptDialog::default();
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();
        let theme = EditorTheme::default();
        prompt.open("New scene", "initial", "Create");
        frame(&mut ui, &input, |ui| prompt.render(ui, &theme));
        ui.focus_text_input(NAME_FIELD, "level_two");
        input.keyboard_mut().handle_key_press(KeyCode::Enter);
        assert!(prompt.consumes_key(KeyCode::Enter));
        let outcome = frame(&mut ui, &input, |ui| prompt.render(ui, &theme));
        assert_eq!(outcome, PromptOutcome::Confirmed("level_two".into()));
        prompt.show_error("that name already exists");
        input.update();
        input.keyboard_mut().handle_key_release(KeyCode::Enter);
        frame(&mut ui, &input, |ui| {
            assert_eq!(prompt.render(ui, &theme), PromptOutcome::Open);
            assert!(ui.is_input_blocked_at(Vec2::new(1.0, WINDOW.y - 1.0)));
        });
        assert!(prompt.is_open());
        assert!(drawn_text(&ui).contains("Error: that name already exists"));
        assert!(prompt.consumes_key(KeyCode::Escape));
        assert_eq!(frame(&mut ui, &input, |ui| prompt.render(ui, &theme)), PromptOutcome::Cancelled);
        assert!(!prompt.is_open());
        assert!(!ui.wants_keyboard());
    }

    #[test]
    fn a_rename_prompt_prefills_the_filename_and_names_its_action() {
        let mut prompt = PromptDialog::default();
        let mut ui = UIContext::new();
        let input = input::InputHandler::new();
        let theme = EditorTheme::default();
        prompt.open("Rename asset", "hero.png", "Rename");
        frame(&mut ui, &input, |ui| prompt.render(ui, &theme));
        let text = drawn_text(&ui);
        assert!(text.contains("hero.png") && text.contains("Rename") && !text.contains("Create"), "{text}");
        prompt.consumes_key(KeyCode::Enter);
        assert_eq!(frame(&mut ui, &input, |ui| prompt.render(ui, &theme)), PromptOutcome::Confirmed("hero.png".into()));
    }
}

#[cfg(test)]
mod wrap_tests {
    use super::wrap_error;

    #[test]
    fn errors_wrap_between_words_and_an_overlong_name_is_split_rather_than_overflowing() {
        let measure = |text: &str| text.chars().count() as f32 * 8.0;
        assert_eq!(wrap_error("Error: that name already exists", 160.0, measure), ["Error: that name", "already exists"]);
        let name = "a".repeat(60);
        let lines = wrap_error(&format!("could not create scenes/{name}.scene.ron: too long"), 160.0, measure);
        assert!(lines.iter().all(|line| measure(line) <= 160.0), "every line fits the dialog: {lines:?}");
        assert_eq!(lines.concat().replace(' ', ""), format!("couldnotcreatescenes/{name}.scene.ron:toolong"), "nothing is lost");
    }
}
