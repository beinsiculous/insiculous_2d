//! Scene-view overlay contracts, through the real render path: each overlay
//! draws only while its toggle is on. The gates live in the caller, so the
//! overlay modules' own tests cannot see them — this file can.

use ecs::World;
use editor::EditorContext;
use glam::Vec2;
use ui::{Color, DrawCommand, UIContext};

use super::render_scene_view;

const WINDOW: Vec2 = Vec2::new(1200.0, 700.0);

fn editor_with_a_scene_view() -> EditorContext {
    let mut editor = EditorContext::new();
    editor.update_layout(WINDOW);
    assert!(editor.scene_view_bounds().is_some(), "the default dock gives the scene panel a viewport");
    editor
}

/// A world whose main camera sits off the origin at zoom 2, so the frame's
/// rect is neither the default nor symmetric about the grid's axes.
fn world_with_a_main_camera() -> World {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::new(40.0, -10.0))).ok();
    let mut camera = common::Camera::default().as_main_camera();
    camera.zoom = 2.0;
    world.add_component(&entity, camera).ok();
    world
}

/// One scene-view frame; the lines drawn in `color`.
fn lines_in(editor: &EditorContext, world: &World, color: Color) -> usize {
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();
    ui.begin_frame(&input, WINDOW);
    render_scene_view(editor, &mut ui, world, &[]);
    ui.end_frame();
    ui.draw_list()
        .commands()
        .iter()
        .filter(|command| matches!(command, DrawCommand::Line { color: drawn, .. } if *drawn == color))
        .count()
}

/// The game frame is four lines in its own token while the toggle is on and
/// the scene has a main camera; the toggle off, or no main camera, draws none.
#[test]
fn test_the_game_frame_draws_only_while_on_and_a_main_camera_exists() {
    let mut editor = editor_with_a_scene_view();
    let world = world_with_a_main_camera();
    let token = editor.theme.game_frame;

    assert_eq!(lines_in(&editor, &world, token), 4, "on, with a main camera: the frame's four edges");

    editor.view.game_frame = false;
    assert_eq!(lines_in(&editor, &world, token), 0, "the toggle off draws no frame");

    editor.view.game_frame = true;
    assert_eq!(lines_in(&editor, &World::new(), token), 0, "no main camera, no frame");
}

/// The authoring grid — its cells and its axes — draws only while its toggle
/// is on; the renderer no longer carries a visibility flag of its own.
#[test]
fn test_the_grid_draws_only_while_its_toggle_is_on() {
    let mut editor = editor_with_a_scene_view();
    let world = World::new();
    let colors = editor.theme.grid_colors();

    assert!(lines_in(&editor, &world, colors.primary) > 0, "on: primary cell lines");
    assert!(lines_in(&editor, &world, colors.axis_x) > 0, "on: the X axis");

    editor.view.grid = false;
    for (name, color) in [
        ("primary", colors.primary),
        ("secondary", colors.secondary),
        ("axis_x", colors.axis_x),
        ("axis_y", colors.axis_y),
    ] {
        assert_eq!(lines_in(&editor, &world, color), 0, "off: no {name} line");
    }
}
