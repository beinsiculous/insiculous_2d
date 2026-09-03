//! Text-input widgets (numeric [`UIContext::float_input`] and free-form
//! [`UIContext::text_input`]) with a real editing model: click-to-focus
//! selects the whole value, a visible cursor, arrow/Home/End navigation,
//! shift-selection, and editing at the cursor position.
//!
//! The editing rules live in [`crate::TextEditState`]; this file translates
//! input-state flags into edit calls and draws the box/selection/caret.

use crate::input_state::InputState;
use crate::{FontHandle, Rect, ScrubState, TextEditState, WidgetId, WidgetState};

use super::{TextAlign, UIContext};

/// Pointer travel (pixels) that turns a press on an unfocused float input
/// into a drag-scrub instead of a click-to-focus.
const SCRUB_THRESHOLD_PX: f32 = 4.0;

/// Options for a [`UIContext::float_input`].
///
/// `min..=max` is a SOFT range: it clamps drag-scrub and arrow-nudge output,
/// but a typed commit may exceed it — the inspector shows the world's real
/// value instead of silently clamping (set `hard_clamp` where the range is a
/// true invariant, e.g. color channels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatFieldOpts {
    /// Soft lower bound (scrub/arrow floor).
    pub min: f32,
    /// Soft upper bound (scrub/arrow ceiling).
    pub max: f32,
    /// Also clamp typed commits to `min..=max`.
    pub hard_clamp: bool,
    /// Arrow-nudge increment AND scrub units per pixel. Shift makes arrows
    /// coarser (×10) and scrubbing finer (×0.1).
    pub step: f32,
    /// Display-only suffix (e.g. `"°"`); never part of the edit buffer.
    pub suffix: &'static str,
    /// Face to draw AND measure the value in (`None` = the default font).
    /// Every measurement — caret, selection band, click-to-cursor — uses
    /// the same face, so a monospace numeric field never mis-places its
    /// caret.
    pub font: Option<FontHandle>,
}

impl FloatFieldOpts {
    /// Soft range with a 1.0 step, no suffix, the default font.
    pub fn range(min: f32, max: f32) -> Self {
        Self { min, max, hard_clamp: false, step: 1.0, suffix: "", font: None }
    }

    /// Hard-clamped range (typed commits clamp too).
    pub fn hard(min: f32, max: f32) -> Self {
        Self { hard_clamp: true, ..Self::range(min, max) }
    }

    /// Set the arrow/scrub step.
    pub fn with_step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Set the display suffix.
    pub fn with_suffix(mut self, suffix: &'static str) -> Self {
        self.suffix = suffix;
        self
    }

    /// Draw and measure in `font` (`None` keeps the default font).
    pub fn with_font(mut self, font: Option<FontHandle>) -> Self {
        self.font = font;
        self
    }
}

/// What a [`UIContext::float_input`] reported this frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FloatInputResult {
    /// The field's value after this frame (live during a scrub or arrow
    /// nudge, the committed value on a commit frame, else the input value).
    pub value: f32,
    /// `value` differs from the input value this frame.
    pub changed: bool,
    /// An edit gesture ended this frame (typed commit or scrub release) —
    /// hosts use it as an undo-merge boundary.
    pub committed: bool,
    /// The focused edit buffer currently fails to parse (red border shown).
    pub invalid: bool,
    /// A drag-scrub gesture is active.
    pub scrubbing: bool,
    /// A typed commit landed outside the SOFT `min..=max` (accepted by
    /// design — the host warns instead of clamping). Never set under
    /// `hard_clamp`.
    pub out_of_range: bool,
}

impl FloatInputResult {
    fn unchanged(value: f32) -> Self {
        Self { value, ..Self::default() }
    }
}

/// Apply one frame's navigation/deletion/typing to an edit state.
/// Shared by every text-input widget.
fn apply_edit_keys(state: &mut TextEditState, input: &InputState) {
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

/// Caret width in pixels.
const CARET_WIDTH: f32 = 1.0;

impl UIContext {
    /// Create a float text input field.
    ///
    /// Click to focus: the whole value is selected so typing overwrites it.
    /// Click again to place the cursor; arrows/Home/End move it (shift
    /// extends the selection), Up/Down nudge the value by `opts.step`
    /// (Shift ×10), Backspace/Delete edit at the cursor, and held keys
    /// repeat. Enter/Tab or clicking outside commits (a parse failure — red
    /// border while focused — reverts to the pre-edit value; the soft range
    /// only clamps unless `opts.hard_clamp`); Escape cancels.
    ///
    /// Dragging horizontally on an unfocused field scrubs the value
    /// (`opts.step` per pixel, Shift for fine ×0.1 control, Ctrl snaps to
    /// whole `opts.step` multiples — applied to the modifier-adjusted value,
    /// snap first, clamp last; clamped to the
    /// soft range); a press that travels less than the threshold is a plain
    /// click-to-focus. Escape mid-scrub restores the start value.
    pub fn float_input(
        &mut self,
        id: impl Into<WidgetId>,
        value: f32,
        opts: FloatFieldOpts,
        bounds: Rect,
    ) -> FloatInputResult {
        let id = id.into();
        let result = self.interaction.interact(id, bounds, true);
        let was_focused = self.interaction.is_focused(id);

        // Snapshot keyboard/mouse state before mutating persistent state
        let input = self.interaction.input().clone();
        let mouse_in_bounds = bounds.contains(input.mouse_pos);
        let padding = self.theme.text_input.padding;
        let font_size = self.theme.text_input.font_size;
        let font = self.field_font(&opts);

        // ---- drag-scrub (unfocused only) ------------------------------
        if !was_focused {
            if let Some(out) = self.float_scrub(id, value, &opts, bounds, &input, mouse_in_bounds) {
                return out;
            }
        }
        // A press that never crossed the threshold falls through to the
        // click-to-focus path below on its release frame.
        let scrub_was_active = self
            .interaction
            .get_state_if_exists(id)
            .and_then(|s| s.scrub)
            .is_some_and(|s| s.active);
        if input.mouse_just_released {
            self.interaction.get_state(id).scrub = None;
        }

        if result.clicked && !was_focused && !scrub_was_active {
            // Enter edit mode with the whole value selected — typing replaces it
            self.interaction.set_focus(id);
            let text = format!("{:.2}", value);
            self.interaction.get_state(id).edit.set_text_select_all(&text);
        } else if result.clicked && was_focused {
            // Click inside while editing: place the cursor at the click
            let text = self.interaction.get_state(id).edit.text.clone();
            let widths = self.prefix_widths(&text, font_size, font);
            let local_x = input.mouse_pos.x - (bounds.x + padding);
            self.interaction.get_state(id).edit.cursor_from_click(&widths, local_x);
        }

        if self.interaction.is_focused(id) {
            // Cancel on Escape
            if input.escape_pressed {
                self.interaction.clear_focus();
                self.draw_float_value(bounds, value, opts.suffix, false, font);
                return FloatInputResult::unchanged(value);
            }

            // Commit on Enter, Tab, or click outside
            if input.enter_pressed || input.tab_pressed || (input.mouse_just_pressed && !mouse_in_bounds) {
                return self.commit_float_input(id, value, &opts, bounds);
            }

            // Up/Down nudge the parsed buffer by the step (Shift ×10),
            // clamped to the soft range; the world updates live.
            if input.up_pressed || input.down_pressed {
                if let Ok(current) = self.interaction.get_state(id).edit.text.parse::<f32>() {
                    let step = opts.step * if input.shift_down { 10.0 } else { 1.0 };
                    let dir = if input.up_pressed { 1.0 } else { -1.0 };
                    let nudged = (current + dir * step).clamp(opts.min, opts.max);
                    let text = format!("{:.2}", nudged);
                    self.interaction.get_state(id).edit.set_text_select_all(&text);
                    let edit = self.interaction.get_state(id).edit.clone();
                    self.draw_text_input_editing_invalid(bounds, &edit, false, font);
                    return FloatInputResult {
                        value: nudged,
                        changed: (nudged - value).abs() > f32::EPSILON,
                        ..FloatInputResult::default()
                    };
                }
            }

            // Navigation, deletion, then typed characters — all cursor-aware
            apply_edit_keys(&mut self.interaction.get_state(id).edit, &input);

            let edit = self.interaction.get_state(id).edit.clone();
            let invalid = edit.text.parse::<f32>().is_err();
            self.draw_text_input_editing_invalid(bounds, &edit, invalid, font);
            return FloatInputResult { invalid, ..FloatInputResult::unchanged(value) };
        }

        // Not focused — draw display value
        let hovered = result.state == WidgetState::Hovered;
        self.draw_float_value(bounds, value, opts.suffix, hovered, font);
        FloatInputResult::unchanged(value)
    }

    /// The scrub half of [`Self::float_input`]: arm on press, activate past
    /// the threshold, emit per-frame values while dragging, commit on
    /// release. Returns `None` when no scrub processing applies this frame.
    fn float_scrub(
        &mut self,
        id: WidgetId,
        value: f32,
        opts: &FloatFieldOpts,
        bounds: Rect,
        input: &InputState,
        mouse_in_bounds: bool,
    ) -> Option<FloatInputResult> {
        let font = self.field_font(opts);
        if input.mouse_just_pressed && mouse_in_bounds {
            // Arm (re-seeding wipes any stale state from a prior gesture).
            self.interaction.get_state(id).scrub =
                Some(ScrubState { press_x: input.mouse_pos.x, start_value: value, active: false });
            self.draw_float_value(bounds, value, opts.suffix, true, font);
            return Some(FloatInputResult::unchanged(value));
        }

        let scrub = self.interaction.get_state_if_exists(id).and_then(|s| s.scrub)?;

        // Escape mid-scrub: restore the start value and end the gesture.
        if input.escape_pressed {
            self.interaction.get_state(id).scrub = None;
            self.draw_float_value(bounds, scrub.start_value, opts.suffix, false, font);
            self.note_edit_commit();
            return Some(FloatInputResult {
                value: scrub.start_value,
                changed: (scrub.start_value - value).abs() > f32::EPSILON,
                committed: true,
                ..FloatInputResult::default()
            });
        }

        if input.mouse_down {
            let pointer_travel_x = input.mouse_pos.x - scrub.press_x;
            let mut scrub = scrub;
            if !scrub.active && pointer_travel_x.abs() >= SCRUB_THRESHOLD_PX {
                scrub.active = true;
            }
            self.interaction.get_state(id).scrub = Some(scrub);
            if scrub.active {
                let step = opts.step * if input.shift_down { 0.1 } else { 1.0 };
                let mut scrubbed = scrub.start_value + pointer_travel_x * step;
                // Ctrl snaps the scrub to whole steps — applied to
                // whatever value the modifiers produced (Shift fine mode
                // included), then clamped: snap first, clamp last.
                if input.ctrl_down && opts.step != 0.0 {
                    scrubbed = (scrubbed / opts.step).round() * opts.step;
                }
                let scrubbed = scrubbed.clamp(opts.min, opts.max);
                self.draw_float_value(bounds, scrubbed, opts.suffix, true, font);
                return Some(FloatInputResult {
                    value: scrubbed,
                    changed: (scrubbed - value).abs() > f32::EPSILON,
                    scrubbing: true,
                    ..FloatInputResult::default()
                });
            }
            // Armed but below threshold: still a potential click.
            self.draw_float_value(bounds, value, opts.suffix, true, font);
            return Some(FloatInputResult::unchanged(value));
        }

        if input.mouse_just_released && scrub.active {
            // Scrub release: the last emitted value stands; seal the gesture.
            self.interaction.get_state(id).scrub = None;
            self.draw_float_value(bounds, value, opts.suffix, false, font);
            self.note_edit_commit();
            return Some(FloatInputResult {
                value,
                committed: true,
                ..FloatInputResult::default()
            });
        }

        None
    }

    /// Create a free-form text input field.
    ///
    /// Same editing model as [`float_input`](Self::float_input): click to
    /// focus with the whole value selected, click again to place the cursor,
    /// arrows/Home/End navigate (shift extends the selection), held keys
    /// repeat. Enter/Tab or clicking outside commits; Escape cancels.
    ///
    /// Returns `Some(new_text)` only on commit; `None` while displaying,
    /// editing, or on cancel.
    pub fn text_input(
        &mut self,
        id: impl Into<WidgetId>,
        value: &str,
        bounds: Rect,
    ) -> Option<String> {
        let id = id.into();
        let result = self.interaction.interact(id, bounds, true);
        let was_focused = self.interaction.is_focused(id);

        // Snapshot keyboard/mouse state before mutating persistent state
        let input = self.interaction.input().clone();
        let mouse_in_bounds = bounds.contains(input.mouse_pos);
        let padding = self.theme.text_input.padding;
        let font_size = self.theme.text_input.font_size;
        let font = self.font_manager.default_font();

        if result.clicked && !was_focused {
            // Enter edit mode with the whole value selected — typing replaces it
            self.interaction.set_focus(id);
            self.interaction.get_state(id).edit.set_text_select_all(value);
        } else if result.clicked && was_focused {
            // Click inside while editing: place the cursor at the click
            let text = self.interaction.get_state(id).edit.text.clone();
            let widths = self.prefix_widths(&text, font_size, font);
            let local_x = input.mouse_pos.x - (bounds.x + padding);
            self.interaction.get_state(id).edit.cursor_from_click(&widths, local_x);
        }

        if self.interaction.is_focused(id) {
            // Cancel on Escape
            if input.escape_pressed {
                self.interaction.clear_focus();
                self.draw_text_input_box(bounds, value, false, font);
                return None;
            }

            // Commit on Enter, Tab, or click outside
            if input.enter_pressed
                || input.tab_pressed
                || (input.mouse_just_pressed && !mouse_in_bounds)
            {
                let new_text = self.interaction.get_state(id).edit.text.clone();
                self.interaction.clear_focus();
                self.draw_text_input_box(bounds, &new_text, false, font);
                self.note_edit_commit();
                return Some(new_text);
            }

            apply_edit_keys(&mut self.interaction.get_state(id).edit, &input);

            let edit = self.interaction.get_state(id).edit.clone();
            self.draw_text_input_editing_invalid(bounds, &edit, false, font);
            return None;
        }

        // Not focused — draw display value
        let hovered = result.state == WidgetState::Hovered;
        self.draw_text_input_box(bounds, value, hovered, font);
        None
    }

    /// Commit the edit buffer of a float input: parse, clamp only under
    /// `hard_clamp`, unfocus, and draw the committed value. A parse failure
    /// reverts to the pre-edit value — never a silent half-result.
    fn commit_float_input(
        &mut self,
        id: WidgetId,
        fallback: f32,
        opts: &FloatFieldOpts,
        bounds: Rect,
    ) -> FloatInputResult {
        let font = self.field_font(opts);
        let parsed = self
            .interaction
            .get_state(id)
            .edit
            .text
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite());
        let new_value = match parsed {
            Some(v) if opts.hard_clamp => v.clamp(opts.min, opts.max),
            Some(v) => v,
            None => fallback,
        };
        self.interaction.clear_focus();
        self.draw_float_value(bounds, new_value, opts.suffix, false, font);
        self.note_edit_commit();
        let changed = (new_value - fallback).abs() > f32::EPSILON;
        // Only a NEW typed value warns — a parse failure reverts, and a
        // value that was already outside its range is not the user's doing.
        let out_of_range = parsed.is_some()
            && changed
            && !opts.hard_clamp
            && (new_value < opts.min || new_value > opts.max);
        FloatInputResult {
            value: new_value,
            changed,
            committed: true,
            out_of_range,
            ..FloatInputResult::default()
        }
    }

    /// Draw a float input showing a numeric value (plus a display-only
    /// suffix, e.g. `"°"`).
    fn draw_float_value(
        &mut self,
        bounds: Rect,
        value: f32,
        suffix: &str,
        highlighted: bool,
        font: Option<FontHandle>,
    ) {
        self.draw_text_input_box(bounds, &format!("{:.2}{}", value, suffix), highlighted, font);
    }

    /// The face a float field draws and measures in: its own when that
    /// handle still resolves, else the default font (a stale handle must not
    /// downgrade a field to placeholders while a usable font is loaded).
    fn field_font(&self, opts: &FloatFieldOpts) -> Option<FontHandle> {
        opts.font
            .filter(|handle| self.font_manager.get_font(*handle).is_some())
            .or(self.font_manager.default_font())
    }

    /// Pixel widths of every prefix of `text` at `font_size`:
    /// `result[i]` = width of the first `i` chars (so `len + 1` entries).
    /// Used to place the caret, the selection band, and click-to-cursor.
    fn prefix_widths(&self, text: &str, font_size: f32, font: Option<FontHandle>) -> Vec<f32> {
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
    /// red border ("this text is not a number") while focused. Every
    /// measurement uses `font`, the face the text is drawn in.
    fn draw_text_input_editing_invalid(
        &mut self,
        bounds: Rect,
        edit: &TextEditState,
        invalid: bool,
        font: Option<FontHandle>,
    ) {
        let style = self.theme.text_input.clone();
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
    fn draw_text_input_box(&mut self, bounds: Rect, text: &str, highlighted: bool, font: Option<FontHandle>) {
        let style = self.theme.text_input.clone();
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
