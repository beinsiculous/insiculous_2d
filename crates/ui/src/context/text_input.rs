//! Text-input widgets (numeric [`UIContext::float_input`] and free-form
//! [`UIContext::text_input`]) with a real editing model: click-to-focus
//! selects the whole value, a visible cursor, arrow/Home/End navigation,
//! shift-selection, and editing at the cursor position.
//!
//! The editing rules live in [`crate::TextEditState`]; the shared shell
//! lives in [`super::edit_field`].

use crate::input_state::InputState;
use crate::{FontHandle, Rect, ScrubState, WidgetId};

use super::edit_field::{EditFieldEvent, EditFieldParams};
use super::UIContext;

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
        let font = self.resolve_font(opts.font);

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

        if scrub_was_active {
            let hovered = bounds.contains(input.mouse_pos);
            self.draw_float_value(bounds, value, opts.suffix, hovered, font);
            return FloatInputResult::unchanged(value);
        }

        let display_text = format!("{:.2}{}", value, opts.suffix);
        let params = EditFieldParams { bounds, font, display_text: &display_text, result, was_focused };
        self.edit_field_click(id, &params, || format!("{:.2}", value));

        // Up/Down nudge the parsed buffer by the step (Shift ×10), clamped to
        // the soft range; the world updates live. Runs on the field's focus as
        // of this frame's click, and only when no cancel/commit is pending.
        if self.interaction.is_focused(id)
            && !input.escape_pressed
            && !input.enter_pressed
            && !input.tab_pressed
            && (!input.mouse_just_pressed || mouse_in_bounds)
            && (input.up_pressed || input.down_pressed)
        {
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

        match self.edit_field_edit_and_draw(id, &params, |t| t.parse::<f32>().is_ok()) {
            EditFieldEvent::Idle { .. } | EditFieldEvent::Cancelled => {
                FloatInputResult::unchanged(value)
            }
            EditFieldEvent::Editing { invalid, .. } => {
                FloatInputResult { invalid, ..FloatInputResult::unchanged(value) }
            }
            EditFieldEvent::Committed(text) => {
                self.commit_float_input(value, &text, &opts, bounds, font)
            }
        }
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
        let font = self.resolve_font(opts.font);
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
        let font = self.resolve_font(None);
        match self.edit_field(id, bounds, font, value, || value.to_string(), |_| true) {
            EditFieldEvent::Committed(new_text) => {
                self.draw_text_input_box(bounds, &new_text, false, font);
                self.note_edit_commit();
                Some(new_text)
            }
            _ => None,
        }
    }

    /// Commit the edit buffer of a float input: parse, clamp only under
    /// `hard_clamp`, unfocus, and draw the committed value. A parse failure
    /// reverts to the pre-edit value — never a silent half-result.
    fn commit_float_input(
        &mut self,
        fallback: f32,
        text: &str,
        opts: &FloatFieldOpts,
        bounds: Rect,
        font: Option<FontHandle>,
    ) -> FloatInputResult {
        let parsed = text
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite());
        let new_value = match parsed {
            Some(v) if opts.hard_clamp => v.clamp(opts.min, opts.max),
            Some(v) => v,
            None => fallback,
        };
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
}
