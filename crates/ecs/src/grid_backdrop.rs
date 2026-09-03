//! `GridBackdrop` — the playfield spring grid as scene data.
//!
//! The component carries the grid's *configuration*: topology, dimensions,
//! color and the simulation tunables. The engine owns the simulated mesh
//! (`engine_core::grid::GridBackdropSystem`) and rebuilds it whenever this
//! data changes, so a grid is authorable per scene and editable in the
//! inspector like any other component. Placement comes from the entity's
//! `Transform2D.position` (the grid's center), so the Move gizmo places it.
//!
//! `Default` is the arcade playfield preset every game used to build by
//! hand — a 44×19 honeycomb with 30px sides, tinted the Normal-theme grid
//! color (an engine test pins that color to `ChaosTheme`).

use glam::Vec4;
use serde::{Deserialize, Serialize};

use crate::component_registry::ComponentMeta;
use crate::DeriveComponentMeta;

/// Lattice shape of a [`GridBackdrop`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GridTopology {
    /// Honeycomb (the Geometry-Wars look). Needs an even column count; the
    /// engine rounds odd counts up.
    #[default]
    Hex,
    /// Classic square lattice.
    Square,
}

impl GridTopology {
    /// Every topology, in cycle order (used by editor cycle selectors).
    pub const ALL: [GridTopology; 2] = [GridTopology::Hex, GridTopology::Square];

    /// Human-readable name for the inspector.
    pub fn label(self) -> &'static str {
        match self {
            GridTopology::Hex => "Hex",
            GridTopology::Square => "Square",
        }
    }

    /// Position in [`GridTopology::ALL`].
    pub fn index(self) -> usize {
        match self {
            GridTopology::Hex => 0,
            GridTopology::Square => 1,
        }
    }
}

/// Configuration of an engine-simulated spring grid drawn beneath the
/// game's own lines. See the module docs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DeriveComponentMeta)]
pub struct GridBackdrop {
    /// Lattice shape.
    pub topology: GridTopology,
    /// Node columns (≥ 2; hex grids need an even count).
    pub cols: u32,
    /// Node rows (≥ 2).
    pub rows: u32,
    /// Hexagon side / lattice pitch in world units.
    pub spacing: f32,
    /// Line color; `.w` is the alpha at full activity.
    pub color: Vec4,
    /// HDR emissive strength (the grid blooms).
    pub emissive: f32,
    /// Hidden grids still simulate, they just draw nothing.
    pub visible: bool,
    /// Spring stiffness.
    pub stiffness: f32,
    /// Velocity damping per step.
    pub damping: f32,
    /// Pull back towards each node's rest position.
    pub rest_pull: f32,
    /// Alpha fraction of `color.w` while the grid rests (0..=1).
    pub rest_alpha_fraction: f32,
    /// Seconds for the activity envelope to rise.
    pub activity_attack: f32,
    /// Seconds for the activity envelope to fall.
    pub activity_release: f32,
    /// Displacement that counts as "fully active", world units.
    pub activity_displacement_ref: f32,
    /// Velocity that counts as "fully active", world units per second.
    pub activity_velocity_ref: f32,
}

impl GridBackdrop {
    /// Largest node count per axis — a 512×512 grid is already 262k nodes;
    /// anything bigger is a typo, not a backdrop.
    pub const MAX_DIMENSION: u32 = 512;
    /// Smallest node pitch; zero would collapse every activity reference.
    pub const MIN_SPACING: f32 = 0.01;

    /// The configuration a grid is actually built from: dimensions clamped
    /// to `2..=MAX_DIMENSION` (odd hex column counts rounded up), spacing
    /// floored at `MIN_SPACING`, every non-finite tunable replaced by the
    /// preset's, and every physical coefficient kept non-negative (a
    /// negative stiffness or rest pull inverts the springs and the grid
    /// explodes to NaN within frames; damping and the rest alpha are
    /// fractions, `0..=1`). Pure; the engine compares it frame to frame (so a NaN in a
    /// scene file cannot read as "changed" forever — `NaN != NaN`) and the
    /// inspector snaps typed values through it, so what you see is what
    /// builds.
    pub fn normalized(&self) -> GridBackdrop {
        let preset = GridBackdrop::default();
        let finite_or = |value: f32, fallback: f32| if value.is_finite() { value } else { fallback };
        let non_negative = |value: f32, fallback: f32| finite_or(value, fallback).max(0.0);
        let fraction = |value: f32, fallback: f32| finite_or(value, fallback).clamp(0.0, 1.0);
        GridBackdrop {
            topology: self.topology,
            cols: Self::normalized_cols(self.cols, self.topology),
            rows: self.rows.clamp(2, Self::MAX_DIMENSION),
            spacing: finite_or(self.spacing, preset.spacing).max(Self::MIN_SPACING),
            color: if self.color.is_finite() { self.color } else { preset.color },
            emissive: non_negative(self.emissive, preset.emissive),
            visible: self.visible,
            stiffness: non_negative(self.stiffness, preset.stiffness),
            damping: fraction(self.damping, preset.damping),
            rest_pull: non_negative(self.rest_pull, preset.rest_pull),
            rest_alpha_fraction: fraction(self.rest_alpha_fraction, preset.rest_alpha_fraction),
            activity_attack: non_negative(self.activity_attack, preset.activity_attack),
            activity_release: non_negative(self.activity_release, preset.activity_release),
            activity_displacement_ref: non_negative(
                self.activity_displacement_ref,
                preset.activity_displacement_ref,
            ),
            activity_velocity_ref: non_negative(self.activity_velocity_ref, preset.activity_velocity_ref),
        }
    }

    /// True when `other` builds the same lattice (topology, dimensions,
    /// spacing) — every other field can be applied to a live mesh without
    /// resetting its simulation.
    pub fn same_shape(&self, other: &GridBackdrop) -> bool {
        self.topology == other.topology
            && self.cols == other.cols
            && self.rows == other.rows
            && self.spacing == other.spacing
    }

    /// Column count as built: clamped, and even for a honeycomb.
    pub fn normalized_cols(cols: u32, topology: GridTopology) -> u32 {
        let cols = cols.clamp(2, Self::MAX_DIMENSION);
        if topology == GridTopology::Hex && !cols.is_multiple_of(2) {
            // MAX_DIMENSION is even, so rounding up never leaves the range.
            cols + 1
        } else {
            cols
        }
    }
}

impl Default for GridBackdrop {
    fn default() -> Self {
        Self {
            topology: GridTopology::Hex,
            cols: 44,
            rows: 19,
            spacing: 30.0,
            color: Vec4::new(0.15, 0.3, 0.7, 0.5),
            emissive: 0.7,
            visible: true,
            stiffness: 60.0,
            damping: 0.07,
            rest_pull: 4.0,
            rest_alpha_fraction: 0.35,
            activity_attack: 0.04,
            activity_release: 0.6,
            activity_displacement_ref: 6.0,
            activity_velocity_ref: 60.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topology_cycle_order_round_trips_through_index() {
        // The editor's cycle row steps through ALL by index.
        for (index, topology) in GridTopology::ALL.iter().enumerate() {
            assert_eq!(topology.index(), index);
            assert_eq!(GridTopology::ALL[topology.index()], *topology);
        }
    }
}
