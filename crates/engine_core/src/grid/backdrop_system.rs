//! The runtime behind [`GridBackdrop`] components (#46): the engine owns one
//! simulated [`GridMesh`] per component entity, rebuilds it when the
//! component's (normalized) data changes, translates it when the entity
//! moves, drains queued impulses, steps it with the engine's time-scaled
//! delta (so the editor's freeze holds it still), and emits its line
//! vertices *under* whatever lines the game pushed this frame.

use std::collections::HashMap;

use ecs::{EntityId, GlobalTransform2D, GridBackdrop, Transform2D, World};
use glam::Vec2;
use renderer::line_pipeline::LineVertex;

use super::build::{apply_grid_tunables, build_grid_mesh};
use super::{GridImpulse, GridMesh};

/// World resource: impulses queued for every backdrop this frame. Push
/// through [`ripple`]; the system drains it each frame.
#[derive(Debug, Default)]
pub struct GridImpulses(pub Vec<GridImpulse>);

/// World resource marker: rebuild every backdrop at rest on the next frame.
/// The editor inserts it on Stop, so a grid frozen mid-ripple by the
/// snapshot restore does not stay deformed.
#[derive(Debug)]
pub struct GridBackdropReset;

/// Queue an impulse for every backdrop in the world (applied on the next
/// engine frame with positive time; dropped while time is frozen).
pub fn ripple(world: &mut World, impulse: GridImpulse) {
    match world.resource_mut::<GridImpulses>() {
        Some(queue) => queue.0.push(impulse),
        None => world.insert_resource(GridImpulses(vec![impulse])),
    }
}

/// Ask the system to rebuild every backdrop at rest on its next update.
pub fn request_backdrop_reset(world: &mut World) {
    world.insert_resource(GridBackdropReset);
}

struct Entry {
    /// The NORMALIZED config the mesh was built from — the compare target.
    config: GridBackdrop,
    origin: Vec2,
    mesh: GridMesh,
}

/// One simulated mesh per `GridBackdrop` entity. Owned by the game runner;
/// see the module docs for the per-frame contract.
#[derive(Default)]
pub struct GridBackdropSystem {
    entries: HashMap<EntityId, Entry>,
    /// Every grid's vertices for the frame, in entity-id order, spliced
    /// into the game's line buffer in one move.
    scratch: Vec<LineVertex>,
}

impl GridBackdropSystem {
    /// Sync meshes to the world's components, apply queued impulses, step by
    /// `delta_time` (a non-positive delta freezes the simulation but still
    /// draws), and prepend the vertices to `out` so game lines stay on top.
    pub fn update(&mut self, world: &mut World, delta_time: f32, out: &mut Vec<LineVertex>) {
        if world.remove_resource::<GridBackdropReset>().is_some() {
            self.entries.clear();
        }
        self.sync_entries(world);

        // Impulses only carry energy into a running simulation; a frozen
        // grid (editor, pause) must not bank them for later.
        let impulses = world.remove_resource::<GridImpulses>().map(|queue| queue.0).unwrap_or_default();
        if delta_time > 0.0 {
            for entry in self.entries.values_mut() {
                for impulse in &impulses {
                    entry.mesh.apply_impulse(impulse);
                }
            }
        }

        let mut order: Vec<EntityId> = self.entries.keys().copied().collect();
        order.sort_by_key(|entity| entity.value());
        self.scratch.clear();
        for entity in order {
            let Some(entry) = self.entries.get_mut(&entity) else { continue };
            entry.mesh.step(delta_time);
            self.scratch.extend_from_slice(entry.mesh.build_line_vertices());
        }
        if !self.scratch.is_empty() {
            out.splice(0..0, self.scratch.drain(..));
        }
    }

    /// The simulated mesh for `entity`, if it carries a backdrop.
    pub fn mesh(&self, entity: EntityId) -> Option<&GridMesh> {
        self.entries.get(&entity).map(|entry| &entry.mesh)
    }

    /// Drop meshes whose entity lost the component, build new ones, rebuild
    /// on a SHAPE change, apply other edits to the live mesh, translate on a
    /// move. The origin is the world-space position: `GlobalTransform2D`
    /// when the hierarchy system has written one (a grid parented to a
    /// moving rig follows it — kimi #46 F2), else the local `Transform2D`.
    fn sync_entries(&mut self, world: &World) {
        let live: Vec<(EntityId, GridBackdrop, Vec2)> = world
            .entities()
            .into_iter()
            .filter_map(|entity| {
                let config = world.get::<GridBackdrop>(entity)?.clone();
                let origin = world
                    .get::<GlobalTransform2D>(entity)
                    .map(|global| global.position)
                    .or_else(|| world.get::<Transform2D>(entity).map(|local| local.position))
                    .unwrap_or(Vec2::ZERO);
                Some((entity, config, origin))
            })
            .collect();
        self.entries.retain(|entity, _| live.iter().any(|(live_entity, _, _)| live_entity == entity));

        for (entity, config, origin) in live {
            let normalized = config.normalized();
            match self.entries.get_mut(&entity) {
                Some(entry) if entry.config.same_shape(&normalized) => {
                    if entry.config != normalized {
                        // Color, visibility, stiffness... land on the live
                        // mesh; an active ripple keeps rippling (F3).
                        apply_grid_tunables(&mut entry.mesh, &normalized);
                        entry.config = normalized;
                    }
                    if entry.origin != origin {
                        entry.mesh.translate(origin - entry.origin);
                        entry.origin = origin;
                    }
                }
                _ => {
                    let mesh = build_grid_mesh(&normalized, origin);
                    self.entries.insert(entity, Entry { config: normalized, origin, mesh });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecs::GridTopology;

    fn small() -> GridBackdrop {
        GridBackdrop { cols: 6, rows: 4, spacing: 10.0, ..GridBackdrop::default() }
    }

    fn spawn(world: &mut World, config: GridBackdrop, origin: Vec2) -> EntityId {
        let entity = world.create_entity();
        world.add_component(&entity, Transform2D::new(origin)).ok();
        world.add_component(&entity, config).ok();
        entity
    }

    fn kick(world: &mut World) {
        ripple(world, GridImpulse::Radial { position: Vec2::ZERO, strength: 500.0, radius: 20.0, attractive: false });
    }

    #[test]
    fn test_component_entity_gets_a_mesh_that_draws_and_loses_it_on_removal() {
        let mut world = World::new();
        let entity = spawn(&mut world, small(), Vec2::ZERO);
        let mut system = GridBackdropSystem::default();
        let mut out = Vec::new();

        system.update(&mut world, 1.0 / 60.0, &mut out);
        assert!(system.mesh(entity).is_some());
        assert!(!out.is_empty(), "the backdrop emits line vertices");

        world.remove_component::<GridBackdrop>(&entity).ok();
        out.clear();
        system.update(&mut world, 1.0 / 60.0, &mut out);
        assert!(system.mesh(entity).is_none());
        assert!(out.is_empty());
    }

    #[test]
    fn test_shape_change_rebuilds_but_a_nan_tunable_does_not_rebuild_every_frame() {
        let mut world = World::new();
        let entity = spawn(&mut world, small(), Vec2::ZERO);
        let mut system = GridBackdropSystem::default();
        let mut out = Vec::new();
        system.update(&mut world, 1.0 / 60.0, &mut out);
        let nodes_before = system.mesh(entity).unwrap().node_count();

        world.get_mut::<GridBackdrop>(entity).unwrap().cols = 10;
        system.update(&mut world, 1.0 / 60.0, &mut out);
        assert!(system.mesh(entity).unwrap().node_count() > nodes_before, "more columns, more nodes");

        // A NaN tunable: built once (falls back to the preset), then left alone —
        // energy from an impulse survives the next update, proving no rebuild.
        world.get_mut::<GridBackdrop>(entity).unwrap().stiffness = f32::NAN;
        system.update(&mut world, 1.0 / 60.0, &mut out);
        kick(&mut world);
        system.update(&mut world, 1.0 / 60.0, &mut out);
        let energy = system.mesh(entity).unwrap().total_energy();
        assert!(energy > 0.0);
        system.update(&mut world, 1.0 / 60.0, &mut out);
        assert!(system.mesh(entity).unwrap().total_energy() > 0.0, "still the same simulation");
    }

    #[test]
    fn test_cosmetic_and_tunable_edits_apply_in_place_without_resetting_a_ripple() {
        let mut world = World::new();
        let entity = spawn(&mut world, small(), Vec2::ZERO);
        let mut system = GridBackdropSystem::default();
        let mut out = Vec::new();
        system.update(&mut world, 1.0 / 60.0, &mut out);
        kick(&mut world);
        system.update(&mut world, 1.0 / 60.0, &mut out);
        let energy = system.mesh(entity).unwrap().total_energy();
        assert!(energy > 0.0);

        {
            let config = world.get_mut::<GridBackdrop>(entity).unwrap();
            config.color = glam::Vec4::new(1.0, 0.0, 0.0, 1.0);
            config.stiffness = 90.0;
            config.visible = false;
        }
        system.update(&mut world, 0.0, &mut out);
        let mesh = system.mesh(entity).unwrap();
        assert_eq!(mesh.color, glam::Vec4::new(1.0, 0.0, 0.0, 1.0));
        assert_eq!(mesh.stiffness, 90.0);
        assert!(!mesh.visible);
        assert!((mesh.total_energy() - energy).abs() < 1e-3, "the ripple survives a cosmetic edit");
    }

    #[test]
    fn test_parented_grid_follows_the_global_transform() {
        // kimi #46 F2: a grid under a moving rig sits where the hierarchy
        // system put it, not at its local offset.
        let mut world = World::new();
        let entity = spawn(&mut world, small(), Vec2::new(5.0, 5.0));
        world
            .add_component(
                &entity,
                GlobalTransform2D { position: Vec2::new(300.0, 40.0), rotation: 0.0, scale: Vec2::ONE },
            )
            .ok();
        let mut system = GridBackdropSystem::default();
        let mut out = Vec::new();
        system.update(&mut world, 0.0, &mut out);
        assert_eq!(system.mesh(entity).unwrap().origin, Vec2::new(300.0, 40.0));
    }

    #[test]
    fn test_moving_the_entity_translates_the_mesh_without_a_rebuild() {
        let mut world = World::new();
        let entity = spawn(&mut world, small(), Vec2::ZERO);
        let mut system = GridBackdropSystem::default();
        let mut out = Vec::new();
        system.update(&mut world, 1.0 / 60.0, &mut out);
        kick(&mut world);
        system.update(&mut world, 1.0 / 60.0, &mut out);
        let energy = system.mesh(entity).unwrap().total_energy();
        let first_vertex = out[0].position;

        world.get_mut::<Transform2D>(entity).unwrap().position = Vec2::new(100.0, -50.0);
        out.clear();
        system.update(&mut world, 0.0, &mut out); // frozen: only the move applies
        let moved = system.mesh(entity).unwrap();
        assert_eq!(moved.origin, Vec2::new(100.0, -50.0));
        assert!((moved.total_energy() - energy).abs() < 1e-3, "the simulation state survives the move");
        assert!((out[0].position[0] - (first_vertex[0] + 100.0)).abs() < 1e-3);
        assert!((out[0].position[1] - (first_vertex[1] - 50.0)).abs() < 1e-3);
    }

    #[test]
    fn test_frozen_time_still_draws_and_drops_impulses() {
        let mut world = World::new();
        let entity = spawn(&mut world, small(), Vec2::ZERO);
        let mut system = GridBackdropSystem::default();
        let mut out = Vec::new();
        system.update(&mut world, 0.0, &mut out);
        assert!(!out.is_empty(), "a frozen grid is still visible");

        kick(&mut world);
        system.update(&mut world, 0.0, &mut out);
        assert!(!world.has_resource::<GridImpulses>(), "the queue is drained even when frozen");
        system.update(&mut world, 1.0 / 60.0, &mut out);
        assert!(system.mesh(entity).unwrap().total_energy() < 1e-6, "no energy was banked");
    }

    #[test]
    fn test_reset_marker_rebuilds_every_grid_at_rest() {
        let mut world = World::new();
        let entity = spawn(&mut world, small(), Vec2::ZERO);
        let mut system = GridBackdropSystem::default();
        let mut out = Vec::new();
        system.update(&mut world, 1.0 / 60.0, &mut out);
        kick(&mut world);
        system.update(&mut world, 1.0 / 60.0, &mut out);
        assert!(system.mesh(entity).unwrap().total_energy() > 0.0);

        request_backdrop_reset(&mut world);
        system.update(&mut world, 0.0, &mut out);
        let rebuilt = system.mesh(entity).unwrap();
        assert_eq!(rebuilt.total_energy(), 0.0);
        assert_eq!(rebuilt.node_count(), 6 * 4);
        assert!(!world.has_resource::<GridBackdropReset>(), "the marker is consumed");
    }

    #[test]
    fn test_grids_emit_in_entity_order_ahead_of_the_games_lines() {
        let mut world = World::new();
        let square = GridBackdrop { topology: GridTopology::Square, cols: 2, rows: 2, ..small() };
        let hex = GridBackdrop { cols: 2, rows: 2, ..small() };
        let later = spawn(&mut world, square.clone(), Vec2::new(1000.0, 0.0));
        let earlier_id = spawn(&mut world, hex.clone(), Vec2::ZERO);
        assert!(earlier_id.value() > later.value(), "spawned second, sorted second");
        let mut system = GridBackdropSystem::default();

        let game_line = LineVertex { position: [-1.0, -1.0], color: [1.0; 4], emissive: 0.0 };
        let mut out = vec![game_line, game_line];
        system.update(&mut world, 1.0 / 60.0, &mut out);

        let square_vertices = build_grid_mesh(&square, Vec2::new(1000.0, 0.0)).build_line_vertices().len();
        let hex_vertices = build_grid_mesh(&hex, Vec2::ZERO).build_line_vertices().len();
        assert_eq!(out.len(), square_vertices + hex_vertices + 2);
        // Lower entity id first (the square at x≈1000), then the hex, then the game's.
        assert!(out[0].position[0] > 900.0, "the lower-id grid comes first");
        assert!(out[square_vertices].position[0] < 100.0, "then the higher-id grid");
        assert_eq!(out[out.len() - 1].position, [-1.0, -1.0], "game lines draw last (on top)");
    }
}
