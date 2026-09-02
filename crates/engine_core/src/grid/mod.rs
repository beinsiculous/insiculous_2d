//! Spring-mass grid — a general-purpose deformable grid effect.
//!
//! A honeycomb of mass-points connected by springs, simulated on the CPU
//! and rendered as glowing lines. Apply impulses with
//! [`GridMesh::apply_impulse`] to deform the grid — gameplay events (hits,
//! explosions, pickups) ripple outward through the hexagons. The grid rests
//! translucent and brightens where it moves (per-node activity envelope →
//! per-vertex alpha), so it reads as a subtle reactive backdrop rather than
//! a constant neon lattice. This is an engine-level effect, usable by any
//! game as a reactive background.
//!
//! The `Default` field values (stiffness, damping, color, glow, activity
//! response) are starting points, not requirements — configure per game via
//! the public fields or [`GridMesh::new`]. Games that prefer the classic
//! square lattice build with [`GridMesh::new_square`] instead — same
//! simulation and motion-driven opacity, different lattice shape.
//!
//! Render path: each frame, build the line vertex buffer with
//! [`GridMesh::build_line_vertices`] and hand it to
//! [`RenderManager::set_lines`](crate::render_manager::RenderManager::set_lines).
//! The line pipeline writes into the HDR target so the grid blooms.

mod backdrop_system;
mod build;
mod grid_mesh;
mod impulse;
#[cfg(test)]
mod opacity_tests;
mod topology;

pub use backdrop_system::{
    request_backdrop_reset, ripple, GridBackdropReset, GridBackdropSystem, GridImpulses,
};
pub use build::{apply_grid_tunables, build_grid_mesh};
pub use grid_mesh::GridMesh;
pub use impulse::GridImpulse;

/// The shared playfield backdrop: a 44×19-node honeycomb with 30px hexagon
/// sides (≈1117×825px), tinted to the chaos theme, sized to cover an
/// ~800×600 window with overscan. Every arcade game uses this exact preset;
/// it IS `ecs::GridBackdrop::default()` with the theme's color — a scene
/// carrying `GridBackdrop()` gets the same grid from the engine (#46).
pub fn default_playfield_grid(theme: &crate::chaos_theme::ChaosTheme) -> GridMesh {
    let config = ecs::GridBackdrop { color: theme.grid_color, ..ecs::GridBackdrop::default() };
    build_grid_mesh(&config, glam::Vec2::ZERO)
}

/// Advance a spring-mass grid (if any) and push its line vertices into the
/// engine's per-frame line buffer; when `debug_colliders` is set, overlay
/// collider outlines in bright emissive magenta so they bloom above sprites.
///
/// The shared per-frame driver every game's render path calls:
/// `grid::step_and_emit_grid(self.grid.as_mut(), ctx.world, ctx.lines, ctx.delta_time, self.debug_colliders)`.
pub fn step_and_emit_grid(
    grid: Option<&mut GridMesh>,
    world: &ecs::World,
    lines: &mut Vec<renderer::line_pipeline::LineVertex>,
    delta_time: f32,
    debug_colliders: bool,
) {
    if let Some(grid) = grid {
        grid.step(delta_time);
        let verts = grid.build_line_vertices();
        lines.extend_from_slice(verts);
    }
    if debug_colliders {
        crate::debug::draw_colliders(world, lines, glam::Vec4::new(1.0, 0.2, 1.0, 0.9), 2.0);
    }
}

#[cfg(test)]
mod step_and_emit_tests {
    use super::*;

    #[test]
    fn test_step_and_emit_pushes_grid_vertices() {
        let mut grid = GridMesh::new(4, 4, 10.0, glam::Vec2::ZERO);
        let world = ecs::World::new();
        let mut lines = Vec::new();
        step_and_emit_grid(Some(&mut grid), &world, &mut lines, 1.0 / 60.0, false);
        assert!(!lines.is_empty(), "grid line vertices must land in the buffer");
    }

    #[test]
    fn test_step_and_emit_without_grid_or_colliders_is_a_noop() {
        let world = ecs::World::new();
        let mut lines = Vec::new();
        step_and_emit_grid(None, &world, &mut lines, 1.0 / 60.0, false);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_default_playfield_grid_adopts_theme_grid_color() {
        use crate::chaos_mode::ChaosMode;
        use crate::chaos_theme::ChaosTheme;

        for mode in ChaosMode::ALL {
            let theme = ChaosTheme::for_mode(mode);
            let grid = default_playfield_grid(&theme);
            assert_eq!(grid.color, theme.grid_color, "grid tint must follow {mode:?}");
        }
    }
}
