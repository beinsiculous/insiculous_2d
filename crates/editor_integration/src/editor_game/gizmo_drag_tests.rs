//! Gizmo drag-state behavior: multi-root capture, single-undo-entry commit,
//! and Escape cancel-and-restore. Drives the real `GizmoDragState` +
//! `commit_gizmo_drag`/`cancel_gizmo_drag` paths headlessly.

use ecs::{World, WorldHierarchyExt};
use engine_core::contexts::GameContext;
use engine_core::Game;
use glam::Vec2;

use super::gizmo_drag::{DragEntity, GizmoDragState};
use super::EditorGame;
use crate::entity_ops::selection_roots;

struct DummyGame;
impl Game for DummyGame {
    fn update(&mut self, _ctx: &mut GameContext) {}
}

fn spawn_at(world: &mut World, pos: Vec2) -> ecs::EntityId {
    let entity = world.create_entity();
    world
        .add_component(&entity, common::Transform2D::new(pos))
        .ok();
    entity
}

#[test]
fn test_selection_roots_excludes_children_of_selected_parents() {
    let mut world = World::new();
    let parent = spawn_at(&mut world, Vec2::ZERO);
    let child = spawn_at(&mut world, Vec2::new(10.0, 0.0));
    let lone = spawn_at(&mut world, Vec2::new(50.0, 0.0));
    world.set_parent(child, parent).ok();

    let mut selection = editor::Selection::new();
    selection.add(parent);
    selection.add(child);
    selection.add(lone);

    let roots = selection_roots(&world, &selection);
    assert!(roots.contains(&parent));
    assert!(roots.contains(&lone));
    assert!(
        !roots.contains(&child),
        "a selected child of a selected parent would be double-moved"
    );
}

#[test]
fn test_selection_roots_puts_the_primary_first() {
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::ZERO);
    let b = spawn_at(&mut world, Vec2::new(10.0, 0.0));
    let c = spawn_at(&mut world, Vec2::new(20.0, 0.0));

    let mut selection = editor::Selection::new();
    selection.add(a);
    selection.add(b);
    selection.add(c);
    selection.set_primary(c);

    let roots = selection_roots(&world, &selection);
    assert_eq!(roots[0], c, "the primary anchors snapping — it must be first");
    assert_eq!(&roots[1..], &[a, b], "the rest keep selection insertion order");
}

/// Build a two-entity drag state as `handle_gizmo` would capture it.
fn drag_state_for(world: &World, ids: &[ecs::EntityId]) -> GizmoDragState {
    GizmoDragState {
        entities: ids
            .iter()
            .map(|&id| DragEntity {
                id,
                start: *world.get::<common::Transform2D>(id).expect("transform"),
                start_collider: world.get::<physics::components::Collider>(id).cloned(),
            })
            .collect(),
        accumulated_rotation: 0.0,
    }
}

#[test]
fn test_commit_records_one_undo_entry_restoring_every_root() {
    let mut game = EditorGame::new(DummyGame);
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::new(0.0, 0.0));
    let b = spawn_at(&mut world, Vec2::new(100.0, 0.0));

    game.gizmo_drag = Some(drag_state_for(&world, &[a, b]));

    // The drag moved both entities by (32, 16)
    for &id in &[a, b] {
        if let Some(t) = world.get_mut::<common::Transform2D>(id) {
            t.position += Vec2::new(32.0, 16.0);
        }
    }

    game.commit_gizmo_drag(&world);
    assert!(game.gizmo_drag.is_none());
    assert!(game.command_history.can_undo());

    // ONE undo restores BOTH entities to their drag-start positions
    game.command_history.undo(&mut world);
    assert_eq!(
        world.get::<common::Transform2D>(a).map(|t| t.position),
        Some(Vec2::new(0.0, 0.0))
    );
    assert_eq!(
        world.get::<common::Transform2D>(b).map(|t| t.position),
        Some(Vec2::new(100.0, 0.0))
    );
    assert!(
        !game.command_history.can_undo(),
        "a multi-entity drag is exactly one history entry"
    );
}

#[test]
fn test_commit_with_no_change_pushes_nothing() {
    let mut game = EditorGame::new(DummyGame);
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::ZERO);

    game.gizmo_drag = Some(drag_state_for(&world, &[a]));
    // Zero-delta click: nothing moved
    game.commit_gizmo_drag(&world);
    assert!(!game.command_history.can_undo());
}

#[test]
fn test_cancel_restores_starts_and_pushes_no_undo_entry() {
    let mut game = EditorGame::new(DummyGame);
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::new(5.0, 5.0));
    let b = spawn_at(&mut world, Vec2::new(50.0, 5.0));
    world
        .add_component(&b, physics::components::Collider::box_collider(16.0, 16.0))
        .ok();

    game.gizmo_drag = Some(drag_state_for(&world, &[a, b]));

    // Mid-drag: entities moved, b's collider resized
    for &id in &[a, b] {
        if let Some(t) = world.get_mut::<common::Transform2D>(id) {
            t.position += Vec2::new(300.0, 0.0);
            t.scale *= 2.0;
        }
    }
    if let Some(c) = world.get_mut::<physics::components::Collider>(b) {
        super::viewport_interaction::scale_collider(c, Vec2::splat(2.0));
    }

    // Escape
    assert!(game.cancel_gizmo_drag(&mut world));
    assert_eq!(
        world.get::<common::Transform2D>(a).map(|t| t.position),
        Some(Vec2::new(5.0, 5.0))
    );
    assert_eq!(
        world.get::<common::Transform2D>(b).map(|t| (t.position, t.scale)),
        Some((Vec2::new(50.0, 5.0), Vec2::ONE))
    );
    assert_eq!(
        world.get::<physics::components::Collider>(b).cloned(),
        Some(physics::components::Collider::box_collider(16.0, 16.0)),
        "the collider rolls back with the transform"
    );
    assert!(!game.command_history.can_undo(), "a cancelled drag leaves no history");
    assert!(!game.cancel_gizmo_drag(&mut world), "second Escape has nothing to cancel");

    // A commit after the cancel is a no-op (state was taken)
    game.commit_gizmo_drag(&world);
    assert!(!game.command_history.can_undo());
}
