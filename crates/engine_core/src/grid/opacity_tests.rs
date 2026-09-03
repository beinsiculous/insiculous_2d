//! Motion-driven opacity behavior tests for [`GridMesh`] (hoisted to a
//! sibling file for size — these exercise only the public API; tests
//! that reach into private node state stay in `grid_mesh.rs`).

use super::{GridImpulse, GridMesh};
use ecs::{GridBackdrop, GridTopology};
use glam::Vec2;

fn kick(g: &mut GridMesh, strength: f32, radius: f32) {
    g.apply_impulse(&GridImpulse::Radial { position: Vec2::ZERO, strength, radius, attractive: false });
}

/// Largest vertex alpha currently emitted.
fn max_alpha(g: &mut GridMesh) -> f32 {
    g.build_line_vertices().iter().map(|v| v.color[3]).fold(0.0, f32::max)
}

#[test]
fn resting_grid_is_more_transparent_than_moving_grid_and_settles_back_on_both_lattices() {
    for (name, mut g) in [
        (
            "hex",
            GridMesh::from_config(
                &GridBackdrop { cols: 6, rows: 5, spacing: 10.0, damping: 0.2, ..Default::default() },
                Vec2::ZERO,
            ),
        ),
        (
            "square",
            GridMesh::from_config(
                &GridBackdrop {
                    topology: GridTopology::Square,
                    cols: 5,
                    rows: 4,
                    spacing: 10.0,
                    damping: 0.2,
                    ..Default::default()
                },
                Vec2::ZERO,
            ),
        ),
    ] {
        let expected_rest = g.config.color.w * g.config.rest_alpha_fraction;
        g.step(0.016);
        let rest_alpha = max_alpha(&mut g);
        assert!((rest_alpha - expected_rest).abs() < 1e-4, "{name}: settled grid sits at rest alpha, got {rest_alpha}");

        kick(&mut g, 500.0, 20.0);
        g.step(0.016);
        let moving_alpha = max_alpha(&mut g);
        assert!(moving_alpha > rest_alpha + 0.05, "{name}: disturbed grid brightens: {rest_alpha} -> {moving_alpha}");

        // ~10 seconds — motion dies, then the envelope releases back to rest.
        for _ in 0..600 {
            g.step(0.016);
        }
        let settled = max_alpha(&mut g);
        assert!((settled - expected_rest).abs() < 0.01, "{name}: settled alpha {settled} returns to {expected_rest}");
    }
}

#[test]
fn activity_alpha_never_exceeds_color_alpha_lingers_after_the_rise_and_fraction_one_is_uniform() {
    // `color.w` is the MAXIMUM: even a violent impulse cannot push a vertex past it.
    let mut g = GridMesh::from_config(
        &GridBackdrop { cols: 6, rows: 5, spacing: 10.0, ..Default::default() },
        Vec2::ZERO,
    );
    kick(&mut g, 5000.0, 100.0);
    for _ in 0..5 {
        g.step(0.016);
    }
    assert!(max_alpha(&mut g) <= g.config.color.w + 1e-6, "activity must never push alpha past color.w");

    // Attack is fast, release is slow: one excited frame brightens far more
    // than one late frame dims, so the glow lingers.
    let mut g = GridMesh::from_config(
        &GridBackdrop { cols: 6, rows: 5, spacing: 10.0, damping: 0.2, ..Default::default() },
        Vec2::ZERO,
    );
    let rest = g.config.color.w * g.config.rest_alpha_fraction;
    kick(&mut g, 500.0, 20.0);
    g.step(0.016);
    let rise = max_alpha(&mut g) - rest;
    assert!(rise > 0.1, "one excited frame should brighten noticeably");
    for _ in 0..100 {
        g.step(0.016);
    }
    let before = max_alpha(&mut g);
    g.step(0.016);
    let per_frame_decay = before - max_alpha(&mut g);
    assert!(per_frame_decay < rise * 0.2, "glow should linger: rise {rise} vs late decay {per_frame_decay}");

    // The legacy look: rest_alpha_fraction 1.0 keeps every vertex at color.w.
    let mut g = GridMesh::from_config(
        &GridBackdrop { cols: 6, rows: 5, spacing: 10.0, rest_alpha_fraction: 1.0, ..Default::default() },
        Vec2::ZERO,
    );
    kick(&mut g, 500.0, 20.0);
    g.step(0.016);
    let max_a = g.config.color.w;
    for v in g.build_line_vertices() {
        assert_eq!(v.color[3], max_a, "legacy look: every vertex at color.w");
    }
}
