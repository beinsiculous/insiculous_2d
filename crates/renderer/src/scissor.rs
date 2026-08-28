//! Pure scissor-rect math for viewport and per-batch UI clipping (issue #41).
//!
//! All functions are CPU-only and headless-testable. Rects are
//! `[x, y, width, height]` in physical surface pixels, matching
//! `wgpu::RenderPass::set_scissor_rect`.

/// Quantize a float rect to integer pixels, rounding outward so a partially
/// covered pixel is kept rather than clipped away.
///
/// Negative origins clamp to 0 (the scissor space starts at the surface
/// origin). Any non-finite input yields the empty rect `[0, 0, 0, 0]`.
pub fn quantize_rect(x: f32, y: f32, w: f32, h: f32) -> [u32; 4] {
    if !(x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite()) {
        return [0, 0, 0, 0];
    }
    let x0 = x.floor().max(0.0);
    let y0 = y.floor().max(0.0);
    let x1 = (x + w).ceil().max(x0);
    let y1 = (y + h).ceil().max(y0);
    [x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32]
}

/// Clamp a rect to the surface. Returns `None` when the intersection is
/// empty — wgpu validates scissor ⊆ attachment, and an empty scissor means
/// the caller should skip the draw entirely.
pub fn clamp_scissor(
    rect: [u32; 4],
    surface_w: u32,
    surface_h: u32,
) -> Option<(u32, u32, u32, u32)> {
    let x0 = rect[0].min(surface_w);
    let y0 = rect[1].min(surface_h);
    let x1 = rect[0].saturating_add(rect[2]).min(surface_w);
    let y1 = rect[1].saturating_add(rect[3]).min(surface_h);
    let (w, h) = (x1 - x0, y1 - y0);
    if w == 0 || h == 0 {
        None
    } else {
        Some((x0, y0, w, h))
    }
}

/// Intersection of two rects (empty result has zero width and/or height).
pub fn intersect_scissor(a: [u32; 4], b: [u32; 4]) -> [u32; 4] {
    let x0 = a[0].max(b[0]);
    let y0 = a[1].max(b[1]);
    let x1 = a[0].saturating_add(a[2]).min(b[0].saturating_add(b[2]));
    let y1 = a[1].saturating_add(a[3]).min(b[1].saturating_add(b[3]));
    [x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0)]
}

/// Decide the scissor for one batch: the batch's own clip intersected with
/// the pass default, clamped to the surface. `None` = draw nothing.
///
/// A batch with no clip under a pass with no default gets the full surface
/// (equivalent to no scissor, but keeps `set_scissor_rect` state tracking
/// uniform across batches).
pub fn batch_scissor(
    batch_clip: Option<[u32; 4]>,
    default_scissor: Option<[u32; 4]>,
    surface: (u32, u32),
) -> Option<(u32, u32, u32, u32)> {
    let rect = match (batch_clip, default_scissor) {
        (None, None) => [0, 0, surface.0, surface.1],
        (Some(c), None) => c,
        (None, Some(d)) => d,
        (Some(c), Some(d)) => intersect_scissor(c, d),
    };
    clamp_scissor(rect, surface.0, surface.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantize_rounds_outward_to_cover_partial_pixels() {
        // 10.3..89.7 must cover pixels 10..90.
        assert_eq!(quantize_rect(10.3, 20.7, 79.4, 49.0), [10, 20, 80, 50]);
    }

    #[test]
    fn test_quantize_exact_integers_pass_through() {
        assert_eq!(quantize_rect(5.0, 6.0, 100.0, 200.0), [5, 6, 100, 200]);
    }

    #[test]
    fn test_quantize_negative_origin_clamps_to_zero_keeping_far_edge() {
        // x -10..+30 → scissor 0..30.
        assert_eq!(quantize_rect(-10.0, -5.0, 40.0, 25.0), [0, 0, 30, 20]);
    }

    #[test]
    fn test_quantize_fully_negative_rect_is_empty() {
        assert_eq!(quantize_rect(-50.0, -50.0, 20.0, 20.0), [0, 0, 0, 0]);
    }

    #[test]
    fn test_quantize_non_finite_inputs_yield_empty() {
        assert_eq!(quantize_rect(f32::NAN, 0.0, 10.0, 10.0), [0, 0, 0, 0]);
        assert_eq!(quantize_rect(0.0, 0.0, f32::INFINITY, 10.0), [0, 0, 0, 0]);
        assert_eq!(quantize_rect(0.0, f32::NEG_INFINITY, 10.0, 10.0), [0, 0, 0, 0]);
    }

    #[test]
    fn test_quantize_zero_size_is_empty() {
        assert_eq!(quantize_rect(10.0, 10.0, 0.0, 0.0), [10, 10, 0, 0]);
        assert_eq!(clamp_scissor(quantize_rect(10.0, 10.0, 0.0, 0.0), 100, 100), None);
    }

    #[test]
    fn test_clamp_keeps_rect_inside_surface() {
        assert_eq!(clamp_scissor([10, 10, 50, 50], 100, 100), Some((10, 10, 50, 50)));
    }

    #[test]
    fn test_clamp_trims_overhang_on_resize_race() {
        // Rect computed from last frame's larger window must clamp to the
        // live surface, never submit out-of-bounds (wgpu validation error).
        assert_eq!(clamp_scissor([50, 50, 100, 100], 100, 80), Some((50, 50, 50, 30)));
    }

    #[test]
    fn test_clamp_fully_outside_is_none() {
        assert_eq!(clamp_scissor([200, 0, 50, 50], 100, 100), None);
        assert_eq!(clamp_scissor([0, 300, 50, 50], 100, 100), None);
    }

    #[test]
    fn test_clamp_exact_fit_survives() {
        assert_eq!(clamp_scissor([0, 0, 100, 80], 100, 80), Some((0, 0, 100, 80)));
    }

    #[test]
    fn test_intersect_overlapping_rects() {
        assert_eq!(intersect_scissor([0, 0, 50, 50], [25, 25, 50, 50]), [25, 25, 25, 25]);
    }

    #[test]
    fn test_intersect_disjoint_rects_is_empty() {
        let r = intersect_scissor([0, 0, 10, 10], [20, 20, 10, 10]);
        assert_eq!(r[2], 0);
        assert_eq!(r[3], 0);
    }

    #[test]
    fn test_intersect_nested_rect_returns_inner() {
        assert_eq!(intersect_scissor([0, 0, 100, 100], [10, 20, 30, 40]), [10, 20, 30, 40]);
    }

    #[test]
    fn test_batch_scissor_unclipped_batch_no_default_gets_full_surface() {
        assert_eq!(batch_scissor(None, None, (640, 480)), Some((0, 0, 640, 480)));
    }

    #[test]
    fn test_batch_scissor_default_applies_to_unclipped_batches() {
        // The sprite pass: game batches carry no clip, the viewport scissor
        // is the pass default.
        assert_eq!(
            batch_scissor(None, Some([100, 50, 200, 150]), (640, 480)),
            Some((100, 50, 200, 150))
        );
    }

    #[test]
    fn test_batch_scissor_clip_intersects_default() {
        assert_eq!(
            batch_scissor(Some([0, 0, 150, 150]), Some([100, 100, 200, 200]), (640, 480)),
            Some((100, 100, 50, 50))
        );
    }

    #[test]
    fn test_batch_scissor_empty_result_skips_draw() {
        // Zero-size default = the "scene panel hidden" case: nothing draws.
        assert_eq!(batch_scissor(None, Some([0, 0, 0, 0]), (640, 480)), None);
        // Clip disjoint from default: nothing draws.
        assert_eq!(
            batch_scissor(Some([0, 0, 10, 10]), Some([500, 400, 50, 50]), (640, 480)),
            None
        );
    }

    #[test]
    fn test_batch_scissor_zero_surface_is_none() {
        assert_eq!(batch_scissor(None, None, (0, 0)), None);
    }
}
