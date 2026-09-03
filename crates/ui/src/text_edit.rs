//! Pure text-editing state: cursor, selection, and edit operations.
//!
//! [`TextEditState`] is the model behind text-input widgets. It knows nothing
//! about drawing or input devices — widgets translate key presses into calls
//! on this state, which keeps every editing rule headless-testable.
//!
//! Indices are `char` indices (not bytes) so the state stays correct even
//! though today's inputs are ASCII-numeric.

/// Editable text buffer with a cursor and an optional selection.
///
/// The selection spans `selection_anchor..cursor` (either order); typing with
/// a selection active replaces it, matching standard text-field behavior.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextEditState {
    /// The text being edited.
    pub text: String,
    /// Cursor position as a char index in `0..=char_len`.
    pub cursor: usize,
    /// Selection anchor (char index). `Some` means text between the anchor
    /// and the cursor is selected; `None` means no selection.
    pub selection_anchor: Option<usize>,
}

impl TextEditState {
    /// Number of chars in the buffer.
    fn char_len(&self) -> usize {
        self.text.chars().count()
    }

    /// Byte offset of char index `i` (clamped to the end).
    fn byte_at(&self, i: usize) -> usize {
        self.text
            .char_indices()
            .nth(i)
            .map(|(b, _)| b)
            .unwrap_or(self.text.len())
    }

    /// Replace the buffer and select all of it, cursor at the end.
    /// This is the click-to-focus behavior: typing overwrites the old value.
    pub fn set_text_select_all(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.char_len();
        self.selection_anchor = if self.text.is_empty() { None } else { Some(0) };
    }

    /// The selected range as `(start, end)` char indices, normalized so
    /// `start <= end`. `None` when there is no (or an empty) selection.
    pub fn selected_range(&self) -> Option<(usize, usize)> {
        let anchor = self.selection_anchor?;
        if anchor == self.cursor {
            return None;
        }
        Some((anchor.min(self.cursor), anchor.max(self.cursor)))
    }

    /// Select the entire buffer, cursor at the end.
    pub fn select_all(&mut self) {
        self.cursor = self.char_len();
        self.selection_anchor = if self.text.is_empty() { None } else { Some(0) };
    }

    /// Delete the selected text (if any) and collapse the cursor to the
    /// selection start. Returns true if something was deleted.
    fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selected_range() else {
            self.selection_anchor = None;
            return false;
        };
        let (bs, be) = (self.byte_at(start), self.byte_at(end));
        self.text.replace_range(bs..be, "");
        self.cursor = start;
        self.selection_anchor = None;
        true
    }

    /// Insert a char at the cursor, replacing the selection if one is active.
    pub fn insert_char(&mut self, c: char) {
        self.delete_selection();
        let at = self.byte_at(self.cursor);
        self.text.insert(at, c);
        self.cursor += 1;
    }

    /// Backspace: delete the selection, or the char before the cursor.
    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor > 0 {
            let bs = self.byte_at(self.cursor - 1);
            let be = self.byte_at(self.cursor);
            self.text.replace_range(bs..be, "");
            self.cursor -= 1;
        }
    }

    /// Forward delete: delete the selection, or the char after the cursor.
    pub fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor < self.char_len() {
            let bs = self.byte_at(self.cursor);
            let be = self.byte_at(self.cursor + 1);
            self.text.replace_range(bs..be, "");
        }
    }

    /// Begin or extend a selection when `shift` is held; otherwise drop it.
    /// Returns the previous selection range for collapse handling.
    fn prepare_move(&mut self, shift: bool) -> Option<(usize, usize)> {
        let range = self.selected_range();
        if shift {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor);
            }
        } else {
            self.selection_anchor = None;
        }
        range
    }

    /// Move the cursor one char left. Plain arrow with an active selection
    /// collapses to the selection start without moving further.
    pub fn move_left(&mut self, shift: bool) {
        let prev = self.prepare_move(shift);
        if !shift {
            if let Some((start, _)) = prev {
                self.cursor = start;
                return;
            }
        }
        self.cursor = self.cursor.saturating_sub(1);
        self.drop_empty_selection();
    }

    /// Move the cursor one char right. Plain arrow with an active selection
    /// collapses to the selection end without moving further.
    pub fn move_right(&mut self, shift: bool) {
        let prev = self.prepare_move(shift);
        if !shift {
            if let Some((_, end)) = prev {
                self.cursor = end;
                return;
            }
        }
        self.cursor = (self.cursor + 1).min(self.char_len());
        self.drop_empty_selection();
    }

    /// Move the cursor to the start of the buffer.
    pub fn home(&mut self, shift: bool) {
        self.prepare_move(shift);
        self.cursor = 0;
        self.drop_empty_selection();
    }

    /// Move the cursor to the end of the buffer.
    pub fn end(&mut self, shift: bool) {
        self.prepare_move(shift);
        self.cursor = self.char_len();
        self.drop_empty_selection();
    }

    /// A shift-move that lands back on the anchor leaves no selection.
    fn drop_empty_selection(&mut self) {
        if self.selection_anchor == Some(self.cursor) {
            self.selection_anchor = None;
        }
    }

    /// Place the cursor from a click at `click_x` (widget-local pixels),
    /// given the prefix widths of the text: `prefix_widths[i]` is the pixel
    /// width of the first `i` chars, so it has `char_len + 1` entries.
    /// Picks the nearest char boundary and clears the selection.
    pub fn cursor_from_click(&mut self, prefix_widths: &[f32], click_x: f32) {
        let mut best = 0usize;
        let mut best_dist = f32::MAX;
        for (index, &prefix_width) in prefix_widths.iter().enumerate() {
            let distance_to_click = (click_x - prefix_width).abs();
            if distance_to_click < best_dist {
                best_dist = distance_to_click;
                best = index;
            }
        }
        self.cursor = best.min(self.char_len());
        self.selection_anchor = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(text: &str, cursor: usize) -> TextEditState {
        TextEditState { text: text.to_string(), cursor, selection_anchor: None }
    }

    #[test]
    fn test_typing_replaces_the_selection_else_inserts_at_the_cursor() {
        // Click-to-focus seeds the buffer fully selected (cursor at the end),
        // so the first key overwrites the old value.
        let mut edit = TextEditState::default();
        edit.set_text_select_all("168.40");
        assert_eq!((edit.selected_range(), edit.cursor), (Some((0, 6)), 6));
        edit.insert_char('5');
        assert_eq!((edit.text.as_str(), edit.cursor, edit.selected_range()), ("5", 1, None));

        let mut empty = TextEditState::default();
        empty.set_text_select_all("");
        assert_eq!((empty.selected_range(), empty.cursor), (None, 0), "an empty seed has nothing to select");

        let mut mid = state("129", 2);
        mid.insert_char('8');
        assert_eq!((mid.text.as_str(), mid.cursor), ("1289", 3), "no selection: insert at the cursor");
    }

    #[test]
    fn test_backspace_and_delete_remove_the_selection_else_the_adjacent_char_and_stop_at_the_edges() {
        // Selection covers chars 2..5 ("3.4"): backspace removes exactly
        // that range, not the char before the cursor.
        let mut selected = state("123.45", 5);
        selected.selection_anchor = Some(2);
        selected.backspace();
        assert_eq!((selected.text.as_str(), selected.cursor, selected.selected_range()), ("125", 2, None));

        let mut before = state("123", 2);
        before.backspace();
        assert_eq!((before.text.as_str(), before.cursor), ("13", 1));
        let mut at_start = state("123", 0);
        at_start.backspace();
        assert_eq!((at_start.text.as_str(), at_start.cursor), ("123", 0), "backspace at the start is a no-op");

        let mut after = state("123", 1);
        after.delete();
        assert_eq!((after.text.as_str(), after.cursor), ("13", 1));
        let mut at_end = state("123", 3);
        at_end.delete();
        assert_eq!(at_end.text, "123", "delete at the end is a no-op");
    }

    #[test]
    fn test_plain_moves_collapse_the_selection_and_shift_moves_extend_it_dropping_on_anchor_return() {
        // Plain arrows clamp at the buffer edges...
        let mut clamped = state("12", 0);
        clamped.move_left(false);
        assert_eq!(clamped.cursor, 0);
        for _ in 0..3 {
            clamped.move_right(false);
        }
        assert_eq!(clamped.cursor, 2);

        // ...and with a selection active collapse to its edge without moving further.
        let mut left = state("12345", 4);
        left.selection_anchor = Some(1);
        left.move_left(false);
        assert_eq!((left.cursor, left.selected_range()), (1, None), "left collapses to the selection start");
        let mut right = state("12345", 4);
        right.selection_anchor = Some(1);
        right.move_right(false);
        assert_eq!(right.cursor, 4, "right collapses to the selection end");

        // Shift extends from an anchor; landing back on the anchor drops the selection.
        let mut shifted = state("1234", 2);
        shifted.move_right(true);
        shifted.move_right(true);
        assert_eq!(shifted.selected_range(), Some((2, 4)));
        shifted.move_left(true);
        assert_eq!(shifted.selected_range(), Some((2, 3)));
        shifted.move_left(true);
        assert_eq!(shifted.selected_range(), None);

        // Home/End follow the same rules.
        let mut home_end = state("12345", 2);
        home_end.end(true);
        assert_eq!(home_end.selected_range(), Some((2, 5)));
        home_end.home(true);
        assert_eq!(home_end.selected_range(), Some((0, 2)));
        home_end.select_all();
        assert_eq!(home_end.selected_range(), Some((0, 5)));
        home_end.home(false);
        assert_eq!((home_end.cursor, home_end.selected_range()), (0, None), "plain Home collapses select-all");
        home_end.end(false);
        assert_eq!(home_end.cursor, 5);
    }

    #[test]
    fn test_cursor_from_click_picks_nearest_boundary() {
        let mut edit = state("124", 0);
        // Prefix widths for "124": 0, 8, 16, 24 px
        let widths = [0.0, 8.0, 16.0, 24.0];
        let cases = [(3.0, 0), (5.0, 1), (19.0, 2), (100.0, 3)];

        for (click_x, boundary) in cases {
            edit.selection_anchor = Some(0);
            edit.cursor_from_click(&widths, click_x);
            assert_eq!(edit.cursor, boundary, "click at {click_x}px");
            assert_eq!(edit.selected_range(), None, "a click clears the selection");
        }
    }

    #[test]
    fn test_empty_string_operations_are_safe() {
        let mut edit = TextEditState::default();
        edit.backspace();
        edit.delete();
        edit.move_left(true);
        edit.move_right(true);
        edit.home(false);
        edit.end(false);
        edit.select_all();
        assert_eq!((edit.text.as_str(), edit.cursor, edit.selected_range()), ("", 0, None));
    }
}
