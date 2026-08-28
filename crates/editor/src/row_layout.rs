//! Pure row-layout math for the editable inspector.
//!
//! The inspector widgets used to place controls with fixed magic offsets
//! (`pos.x + label_width`, `label_width + 90.0`) that ignored the panel's
//! actual width — labels overran their input boxes and the [X] remove
//! button floated at an offset that never tracked the panel edge. Every
//! horizontal decision now goes through these functions, which are pure
//! (text measurement is injected) so they stay headless-testable.

use glam::Vec2;

use crate::field_style::EditableFieldStyle;

/// Horizontal gap between an axis/channel badge and its input box.
const BADGE_GAP: f32 = 4.0;

/// Minimum width an input box may shrink to on a narrow panel. Below this
/// an input is unusable, so degenerate panel widths accept a bounded
/// overdraw of at most this many pixels per input instead (panel content is
/// culled at the dock edge until #41 lands real scissoring).
const MIN_INPUT_WIDTH: f32 = 24.0;

/// Resolved layout for one positioned field row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowLayout {
    /// Top-left of the row (where the label draws).
    pub pos: Vec2,
    /// X where the row's control (input box, checkbox, first badge) starts.
    pub control_x: f32,
    /// Rightmost X any control in the row may extend to (the panel's
    /// content right edge).
    pub right: f32,
}

impl RowLayout {
    /// Width available to controls, never negative.
    pub fn available(&self) -> f32 {
        (self.right - self.control_x).max(0.0)
    }

    /// Clamp a desired control width to the available span (with a floor so
    /// inputs stay clickable on absurdly narrow panels).
    pub fn clamp_width(&self, desired: f32) -> f32 {
        desired.min(self.available()).max(MIN_INPUT_WIDTH)
    }
}

/// Layout for a field row whose label starts at `pos` inside an inspector
/// anchored at `origin_x` with `width` of content space.
pub fn field_row(pos: Vec2, origin_x: f32, width: f32, style: &EditableFieldStyle) -> RowLayout {
    RowLayout {
        pos,
        control_x: pos.x + style.label_width,
        right: origin_x + width,
    }
}

/// X position of the [X] remove button: right-aligned to the panel's content
/// edge instead of floating at a fixed offset from the label column.
pub fn remove_button_x(origin_x: f32, width: f32, btn_size: f32) -> f32 {
    (origin_x + width - btn_size).max(origin_x)
}

/// One badge+input slot inside a paired row (Vec2 X/Y, color channel column).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairSlot {
    /// X of the badge label ("X", "R", ...).
    pub badge_x: f32,
    /// X of the input box following the badge.
    pub input_x: f32,
    /// Width of the input box.
    pub input_width: f32,
}

/// Split a row's control span into two badge+input slots. `badge_widths` are
/// the measured badge label widths, `gap` separates the two slots, and the
/// input width fills the remaining space up to `max_w`, shrinking to fit the
/// panel down to the usability floor. Badges can never overlap their inputs
/// because the input starts a fixed [`BADGE_GAP`] after the measured badge
/// width.
pub fn pair_slots(
    layout: &RowLayout,
    badge_widths: [f32; 2],
    gap: f32,
    max_w: f32,
) -> [PairSlot; 2] {
    let fixed = badge_widths[0] + badge_widths[1] + 2.0 * BADGE_GAP + gap;
    let input_width = ((layout.available() - fixed) / 2.0).clamp(MIN_INPUT_WIDTH, max_w.max(MIN_INPUT_WIDTH));

    let first_badge_x = layout.control_x;
    let first_input_x = first_badge_x + badge_widths[0] + BADGE_GAP;
    let second_badge_x = first_input_x + input_width + gap;
    let second_input_x = second_badge_x + badge_widths[1] + BADGE_GAP;

    [
        PairSlot { badge_x: first_badge_x, input_x: first_input_x, input_width },
        PairSlot { badge_x: second_badge_x, input_x: second_input_x, input_width },
    ]
}

/// Vertical space a color field occupies: two channel rows plus the paddings
/// the drawing code uses (replaces the old `row_height * 1.8` fudge).
pub fn color_block_height(style: &EditableFieldStyle) -> f32 {
    // top pad + row + inner gap + row + bottom pad
    2.0 + style.color_input_height + 4.0 + style.color_input_height + 4.0
}

/// Truncate `text` with a trailing ellipsis so it fits in `max_w` according
/// to the injected measurement. Returns the text unchanged when it already
/// fits; degrades to a bare ellipsis when nothing fits.
pub fn ellipsize(text: &str, max_w: f32, measure: impl Fn(&str) -> f32) -> String {
    if measure(text) <= max_w {
        return text.to_string();
    }
    const ELLIPSIS: char = '…';
    let mut best = ELLIPSIS.to_string();
    for (i, _) in text.char_indices().skip(1) {
        let mut candidate = text[..i].to_string();
        candidate.push(ELLIPSIS);
        if measure(&candidate) <= max_w {
            best = candidate;
        } else {
            break;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn char_measure(s: &str) -> f32 {
        s.chars().count() as f32 * 7.0
    }

    #[test]
    fn test_remove_button_right_aligns_to_panel_edge() {
        // Wide panel: button hugs the right content edge regardless of label column.
        assert_eq!(remove_button_x(10.0, 400.0, 18.0), 392.0);
        // Degenerate panel: never left of the origin.
        assert_eq!(remove_button_x(10.0, 4.0, 18.0), 10.0);
    }

    #[test]
    fn test_field_row_right_edge_tracks_panel_width() {
        let style = EditableFieldStyle::default();
        let pos = Vec2::new(26.0, 40.0);
        let row = field_row(pos, 10.0, 300.0, &style);
        assert_eq!(row.pos, pos);
        assert_eq!(row.control_x, 26.0 + style.label_width);
        assert_eq!(row.right, 310.0);
        // Narrower panel moves only the right edge.
        let narrow = field_row(pos, 10.0, 150.0, &style);
        assert_eq!(narrow.control_x, row.control_x);
        assert_eq!(narrow.right, 160.0);
    }

    #[test]
    fn test_clamp_width_floors_on_narrow_panels() {
        let row = RowLayout { pos: Vec2::ZERO, control_x: 100.0, right: 110.0 };
        assert_eq!(row.clamp_width(100.0), MIN_INPUT_WIDTH);
        assert_eq!(MIN_INPUT_WIDTH, 24.0);
        let wide = RowLayout { pos: Vec2::ZERO, control_x: 100.0, right: 400.0 };
        assert_eq!(wide.clamp_width(100.0), 100.0);
    }

    #[test]
    fn test_pair_slots_never_overlap_badges() {
        let layout = RowLayout { pos: Vec2::ZERO, control_x: 120.0, right: 340.0 };
        let [a, b] = pair_slots(&layout, [9.0, 8.0], 8.0, 90.0);
        // Each input starts strictly after its measured badge.
        assert!(a.input_x >= a.badge_x + 9.0);
        assert!(b.input_x >= b.badge_x + 8.0);
        // The second badge starts after the first input ends.
        assert!(b.badge_x >= a.input_x + a.input_width);
    }

    #[test]
    fn test_pair_slots_shrink_on_narrow_panel_and_cap_on_wide() {
        // Narrow: inputs shrink to exactly fit the available span.
        let narrow = RowLayout { pos: Vec2::ZERO, control_x: 120.0, right: 240.0 };
        let [a, b] = pair_slots(&narrow, [9.0, 8.0], 8.0, 90.0);
        let fixed = 9.0 + 8.0 + 2.0 * 4.0 + 8.0;
        assert_eq!(a.input_width, (narrow.available() - fixed) / 2.0);
        assert!(b.input_x + b.input_width <= narrow.right + 0.01);
        // Wide: capped at max_w.
        let wide = RowLayout { pos: Vec2::ZERO, control_x: 120.0, right: 900.0 };
        let [w, _] = pair_slots(&wide, [9.0, 8.0], 8.0, 90.0);
        assert_eq!(w.input_width, 90.0);
    }

    #[test]
    fn test_pair_slots_degenerate_panel_overdraw_is_bounded() {
        // A panel narrower than the floor: inputs hold the usability floor
        // and overdraw is bounded by it instead of collapsing to zero.
        let tiny = RowLayout { pos: Vec2::ZERO, control_x: 120.0, right: 130.0 };
        let [a, b] = pair_slots(&tiny, [9.0, 8.0], 8.0, 90.0);
        assert_eq!(a.input_width, 24.0);
        assert!(b.input_x + b.input_width <= tiny.control_x + 24.0 * 2.0 + 9.0 + 8.0 + 2.0 * 4.0 + 8.0 + 0.01);
    }

    #[test]
    fn test_color_block_height_covers_two_rows() {
        let style = EditableFieldStyle::default();
        let h = color_block_height(&style);
        // Two 16px rows + 2px top + 4px inner + 4px bottom.
        assert_eq!(h, 42.0);
        // Content (top pad + both rows + inner gap) fits inside the block.
        assert!(2.0 + style.color_input_height * 2.0 + 4.0 <= h);
    }

    #[test]
    fn test_ellipsize_preserves_short_labels() {
        assert_eq!(ellipsize("Scale", 100.0, char_measure), "Scale");
    }

    #[test]
    fn test_ellipsize_truncates_with_ellipsis() {
        // "Angular Damping" is 15 chars = 105.0 wide; budget 60 = 8 chars.
        let out = ellipsize("Angular Damping", 60.0, char_measure);
        assert!(out.ends_with('…'));
        assert!(char_measure(&out) <= 60.0);
        assert!(out.len() > 1, "should keep a prefix, got {out:?}");
    }

    #[test]
    fn test_ellipsize_degrades_to_bare_ellipsis() {
        assert_eq!(ellipsize("Anything", 3.0, char_measure), "…");
    }
}
