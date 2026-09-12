//! Hover tooltips: the rest-to-show timer, the eligibility test behind it,
//! and the panel drawn on [`UiLayer::Tooltip`].
//!
//! A widget's owner calls [`UIContext::tooltip`] with the widget's rect and
//! the one sentence it explains, right after it draws that widget and from
//! inside whatever overlay scope the widget lives in. The context keeps the
//! single tooltip a frame may raise: the pointer must come to rest inside
//! the anchor and the anchor must be live under the same blocking test
//! [`UIContext::interact`] runs. Which anchor the frame is about is decided
//! at the end of the frame, once every widget has offered its own and every
//! overlay has pushed its blocking rect, and the panel is drawn there too —
//! so a widget drawn early in the frame still gets a panel above everything
//! else, and a scrim pushed after it is consulted.

use glam::Vec2;

use crate::{Rect, TextAlign, TooltipStyle, UiLayer};

use super::UIContext;

/// How long the pointer must rest on an anchor before its tooltip shows.
pub const TOOLTIP_DELAY: f32 = 0.5;

/// Distance from the pointer to the tooltip's near corner — far enough that
/// the pointer never covers the words it raised.
const POINTER_GAP: f32 = 12.0;

/// Padding between the tooltip's border and its text.
const TEXT_PADDING: f32 = 6.0;

/// Corner radius of the tooltip panel.
const CORNER_RADIUS: f32 = 3.0;

/// One anchor offered this frame with the pointer inside it.
#[derive(Debug, Clone)]
struct Candidate {
    anchor: Rect,
    /// The overlay scope the anchor was drawn in — what its eligibility is
    /// judged against, since the overlay it lives in must not blind it.
    layer: Option<UiLayer>,
    text: String,
}

/// The tooltip a frame may raise.
///
/// Only one exists at a time: a second anchor taking over restarts the
/// delay rather than queueing behind the first.
#[derive(Debug, Clone, Default)]
pub(crate) struct TooltipState {
    /// The rect the pointer is resting on.
    anchor: Option<Rect>,
    /// The overlay scope the anchor was drawn in, so the panel's own layer
    /// is not mistaken for something covering it.
    anchor_layer: Option<UiLayer>,
    /// The sentence the anchor's owner named.
    text: String,
    /// Seconds the pointer has rested on the anchor without moving.
    hovered_for: f32,
    /// Pointer position at the previous frame's end — a moving pointer
    /// before the delay elapses restarts it.
    pointer: Vec2,
    /// Whether the delay has elapsed for this anchor at least once. Set
    /// once, it keeps the panel up while the pointer stays inside.
    shown: bool,
    /// Every anchor offered this frame with the pointer inside it. Widgets
    /// nest — a panel's header and the chevron inside it — and an overlay
    /// can open after a widget it covers has already offered, so the one
    /// the pointer is "on" is only known once the frame is complete: the
    /// smallest that is still live then.
    candidates: Vec<Candidate>,
}

impl TooltipState {
    /// Take up a new anchor, restarting the delay at zero.
    fn take_up(&mut self, candidate: Candidate, pointer: Vec2) {
        self.anchor = Some(candidate.anchor);
        self.anchor_layer = candidate.layer;
        self.text = candidate.text;
        self.hovered_for = 0.0;
        self.shown = false;
        self.pointer = pointer;
    }

    /// Drop everything: the panel is not drawn and the next anchor rests
    /// from zero.
    fn drop_all(&mut self) {
        self.anchor = None;
        self.anchor_layer = None;
        self.text.clear();
        self.hovered_for = 0.0;
        self.shown = false;
        self.candidates.clear();
    }

    /// The anchor whose panel is due, and the layer it was drawn in.
    fn due(&self) -> Option<(Rect, Option<UiLayer>)> {
        if self.shown {
            self.anchor.map(|anchor| (anchor, self.anchor_layer))
        } else {
            None
        }
    }
}

impl UIContext {
    /// Offer `anchor`'s tooltip for this frame: `text` explains what the
    /// rect does.
    ///
    /// Call this every frame the anchor is drawn, next to the widget, and
    /// inside the same overlay scope. Nothing appears unless the pointer is
    /// inside `anchor` and the anchor is live once the frame is complete —
    /// a control under a modal's scrim neither accumulates nor draws, even
    /// when the modal opened after it was offered. Where live anchors nest,
    /// the smallest one under the pointer is the one it is resting on; of
    /// two the same size, the one offered last, which is the inner one. The
    /// panel appears once the pointer has rested on the anchor for
    /// [`TOOLTIP_DELAY`] without moving, stays up for as long as the pointer
    /// remains inside it, and goes the frame the pointer leaves, no call
    /// names an anchor, or a mouse button goes down — a click that begins
    /// and ends inside one frame included — and stays down for as long as
    /// the button is held: a gesture is not a rest.
    pub fn tooltip(&mut self, anchor: Rect, text: &str) {
        let pointer = self.interaction.mouse_pos();
        // An anchor the pointer is not on says nothing about the tooltip:
        // every other widget on screen calls this too, and the ones under
        // the pointer are the only ones that may claim the frame. An anchor
        // that stops being claimed is dropped at the end of the frame.
        if !anchor.contains(pointer) {
            return;
        }
        let input = self.interaction.input();
        if input.mouse_down || input.mouse_just_pressed {
            self.tooltip_state.drop_all();
            return;
        }
        self.tooltip_state.candidates.push(Candidate {
            anchor,
            layer: self.interaction.overlay_scope(),
            text: text.to_owned(),
        });
    }

    /// Settle the frame's anchor from the calls it made, then draw the
    /// tooltip that is due.
    ///
    /// `UIContext::end_frame` calls this before flushing the layers, which
    /// is what puts the panel above the modal that did not block it. Each
    /// candidate's own layer is what the blocking test runs against, over
    /// the frame's complete set of regions: a scrim pushed by an overlay
    /// drawn after the anchor still rules it out, while the overlay the
    /// anchor itself lives in does not.
    pub(super) fn end_frame_tooltip(&mut self) {
        let pointer = self.interaction.mouse_pos();
        let delta_time = self.interaction.frame_dt();
        let winner = self
            .tooltip_state
            .candidates
            .iter()
            .filter(|candidate| !self.interaction.is_blocked_for_scope(candidate.layer, pointer))
            .min_by(|a, b| {
                let area = |rect: Rect| rect.width * rect.height;
                area(a.anchor).total_cmp(&area(b.anchor))
            })
            .cloned();
        let state = &mut self.tooltip_state;
        state.candidates.clear();
        match winner {
            None => state.drop_all(),
            Some(candidate) => {
                match state.anchor {
                    Some(current) if current == candidate.anchor => {
                        // A pointer still travelling is not resting; once
                        // the panel is up it stays, which is what makes it
                        // readable.
                        if state.shown || pointer == state.pointer {
                            state.hovered_for += delta_time;
                        } else {
                            state.hovered_for = 0.0;
                        }
                        state.text = candidate.text;
                    }
                    _ => state.take_up(candidate, pointer),
                }
                state.pointer = pointer;
                if state.hovered_for >= TOOLTIP_DELAY {
                    state.shown = true;
                }
            }
        }
        if self.tooltip_state.due().is_some() {
            self.draw_tooltip(pointer);
        }
    }

    /// One line of text in a bordered panel, below-right of the pointer and
    /// clamped into the window.
    fn draw_tooltip(&mut self, pointer: Vec2) {
        let tooltip: TooltipStyle = self.theme.tooltip;
        let text = self.tooltip_state.text.clone();

        let text_size =
            self.measure_text_with_font(&text, tooltip.font_size, self.font_manager.default_font());
        let size = text_size + Vec2::splat(TEXT_PADDING * 2.0);
        let window = self.window_size;
        // Clamp rather than flip: a panel that fits stays whole, and one
        // wider than the window pins to the left edge instead of hanging
        // off the right.
        let bounds = Rect::new(
            (pointer.x + POINTER_GAP).clamp(0.0, (window.x - size.x).max(0.0)),
            (pointer.y + POINTER_GAP).clamp(0.0, (window.y - size.y).max(0.0)),
            size.x,
            size.y,
        );

        self.draw_list.push_layer(UiLayer::Tooltip);
        self.draw_list.rect_rounded(bounds, tooltip.background, CORNER_RADIUS);
        self.draw_list
            .rect_border_rounded(bounds, tooltip.border, 1.0, CORNER_RADIUS);
        self.label_in_bounds_styled(
            &text,
            bounds,
            TextAlign::Left,
            tooltip.text_color,
            tooltip.font_size,
            TEXT_PADDING,
        );
        self.draw_list.pop_layer();
    }
}
