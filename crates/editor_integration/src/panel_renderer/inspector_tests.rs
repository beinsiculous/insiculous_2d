//! The inspector panel's play-session contracts: the add-component button
//! stays and opens nothing while Playing, the heading says live only while
//! the simulation runs, and a scroll offset survives Editing → Play →
//! Editing. The frame and click helpers here serve `color_editor_tests` too.

use super::inspector::{render_inspector, LIVE_MARKER};
use ecs::World;
use editor::{CommandHistory, EditorContext};
use glam::Vec2;
use ui::UIContext;

pub(super) const BOUNDS: common::Rect = common::Rect { x: 0.0, y: 0.0, width: 300.0, height: 600.0 };
pub(super) const WINDOW: Vec2 = Vec2::new(800.0, 600.0);

/// An entity with the components the inspector's rows are built from.
pub(super) fn world_with_selection(editor: &mut EditorContext) -> World {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::ZERO)).ok();
    world.add_component(&entity, ecs::sprite_components::Sprite::new(0)).ok();
    world.add_component(&entity, physics::components::RigidBody::default()).ok();
    world.add_component(&entity, physics::components::Collider::default()).ok();
    world.add_component(&entity, ecs::audio_components::AudioSource::default()).ok();
    world.add_component(&entity, ecs::ui_components::UiPanel::default()).ok();
    editor.selection.select(entity);
    world
}

/// One inspector frame with the given input state, recording into a
/// history of its own.
pub(super) fn inspector_frame(
    editor: &mut EditorContext,
    world: &mut World,
    input: &input::InputHandler,
) -> UIContext {
    let mut command_history = CommandHistory::new();
    let mut ui = UIContext::new();
    inspector_frame_on(&mut ui, editor, world, input, &mut command_history, |_| {});
    ui
}

/// One inspector frame on a context that persists across frames — a
/// click is a press frame and a release frame, and the widget it
/// activated is remembered in between. `prepare` runs before the frame
/// opens (a test arms a field's focus there).
pub(super) fn inspector_frame_on(
    ui: &mut UIContext,
    editor: &mut EditorContext,
    world: &mut World,
    input: &input::InputHandler,
    command_history: &mut CommandHistory,
    prepare: impl FnOnce(&mut UIContext),
) {
    frame_on(ui, editor, world, input, command_history, FrameShape::default(), prepare);
}

/// The window and panel one test frame runs at, and whether the panel
/// is the dock's narrow-mode overlay (which puts it in a Floating scope
/// of its own).
#[derive(Clone, Copy)]
pub(super) struct FrameShape {
    pub(super) window: Vec2,
    pub(super) bounds: common::Rect,
    pub(super) narrow: bool,
    /// A confirm dialog is up this frame.
    pub(super) dialog_up: bool,
}

impl Default for FrameShape {
    fn default() -> Self {
        Self { window: WINDOW, bounds: BOUNDS, narrow: false, dialog_up: false }
    }
}

/// A window short enough that a popup opened from the sprite's colour
/// row cannot fit below it and has to flip above, over the rows the
/// walk already drew.
pub(super) fn short_shape() -> FrameShape {
    FrameShape {
        window: Vec2::new(WINDOW.x, 360.0),
        bounds: common::Rect::new(BOUNDS.x, BOUNDS.y, BOUNDS.width, 360.0),
        ..FrameShape::default()
    }
}

/// One editor frame in the order `render_panels` runs it: the colour
/// editor's pass first, then the inspector panel.
pub(super) fn frame_on(
    ui: &mut UIContext,
    editor: &mut EditorContext,
    world: &mut World,
    input: &input::InputHandler,
    command_history: &mut CommandHistory,
    shape: FrameShape,
    prepare: impl FnOnce(&mut UIContext),
) {
    prepare(ui);
    ui.begin_frame(input, shape.window);
    super::color_editor::render_color_editor_pass(editor, ui, shape.dialog_up);
    if shape.narrow {
        ui.begin_overlay_in(ui::UiLayer::Floating, shape.bounds);
    }
    render_inspector(editor, ui, world, &|_| None, shape.bounds, command_history);
    if shape.narrow {
        ui.end_overlay();
    }
    ui.end_frame();
}

/// A full click at `point`: the press frame, then the release frame
/// the click fires on.
pub(super) fn click_at(
    ui: &mut UIContext,
    editor: &mut EditorContext,
    world: &mut World,
    input: &mut input::InputHandler,
    history: &mut CommandHistory,
    point: Vec2,
) {
    click_at_in(ui, editor, world, input, history, FrameShape::default(), point);
}

/// [`click_at`] at a given frame shape.
pub(super) fn click_at_in(
    ui: &mut UIContext,
    editor: &mut EditorContext,
    world: &mut World,
    input: &mut input::InputHandler,
    history: &mut CommandHistory,
    shape: FrameShape,
    point: Vec2,
) {
    input.update();
    input.mouse_mut().update_position(point.x, point.y);
    input.mouse_mut().handle_button_press(input::prelude::MouseButton::Left);
    frame_on(ui, editor, world, input, history, shape, |_| {});
    input.update();
    input.mouse_mut().handle_button_release(input::prelude::MouseButton::Left);
    frame_on(ui, editor, world, input, history, shape, |_| {});
}

/// The sprite colour row's swatch: the only rounded rect drawn at the
/// colour-preview size.
pub(super) fn swatch_center(ui: &UIContext) -> Vec2 {
    let size = editor::EditableFieldStyle::default().color_preview_size;
    let bounds = ui
        .draw_list()
        .commands()
        .iter()
        .find_map(|command| match command {
            ui::DrawCommand::Rect { bounds, corner_radius, .. }
                if (bounds.width - size).abs() < 0.01
                    && (bounds.height - size).abs() < 0.01
                    && *corner_radius > 0.0 =>
            {
                Some(*bounds)
            }
            _ => None,
        })
        .expect("the sprite's colour row draws a swatch");
    Vec2::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0)
}

/// Every text this frame drew, with where it drew it.
pub(super) fn drawn_text(ui: &UIContext) -> Vec<(String, Vec2)> {
    ui.draw_list()
        .commands()
        .iter()
        .filter_map(|command| match command {
            ui::DrawCommand::TextPlaceholder { text, position, .. } => {
                Some((text.clone(), *position))
            }
            ui::DrawCommand::Text { data, .. } => Some((data.text.clone(), data.position)),
            _ => None,
        })
        .collect()
}

pub(super) fn position_of(ui: &UIContext, label: &str) -> Option<Vec2> {
    drawn_text(ui)
        .into_iter()
        .find(|(text, _)| text == label)
        .map(|(_, position)| position)
}

#[test]
fn test_add_component_keeps_its_row_while_playing_and_a_click_opens_nothing() {
    // The button holds its place so the section is the same height in
    // both states, but it is dead: a component added mid-simulation
    // would mutate the live world outside the history.
    let mut editor = EditorContext::new();
    let mut world = world_with_selection(&mut editor);
    let input = input::InputHandler::new();

    let editing = inspector_frame(&mut editor, &mut world, &input);
    let editing_button = position_of(&editing, "+ Add Component")
        .expect("editing inspector offers + Add Component");

    editor.set_play_state(editor::EditorPlayState::Playing);
    let playing = inspector_frame(&mut editor, &mut world, &input);
    assert_eq!(
        position_of(&playing, "+ Add Component"),
        Some(editing_button),
        "the button stays, in the same place"
    );

    let mut ui = UIContext::new();
    let mut history = CommandHistory::new();
    let mut input = input::InputHandler::new();
    click_at(&mut ui, &mut editor, &mut world, &mut input, &mut history, editing_button);

    assert!(
        !editor.is_add_component_popup_open(),
        "clicking the disabled button while Playing must open nothing"
    );
}

#[test]
fn test_the_heading_says_live_only_while_the_simulation_runs() {
    let mut editor = EditorContext::new();
    let mut world = world_with_selection(&mut editor);
    let input = input::InputHandler::new();

    let carries_marker = |ui: &UIContext| {
        drawn_text(ui).iter().any(|(text, _)| text.ends_with(LIVE_MARKER))
    };

    let editing = inspector_frame(&mut editor, &mut world, &input);
    assert!(!carries_marker(&editing), "an editing heading carries no marker");

    editor.set_play_state(editor::EditorPlayState::Playing);
    let playing = inspector_frame(&mut editor, &mut world, &input);
    assert!(carries_marker(&playing), "the detail line says the values are live");

    editor.set_play_state(editor::EditorPlayState::Paused);
    let paused = inspector_frame(&mut editor, &mut world, &input);
    assert!(!carries_marker(&paused), "a paused session is editable again");
}

#[test]
fn test_a_scroll_offset_near_the_maximum_survives_editing_to_play_and_back() {
    // The rows are the same height in both states, so nothing shortens
    // the content and clamps the offset away underneath the user.
    let mut editor = EditorContext::new();
    let mut world = world_with_selection(&mut editor);
    let input = input::InputHandler::new();

    inspector_frame(&mut editor, &mut world, &input);
    let content_height = editor.inspector_scroll.offset();
    assert_eq!(content_height, 0.0, "the panel opens at the top");
    editor.inspector_scroll.scroll_to(f32::MAX);
    inspector_frame(&mut editor, &mut world, &input);
    let at_maximum = editor.inspector_scroll.offset();
    assert!(at_maximum > 0.0, "the fixture is taller than the panel");

    editor.set_play_state(editor::EditorPlayState::Playing);
    inspector_frame(&mut editor, &mut world, &input);
    assert_eq!(editor.inspector_scroll.offset(), at_maximum, "Play kept the offset");

    editor.set_play_state(editor::EditorPlayState::Editing);
    inspector_frame(&mut editor, &mut world, &input);
    assert_eq!(editor.inspector_scroll.offset(), at_maximum, "Stop kept the offset");
}
