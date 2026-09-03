//! Pure scissor-rect math for viewport and per-batch UI clipping.
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
fn intersect_scissor(a: [u32; 4], b: [u32; 4]) -> [u32; 4] {
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

/// A pass-level scissor decision: `Fullscreen` = no scissor call, `Rect` = set it,
/// `Empty` = nothing visible (clear-only passes still run, draws are skipped).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassScissor {
    Fullscreen,
    Rect([u32; 4]),
    Empty,
}

impl PassScissor {
    /// Resolve a requested scissor against the surface dimensions.
    pub fn resolve(request: Option<[u32; 4]>, surface: (u32, u32)) -> Self {
        match request {
            None => Self::Fullscreen,
            Some(rect) => match clamp_scissor(rect, surface.0, surface.1) {
                Some((x, y, w, h)) => Self::Rect([x, y, w, h]),
                None => Self::Empty,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A UI clip rect arrives in float pixels; the scissor must cover every
    /// pixel the rect touches (outward rounding), start at the surface
    /// origin (negative origins clamp, keeping the far edge), and never
    /// carry a non-finite value into `set_scissor_rect`.
    #[test]
    fn test_quantize_rounds_outward_clamps_the_origin_and_empties_non_finite() {
        let cases = [
            ((10.3, 20.7, 79.4, 49.0), [10, 20, 80, 50], "10.3..89.7 covers pixels 10..90"),
            ((5.0, 6.0, 100.0, 200.0), [5, 6, 100, 200], "exact integers pass through"),
            ((-10.0, -5.0, 40.0, 25.0), [0, 0, 30, 20], "negative origin clamps, far edge kept"),
            ((-50.0, -50.0, 20.0, 20.0), [0, 0, 0, 0], "a fully negative rect is empty"),
            ((10.0, 10.0, 0.0, 0.0), [10, 10, 0, 0], "zero size stays zero size"),
            ((f32::NAN, 0.0, 10.0, 10.0), [0, 0, 0, 0], "a NaN origin is empty"),
            ((0.0, 0.0, f32::INFINITY, 10.0), [0, 0, 0, 0], "an infinite width is empty"),
            ((0.0, f32::NEG_INFINITY, 10.0, 10.0), [0, 0, 0, 0], "a -inf origin is empty"),
        ];

        for ((x, y, w, h), expected, why) in cases {
            assert_eq!(quantize_rect(x, y, w, h), expected, "{why}");
        }
    }

    /// wgpu validates scissor ⊆ attachment. A rect computed from last
    /// frame's larger window must clamp to the live surface, and an empty
    /// intersection comes back `None` so the caller skips the draw.
    #[test]
    fn test_clamp_trims_to_the_live_surface_and_empties_to_none() {
        let cases = [
            ([10, 10, 50, 50], (100, 100), Some((10, 10, 50, 50)), "inside stays"),
            ([0, 0, 100, 80], (100, 80), Some((0, 0, 100, 80)), "an exact fit survives"),
            ([50, 50, 100, 100], (100, 80), Some((50, 50, 50, 30)), "overhang trimmed after a shrink"),
            ([200, 0, 50, 50], (100, 100), None, "past the right edge"),
            ([0, 300, 50, 50], (100, 100), None, "past the bottom edge"),
            ([10, 10, 0, 0], (100, 100), None, "zero size draws nothing"),
        ];

        for (rect, (surface_w, surface_h), expected, why) in cases {
            assert_eq!(clamp_scissor(rect, surface_w, surface_h), expected, "{why}");
        }
    }

    /// The per-batch decision: no clip and no default = the full surface;
    /// the pass default (the editor's scene panel) applies to unclipped game
    /// batches; a UI clip intersects it; an empty result skips the draw.
    #[test]
    fn test_batch_scissor_intersects_clip_with_default_and_skips_empty() {
        let surface = (640, 480);
        let cases = [
            (None, None, Some((0, 0, 640, 480)), "no clip, no default: the full surface"),
            (None, Some([100, 50, 200, 150]), Some((100, 50, 200, 150)), "the default applies to unclipped batches"),
            (Some([0, 0, 50, 50]), Some([25, 25, 50, 50]), Some((25, 25, 25, 25)), "overlapping: the overlap"),
            (Some([0, 0, 150, 150]), Some([100, 100, 200, 200]), Some((100, 100, 50, 50)), "clip ∩ default"),
            (Some([0, 0, 100, 100]), Some([10, 20, 30, 40]), Some((10, 20, 30, 40)), "nested: the inner rect"),
            (Some([100, 100, 1000, 1000]), None, Some((100, 100, 540, 380)), "a clip alone still clamps to the surface"),
            (Some([0, 0, 10, 10]), Some([500, 400, 50, 50]), None, "a disjoint clip and default draw nothing"),
            (None, Some([0, 0, 0, 0]), None, "a zero-size default (scene panel hidden) draws nothing"),
        ];

        for (clip, default, expected, why) in cases {
            assert_eq!(batch_scissor(clip, default, surface), expected, "{why}");
        }
        assert_eq!(batch_scissor(None, None, (0, 0)), None, "a zero surface draws nothing");
    }

    #[test]
    fn test_pass_scissor_resolve_resolves_all_variants() {
        let surface = (800, 600);
        assert_eq!(PassScissor::resolve(None, surface), PassScissor::Fullscreen);
        assert_eq!(
            PassScissor::resolve(Some([10, 20, 100, 200]), surface),
            PassScissor::Rect([10, 20, 100, 200])
        );
        assert_eq!(
            PassScissor::resolve(Some([900, 700, 10, 10]), surface),
            PassScissor::Empty
        );
    }
}
