//! The sidebar's downward walk: one row after another, every control counted in order so
//! the arrows' cursor names the same control the render draws, and a row scrolled out of
//! the sidebar neither drawn nor clickable — it would sit over the transport above.

use ui::{FloatFieldOpts, Rect, UIContext, WidgetId};

use crate::composer::state::{ComposerState, ControlRequest};
use crate::composer::{ComposerAction, ComposerFocus};
use crate::theme::EditorTheme;

/// One row of the sidebar.
pub const ROW_HEIGHT: f32 = 22.0;
/// The space between two rows.
const ROW_GAP: f32 = 2.0;
/// A field's label column.
const LABEL_WIDTH: f32 = 84.0;
/// The sidebar's inner margin.
const INSET: f32 = 6.0;

pub(in crate::composer) struct SidebarWalk<'a> {
    pub ui: &'a mut UIContext,
    theme: &'a EditorTheme,
    area: Rect,
    /// Where the content's first row sits on screen, the scroll applied.
    top: f32,
    y: f32,
    control: usize,
    cursor: usize,
    focused: bool,
    request: Option<ControlRequest>,
    read_only: bool,
    reveal: bool,
    /// The cursor's control's top, from the scroll origin, once drawn.
    cursor_top: Option<f32>,
    /// The row placed last, which [`SidebarWalk::tooltip`] explains.
    last_row: Rect,
    /// The number field whose gesture was open when the walk began, or opened during it.
    field_gesture: Option<WidgetId>,
    /// Whether that field was drawn this frame focused or scrubbing; otherwise the walk's
    /// end closes the gesture.
    gesture_live: bool,
    actions: Vec<ComposerAction>,
}

/// What the walk leaves behind for the state and the render.
pub(in crate::composer) struct WalkEnd {
    pub content_height: f32,
    pub control_count: usize,
    /// The cursor, moved by a click or a Songs request.
    pub cursor: usize,
    /// The cursor's control's top, when a key asked to reveal it.
    pub reveal_top: Option<f32>,
    /// The number field whose gesture is still open after this frame.
    pub field_gesture: Option<WidgetId>,
}

impl<'a> SidebarWalk<'a> {
    pub fn new(
        ui: &'a mut UIContext,
        theme: &'a EditorTheme,
        area: Rect,
        offset: f32,
        state: &ComposerState,
        request: Option<ControlRequest>,
        read_only: bool,
    ) -> Self {
        ui.push_clip_rect(area);
        let top = area.y + INSET - offset;
        Self {
            ui,
            theme,
            area,
            top,
            y: top,
            control: 0,
            cursor: state.sidebar_control,
            focused: state.focus == ComposerFocus::Sidebar,
            request,
            read_only,
            reveal: state.reveal_control,
            cursor_top: None,
            last_row: Rect::default(),
            field_gesture: state.field_gesture,
            gesture_live: false,
            actions: Vec::new(),
        }
    }

    /// The index the next control drawn will take.
    pub fn next_control(&self) -> usize {
        self.control
    }

    pub fn push(&mut self, action: ComposerAction) {
        self.actions.push(action);
    }

    fn row(&mut self, height: f32) -> Rect {
        let row = Rect::new(self.area.x + INSET, self.y, (self.area.width - 2.0 * INSET).max(0.0), height);
        self.y += height + ROW_GAP;
        self.last_row = row;
        row
    }

    /// Offer the last row's tooltip, while it is inside the sidebar.
    pub fn tooltip(&mut self, text: &str) {
        if self.inside(self.last_row) {
            self.ui.tooltip(self.last_row, text);
        }
    }

    /// Whether a row lies wholly inside the sidebar, where it may be clicked.
    fn inside(&self, row: Rect) -> bool {
        row.y >= self.area.y && row.bottom() <= self.area.bottom()
    }

    /// Text wrapped at word boundaries onto as many rows as the sidebar's width needs: a
    /// note cut at the edge would drop the very words that say what to do.
    fn text_row(&mut self, text: &str, color: ui::Color, font_size: f32) {
        let width = (self.area.width - 2.0 * INSET).max(0.0);
        for line in wrap_to_width(text, width, |candidate| self.ui.measure_text_styled(candidate, font_size).x) {
            let row = self.row(ROW_HEIGHT - 4.0);
            if row.bottom() >= self.area.y && row.y <= self.area.bottom() {
                self.ui.label_in_bounds_styled(&line, row, ui::TextAlign::Left, color, font_size, 0.0);
            }
        }
    }

    pub fn heading(&mut self, text: &str) {
        self.y += ROW_GAP * 2.0;
        self.text_row(text, self.theme.accent_cyan, self.theme.fonts.body);
    }

    pub fn note(&mut self, text: &str) {
        self.text_row(text, self.theme.text_secondary, self.theme.fonts.small);
    }

    pub fn warning(&mut self, text: &str) {
        self.text_row(text, self.theme.warn_yellow, self.theme.fonts.small);
    }

    /// Put the arrows' cursor on the next control when Open Song… asked for the Songs list.
    pub fn mark_songs_list(&mut self) {
        if self.request == Some(ControlRequest::FocusSongsList) {
            self.cursor = self.control;
            self.reveal = true;
        }
    }

    /// Count the next control: whether the cursor is on it, and whether Enter acts on it.
    fn take_control(&mut self, row: Rect) -> (bool, bool) {
        let index = self.control;
        self.control += 1;
        let at_cursor = index == self.cursor;
        if at_cursor {
            // From the scroll origin, the top inset included: measured from the first row
            // instead, a revealed bottom row lands an inset short of fully inside, and a row
            // not wholly inside is neither ringed nor clickable.
            self.cursor_top = Some(row.y - self.top + INSET);
        }
        let activated = at_cursor && self.request == Some(ControlRequest::ActivateSidebar);
        (self.focused && at_cursor, activated)
    }

    fn ring(&mut self, row: Rect, at_cursor: bool) {
        if at_cursor && self.inside(row) {
            self.ui.rect_border(row, self.theme.focus_ring, 2.0, 2.0);
        }
    }

    /// A button row; `true` when it was clicked, or when Enter acted on it (a disabled one
    /// included, so the caller can say why it is disabled).
    pub fn button(&mut self, id: WidgetId, label: &str, enabled: bool) -> bool {
        let row = self.row(ROW_HEIGHT);
        let (at_cursor, activated) = self.take_control(row);
        let mut clicked = false;
        if self.inside(row) {
            clicked = self.ui.button_styled(id, label, row, enabled && !self.read_only);
        }
        self.ring(row, at_cursor);
        if clicked {
            self.cursor = self.control - 1;
        }
        clicked || activated
    }

    /// A labelled text field; its commit, if one landed this frame. Read-only, the value
    /// is drawn as text.
    pub fn field(&mut self, id: WidgetId, label: &str, value: &str) -> Option<String> {
        let row = self.row(ROW_HEIGHT);
        let (at_cursor, activated) = self.take_control(row);
        let (label_rect, input) = self.split(row);
        let mut committed = None;
        if self.inside(row) {
            self.ui.label_in_bounds_styled(label, label_rect, ui::TextAlign::Left, self.theme.text_secondary, self.theme.fonts.small, 0.0);
            if self.read_only {
                self.ui.label_in_bounds_styled(value, input, ui::TextAlign::Left, self.theme.text_muted, self.theme.fonts.small, 4.0);
            } else {
                committed = self.ui.text_input(id, value, input);
            }
        }
        if activated && !self.read_only {
            self.ui.focus_text_input(id, value);
        }
        self.ring(row, at_cursor);
        committed
    }

    /// A labelled number field; its value on every frame it changes — a scrub frame, an
    /// arrow nudge, a typed commit — and whether that frame is the first of the field's
    /// gesture (the rest continue it). A scrub's release frame reports the input value
    /// with only the commit flag set, so a host that waited for the commit would see the
    /// document's own value back. The gesture closes on the field's commit (a release,
    /// Enter, Escape), when the field is drawn neither focused nor scrubbing, and when it
    /// is not drawn at all — the walk decides from the field itself, so no ui-wide commit
    /// flag is shared with another panel.
    pub fn number(&mut self, id: WidgetId, label: &str, value: f32, options: FloatFieldOpts) -> Option<(f32, bool)> {
        let row = self.row(ROW_HEIGHT);
        let (at_cursor, activated) = self.take_control(row);
        let (label_rect, input) = self.split(row);
        let mut changed = None;
        if self.inside(row) {
            self.ui.label_in_bounds_styled(label, label_rect, ui::TextAlign::Left, self.theme.text_secondary, self.theme.fonts.small, 0.0);
            if self.read_only {
                let text = format!("{value}{}", options.suffix);
                self.ui.label_in_bounds_styled(&text, input, ui::TextAlign::Left, self.theme.text_muted, self.theme.fonts.small, 4.0);
            } else {
                let result = self.ui.float_input(id, value, options, input);
                if result.changed {
                    changed = Some((result.value, self.field_gesture != Some(id)));
                    self.field_gesture = Some(id);
                }
                if result.committed {
                    self.field_gesture = None;
                } else if self.field_gesture == Some(id) && (result.scrubbing || self.ui.is_focused(id)) {
                    self.gesture_live = true;
                }
            }
        }
        if activated && !self.read_only {
            self.ui.focus_text_input(id, &value.to_string());
        }
        self.ring(row, at_cursor);
        changed
    }

    fn split(&self, row: Rect) -> (Rect, Rect) {
        let label_width = LABEL_WIDTH.min(row.width * 0.45);
        (
            Rect::new(row.x, row.y, label_width, row.height),
            Rect::new(row.x + label_width, row.y, (row.width - label_width).max(0.0), row.height),
        )
    }

    /// End the walk: hand back its actions and what the state keeps. A field gesture whose
    /// field was not drawn live this frame is closed here.
    pub fn finish(mut self, actions: &mut Vec<ComposerAction>) -> WalkEnd {
        self.ui.pop_clip_rect();
        actions.append(&mut self.actions);
        WalkEnd {
            content_height: self.y - self.top + INSET,
            control_count: self.control,
            cursor: self.cursor,
            reveal_top: if self.reveal { self.cursor_top } else { None },
            field_gesture: self.field_gesture.filter(|_| self.gesture_live),
        }
    }
}

/// `text` broken at spaces into lines no wider than `width` by `measure`; a single word wider
/// than `width` keeps a line of its own rather than being cut.
fn wrap_to_width(text: &str, width: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        let candidate = if current.is_empty() { word.to_string() } else { format!("{current} {word}") };
        if current.is_empty() || measure(&candidate) <= width {
            current = candidate;
        } else {
            lines.push(std::mem::replace(&mut current, word.to_string()));
        }
    }
    lines.push(current);
    lines
}

#[cfg(test)]
mod tests {
    use super::wrap_to_width;

    #[test]
    fn a_note_wraps_at_spaces_to_the_width_and_a_long_word_keeps_its_own_line() {
        let measure = |text: &str| text.chars().count() as f32;
        assert_eq!(wrap_to_width("one two three", 7.0, measure), ["one two", "three"]);
        assert_eq!(wrap_to_width("short", 40.0, measure), ["short"]);
        assert_eq!(wrap_to_width("a extraordinarily b", 5.0, measure), ["a", "extraordinarily", "b"]);
    }
}
