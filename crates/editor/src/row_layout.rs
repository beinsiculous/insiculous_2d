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
/// culled at the dock edge until real scissoring lands).
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

/// Derive a drag-scrub / arrow-nudge step from a field's soft range: about
/// 200 pixels of drag traverses a bounded range, unbounded or huge ranges
/// fall back to 1.0/px, and narrow ranges stay finely scrubable.
pub fn scrub_step(range: &std::ops::RangeInclusive<f32>) -> f32 {
    let span = range.end() - range.start();
    if !span.is_finite() || span > 400.0 {
        1.0
    } else {
        (span / 200.0).max(0.001)
    }
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
    fn test_field_row_keeps_the_label_column_and_moves_only_the_right_edge() {
        // A narrower panel never shifts the controls left of the label
        // column; only the right edge tracks the width, and an input still
        // gets MIN_INPUT_WIDTH when the row is too cramped to fit it.
        let style = EditableFieldStyle::default();
        let pos = Vec2::new(26.0, 40.0);
        let row = field_row(pos, 10.0, 300.0, &style);
        assert_eq!((row.pos, row.control_x, row.right), (pos, 26.0 + style.label_width, 310.0));
        let narrow = field_row(pos, 10.0, 150.0, &style);
        assert_eq!((narrow.control_x, narrow.right), (row.control_x, 160.0));

        let cramped = RowLayout { pos: Vec2::ZERO, control_x: 100.0, right: 110.0 };
        assert_eq!(cramped.clamp_width(100.0), MIN_INPUT_WIDTH, "10px of room still draws a usable input");
        let wide = RowLayout { pos: Vec2::ZERO, control_x: 100.0, right: 400.0 };
        assert_eq!(wide.clamp_width(100.0), 100.0);
    }

    #[test]
    fn test_remove_button_right_aligns_to_the_panel_edge() {
        assert_eq!(remove_button_x(10.0, 400.0, 18.0), 392.0, "hugs the right content edge");
        assert_eq!(remove_button_x(10.0, 4.0, 18.0), 10.0, "never left of the origin on a degenerate panel");
    }

    #[test]
    fn test_pair_slots_shrink_on_narrow_panels_cap_on_wide_ones_and_never_overlap() {
        let badges = [9.0, 8.0];
        let fixed = 9.0 + 8.0 + 2.0 * 4.0 + 8.0;

        // Narrow: inputs shrink to exactly fit the available span, each
        // input starts after its measured badge and the second badge after
        // the first input — X and Y never draw over each other.
        let narrow = RowLayout { pos: Vec2::ZERO, control_x: 120.0, right: 240.0 };
        let [a, b] = pair_slots(&narrow, badges, 8.0, 90.0);
        assert_eq!(a.input_width, (narrow.available() - fixed) / 2.0);
        assert!(b.input_x + b.input_width <= narrow.right + 0.01, "the Y input stays inside the row");
        assert!(a.input_x >= a.badge_x + 9.0, "X input starts after its badge");
        assert!(b.input_x >= b.badge_x + 8.0, "Y input starts after its badge");
        assert!(b.badge_x >= a.input_x + a.input_width, "Y badge starts after the X input ends");

        // Wide: capped at max_w.
        let wide = RowLayout { pos: Vec2::ZERO, control_x: 120.0, right: 900.0 };
        let [w, _] = pair_slots(&wide, badges, 8.0, 90.0);
        assert_eq!(w.input_width, 90.0);

        // Narrower than the floor: inputs hold the usability floor and the
        // overdraw is bounded by it instead of collapsing to zero.
        let tiny = RowLayout { pos: Vec2::ZERO, control_x: 120.0, right: 130.0 };
        let [a, b] = pair_slots(&tiny, badges, 8.0, 90.0);
        assert_eq!(a.input_width, MIN_INPUT_WIDTH);
        assert!(b.input_x + b.input_width <= tiny.control_x + MIN_INPUT_WIDTH * 2.0 + fixed + 0.01);
    }

    #[test]
    fn test_color_block_reserves_room_for_both_channel_rows() {
        let style = EditableFieldStyle::default();
        let height = color_block_height(&style);
        assert_eq!(height, 42.0, "two 16px rows + 2px top + 4px inner + 4px bottom");
        assert!(2.0 + style.color_input_height * 2.0 + 4.0 <= height, "the content fits inside the block");
    }

    #[test]
    fn test_scrub_step_scales_with_the_range_and_falls_back_to_a_unit_per_pixel_when_unbounded() {
        // A tight 0..=1 range scrubs finely (~200px of drag spans it), mid
        // ranges scale with the span, huge/unbounded ranges move one unit
        // per pixel.
        assert!((scrub_step(&(0.0..=1.0)) - 0.005).abs() < 1e-6);
        assert!((scrub_step(&(0.0..=10.0)) - 0.05).abs() < 1e-6);
        assert_eq!(scrub_step(&(-1000.0..=1000.0)), 1.0);
        assert_eq!(scrub_step(&(f32::NEG_INFINITY..=f32::INFINITY)), 1.0);
    }

    #[test]
    fn test_ellipsize_keeps_short_labels_and_truncates_long_ones_within_budget() {
        let cases: [(&str, f32, &str); 3] = [
            ("Scale", 100.0, "Scale"),
            // 15 chars = 105px; an 8-char budget keeps a prefix plus "…".
            ("Angular Damping", 60.0, "Angular…"),
            // No prefix fits: a bare ellipsis is the floor.
            ("Anything", 3.0, "…"),
        ];
        for (label, budget, expected) in cases {
            assert_eq!(ellipsize(label, budget, char_measure), expected, "{label:?} in {budget}px");
        }
    }
}
