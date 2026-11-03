//! The settings panel's downward walk, on the model of the Conductor panel's sidebar: one row after
//! another, every control counted in order so the arrows' cursor names the same control
//! the render draws, and a row scrolled out of the panel neither drawn nor clickable.

use glam::Vec2;
use ui::{Rect, UIContext, WidgetId};

use super::SettingsRequest;
use crate::conductor::sidebar::wrap_to_width;
use crate::field_style::{EditResult, EditableFieldStyle, FieldId};
use crate::field_widgets::{cycle_step, edit_cycle, CycleRow};
use crate::row_layout::{field_row, RowLayout};
use crate::text_field::edit_string_validated;
use crate::theme::EditorTheme;

/// The space between two rows.
const ROW_GAP: f32 = 2.0;
/// The panel's inner margin.
const INSET: f32 = 6.0;
/// The widest a button grows in the control column.
const BUTTON_WIDTH: f32 = 120.0;
/// Where a focused field scrolled out of the panel is drawn: past any window, so no
/// press can land inside it.
const OFF_SCREEN: f32 = -100_000.0;

/// What a field did this frame.
#[derive(Debug, Default)]
pub(super) struct FieldOutcome {
    /// The text a commit landed, when it differs from the saved value
    pub committed: Option<String>,
    /// The commit came from the keyboard — Enter or Tab — and not from a press away
    pub committed_by_key: bool,
    /// Focus left the field with nothing committed: Escape, or a commit of the saved text
    pub cancelled: bool,
    /// The field's index among the walk's controls, for a reveal
    pub control: usize,
}

pub(super) struct SettingsWalk<'a> {
    pub ui: &'a mut UIContext,
    theme: &'a EditorTheme,
    style: EditableFieldStyle,
    area: Rect,
    /// Where the content's first row sits on screen, the scroll applied.
    top: f32,
    y: f32,
    control: usize,
    cursor: usize,
    holds_keyboard: bool,
    request: Option<SettingsRequest>,
    /// The field focused as the frame began: a field drawn earlier this frame may take the
    /// focus from one drawn later, which still has to commit on the press that moved it.
    focused_at_start: Option<WidgetId>,
    /// The cursor's control's top from the scroll origin, once drawn.
    cursor_top: Option<f32>,
    /// The bottom of the cursor's control from the scroll origin, the note under it
    /// included, so a reveal shows the error with the field.
    cursor_bottom: Option<f32>,
    /// A control asked to be scrolled into view.
    reveal_requested: bool,
    last_row: Rect,
}

/// What the walk leaves behind for the state.
pub(super) struct WalkEnd {
    pub content_height: f32,
    pub control_count: usize,
    /// The cursor, moved by a click or a reveal.
    pub cursor: usize,
    pub cursor_top: Option<f32>,
    pub cursor_bottom: Option<f32>,
    /// Scroll the cursor's control into view: a failed commit's field.
    pub reveal: bool,
}

/// One control's turn: whether the ring is on it, and what a key asked of it.
struct Turn {
    ringed: bool,
    request: Option<SettingsRequest>,
}

impl<'a> SettingsWalk<'a> {
    pub fn new(
        ui: &'a mut UIContext,
        theme: &'a EditorTheme,
        area: Rect,
        offset: f32,
        state: &super::SettingsPanelState,
        request: Option<SettingsRequest>,
    ) -> Self {
        ui.push_clip_rect(area);
        let focused_at_start = ui.focused_widget();
        let mut style = theme.editable_field_style();
        style.label_width = style.label_width.min(area.width * 0.4);
        let top = area.y + INSET - offset;
        Self {
            ui,
            theme,
            style,
            area,
            top,
            y: top,
            control: 0,
            cursor: state.cursor,
            holds_keyboard: state.holds_keyboard,
            request,
            focused_at_start,
            cursor_top: None,
            cursor_bottom: None,
            reveal_requested: false,
            last_row: Rect::default(),
        }
    }

    /// A row's top or bottom measured from the scroll origin, the top inset included, as
    /// the sidebar measures it: measured from the first row, a revealed bottom row landed
    /// an inset short of inside.
    fn measured_from_origin(&self, y: f32) -> f32 {
        y - self.top + INSET
    }

    /// Put the cursor on the control just drawn and scroll it wholly into view, the note
    /// drawn under it after this call included: a field re-focused on a failed commit
    /// while scrolled out of the panel, so the typist sees the field and its error.
    pub fn reveal_control(&mut self, control: usize) {
        self.cursor = control;
        self.cursor_top = Some(self.measured_from_origin(self.last_row.y));
        self.cursor_bottom = Some(self.measured_from_origin(self.last_row.bottom()));
        self.reveal_requested = true;
    }

    fn row(&mut self, height: f32) -> Rect {
        let row = Rect::new(self.area.x + INSET, self.y, (self.area.width - 2.0 * INSET).max(0.0), height);
        self.y += height + ROW_GAP;
        self.last_row = row;
        row
    }

    /// Whether a row lies wholly inside the panel, where it may be drawn and clicked.
    fn inside(&self, row: Rect) -> bool {
        row.y >= self.area.y && row.bottom() <= self.area.bottom()
    }

    fn layout(&self, row: Rect) -> RowLayout {
        field_row(Vec2::new(row.x, row.y), row.x, row.width, &self.style)
    }

    fn take_control(&mut self, row: Rect) -> Turn {
        let index = self.control;
        self.control += 1;
        if index != self.cursor {
            return Turn { ringed: false, request: None };
        }
        self.cursor_top = Some(self.measured_from_origin(row.y));
        self.cursor_bottom = Some(self.measured_from_origin(row.bottom()));
        Turn { ringed: self.holds_keyboard, request: self.request }
    }

    fn ring(&mut self, row: Rect, ringed: bool) {
        if ringed && self.inside(row) {
            self.ui.rect_border(row, self.theme.focus_ring, 2.0, 2.0);
        }
    }

    /// Put the cursor on the control just counted: a click there.
    fn mark_clicked(&mut self) {
        self.cursor = self.control - 1;
    }

    /// Offer the last row's tooltip, while it is inside the panel.
    pub fn tooltip(&mut self, text: &str) {
        if self.inside(self.last_row) {
            self.ui.tooltip(self.last_row, text);
        }
    }

    /// Text wrapped at word boundaries onto as many rows as the panel's width needs.
    fn text_row(&mut self, text: &str, color: ui::Color, font_size: f32) {
        let width = (self.area.width - 2.0 * INSET).max(0.0);
        for line in wrap_to_width(text, width, |candidate| self.ui.measure_text_styled(candidate, font_size).x) {
            let row = self.row(self.style.row_height - 4.0);
            if row.bottom() >= self.area.y && row.y <= self.area.bottom() {
                self.ui.label_in_bounds_styled(&line, row, ui::TextAlign::Left, color, font_size, 0.0);
            }
        }
    }

    /// A section's heading. The rows under it draw their labels above their own centre,
    /// as the inspector's do, so the heading keeps a gap below it too.
    pub fn heading(&mut self, text: &str) {
        self.y += ROW_GAP * 3.0;
        self.text_row(text, self.theme.accent_cyan, self.theme.fonts.body);
        self.y += ROW_GAP * 3.0;
    }

    /// A note in the status bar's error colour under the control just drawn; the caller's
    /// words say it is an error. It keeps the heading's gap below it, for the same raised
    /// labels, and extends the cursor's reveal when it sits under the cursor's control.
    pub fn error(&mut self, text: &str) {
        self.text_row(text, self.theme.status_error, self.theme.fonts.small);
        self.y += ROW_GAP * 3.0;
        if self.control.checked_sub(1) == Some(self.cursor) {
            self.cursor_bottom = Some(self.measured_from_origin(self.last_row.bottom()));
        }
    }

    /// A labelled cycle row over `names`; the index chosen this frame, by a click on an
    /// arrow, Enter (a step forward) or Left and Right. The row's tooltip is its label and
    /// whole value, which a narrow panel ellipsizes.
    pub fn cycle(&mut self, id: FieldId, label: &str, names: &[String], index: usize) -> Option<usize> {
        let row = self.row(self.style.row_height);
        let turn = self.take_control(row);
        let mut chosen = None;
        if self.inside(row) {
            let value_name = names.get(index).map_or("", String::as_str);
            let cycle = CycleRow { label, value_name, index, count: names.len() };
            if let EditResult::Changed(clicked) = edit_cycle(self.ui, id, cycle, self.layout(row), &self.style) {
                chosen = Some(clicked);
                self.mark_clicked();
            }
            self.ui.tooltip(row, &format!("{label}: {value_name}"));
        }
        chosen = chosen.or(match turn.request {
            Some(SettingsRequest::Activate) => Some(cycle_step(index, names.len(), true)),
            Some(SettingsRequest::Cycle { forward }) => Some(cycle_step(index, names.len(), forward)),
            None => None,
        });
        self.ring(row, turn.ringed);
        chosen
    }

    /// A labelled text field drawing the ui's invalid border while `is_valid` fails.
    /// Unfocused it shows `draft` when there is one, else `saved`; focused, its commit is
    /// compared with `saved`, so committing a draft unchanged is still a commit. Enter on
    /// it gives it the keyboard.
    pub fn field(&mut self, id: FieldId, label: &str, saved: &str, draft: Option<&str>, is_valid: impl Fn(&str) -> bool) -> FieldOutcome {
        let row = self.row(self.style.row_height);
        let turn = self.take_control(row);
        let widget: WidgetId = id.into();
        let was_focused = self.focused_at_start == Some(widget);
        let shown = if was_focused { saved } else { draft.unwrap_or(saved) };
        let mut outcome = FieldOutcome { control: self.control - 1, ..FieldOutcome::default() };
        let drawn = self.inside(row);
        if drawn || was_focused {
            // The ui keeps a focused field until it is drawn, so a focused field scrolled
            // out of the panel is drawn off-screen: its keys and its commit still land —
            // left undrawn, it would hold the keyboard with nothing to receive it — while
            // every press is a press away, which commits as on a drawn field.
            let bounds = if drawn { row } else { Rect::new(OFF_SCREEN, OFF_SCREEN, row.width, row.height) };
            let by_key = self.ui.commit_key_pressed();
            if let EditResult::Changed(text) = edit_string_validated(self.ui, id, label, shown, self.layout(bounds), &self.style, is_valid) {
                outcome.committed = Some(text);
                outcome.committed_by_key = by_key;
            }
            outcome.cancelled = was_focused && !self.ui.is_focused(widget) && outcome.committed.is_none();
            if drawn && !was_focused && self.ui.is_focused(widget) {
                self.mark_clicked();
            }
        }
        if turn.request == Some(SettingsRequest::Activate) {
            self.ui.focus_text_input(widget, draft.unwrap_or(saved));
        }
        self.ring(row, turn.ringed);
        outcome
    }

    /// A button in the control column under `label`; `true` when it was clicked, or when
    /// Enter acted on it (a disabled one included, so the caller can say why). A disabled
    /// button's `hint` is its tooltip.
    pub fn button(&mut self, id: WidgetId, label: &str, enabled: bool, hint: Option<&str>) -> bool {
        let row = self.row(self.style.row_height);
        let turn = self.take_control(row);
        let mut clicked = false;
        if self.inside(row) {
            let layout = self.layout(row);
            let width = layout.available().min(BUTTON_WIDTH);
            let bounds = Rect::new(layout.control_x, row.y + 2.0, width, self.style.field_height());
            clicked = self.ui.button_styled(id, label, bounds, enabled);
            if let Some(hint) = hint.filter(|_| !enabled) {
                self.ui.tooltip(bounds, hint);
            }
        }
        if clicked {
            self.mark_clicked();
        }
        self.ring(row, turn.ringed);
        clicked || turn.request == Some(SettingsRequest::Activate)
    }

    /// End the walk: the content's height, the controls counted and the cursor.
    pub fn finish(self) -> WalkEnd {
        self.ui.pop_clip_rect();
        WalkEnd {
            content_height: self.y - self.top + INSET,
            control_count: self.control,
            cursor: self.cursor,
            cursor_top: self.cursor_top,
            cursor_bottom: self.cursor_bottom,
            reveal: self.reveal_requested,
        }
    }
}
