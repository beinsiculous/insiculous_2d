//! Unified-shortcut behavior (#40): arrow nudge (merge + seal), the Escape
//! cancel cascade, select-all, and the clipboard round trip — all driven
//! headlessly against the real EditorGame state.

use ecs::{World, WorldHierarchyExt};
use engine_core::contexts::GameContext;
use engine_core::Game;
use glam::Vec2;

use super::gizmo_drag::{DragEntity, GizmoDragState};
use super::EditorGame;
use crate::entity_ops;

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

fn pos(world: &World, entity: ecs::EntityId) -> Vec2 {
    world
        .get::<common::Transform2D>(entity)
        .map(|t| t.position)
        .expect("transform")
}

#[test]
fn test_nudge_moves_roots_one_unit_ten_with_shift() {
    let mut game = EditorGame::new(DummyGame);
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::ZERO);
    game.editor.selection.select(a);

    game.nudge_selection(&mut world, Vec2::new(1.0, 0.0), false);
    assert_eq!(pos(&world, a), Vec2::new(1.0, 0.0));

    game.nudge_selection(&mut world, Vec2::new(0.0, 1.0), true);
    assert_eq!(pos(&world, a), Vec2::new(1.0, 10.0));
}

#[test]
fn test_held_arrow_merges_into_one_undo_entry_sealed_on_release() {
    let mut game = EditorGame::new(DummyGame);
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::ZERO);
    game.editor.selection.select(a);

    // A held key repeat: three presses before the release
    for _ in 0..3 {
        game.nudge_selection(&mut world, Vec2::new(1.0, 0.0), false);
    }
    assert_eq!(pos(&world, a), Vec2::new(3.0, 0.0));

    // Key release seals the entry
    game.command_history.break_merge();

    // A second hold is a separate entry
    game.nudge_selection(&mut world, Vec2::new(1.0, 0.0), false);
    assert_eq!(pos(&world, a), Vec2::new(4.0, 0.0));

    // Undo #1: only the second hold reverts
    assert!(game.command_history.undo(&mut world));
    assert_eq!(pos(&world, a), Vec2::new(3.0, 0.0));
    // Undo #2: the whole first hold reverts in one step
    assert!(game.command_history.undo(&mut world));
    assert_eq!(pos(&world, a), Vec2::ZERO);
    assert!(!game.command_history.can_undo(), "exactly two entries for two holds");
}

#[test]
fn test_nudge_moves_only_selection_roots() {
    let mut game = EditorGame::new(DummyGame);
    let mut world = World::new();
    let parent = spawn_at(&mut world, Vec2::ZERO);
    let child = spawn_at(&mut world, Vec2::new(5.0, 0.0));
    world.set_parent(child, parent).ok();
    game.editor.selection.select(parent);
    game.editor.selection.add(child);

    game.nudge_selection(&mut world, Vec2::new(1.0, 0.0), false);
    assert_eq!(pos(&world, parent), Vec2::new(1.0, 0.0));
    assert_eq!(
        pos(&world, child),
        Vec2::new(5.0, 0.0),
        "the child's LOCAL transform must not double-move"
    );
}

#[test]
fn test_nudge_is_suppressed_during_a_gizmo_drag() {
    let mut game = EditorGame::new(DummyGame);
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::ZERO);
    game.editor.selection.select(a);
    game.gizmo_drag = Some(GizmoDragState {
        entities: vec![DragEntity {
            id: a,
            start: common::Transform2D::new(Vec2::ZERO),
            start_collider: None,
        }],
        accumulated_rotation: 0.0,
    });

    game.nudge_selection(&mut world, Vec2::new(1.0, 0.0), false);
    assert_eq!(
        pos(&world, a),
        Vec2::ZERO,
        "a mid-drag nudge would be swallowed by the drag's release commit"
    );
    assert!(!game.command_history.can_undo());
}

#[test]
fn test_cancel_cascade_marquee_then_selection() {
    let mut game = EditorGame::new(DummyGame);
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::ZERO);
    game.editor.selection.select(a);

    // No drag, no marquee: first Escape clears the selection
    game.cancel_cascade(&mut world);
    assert!(game.editor.selection.is_empty());

    // With a gizmo drag live, Escape cancels THAT and leaves selection alone
    game.editor.selection.select(a);
    game.gizmo_drag = Some(GizmoDragState {
        entities: vec![DragEntity {
            id: a,
            start: common::Transform2D::new(Vec2::ZERO),
            start_collider: None,
        }],
        accumulated_rotation: 0.0,
    });
    game.cancel_cascade(&mut world);
    assert!(game.gizmo_drag.is_none(), "the drag cancels first");
    assert!(
        !game.editor.selection.is_empty(),
        "exactly one cancel per press — selection survives"
    );
}

#[test]
fn test_select_all_matches_the_world() {
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::ZERO);
    let b = spawn_at(&mut world, Vec2::new(1.0, 0.0));
    // A camera-like entity without a sprite is still selectable
    let empty = world.create_entity();

    let all = entity_ops::selectable_entities(&world);
    assert!(all.contains(&a) && all.contains(&b) && all.contains(&empty));
    assert_eq!(all.len(), 3);
}

#[test]
fn test_resolve_dispatch_table_is_the_single_shortcut_system() {
    // Drift lock for audit §4.9: the old hardcoded matches are gone; every
    // shortcut the editor ships resolves through the ONE mapping.
    let game = EditorGame::new(DummyGame);
    use editor::EditorAction as A;
    use winit::keyboard::KeyCode;
    assert_eq!(
        game.editor.input_mapping.resolve(KeyCode::KeyW, false, false),
        Some(A::ToolMove)
    );
    assert_eq!(
        game.editor.input_mapping.resolve(KeyCode::KeyP, true, true),
        Some(A::StopPlay)
    );
    assert_eq!(
        game.editor.input_mapping.resolve(KeyCode::ArrowRight, false, true),
        Some(A::NudgeRight)
    );
}
