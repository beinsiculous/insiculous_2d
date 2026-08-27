//! Motion-driven opacity behavior tests for [`GridMesh`] (hoisted to a
//! sibling file for size — these exercise only the public API; tests
//! that reach into private node state stay in `grid_mesh.rs`).

use super::{GridImpulse, GridMesh};
use glam::Vec2;

    #[test]
    fn square_grid_allows_odd_columns_and_shares_motion_opacity() {
        // The square variant keeps the old lattice contract (odd cols legal)
        // and runs the same activity-driven alpha as the honeycomb.
        let mut g = GridMesh::new_square(5, 4, 10.0, Vec2::ZERO);
        assert_eq!(g.node_count(), 20);
        // Springs: (cols-1)*rows + cols*(rows-1) = 4*4 + 5*3 = 31.
        assert_eq!(g.spring_count(), 31);

        g.step(0.016);
        let rest_alpha = max_alpha(&mut g);
        g.apply_impulse(&GridImpulse::Radial {
            position: Vec2::ZERO,
            strength: 500.0,
            radius: 20.0,
            attractive: false,
        });
        g.step(0.016);
        assert!(max_alpha(&mut g) > rest_alpha + 0.05, "square grid must brighten too");
    }

    /// Largest vertex alpha currently emitted (helper for opacity tests).
    fn max_alpha(g: &mut GridMesh) -> f32 {
        g.build_line_vertices()
            .iter()
            .map(|v| v.color[3])
            .fold(0.0, f32::max)
    }

    #[test]
    fn resting_grid_is_more_transparent_than_moving_grid() {
        let mut g = GridMesh::new(6, 5, 10.0, Vec2::ZERO);
        g.step(0.016);
        let rest_alpha = max_alpha(&mut g);
        let expected_rest = g.color.w * g.rest_alpha_fraction;
        assert!(
            (rest_alpha - expected_rest).abs() < 1e-4,
            "settled grid should sit at rest alpha, got {rest_alpha}"
        );

        g.apply_impulse(&GridImpulse::Radial {
            position: Vec2::ZERO,
            strength: 500.0,
            radius: 20.0,
            attractive: false,
        });
        g.step(0.016);
        let moving_alpha = max_alpha(&mut g);
        assert!(
            moving_alpha > rest_alpha + 0.05,
            "disturbed grid should brighten: {rest_alpha} -> {moving_alpha}"
        );
    }

    #[test]
    fn grid_opacity_returns_to_rest_level_after_settling() {
        let mut g = GridMesh::new(6, 5, 10.0, Vec2::ZERO).with_damping(0.2);
        g.apply_impulse(&GridImpulse::Radial {
            position: Vec2::ZERO,
            strength: 500.0,
            radius: 20.0,
            attractive: false,
        });
        for _ in 0..600 { // ~10 seconds — motion dies, then the envelope releases
            g.step(0.016);
        }
        let alpha = max_alpha(&mut g);
        let expected_rest = g.color.w * g.rest_alpha_fraction;
        assert!(
            (alpha - expected_rest).abs() < 0.01,
            "settled alpha {alpha} should return to rest level {expected_rest}"
        );
    }

    #[test]
    fn rest_alpha_fraction_one_keeps_uniform_alpha() {
        let mut g = GridMesh::new(6, 5, 10.0, Vec2::ZERO).with_rest_alpha_fraction(1.0);
        g.apply_impulse(&GridImpulse::Radial {
            position: Vec2::ZERO,
            strength: 500.0,
            radius: 20.0,
            attractive: false,
        });
        g.step(0.016);
        let max_a = g.color.w;
        for v in g.build_line_vertices() {
            assert_eq!(v.color[3], max_a, "legacy look: every vertex at color.w");
        }
    }

    #[test]
    fn moving_grid_alpha_never_exceeds_color_alpha() {
        let mut g = GridMesh::new(6, 5, 10.0, Vec2::ZERO);
        g.apply_impulse(&GridImpulse::Radial {
            position: Vec2::ZERO,
            strength: 5000.0,
            radius: 100.0,
            attractive: false,
        });
        for _ in 0..5 {
            g.step(0.016);
        }
        let max_a = g.color.w;
        assert!(
            max_alpha(&mut g) <= max_a + 1e-6,
            "activity must never push alpha past color.w"
        );
    }

    #[test]
    fn activity_envelope_decays_slower_than_it_rises() {
        let mut g = GridMesh::new(6, 5, 10.0, Vec2::ZERO).with_damping(0.2);
        let rest = g.color.w * g.rest_alpha_fraction;
        g.apply_impulse(&GridImpulse::Radial {
            position: Vec2::ZERO,
            strength: 500.0,
            radius: 20.0,
            attractive: false,
        });
        g.step(0.016);
        let rise = max_alpha(&mut g) - rest;
        assert!(rise > 0.1, "one excited frame should brighten noticeably");

        // Late in the settle the motion is gone and only the release drains
        // the envelope — one frame must shed far less than the attack gained.
        for _ in 0..100 {
            g.step(0.016);
        }
        let before = max_alpha(&mut g);
        g.step(0.016);
        let per_frame_decay = before - max_alpha(&mut g);
        assert!(
            per_frame_decay < rise * 0.2,
            "glow should linger: rise {rise} vs late decay {per_frame_decay}"
        );
    }
