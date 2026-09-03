//! Shared per-panel vertical scroll state, hoisted from the
//! asset browser's ad-hoc offset so the inspector and hierarchy scroll
//! the same way.
//!
//! Two valid call orders (pick by when content height is known):
//! - **Measured during render** (inspector, hierarchy — a downward `y`
//!   walk): `begin_frame` at panel start, `end_frame(measured)` after.
//!   Clamping then uses LAST frame's height — one frame of lag,
//!   invisible in practice.
//! - **Known up front** (asset browser — height derives from the entry
//!   count): `end_frame(height)` FIRST, then `begin_frame`, which makes
//!   the clamp lag-free. Copying the measured-order into such a panel
//!   only costs the one-frame lag back, never a failure.
//!
//! Scroll-wheel ownership convention: `ui.scroll_delta()` is a shared
//! per-frame value that is never consumed — every consumer (these panels,
//! viewport zoom) must gate on its own mouse-in-bounds check, and dock
//! panel bounds are disjoint, so exactly one consumer reacts per notch.
//!
//! Clipping note: the renderer culls only fully-outside commands, so a
//! partially-visible row bleeds up to one row height past the panel edge
//! while scrolled.

use glam::Vec2;

use common::Rect;

/// Pixels scrolled per wheel notch (matches the asset browser's feel).
const WHEEL_STEP: f32 = 30.0;

/// Vertical scroll offset for one panel.
#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    offset: f32,
    /// Content height measured at the end of the previous frame.
    last_content_height: f32,
}

impl ScrollState {
    /// Consume this frame's wheel input (when the mouse is inside
    /// `bounds`) and return the clamped offset to subtract from the
    /// panel's starting `y`. Call at panel-render start.
    pub fn begin_frame(
        &mut self,
        bounds: Rect,
        mouse: Vec2,
        scroll_delta: f32,
        viewport_height: f32,
    ) -> f32 {
        if scroll_delta != 0.0 && bounds.contains(mouse) {
            self.offset -= scroll_delta * WHEEL_STEP;
        }
        self.offset = self.offset.clamp(0.0, self.max_scroll(viewport_height));
        self.offset
    }

    /// Record the content height this frame actually laid out, and
    /// re-clamp so a shrink (collapsed section, deselected entity)
    /// snaps the offset back within range for the next frame.
    pub fn end_frame(&mut self, content_height: f32, viewport_height: f32) {
        self.last_content_height = content_height.max(0.0);
        self.offset = self.offset.clamp(0.0, self.max_scroll(viewport_height));
    }

    /// The current offset (as last clamped).
    pub fn offset(&self) -> f32 {
        self.offset
    }

    fn max_scroll(&self, viewport_height: f32) -> f32 {
        (self.last_content_height - viewport_height).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INSIDE: Vec2 = Vec2::new(50.0, 50.0);

    fn bounds() -> Rect {
        Rect::new(0.0, 0.0, 200.0, 100.0)
    }

    #[test]
    fn test_wheel_scrolls_only_with_the_mouse_inside_the_panel() {
        let mut scroll = ScrollState::default();
        scroll.end_frame(500.0, 100.0); // plenty of content

        let outside = Vec2::new(500.0, 500.0);
        assert_eq!(scroll.begin_frame(bounds(), outside, -1.0, 100.0), 0.0, "a wheel over another panel is ignored");
        assert_eq!(scroll.begin_frame(bounds(), INSIDE, -1.0, 100.0), WHEEL_STEP);
    }

    #[test]
    fn test_offset_clamps_to_the_content_measured_last_frame() {
        let mut scroll = ScrollState::default();

        // Content that fits the viewport never scrolls.
        scroll.end_frame(60.0, 100.0);
        assert_eq!(scroll.begin_frame(bounds(), INSIDE, -3.0, 100.0), 0.0);

        // 150px of content in a 100px viewport allows 50; ten notches down
        // would be 300.
        scroll.end_frame(150.0, 100.0);
        for _ in 0..10 {
            scroll.begin_frame(bounds(), INSIDE, -1.0, 100.0);
        }
        assert_eq!(scroll.offset(), 50.0);
        for _ in 0..10 {
            scroll.begin_frame(bounds(), INSIDE, 1.0, 100.0);
        }
        assert_eq!(scroll.offset(), 0.0, "scrolling back up clamps at the top");
    }

    #[test]
    fn test_shrinking_content_reclamps_the_offset() {
        let mut scroll = ScrollState::default();
        scroll.end_frame(500.0, 100.0);
        for _ in 0..5 {
            scroll.begin_frame(bounds(), INSIDE, -1.0, 100.0);
        }
        assert_eq!(scroll.offset(), 150.0);

        // A collapse shrinks the content below the current offset: the
        // end-of-frame measurement snaps the offset back into range, so the
        // panel never shows blank space past its last row.
        scroll.end_frame(120.0, 100.0);
        assert_eq!(scroll.offset(), 20.0);
    }
}
