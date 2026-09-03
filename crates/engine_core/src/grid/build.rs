//! Build a [`GridMesh`] from a scene-authored [`GridBackdrop`].
//!
//! Scene data is user-edited, so instead of the constructors' asserts this
//! path builds from [`GridBackdrop::normalized`] — dimensions clamped, odd
//! hex column counts rounded up, non-finite tunables replaced. The rule
//! lives with the data in `ecs` (the inspector snaps through the same one);
//! this module is where the engine's expectations of it are tested.

use ecs::GridBackdrop;
use glam::Vec2;

use super::GridMesh;

/// Apply a (normalized) config's tunables to a live mesh without touching
/// its lattice or simulation state — a color, visibility or stiffness edit
/// must not snap an active ripple to rest.
pub fn apply_grid_tunables(mesh: &mut GridMesh, config: &GridBackdrop) {
    debug_assert!(mesh.config.same_shape(config), "tunable apply must not change the lattice");
    mesh.config = config.clone();
}

/// Build the mesh a [`GridBackdrop`] describes, centered at `origin`.
/// Corrections made by [`GridBackdrop::normalized`] are logged once, here —
/// never per frame.
pub fn build_grid_mesh(config: &GridBackdrop, origin: Vec2) -> GridMesh {
    let normalized = config.normalized();
    if normalized != *config {
        log::warn!(
            "GridBackdrop normalized before building: {}x{} @ {} -> {}x{} @ {}",
            config.cols, config.rows, config.spacing,
            normalized.cols, normalized.rows, normalized.spacing
        );
    }
    GridMesh::from_config(&normalized, origin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecs::GridTopology;
    use crate::chaos_mode::ChaosMode;
    use crate::chaos_theme::ChaosTheme;

    #[test]
    fn test_scene_tunables_are_normalized_before_the_mesh_is_built() {
        // Scene data is user-edited: odd hex column counts round up (the
        // square lattice keeps them), dimensions and spacing clamp to the
        // sane range, non-finite values fall back to the preset, and
        // negative coefficients clamp to zero instead of inverting the
        // springs (a negative stiffness pushes every node AWAY
        // from rest and the grid explodes to NaN within frames).
        let hex = GridBackdrop { cols: 45, ..GridBackdrop::default() };
        assert_eq!(build_grid_mesh(&hex, Vec2::ZERO).config.cols, 46, "odd hex columns round up, no panic");
        let square = GridBackdrop { topology: GridTopology::Square, cols: 45, ..GridBackdrop::default() };
        assert_eq!(build_grid_mesh(&square, Vec2::ZERO).config.cols, 45);

        let tiny = GridBackdrop { cols: 1, rows: 0, spacing: 0.0, ..GridBackdrop::default() }.normalized();
        assert_eq!((tiny.cols, tiny.rows, tiny.spacing), (2, 2, GridBackdrop::MIN_SPACING));
        let huge = GridBackdrop { cols: 10_000, rows: 10_000, ..GridBackdrop::default() }.normalized();
        assert_eq!((huge.cols, huge.rows), (GridBackdrop::MAX_DIMENSION, GridBackdrop::MAX_DIMENSION));

        let broken = GridBackdrop { stiffness: f32::NAN, spacing: f32::INFINITY, ..GridBackdrop::default() };
        let normalized = broken.normalized();
        assert_eq!(normalized.stiffness, GridBackdrop::default().stiffness);
        assert_eq!(normalized.spacing, GridBackdrop::default().spacing);
        // A NaN scene value must not read as "changed" every frame.
        assert_eq!(broken.normalized(), broken.normalized());

        let inverted = GridBackdrop { stiffness: -60.0, rest_pull: -4.0, damping: -1.0, ..GridBackdrop::default() };
        let normalized = inverted.normalized();
        assert_eq!((normalized.stiffness, normalized.rest_pull, normalized.damping), (0.0, 0.0, 0.0));
        let mut mesh = build_grid_mesh(&inverted, Vec2::ZERO);
        mesh.apply_impulse(&super::super::GridImpulse::Radial {
            position: Vec2::ZERO, strength: 500.0, radius: 20.0, attractive: false,
        });
        for _ in 0..120 {
            mesh.step(1.0 / 60.0);
        }
        assert!(mesh.total_energy().is_finite(), "no explosion");
        assert!(mesh.build_line_vertices().iter().all(|v| v.position.iter().all(|c| c.is_finite())));
    }

    #[test]
    fn test_default_backdrop_is_the_playfield_preset_in_every_theme() {
        // A scene carrying a bare `GridBackdrop()` must get the exact grid
        // the arcade games build from `default_playfield_grid`, tinted to
        // the chaos theme — ecs cannot see the theme, so the color is pinned HERE.
        for mode in ChaosMode::ALL {
            let theme = ChaosTheme::for_mode(mode);
            let preset = super::super::default_playfield_grid(&theme);
            assert_eq!(preset.config.color, theme.grid_color, "grid tint must follow {mode:?}");
        }
        let theme = ChaosTheme::for_mode(ChaosMode::Normal);
        let preset = super::super::default_playfield_grid(&theme);
        let built = build_grid_mesh(&GridBackdrop::default(), Vec2::ZERO);
        assert_eq!(built.node_count(), preset.node_count());
        assert_eq!(built.spring_count(), preset.spring_count());
        assert_eq!(built.config.stiffness, preset.config.stiffness);
        assert_eq!(built.config.damping, preset.config.damping);
        assert_eq!(built.config.emissive, preset.config.emissive);
        assert_eq!(built.config.rest_pull, preset.config.rest_pull);
        assert_eq!(built.config.activity_displacement_ref, preset.config.activity_displacement_ref);
        assert_eq!(built.config.activity_velocity_ref, preset.config.activity_velocity_ref);
        assert_eq!(GridBackdrop::default().color, theme.grid_color);
    }
}
