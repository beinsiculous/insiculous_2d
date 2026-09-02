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

    /// Fail-stop: once wgpu's lost callback marks the latch, every clone the
    /// render path polls sees it, and nothing ever clears it.
    #[test]
    fn test_device_loss_latch_is_one_way_and_shared_by_clones() {
        let latch = DeviceLossLatch::new();
        let callback_copy = latch.clone();
        assert!(!latch.is_lost(), "a fresh latch is not lost");

        callback_copy.mark_lost();
        callback_copy.mark_lost();

        assert!(latch.is_lost(), "a mark on one clone is visible on the other");
        assert!(callback_copy.is_lost(), "a second mark keeps it lost");
    }

    /// Same size is a no-op (ResizeObserver echoes must not rebuild the
    /// swapchain); zero is always skipped (a wgpu validation error, and what
    /// a hidden web canvas reports); a new size reconfigures; and `force`
    /// reconfigures at the same size — the hidden-canvas round trip
    /// 800x600 → 0x0 → 800x600 must not leave a stale surface.
    #[test]
    fn test_resize_action_skips_same_size_and_zero_unless_forced_non_zero() {
        let current = (800, 600);
        let cases = [
            ((800, 600), false, None, "the same size is a no-op"),
            ((0, 600), false, None, "zero width is skipped"),
            ((800, 0), false, None, "zero height is skipped"),
            ((0, 0), true, None, "zero stays skipped even when forced"),
            ((1024, 768), false, Some((1024, 768)), "a new size reconfigures"),
            ((800, 600), true, Some((800, 600)), "force reconfigures at the same size"),
        ];

        for (requested, force, expected, why) in cases {
            assert_eq!(resize_action(current, requested, force), expected, "{why}");
        }
    }
}
