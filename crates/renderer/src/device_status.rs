//! Device-loss tracking primitives (pure, headless-testable).
//!
//! [`DeviceLossLatch`] is the one-way flag wgpu's device-lost callback sets
//! and the render path polls before touching the queue or surface. It is
//! deliberately one-way: recovery from device loss means rebuilding the whole
//! [`Renderer`](crate::Renderer) (the device/queue Arcs fan out into every
//! pipeline and texture), so "un-losing" a device makes no sense here.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// One-way latch set by wgpu's device-lost callback, polled by the render
/// path. Clones share the same flag.
///
/// Ordering is `Relaxed` on purpose: the flag guards only itself — no other
/// shared state is published through it. On wasm everything runs on the JS
/// main thread; on native, atomics are coherent per-location, so the render
/// loop observes the mark within a frame, which is all fail-stop needs.
#[derive(Clone, Debug, Default)]
pub struct DeviceLossLatch {
    lost: Arc<AtomicBool>,
}

impl DeviceLossLatch {
    /// A fresh latch in the not-lost state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark the device as lost. Idempotent; never resets.
    pub fn mark_lost(&self) {
        self.lost.store(true, Ordering::Relaxed);
    }

    /// Has the device been reported lost?
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Relaxed)
    }
}

/// Decide what a surface resize request should do.
///
/// Returns `Some(size)` when the surface must actually be reconfigured:
/// the requested size is non-zero AND (it differs from `current`, or `force`
/// is set). Returns `None` for zero-dimension requests (a wgpu validation
/// error — on the web the canvas reports 0x0 while hidden) and for no-op
/// same-size requests, which would otherwise tear down and recreate the
/// swapchain on every ResizeObserver echo.
///
/// `force` exists for the hidden-canvas round trip: after a zero-size request
/// was skipped, the next non-zero request must reconfigure even if it matches
/// the last configured size.
pub fn resize_action(
    current: (u32, u32),
    requested: (u32, u32),
    force: bool,
) -> Option<(u32, u32)> {
    let (width, height) = requested;
    if width == 0 || height == 0 {
        return None;
    }
    if requested == current && !force {
        return None;
    }
    Some(requested)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latch_starts_not_lost() {
        assert!(!DeviceLossLatch::new().is_lost());
    }

    #[test]
    fn latch_reports_lost_after_mark() {
        let latch = DeviceLossLatch::new();
        latch.mark_lost();
        assert!(latch.is_lost());
    }

    #[test]
    fn latch_clones_share_state() {
        let latch = DeviceLossLatch::new();
        let clone = latch.clone();
        clone.mark_lost();
        assert!(latch.is_lost());
    }

    #[test]
    fn latch_mark_is_idempotent() {
        let latch = DeviceLossLatch::new();
        latch.mark_lost();
        latch.mark_lost();
        assert!(latch.is_lost());
    }

    #[test]
    fn resize_action_skips_unchanged_size() {
        assert_eq!(resize_action((800, 600), (800, 600), false), None);
    }

    #[test]
    fn resize_action_skips_zero_width_or_height() {
        assert_eq!(resize_action((800, 600), (0, 600), false), None);
        assert_eq!(resize_action((800, 600), (800, 0), false), None);
        // Zero stays skipped even when forced — configuring a zero-size
        // surface is a wgpu validation error.
        assert_eq!(resize_action((800, 600), (0, 0), true), None);
    }

    #[test]
    fn resize_action_returns_new_size_when_changed() {
        assert_eq!(resize_action((800, 600), (1024, 768), false), Some((1024, 768)));
    }

    #[test]
    fn resize_action_forces_reconfigure_at_same_size() {
        // The hidden-canvas round trip: 800x600 -> 0x0 (skipped) -> 800x600
        // must reconfigure, or the surface stays stale after the canvas
        // becomes visible again.
        assert_eq!(resize_action((800, 600), (800, 600), true), Some((800, 600)));
    }
}
