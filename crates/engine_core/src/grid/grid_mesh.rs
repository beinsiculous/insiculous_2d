//! The grid mesh itself: nodes, springs, and the semi-implicit Euler
//! integrator.
//!
//! Each node has a rest position; springs pull every node toward its
//! neighbors with the rest length set to the configured spacing (a
//! honeycomb lattice — see [`topology`](super::topology)). When an impulse
//! displaces a node, the connected springs propagate the displacement and
//! a ripple emerges naturally. The grid rests translucent and brightens
//! where it moves: each node carries a smoothed activity envelope that
//! drives its vertex alpha, so ripples glow and fade back as they settle.
//!
//! Border nodes are pinned (inv_mass = 0) so the grid doesn't drift.

use glam::{Vec2, Vec4};
use renderer::line_pipeline::LineVertex;

use super::impulse::GridImpulse;
use super::topology::{build_hex_topology, build_square_topology, GridNode, Spring};

/// Raw activity below this clamps to zero so a settling grid genuinely
/// reaches its rest alpha instead of shimmering on residual creep (the
/// honeycomb's floppy modes take a while to drain the last few percent).
const ACTIVITY_DEADBAND: f32 = 0.05;

/// The spring-mass grid.
///
/// Build with [`GridMesh::new`]. Each frame:
/// 1. Apply impulses with [`apply_impulse`](Self::apply_impulse).
/// 2. Step the simulation with [`step`](Self::step).
/// 3. Build line vertices with [`build_line_vertices`](Self::build_line_vertices).
pub struct GridMesh {
    pub cols: u32,
    pub rows: u32,
    pub spacing: f32,
    pub origin: Vec2,
    nodes: Vec<GridNode>,
    springs: Vec<Spring>,
    /// Per-node motion envelope in 0..=1 driving vertex alpha: rises fast
    /// while a node is displaced/moving, decays slowly as it settles.
    activity: Vec<f32>,
    /// Pre-allocated line vertex scratch reused every frame.
    line_scratch: Vec<LineVertex>,
    /// Pre-allocated spring-force scratch reused every substep.
    force_scratch: Vec<Vec2>,
    /// Visual params. The alpha component of `color` controls grid
    /// translucency — fade it to 0 to hide the grid while keeping the
    /// simulation running.
    pub color: Vec4,
    pub emissive: f32,
    /// When `false`, [`build_line_vertices`](Self::build_line_vertices)
    /// returns an empty slice and no lines render. The simulation still
    /// steps (so re-enabling later resumes a settled grid).
    pub visible: bool,
    /// Spring stiffness (force per unit of stretch). Higher = faster ripples,
    /// lower = wobblier.
    pub stiffness: f32,
    /// Per-frame velocity decay. 0.0 = no damping (energy preserved),
    /// 1.0 = critical damping (no movement). Typical 0.05–0.15.
    pub damping: f32,
    /// Pull-to-rest force coefficient. Without this the grid never returns
    /// to its rest position once excited. The honeycomb needs more of it
    /// than a square lattice would: degree-3 nodes have floppy bending
    /// modes that springs barely resist, so this is what brings those
    /// displacements home. Typical 2.0–8.0.
    pub rest_pull: f32,
    /// Number of physics substeps per frame. Higher = more stable at large
    /// dt or high stiffness.
    pub substeps: u32,
    /// Fraction of `color.w` shown while a node rests (0..=1). The grid sits
    /// at this translucency until motion brightens it toward `color.w`;
    /// `1.0` restores the legacy uniform-alpha look.
    pub rest_alpha_fraction: f32,
    /// Activity envelope rise time constant in seconds (smaller = the grid
    /// lights up faster when disturbed).
    pub activity_attack: f32,
    /// Activity envelope fall time constant in seconds (larger = the glow
    /// lingers longer after motion stops).
    pub activity_release: f32,
    /// Displacement from rest (world units) that counts as full activity.
    pub activity_displacement_ref: f32,
    /// Node speed (world units/sec) that counts as full activity.
    pub activity_velocity_ref: f32,
}

impl GridMesh {
    /// Build a `cols × rows` honeycomb grid with hexagon side length
    /// `spacing`, centered at `origin`. `cols` must be even (the zigzag
    /// vertical-spring rule needs it to stay regular to the right edge).
    /// For a classic square lattice use [`new_square`](Self::new_square).
    pub fn new(cols: u32, rows: u32, spacing: f32, origin: Vec2) -> Self {
        assert!(cols >= 2 && rows >= 2, "grid must be at least 2x2");
        assert!(cols.is_multiple_of(2), "hex grid needs an even column count");
        Self::from_topology(cols, rows, spacing, origin, build_hex_topology(cols, rows, spacing, origin))
    }

    /// Build a `cols × rows` classic square-lattice grid with `spacing`
    /// world units between nodes, centered at `origin`. Any `cols >= 2`
    /// works (no even requirement). Same simulation and motion-driven
    /// opacity as the honeycomb — only the lattice shape differs.
    pub fn new_square(cols: u32, rows: u32, spacing: f32, origin: Vec2) -> Self {
        assert!(cols >= 2 && rows >= 2, "grid must be at least 2x2");
        Self::from_topology(cols, rows, spacing, origin, build_square_topology(cols, rows, spacing, origin))
    }

    fn from_topology(
        cols: u32,
        rows: u32,
        spacing: f32,
        origin: Vec2,
        (nodes, springs): (Vec<GridNode>, Vec<Spring>),
    ) -> Self {
        let force_scratch = vec![Vec2::ZERO; nodes.len()];
        let activity = vec![0.0; nodes.len()];
        Self {
            cols,
            rows,
            spacing,
            origin,
            nodes,
            springs,
            activity,
            line_scratch: Vec::new(),
            force_scratch,
            color: Vec4::new(0.2, 0.5, 1.0, 0.8),
            emissive: 0.6,
            visible: true,
            stiffness: 24.0,
            damping: 0.08,
            rest_pull: 4.0,
            substeps: 4,
            rest_alpha_fraction: 0.35,
            activity_attack: 0.04,
            activity_release: 0.6,
            activity_displacement_ref: spacing * 0.2,
            activity_velocity_ref: spacing * 2.0,
        }
    }

    pub fn with_color(mut self, color: Vec4) -> Self { self.color = color; self }
    pub fn with_emissive(mut self, emissive: f32) -> Self { self.emissive = emissive; self }
    pub fn with_stiffness(mut self, stiffness: f32) -> Self { self.stiffness = stiffness; self }
    pub fn with_damping(mut self, damping: f32) -> Self { self.damping = damping; self }

    /// Builder for [`rest_pull`](Self::rest_pull). The default is 4.0
    /// (raised from the old square-lattice 1.0 — the honeycomb's floppy
    /// bending modes rely on this pull to settle).
    pub fn with_rest_pull(mut self, rest_pull: f32) -> Self { self.rest_pull = rest_pull; self }

    /// Builder for [`rest_alpha_fraction`](Self::rest_alpha_fraction),
    /// clamped to 0..=1.
    pub fn with_rest_alpha_fraction(mut self, fraction: f32) -> Self {
        self.rest_alpha_fraction = fraction.clamp(0.0, 1.0);
        self
    }

    /// Builder for the activity envelope time constants (seconds).
    pub fn with_activity_response(mut self, attack: f32, release: f32) -> Self {
        self.activity_attack = attack;
        self.activity_release = release;
        self
    }

    /// Builder for the activity normalization references: the displacement
    /// (world units) and speed (world units/sec) that count as full activity.
    pub fn with_activity_refs(mut self, displacement: f32, velocity: f32) -> Self {
        self.activity_displacement_ref = displacement;
        self.activity_velocity_ref = velocity;
        self
    }

    /// Replace just the alpha of the grid's color. Useful for fading the
    /// grid in/out without touching the RGB tint.
    pub fn with_alpha(mut self, alpha: f32) -> Self { self.color.w = alpha.clamp(0.0, 1.0); self }

    /// Same as [`with_alpha`](Self::with_alpha) but on `&mut self` for
    /// imperative fading from gameplay code.
    pub fn set_alpha(&mut self, alpha: f32) { self.color.w = alpha.clamp(0.0, 1.0); }

    /// Builder variant of the [`visible`](Self::visible) field. Set to false
    /// to start the grid hidden — useful for games that toggle it on certain
    /// events (boss fights, menus, etc.).
    pub fn with_visible(mut self, visible: bool) -> Self { self.visible = visible; self }

    /// Number of mass points.
    pub fn node_count(&self) -> usize { self.nodes.len() }
    /// Number of spring connections.
    pub fn spring_count(&self) -> usize { self.springs.len() }

    /// Move the whole grid by `delta` — rest and current positions alike,
    /// velocities untouched — so placement changes never reset the
    /// simulation (#46: a backdrop following a scrolling camera).
    pub fn translate(&mut self, delta: Vec2) {
        self.origin += delta;
        for node in &mut self.nodes {
            node.rest += delta;
            node.position += delta;
        }
    }

    /// Apply an impulse to the grid for one frame.
    pub fn apply_impulse(&mut self, impulse: &GridImpulse) {
        match *impulse {
            GridImpulse::Point { position, force, radius } => {
                let r2 = (radius * radius).max(1e-3);
                for node in &mut self.nodes {
                    if node.inv_mass == 0.0 { continue; }
                    let d2 = node.rest.distance_squared(position);
                    if d2 > r2 * 9.0 { continue; }
                    // Gaussian falloff: exp(-d^2 / (2 * sigma^2)) with sigma = radius/2.
                    let falloff = (-d2 / r2).exp();
                    node.velocity += force * falloff * node.inv_mass;
                }
            }
            GridImpulse::Radial { position, strength, radius, attractive } => {
                let r2 = (radius * radius).max(1e-3);
                let sign = if attractive { -1.0 } else { 1.0 };
                for node in &mut self.nodes {
                    if node.inv_mass == 0.0 { continue; }
                    let diff = node.rest - position;
                    let d2 = diff.length_squared();
                    if d2 > r2 * 9.0 || d2 < 1e-6 { continue; }
                    let dist = d2.sqrt();
                    let dir = diff / dist;
                    let falloff = (-d2 / r2).exp();
                    node.velocity += dir * (strength * falloff * sign) * node.inv_mass;
                }
            }
        }
    }

    /// Advance the simulation by `dt` seconds. Non-finite or non-positive
    /// `dt` is a no-op (a single NaN frame must not poison node state or
    /// the activity envelope).
    pub fn step(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 { return; }
        let substeps = self.substeps.max(1);
        let h = dt / substeps as f32;
        // Per-substep damping factor so total damping doesn't depend on dt.
        let damp = (1.0 - self.damping).clamp(0.0, 1.0).powf(h * 60.0);

        for _ in 0..substeps {
            self.accumulate_forces();
            for (node, force) in self.nodes.iter_mut().zip(self.force_scratch.iter()) {
                if node.inv_mass == 0.0 { continue; }
                node.velocity += *force * h * node.inv_mass;
                node.velocity *= damp;
                node.position += node.velocity * h;
            }
        }

        self.update_activity(dt);
    }

    /// Advance each node's activity envelope from its displacement and
    /// speed. Displacement peaks exactly when velocity crosses zero, so the
    /// blended signal doesn't strobe as a node oscillates; the asymmetric
    /// attack/release smooths what remains. `1 - exp(-dt/tau)` keeps both
    /// time constants frame-rate independent.
    fn update_activity(&mut self, dt: f32) {
        let d_ref = self.activity_displacement_ref.max(1e-3);
        let v_ref = self.activity_velocity_ref.max(1e-3);
        let attack = 1.0 - (-dt / self.activity_attack.max(1e-3)).exp();
        let release = 1.0 - (-dt / self.activity_release.max(1e-3)).exp();
        for (node, env) in self.nodes.iter().zip(self.activity.iter_mut()) {
            let mut raw = ((node.position - node.rest).length() / d_ref
                + node.velocity.length() / v_ref)
                .min(1.0);
            if raw < ACTIVITY_DEADBAND {
                raw = 0.0;
            }
            let k = if raw > *env { attack } else { release };
            *env += (raw - *env) * k;
        }
    }

    /// Sum spring forces and rest-position pulls into `force_scratch`.
    fn accumulate_forces(&mut self) {
        self.force_scratch.fill(Vec2::ZERO);

        // Spring forces.
        for s in &self.springs {
            let a = self.nodes[s.a as usize].position;
            let b = self.nodes[s.b as usize].position;
            let delta = b - a;
            let len = delta.length();
            if len < 1e-6 { continue; }
            let stretch = len - s.rest_length;
            let dir = delta / len;
            let f = dir * (stretch * self.stiffness);
            self.force_scratch[s.a as usize] += f;
            self.force_scratch[s.b as usize] -= f;
        }

        // Pull every node back toward its rest position so excitement decays.
        for (i, node) in self.nodes.iter().enumerate() {
            let pull = (node.rest - node.position) * self.rest_pull;
            self.force_scratch[i] += pull;
        }
    }

    /// Emit one [`LineVertex`] pair per spring into the scratch buffer and
    /// return the slice. Returned slice is invalidated by the next call.
    ///
    /// Each vertex gets a per-node alpha between
    /// `color.w * rest_alpha_fraction` (at rest) and `color.w` (fully
    /// active), interpolated by that node's activity envelope — so
    /// `color.w` stays the *maximum* alpha and ripples brighten the grid
    /// locally. `emissive` is uniform: alpha blending already scales the
    /// HDR contribution, so bloom follows the fade for free.
    ///
    /// Returns an empty slice when [`visible`](Self::visible) is `false` or
    /// the color is fully transparent — skipping the per-spring loop
    /// entirely so a hidden grid costs nothing to "render".
    pub fn build_line_vertices(&mut self) -> &[LineVertex] {
        self.line_scratch.clear();
        if !self.visible || self.color.w <= 0.0 {
            return &self.line_scratch;
        }
        // Clamp here too: the field is public, and alpha above `color.w`
        // would break the documented "color.w is the maximum" invariant.
        let rest_a = self.color.w * self.rest_alpha_fraction.clamp(0.0, 1.0);
        let emissive = self.emissive;
        for s in &self.springs {
            for idx in [s.a as usize, s.b as usize] {
                let node = &self.nodes[idx];
                let alpha = rest_a + (self.color.w - rest_a) * self.activity[idx];
                self.line_scratch.push(LineVertex {
                    position: node.position.to_array(),
                    color: [self.color.x, self.color.y, self.color.z, alpha],
                    emissive,
                });
            }
        }
        &self.line_scratch
    }

    /// Total kinetic energy in the grid — handy for tests verifying that
    /// undamped impulses keep the grid bounded and damped impulses decay.
    pub fn total_energy(&self) -> f32 {
        self.nodes.iter().map(|n| 0.5 * n.velocity.length_squared()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Interior node (2,2) of a 6-column grid.
    const CENTER: usize = 14;

    #[test]
    fn energy_decays_with_damping_and_nodes_return_to_rest_while_pinned_corners_never_move() {
        let mut g = GridMesh::new(6, 5, 10.0, Vec2::ZERO).with_damping(0.2);
        let rest = g.nodes[CENTER].rest;
        g.apply_impulse(&GridImpulse::Point { position: rest, force: Vec2::new(0.0, 100.0), radius: 5.0 });

        g.step(0.05);
        assert!(g.nodes[CENTER].position.y > rest.y, "the impulse moved the interior node up");
        let e_start = g.total_energy();
        for _ in 0..600 { // ~10 seconds
            g.step(0.016);
        }

        assert!(g.total_energy() < e_start * 0.1, "energy {e_start} -> {} should decay heavily", g.total_energy());
        let final_offset = (g.nodes[CENTER].position - rest).length();
        assert!(final_offset < 0.1, "node should settle near rest, offset = {final_offset}");

        // A pinned corner ignores even a violent impulse centered on it.
        let mut g = GridMesh::new(6, 5, 10.0, Vec2::ZERO);
        let corner_rest = g.nodes[0].rest;
        g.apply_impulse(&GridImpulse::Radial {
            position: corner_rest, strength: 1000.0, radius: 100.0, attractive: false,
        });
        for _ in 0..30 {
            g.step(0.016);
        }
        assert_eq!(g.nodes[0].position, corner_rest, "a pinned corner never moves");
    }

    #[test]
    fn line_vertices_come_in_spring_pairs_at_node_positions_with_rest_alpha() {
        let mut g = GridMesh::new(4, 3, 1.0, Vec2::new(2.0, 3.0));
        let expected_alpha = g.color.w * g.rest_alpha_fraction;

        let verts: Vec<LineVertex> = g.build_line_vertices().to_vec();

        assert_eq!(verts.len(), g.spring_count() * 2);
        for (spring, pair) in g.springs.iter().zip(verts.chunks(2)) {
            assert_eq!(pair[0].position, g.nodes[spring.a as usize].position.to_array());
            assert_eq!(pair[1].position, g.nodes[spring.b as usize].position.to_array());
            for v in pair {
                assert!((v.color[3] - expected_alpha).abs() < 1e-6, "at rest every alpha is color.w * fraction");
                assert_eq!(v.emissive, g.emissive);
            }
        }
    }

    #[test]
    fn hidden_grid_still_simulates_but_emits_no_vertices() {
        // Re-enabling a hidden grid resumes from the same physics state, so
        // invisible (or fully transparent) grids keep stepping — they only
        // skip the per-spring vertex loop.
        let mut hidden = GridMesh::new(6, 5, 10.0, Vec2::ZERO);
        hidden.visible = false;
        let mut transparent = GridMesh::new(6, 5, 10.0, Vec2::ZERO).with_alpha(0.0);
        for g in [&mut hidden, &mut transparent] {
            let rest = g.nodes[CENTER].rest;
            g.apply_impulse(&GridImpulse::Point { position: rest, force: Vec2::new(0.0, 100.0), radius: 5.0 });

            g.step(0.05);

            assert!(g.nodes[CENTER].position.y > rest.y, "the hidden grid still steps");
            assert_eq!(g.build_line_vertices().len(), 0, "and draws nothing");
        }
    }

    #[test]
    fn translate_shifts_rest_and_position_but_not_velocity() {
        // #46: a backdrop following a scrolling camera moves without
        // resetting its simulation — the ripple travels with it.
        let mut g = GridMesh::new(6, 5, 10.0, Vec2::ZERO);
        g.apply_impulse(&GridImpulse::Point {
            position: g.nodes[CENTER].rest, force: Vec2::new(0.0, 100.0), radius: 5.0,
        });
        g.step(0.05);
        let before: Vec<GridNode> = g.nodes.clone();
        let delta = Vec2::new(100.0, -50.0);

        g.translate(delta);

        assert_eq!(g.origin, delta);
        for (was, now) in before.iter().zip(&g.nodes) {
            assert_eq!(now.rest, was.rest + delta);
            assert_eq!(now.position, was.position + delta);
            assert_eq!(now.velocity, was.velocity, "velocity is untouched");
        }
        assert_ne!(before[CENTER].velocity, Vec2::ZERO, "the ripple was live when translated");
    }
}
