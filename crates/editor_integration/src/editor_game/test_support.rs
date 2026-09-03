//! Shared fixtures for the editor-integration tests.
//!
//! Three facts every test file used to restate: the inner game is inert
//! (the `EditorGame` wrapper is the subject), an entity under test is a
//! `Transform2D` at a position, and a gizmo drag begins by capturing each
//! selection root's transform and collider. They live here once. The
//! GPU-free scene helpers (`StubResolver`, `test_texture_path`) come from
//! `engine_core::test_support`.

use ecs::{EntityId, World};
use engine_core::contexts::GameContext;
use engine_core::Game;
use glam::Vec2;

use super::gizmo_drag::{DragEntity, GizmoDragState};
use super::EditorGame;

/// An inner game that does nothing — the editor wrapper is the subject.
pub(super) struct DummyGame;
impl Game for DummyGame {
    fn update(&mut self, _ctx: &mut GameContext) {}
}

/// A fresh editor session wrapping [`DummyGame`], in the Editing state.
pub(super) fn editor_game() -> EditorGame<DummyGame> {
    EditorGame::new(DummyGame)
}

/// A world entity with only a `Transform2D` at `pos`.
pub(super) fn spawn_at(world: &mut World, pos: Vec2) -> EntityId {
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(pos)).ok();
    entity
}

/// The entity's local position.
pub(super) fn position(world: &World, entity: EntityId) -> Vec2 {
    world
        .get::<common::Transform2D>(entity)
        .map(|t| t.position)
        .expect("entity has a Transform2D")
}

/// A session whose history already holds one recorded command (an
/// entity creation), so the scene reads dirty.
pub(super) fn dirty_editor(world: &mut World) -> EditorGame<DummyGame> {
    let mut editor = editor_game();
    let entity = world.create_entity();
    editor.command_history.execute(
        Box::new(editor::commands::CreateEntityCommand::already_created(world, entity)),
        world,
    );
    assert!(editor.command_history.is_dirty(), "fixture: a recorded command dirties the scene");
    // The EditorContext flag is a per-frame mirror of the history, synced
    // by `EditorGame::update` (unreachable headless until batch 9's seam);
    // set it here so both halves agree the way a real frame leaves them.
    editor.editor.set_dirty(true);
    editor
}

/// The drag-start capture `handle_gizmo` makes for `ids`: every root's
/// transform and collider exactly as they are now.
pub(super) fn drag_state_for(world: &World, ids: &[EntityId]) -> GizmoDragState {
    GizmoDragState {
        entities: ids
            .iter()
            .map(|&id| DragEntity {
                id,
                start: *world.get::<common::Transform2D>(id).expect("drag root has a transform"),
                start_collider: world.get::<physics::components::Collider>(id).cloned(),
            })
            .collect(),
        accumulated_rotation: 0.0,
    }
}

/// One translate-tool frame with the given cumulative screen offset.
pub(super) fn translate_interaction(cumulative: Vec2) -> editor::GizmoInteraction {
    editor::GizmoInteraction {
        handle: Some(editor::GizmoHandle::Center),
        translation: cumulative,
        ..Default::default()
    }
}

/// One scale-tool frame with the given cumulative scale factor.
pub(super) fn scale_interaction(factor: Vec2) -> editor::GizmoInteraction {
    editor::GizmoInteraction {
        handle: Some(editor::GizmoHandle::ScaleCorner(editor::Corner::BottomRight)),
        scale_factor: factor,
        ..Default::default()
    }
}

/// Answer one command-API line against a session whose resolver issued
/// only the built-in `#white` (handle 0).
pub(super) fn api_line(editor: &mut EditorGame<DummyGame>, world: &mut World, line: &str) -> String {
    let resolver = |handle: u32| (handle == 0).then(|| "#white".to_string());
    let responses = editor.answer_api_lines(&[line.to_string()], world, &resolver);
    responses.into_iter().next().expect("one response per request line")
}

/// The response line is an `ok` envelope.
pub(super) fn assert_ok(response: &str) {
    assert!(response.contains("\"ok\":true"), "expected ok: {response}");
}
