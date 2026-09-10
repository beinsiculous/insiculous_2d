//! The colour editor: a popup opened from an RGBA row's swatch, with a
//! large preview, four channel fields and a hex field.
//!
//! It is drawn as a **pass of its own, before any panel renders**, not from
//! inside the row it edits. Blocking rects are consulted at each widget's
//! own `interact` call, so a popup pushed mid-walk cannot make the rows
//! already drawn above it inert — a drag on a channel would scrub whatever
//! row sat underneath. Drawing first, on the Modal band, makes its rect
//! cover every widget of the frame, including a panel that is itself a
//! Floating overlay in the dock's narrow mode.
//!
//! The write path is still the row's: the pass records the frame's edit and
//! the colour row takes it as its own `EditResult::Changed` on the next
//! frame, so the component's ordinary undo merge gives one entry per
//! gesture.

use glam::{Vec2, Vec4};
use ui::{Color, Rect, UIContext};

use crate::field_style::{EditResult, EditableFieldStyle, FieldId};

/// Widget-id space the popup owns. It is a singleton — one colour editor is
/// open at a time — so its ids sit past every component block's rather than
/// riding in the row's, whose index the pass no longer knows.
const POPUP_COMPONENT_SLOT: usize = 900;

/// Height of one popup row (a channel field, the hex field).
const ROW_HEIGHT: f32 = 22.0;

/// Size of the popup's preview swatch.
const PREVIEW_SIZE: f32 = 32.0;

/// Popup width, and the inner padding around its content.
const WIDTH: f32 = 200.0;
const PADDING: f32 = 8.0;

/// Gap between the popup and the swatch it hangs off.
const ANCHOR_GAP: f32 = 2.0;

/// What one frame of the popup did.
pub struct ColorEditorFrame {
    /// The edited colour, when a channel or the hex field changed it.
    pub result: EditResult<Vec4>,
    /// The popup asked to close: a click landed outside it.
    pub close: bool,
    /// Text committed in the hex field that is not a colour — the host says
    /// so on the status bar, where every other rejected inspector input goes.
    pub invalid_hex: Option<String>,
}

/// Widget id of one of the popup's fields: 0..=3 are R, G, B, A and 4 is
/// the hex field. Public so a host can reach a field by keyboard without
/// knowing where it was drawn.
pub fn color_editor_channel_id(channel: usize) -> FieldId {
    FieldId::new(POPUP_COMPONENT_SLOT, channel, 0)
}

/// Total popup height: the preview, the four channels and the hex field.
fn popup_height() -> f32 {
    2.0 * PADDING + PREVIEW_SIZE + 5.0 * ROW_HEIGHT
}

/// Where the popup draws for a swatch at `anchor`, inside `window_size`.
///
/// Vertically it opens below the swatch and flips above when it would leave
/// the window. Horizontally it starts at the swatch and is clamped to the
/// window — a right-docked inspector would otherwise push half of it off
/// screen — and if the clamped popup would then cover its own swatch it
/// moves to the swatch's left, when that fits.
pub fn color_editor_bounds(anchor: Rect, window_size: Vec2) -> Rect {
    let height = popup_height();
    let below = anchor.y + anchor.height + ANCHOR_GAP;
    let y = if below + height <= window_size.y {
        below
    } else {
        (anchor.y - ANCHOR_GAP - height).max(0.0)
    };

    let clamped_x = anchor.x.min(window_size.x - WIDTH).max(0.0);
    let bounds = Rect::new(clamped_x, y, WIDTH, height);
    if !overlaps(bounds, anchor) {
        return bounds;
    }
    let to_the_left = anchor.x - ANCHOR_GAP - WIDTH;
    if to_the_left >= 0.0 {
        Rect::new(to_the_left, y, WIDTH, height)
    } else {
        bounds
    }
}

/// Bounds of one of the popup's four channel fields.
pub fn color_editor_channel_bounds(popup: Rect, channel: usize) -> Rect {
    let x = popup.x + PADDING + PREVIEW_SIZE + PADDING;
    let y = popup.y + PADDING + PREVIEW_SIZE + ANCHOR_GAP + channel as f32 * ROW_HEIGHT;
    Rect::new(x, y, popup.x + WIDTH - PADDING - x, ROW_HEIGHT - 4.0)
}

/// Whether two rects share any area.
fn overlaps(first: Rect, second: Rect) -> bool {
    first.x < second.x + second.width
        && second.x < first.x + first.width
        && first.y < second.y + second.height
        && second.y < first.y + first.height
}

/// Draw the colour editor for the row whose swatch is at `anchor`.
///
/// Runs before the panels, as one sequential overlay on the Modal band, so
/// its blocking rect reaches every widget drawn after it.
pub fn render_color_editor(
    ui: &mut UIContext,
    value: Vec4,
    anchor: Rect,
    style: &EditableFieldStyle,
) -> ColorEditorFrame {
    let bounds = color_editor_bounds(anchor, ui.window_size());
    // A click that lands neither on the popup nor back on the swatch that
    // opened it is the ordinary way out of a popup.
    let pointer = ui.mouse_pos();
    let close = ui.mouse_just_pressed() && !bounds.contains(pointer) && !anchor.contains(pointer);

    ui.begin_overlay_in(ui::UiLayer::Modal, bounds);
    ui.panel_styled(bounds, style.slot_bg, style.label_color, 1.0);

    let preview = Rect::new(bounds.x + PADDING, bounds.y + PADDING, PREVIEW_SIZE, PREVIEW_SIZE);
    ui.rect_rounded(preview, Color::new(value.x, value.y, value.z, value.w), 3.0);

    let mut new_value = value;
    let mut changed = false;
    // Channels are a true 0..=1 invariant, so typed commits clamp too.
    let opts = ui::FloatFieldOpts::hard(0.0, 1.0)
        .with_step(0.005)
        .with_font(style.numeric_font);
    for (channel, (badge, channel_value)) in [
        ("R", value.x),
        ("G", value.y),
        ("B", value.z),
        ("A", value.w),
    ]
    .into_iter()
    .enumerate()
    {
        let input = color_editor_channel_bounds(bounds, channel);
        ui.label_styled(
            badge,
            Vec2::new(bounds.x + PADDING, input.y + 4.0),
            style.channel_labels[channel],
            style.channel_font,
        );
        let edited = ui.float_input(color_editor_channel_id(channel), channel_value, opts, input);
        if edited.changed {
            match channel {
                0 => new_value.x = edited.value,
                1 => new_value.y = edited.value,
                2 => new_value.z = edited.value,
                _ => new_value.w = edited.value,
            }
            changed = true;
        }
    }

    let last_channel = color_editor_channel_bounds(bounds, 3);
    let hex_bounds = Rect::new(
        bounds.x + PADDING,
        last_channel.y + ROW_HEIGHT,
        WIDTH - 2.0 * PADDING,
        ROW_HEIGHT - 4.0,
    );
    let shown_hex = crate::color_hex::to_hex(value);
    let mut invalid_hex = None;
    if let Some(typed) = ui.text_input(color_editor_channel_id(4), &shown_hex, hex_bounds) {
        if typed != shown_hex {
            match crate::color_hex::from_hex(&typed, value.w) {
                Some(parsed) => {
                    new_value = parsed;
                    changed = true;
                }
                None => invalid_hex = Some(typed),
            }
        }
    }

    ui.end_overlay();

    ColorEditorFrame {
        result: if changed {
            EditResult::Changed(new_value)
        } else {
            EditResult::Unchanged
        },
        close,
        invalid_hex,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Vec2 = Vec2::new(800.0, 600.0);

    #[test]
    fn test_the_popup_flips_above_the_swatch_when_it_would_leave_the_window() {
        let swatch = Rect::new(100.0, 40.0, 20.0, 20.0);
        assert_eq!(color_editor_bounds(swatch, WINDOW).y, 62.0, "it opens below the swatch");

        let low = Rect::new(100.0, 560.0, 20.0, 20.0);
        let flipped = color_editor_bounds(low, WINDOW);
        assert_eq!(flipped.y, 560.0 - ANCHOR_GAP - popup_height(), "it flips above");

        let short_window = Vec2::new(800.0, 120.0);
        assert_eq!(
            color_editor_bounds(swatch, short_window).y,
            0.0,
            "taller than the window clamps to the top"
        );
    }

    #[test]
    fn test_a_swatch_at_the_right_edge_keeps_the_whole_popup_on_screen() {
        // A right-docked inspector puts the swatch within a popup's width of
        // the window edge; unclamped, the hex field is off screen.
        let swatch = Rect::new(760.0, 200.0, 20.0, 20.0);
        let bounds = color_editor_bounds(swatch, WINDOW);
        assert!(bounds.x + bounds.width <= WINDOW.x, "the popup stays inside the window");
        assert!(bounds.x >= 0.0);
        assert!(!overlaps(bounds, swatch), "and never covers the swatch it opened from");

        // In a window too short for the popup to sit either side of the
        // swatch, the clamped x would put it straight over the swatch;
        // moving to the swatch's left is what keeps it visible.
        let short_window = Vec2::new(800.0, 120.0);
        let squeezed = color_editor_bounds(Rect::new(760.0, 40.0, 20.0, 20.0), short_window);
        assert_eq!(squeezed.x, 760.0 - ANCHOR_GAP - WIDTH, "it moves to the swatch's left");
        assert!(squeezed.x + squeezed.width <= short_window.x);
    }

    #[test]
    fn test_channel_bounds_stack_inside_the_popup_and_never_overlap() {
        let bounds = color_editor_bounds(Rect::new(100.0, 40.0, 20.0, 20.0), WINDOW);
        let mut previous: Option<Rect> = None;
        for channel in 0..4 {
            let row = color_editor_channel_bounds(bounds, channel);
            assert!(row.x >= bounds.x && row.x + row.width <= bounds.x + bounds.width);
            assert!(row.y >= bounds.y && row.y + row.height <= bounds.y + bounds.height);
            if let Some(previous) = previous {
                assert!(row.y >= previous.y + previous.height, "channel {channel} clears the one above");
            }
            previous = Some(row);
        }
    }
}
