//! Shared core for text input fields (numeric [`UIContext::float_input`] and
//! free-form [`UIContext::text_input`]).

use crate::context::{TextAlign, UIContext};
use crate::input_state::InputState;
use crate::text_edit::TextEditState;
use crate::{FontHandle, Rect, WidgetId, WidgetState};

const CARET_WIDTH: f32 = 1.0;

/// Event emitted by the shared `edit_field` shell.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EditFieldEvent {
    Idle { hovered: bool },
    Editing { text: String, invalid: bool },
    Committed(String),
    Cancelled,
}

/// Apply cursor navigation, deletion, and typed character edits to `state`
/// from the input state.
pub(crate) fn apply_edit_keys(state: &mut TextEditState, input: &InputState) {
    if input.left_pressed {
        state.move_left(input.shift_down);
    }
    if input.right_pressed {
        state.move_right(input.shift_down);
    }
    if input.home_pressed {
        state.home(input.shift_down);
    }
    if input.end_pressed {
        state.end(input.shift_down);
    }
    if input.backspace_pressed {
        state.backspace();
    }
    if input.delete_pressed {
        state.delete();
    }
    for ch in &input.typed_chars {
        state.insert_char(*ch);
    }
}

/// One frame's inputs to the shell: the widget's interaction result and
/// its focus state before that interaction was applied.
pub(crate) struct EditFieldParams<'a> {
    pub bounds: Rect,
    pub font: Option<FontHandle>,
    pub display_text: &'a str,
    pub result: crate::InteractionResult,
    pub was_focused: bool,
}

impl UIContext {
    /// The face a field draws and measures in: the requested handle when it still
    /// resolves, else the default font. Shared by float_input and text_input.
    pub(crate) fn resolve_font(&self, requested: Option<FontHandle>) -> Option<FontHandle> {
        requested
            .filter(|handle| self.font_manager.get_font(*handle).is_some())
            .or_else(|| self.font_manager.default_font())
    }

    /// The editing shell every text widget shares: click-to-focus (seeded, select-all),
    /// click-to-place-cursor, Escape cancel, Enter/Tab/click-away commit, key edits.
    /// Draws the unfocused box, the editing box, and the cancelled box itself; the
    /// caller draws the committed text (it knows the formatted value) and calls
    /// `note_edit_commit`.
    pub(crate) fn edit_field(
        &mut self,
        id: WidgetId,
        bounds: Rect,
        font: Option<FontHandle>,
        display_text: &str,
        seed_on_focus: impl FnOnce() -> String,
        is_valid: impl Fn(&str) -> bool,
    ) -> EditFieldEvent {
        let result = self.interaction.interact(id, bounds, true);
        let was_focused = self.interaction.is_focused(id);
        let params = EditFieldParams { bounds, font, display_text, result, was_focused };
        self.edit_field_click(id, &params, seed_on_focus);
        self.edit_field_edit_and_draw(id, &params, is_valid)
    }

    /// The click half of the shell: a click on an unfocused field focuses it
    /// with the seeded text selected; a click while editing places the caret.
    /// Runs before any key handling so a widget with its own key semantics
    /// (the float field's Up/Down nudge) sees the field's focus as of THIS
    /// frame's click.
    pub(crate) fn edit_field_click(
        &mut self,
        id: WidgetId,
        params: &EditFieldParams<'_>,
        seed_on_focus: impl FnOnce() -> String,
    ) {
        if !params.result.clicked {
            return;
        }
        if !params.was_focused {
            // Enter edit mode with the whole value selected — typing replaces it
            self.interaction.set_focus(id);
            let text = seed_on_focus();
            self.interaction.get_state(id).edit.set_text_select_all(&text);
            return;
        }
        // Click inside while editing: place the cursor at the click
        let mouse_x = self.interaction.input().mouse_pos.x;
        let padding = self.theme.text_input.padding;
        let font_size = self.theme.text_input.font_size;
        let text = self.interaction.get_state(id).edit.text.clone();
        let widths = self.prefix_widths(&text, font_size, params.font);
        let local_x = mouse_x - (params.bounds.x + padding);
        self.interaction.get_state(id).edit.cursor_from_click(&widths, local_x);
    }

    /// The keys-and-draw half of the shell: Escape cancels, Enter/Tab/click-away
    /// commit, edits apply, and the field draws in its current state.
    pub(crate) fn edit_field_edit_and_draw(
        &mut self,
        id: WidgetId,
        params: &EditFieldParams<'_>,
        is_valid: impl Fn(&str) -> bool,
    ) -> EditFieldEvent {
        let input = self.interaction.input().clone();
        let mouse_in_bounds = params.bounds.contains(input.mouse_pos);

        if self.interaction.is_focused(id) {
            // Cancel on Escape
            if input.escape_pressed {
                self.interaction.clear_focus();
                self.draw_text_input_box(params.bounds, params.display_text, false, params.font);
                return EditFieldEvent::Cancelled;
            }

            // Commit on Enter, Tab, or click outside
            if input.enter_pressed
                || input.tab_pressed
                || (input.mouse_just_pressed && !mouse_in_bounds)
            {
                let text = self.interaction.get_state(id).edit.text.clone();
                self.interaction.clear_focus();
                return EditFieldEvent::Committed(text);
            }

            apply_edit_keys(&mut self.interaction.get_state(id).edit, &input);

            let edit = self.interaction.get_state(id).edit.clone();
            let invalid = !is_valid(&edit.text);
            self.draw_text_input_editing_invalid(params.bounds, &edit, invalid, params.font);
            return EditFieldEvent::Editing {
                text: edit.text,
                invalid,
            };
        }

        // Not focused — draw display value
        let hovered = params.result.state == WidgetState::Hovered;
        self.draw_text_input_box(params.bounds, params.display_text, hovered, params.font);
        EditFieldEvent::Idle { hovered }
    }

    /// Pixel widths of every prefix of `text` at `font_size`:
    /// `result[i]` = width of the first `i` chars (so `len + 1` entries).
    /// Used to place the caret, the selection band, and click-to-cursor.
    pub(crate) fn prefix_widths(&self, text: &str, font_size: f32, font: Option<FontHandle>) -> Vec<f32> {
        let mut widths = Vec::with_capacity(text.chars().count() + 1);
        widths.push(0.0);
        let mut end = 0;
        for c in text.chars() {
            end += c.len_utf8();
            widths.push(self.measure_text_with_font(&text[..end], font_size, font).x);
        }
        widths
    }

    /// Draw a focused text input: box, selection band, text, and caret,
    /// clipped to the bounds so long edits don't overflow. `invalid` adds a
    /// red border while focused. Every measurement uses `font`, the face
    /// the text is drawn in.
    pub(crate) fn draw_text_input_editing_invalid(
        &mut self,
        bounds: Rect,
        edit: &TextEditState,
        invalid: bool,
        font: Option<FontHandle>,
    ) {
        let style = self.theme.text_input;
        let border = if invalid { style.border_invalid } else { style.border_focused };

        self.draw_list.rect_rounded(bounds, style.background_focused, style.corner_radius);
        self.draw_list
            .rect_border_rounded(bounds, border, style.border_width, style.corner_radius);

        self.push_clip_rect(bounds);

        let widths = self.prefix_widths(&edit.text, style.font_size, font);
        let text_origin_x = bounds.x + style.padding;
        // Vertical band for selection/caret: centered, sized from the font
        let band_height = (style.font_size * 1.2).min(bounds.height - 2.0);
        let band_y = bounds.y + (bounds.height - band_height) / 2.0;

        // Selection highlight behind the text
        if let Some((start, end)) = edit.selected_range() {
            let x0 = text_origin_x + widths[start.min(widths.len() - 1)];
            let x1 = text_origin_x + widths[end.min(widths.len() - 1)];
            self.draw_list.rect(
                Rect::new(x0, band_y, x1 - x0, band_height),
                style.selection_color,
            );
        }

        let text_pos = self.text_pos_in_bounds_with_font(
            &edit.text, bounds, TextAlign::Left, style.font_size, style.padding, font,
        );
        self.draw_text_with_font(font, &edit.text, text_pos, style.text_color, style.font_size);

        // Caret at the cursor position
        let caret_x = text_origin_x + widths[edit.cursor.min(widths.len() - 1)];
        self.draw_list.rect(
            Rect::new(caret_x, band_y, CARET_WIDTH, band_height),
            style.cursor_color,
        );

        self.pop_clip_rect();
    }

    /// Draw a text input box (shared by unfocused and committed states).
    pub(crate) fn draw_text_input_box(&mut self, bounds: Rect, text: &str, highlighted: bool, font: Option<FontHandle>) {
        let style = self.theme.text_input;
        let background = if highlighted { style.background_focused } else { style.background };
        let border = if highlighted { style.border_focused } else { style.border };

        self.draw_list.rect_rounded(bounds, background, style.corner_radius);
        self.draw_list
            .rect_border_rounded(bounds, border, style.border_width, style.corner_radius);

        let text_pos = self.text_pos_in_bounds_with_font(
            text, bounds, TextAlign::Left, style.font_size, style.padding, font,
        );
        self.draw_text_with_font(font, text, text_pos, style.text_color, style.font_size);
    }
}
