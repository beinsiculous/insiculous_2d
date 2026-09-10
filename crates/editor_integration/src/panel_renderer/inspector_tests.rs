//! The inspector panel's contract tests: what changes when a play session
//! runs, and the colour editor's pass — where it draws, what it blocks, and
//! whose undo entry its edits land in.

use super::inspector::{render_inspector, LIVE_MARKER};
use ecs::World;
use editor::{CommandHistory, EditorContext};
use glam::Vec2;
use ui::UIContext;

const BOUNDS: common::Rect = common::Rect { x: 0.0, y: 0.0, width: 300.0, height: 600.0 };
const WINDOW: Vec2 = Vec2::new(800.0, 600.0);

/// An entity with the components the inspector's rows are built from.
fn world_with_selection(editor: &mut EditorContext) -> World {
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
fn inspector_frame(
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
fn inspector_frame_on(
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
struct FrameShape {
    window: Vec2,
    bounds: common::Rect,
    narrow: bool,
    /// A confirm dialog is up this frame.
    dialog_up: bool,
}

impl Default for FrameShape {
    fn default() -> Self {
        Self { window: WINDOW, bounds: BOUNDS, narrow: false, dialog_up: false }
    }
}

/// A window short enough that a popup opened from the sprite's colour
/// row cannot fit below it and has to flip above, over the rows the
/// walk already drew.
fn short_shape() -> FrameShape {
    FrameShape {
        window: Vec2::new(WINDOW.x, 360.0),
        bounds: common::Rect::new(BOUNDS.x, BOUNDS.y, BOUNDS.width, 360.0),
        ..FrameShape::default()
    }
}

/// One editor frame in the order `render_panels` runs it: the colour
/// editor's pass first, then the inspector panel.
fn frame_on(
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
fn click_at(
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
fn click_at_in(
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
fn swatch_center(ui: &UIContext) -> Vec2 {
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
fn drawn_text(ui: &UIContext) -> Vec<(String, Vec2)> {
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

fn position_of(ui: &UIContext, label: &str) -> Option<Vec2> {
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

#[test]
fn test_the_swatch_opens_the_colour_editor_and_each_gesture_is_one_undo_entry() {
    // The editor writes through the colour row it lives in, so a scrub
    // merges into that row's undo entry and the frame's commit seal
    // ends the gesture — a second one must not join the first.
    let mut editor = EditorContext::new();
    let mut world = world_with_selection(&mut editor);
    let mut history = CommandHistory::new();
    let mut input = input::InputHandler::new();

    let mut ui = UIContext::new();
    inspector_frame_on(&mut ui, &mut editor, &mut world, &input, &mut history, |_| {});
    let swatch = swatch_center(&ui);
    click_at(&mut ui, &mut editor, &mut world, &mut input, &mut history, swatch);

    let target = editor
        .inspector_state
        .color_editor()
        .expect("clicking the swatch opens the colour editor");
    assert_eq!(target.component, "Sprite", "the target names its component, not an index");
    assert!(!history.can_undo(), "opening a popup is not an edit");

    let red = editor::color_editor_channel_id(0);
    let commit_channel = |ui: &mut UIContext,
                              editor: &mut EditorContext,
                              world: &mut World,
                              history: &mut CommandHistory,
                              typed: &str| {
        let mut typing = input::InputHandler::new();
        typing.keyboard_mut().handle_key_press(input::prelude::KeyCode::Enter);
        inspector_frame_on(ui, editor, world, &typing, history, |ui| {
            ui.focus_text_input(red, typed);
        });
    };

    commit_channel(&mut ui, &mut editor, &mut world, &mut history, "0.5");
    assert!(history.can_undo(), "the scrub reached the world through the row");
    let mut history_after_one = CommandHistory::new();
    std::mem::swap(&mut history, &mut history_after_one);
    assert_eq!(undo_all(&mut history_after_one, &mut world), 1, "one gesture is one undo entry");

    commit_channel(&mut ui, &mut editor, &mut world, &mut history, "0.5");
    commit_channel(&mut ui, &mut editor, &mut world, &mut history, "0.25");
    assert_eq!(undo_all(&mut history, &mut world), 2, "a second gesture is its own entry");
}

/// Undo everything the history holds; returns how many entries it took.
fn undo_all(history: &mut CommandHistory, world: &mut World) -> usize {
    let mut entries = 0;
    while history.undo(world) {
        entries += 1;
    }
    entries
}

/// Click the sprite's colour swatch open at `shape`; returns its bounds.
fn open_color_editor(
    ui: &mut UIContext,
    editor: &mut EditorContext,
    world: &mut World,
    input: &mut input::InputHandler,
    history: &mut CommandHistory,
    shape: FrameShape,
) -> ui::Rect {
    frame_on(ui, editor, world, input, history, shape, |_| {});
    let swatch = swatch_bounds(ui);
    let center = Vec2::new(swatch.x + swatch.width / 2.0, swatch.y + swatch.height / 2.0);
    click_at_in(ui, editor, world, input, history, shape, center);
    swatch
}

#[test]
fn test_a_drag_on_the_popup_edits_the_colour_and_never_the_row_underneath_it() {
    // The popup hangs over rows that were drawn before it. Blocking
    // rects are consulted at each widget's own interact call, so a
    // popup pushed mid-walk leaves those rows live and a channel drag
    // scrubs whichever one lies under the pointer.
    let mut editor = EditorContext::new();
    let mut world = world_with_selection(&mut editor);
    let mut history = CommandHistory::new();
    let mut input = input::InputHandler::new();
    let mut ui = UIContext::new();

    let entity = editor.selection.primary().expect("a selected entity");
    let shape = short_shape();
    let swatch = open_color_editor(&mut ui, &mut editor, &mut world, &mut input, &mut history, shape);
    let popup = editor::color_editor_bounds(swatch, shape.window);
    assert!(popup.y < swatch.y, "the fixture puts the popup above its swatch");
    assert!(popup.y < shape.bounds.y + shape.bounds.height, "and over the panel's rows");

    let before = *world.get::<common::Transform2D>(entity).expect("a transform");
    let color_before = world.get::<ecs::sprite_components::Sprite>(entity).expect("a sprite").color;

    // Press on the red channel and drag sideways: a scrub, not a click.
    let red = editor::color_editor_channel_bounds(popup, 0);
    let start = Vec2::new(red.x + red.width / 2.0, red.y + red.height / 2.0);
    input.update();
    input.mouse_mut().update_position(start.x, start.y);
    input.mouse_mut().handle_button_press(input::prelude::MouseButton::Left);
    frame_on(&mut ui, &mut editor, &mut world, &input, &mut history, shape, |_| {});
    for step in 1..=3 {
        input.update();
        // Leftwards: the fixture's colour is white, and a rightward
        // scrub on a channel already at its ceiling changes nothing.
        input.mouse_mut().update_position(start.x - 20.0 * step as f32, start.y);
        frame_on(&mut ui, &mut editor, &mut world, &input, &mut history, shape, |_| {});
    }
    input.update();
    input.mouse_mut().handle_button_release(input::prelude::MouseButton::Left);
    frame_on(&mut ui, &mut editor, &mut world, &input, &mut history, shape, |_| {});

    let after = *world.get::<common::Transform2D>(entity).expect("a transform");
    assert_eq!(after.position, before.position, "the row under the popup is inert");
    assert_eq!(after.rotation, before.rotation, "the row under the popup is inert");
    assert_eq!(after.scale, before.scale, "the row under the popup is inert");
    let color_after = world.get::<ecs::sprite_components::Sprite>(entity).expect("a sprite").color;
    assert_ne!(color_after, color_before, "the drag scrubbed the colour");
    assert_eq!(undo_all(&mut history, &mut world), 1, "one scrub is one undo entry");
}

#[test]
fn test_the_popup_stays_inside_the_window_and_draws_on_the_modal_band() {
    // The inspector is docked right; a popup that took the swatch's x
    // with a fixed width would hang off the window. And a Floating
    // popup cannot block a panel that is itself a Floating scope, which
    // is what the dock's narrow mode makes the inspector.
    let mut editor = EditorContext::new();
    let mut world = world_with_selection(&mut editor);
    let mut history = CommandHistory::new();
    let mut input = input::InputHandler::new();
    let mut ui = UIContext::new();

    let right_docked = ui::Rect::new(WINDOW.x - BOUNDS.width, 0.0, BOUNDS.width, BOUNDS.height);
    let swatch = ui::Rect::new(right_docked.x + 150.0, 200.0, 20.0, 20.0);
    let popup = editor::color_editor_bounds(swatch, WINDOW);
    assert!(popup.x >= 0.0 && popup.x + popup.width <= WINDOW.x, "the popup is on screen");

    open_color_editor(&mut ui, &mut editor, &mut world, &mut input, &mut history, FrameShape::default());
    let narrow = FrameShape { narrow: true, ..FrameShape::default() };
    frame_on(&mut ui, &mut editor, &mut world, &input, &mut history, narrow, |_| {});

    let drawn = editor::color_editor_bounds(
        editor.inspector_state.color_editor_frame().expect("an open editor").0,
        WINDOW,
    );
    let depth = ui
        .draw_list()
        .commands()
        .iter()
        .find_map(|command| match command {
            ui::DrawCommand::Rect { bounds, depth, .. }
                if (bounds.width - drawn.width).abs() < 0.01
                    && (bounds.height - drawn.height).abs() < 0.01 =>
            {
                Some(*depth)
            }
            _ => None,
        })
        .expect("the popup drew its own panel");
    assert!(
        depth >= ui::UiLayer::Modal.depth_base() && depth < ui::UiLayer::Tooltip.depth_base(),
        "the popup draws on the Modal band, above a narrow-mode panel's Floating scope"
    );

    // And the rest of the panel is still live: the popup's rect blocks
    // what it covers, not the scope it hangs over.
    let header = position_of(&ui, "▾ Transform2D").expect("the transform section's header");
    assert!(!drawn.contains(header), "the fixture's header is clear of the popup");
    click_at_in(
        &mut ui,
        &mut editor,
        &mut world,
        &mut input,
        &mut history,
        narrow,
        Vec2::new(header.x + 30.0, header.y + 8.0),
    );
    assert!(
        editor.inspector_state.is_collapsed("Transform2D"),
        "a row outside the popup still takes its click in narrow mode"
    );
}

#[test]
fn test_the_editor_follows_its_component_by_name_and_closes_when_that_component_goes() {
    // The target used to be the walk-time component index, so removing
    // a component above it re-aimed the open editor at whatever colour
    // row slid into that place.
    let mut editor = EditorContext::new();
    let mut world = world_with_selection(&mut editor);
    let mut history = CommandHistory::new();
    let mut input = input::InputHandler::new();
    let mut ui = UIContext::new();

    let entity = editor.selection.primary().expect("a selected entity");
    open_color_editor(&mut ui, &mut editor, &mut world, &mut input, &mut history, FrameShape::default());
    assert_eq!(
        editor.inspector_state.color_editor().map(|target| target.component.clone()),
        Some("Sprite".to_string())
    );

    // A component above the sprite goes: every index below it shifts.
    world.remove_component::<physics::components::RigidBody>(&entity).ok();
    inspector_frame_on(&mut ui, &mut editor, &mut world, &input, &mut history, |_| {});
    inspector_frame_on(&mut ui, &mut editor, &mut world, &input, &mut history, |_| {});
    assert_eq!(
        editor.inspector_state.color_editor().map(|target| target.component.clone()),
        Some("Sprite".to_string()),
        "the editor stays on the component it was opened from"
    );

    // The component itself goes: its row stops drawing, and the editor
    // has nothing left to hang off.
    world.remove_component::<ecs::sprite_components::Sprite>(&entity).ok();
    inspector_frame_on(&mut ui, &mut editor, &mut world, &input, &mut history, |_| {});
    inspector_frame_on(&mut ui, &mut editor, &mut world, &input, &mut history, |_| {});
    assert_eq!(editor.inspector_state.color_editor(), None, "the editor closed with its row");
}

#[test]
fn test_an_unparsable_hex_value_says_so_on_the_status_bar_and_keeps_the_colour() {
    let mut editor = EditorContext::new();
    let mut world = world_with_selection(&mut editor);
    let mut history = CommandHistory::new();
    let mut input = input::InputHandler::new();
    let mut ui = UIContext::new();

    let entity = editor.selection.primary().expect("a selected entity");
    open_color_editor(&mut ui, &mut editor, &mut world, &mut input, &mut history, FrameShape::default());
    let before = world.get::<ecs::sprite_components::Sprite>(entity).expect("a sprite").color;

    let hex = editor::color_editor_channel_id(4);
    let mut typing = input::InputHandler::new();
    typing.keyboard_mut().handle_key_press(input::prelude::KeyCode::Enter);
    inspector_frame_on(&mut ui, &mut editor, &mut world, &typing, &mut history, |ui| {
        ui.focus_text_input(hex, "#gggggg");
    });

    assert_eq!(
        editor.status_bar.message(),
        Some("Not a colour: #gggggg — use #rrggbb or #rrggbbaa"),
        "a rejected value reaches the status bar, like every other one"
    );
    assert_eq!(
        world.get::<ecs::sprite_components::Sprite>(entity).expect("a sprite").color,
        before,
        "and the colour is untouched"
    );
    assert!(!history.can_undo(), "a rejected value is not an edit");
}

/// The sprite colour row's swatch bounds.
fn swatch_bounds(ui: &UIContext) -> ui::Rect {
    let size = editor::EditableFieldStyle::default().color_preview_size;
    ui.draw_list()
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
        .expect("the sprite's colour row draws a swatch")
}

#[test]
fn test_a_click_away_commits_the_focused_field_before_the_popup_closes() {
    // A click outside the popup commits a focused field on that same press.
    // The row takes the edit in the walk that follows, and only then does
    // the popup close — and the keyboard goes with it.
    let mut editor = EditorContext::new();
    let mut world = world_with_selection(&mut editor);
    let mut history = CommandHistory::new();
    let mut input = input::InputHandler::new();
    let mut ui = UIContext::new();

    let entity = editor.selection.primary().expect("a selected entity");
    open_color_editor(&mut ui, &mut editor, &mut world, &mut input, &mut history, FrameShape::default());

    let hex = editor::color_editor_channel_id(4);
    inspector_frame_on(&mut ui, &mut editor, &mut world, &input, &mut history, |ui| {
        ui.focus_text_input(hex, "#ff0000");
    });
    assert!(ui.wants_keyboard(), "the hex field owns the keyboard");

    let outside = Vec2::new(WINDOW.x - 5.0, WINDOW.y - 5.0);
    click_at(&mut ui, &mut editor, &mut world, &mut input, &mut history, outside);

    assert_eq!(
        world.get::<ecs::sprite_components::Sprite>(entity).expect("a sprite").color,
        glam::Vec4::new(1.0, 0.0, 0.0, 1.0),
        "the click-away commit reached the world"
    );
    assert_eq!(undo_all(&mut history, &mut world), 1, "as one undo entry");
    assert!(editor.inspector_state.color_editor().is_none(), "and the popup closed");
    assert!(!ui.wants_keyboard(), "releasing the keyboard");
}

#[test]
fn test_a_confirm_dialog_closes_the_popup_and_a_close_leaves_another_fields_focus_alone() {
    // A dialog owns the frame, so the popup goes; and a close never takes
    // the keyboard from a field that is not its own — the one a click-away
    // landed on, or a rename in progress.
    let mut editor = EditorContext::new();
    let mut world = world_with_selection(&mut editor);
    let mut history = CommandHistory::new();
    let mut input = input::InputHandler::new();
    let mut ui = UIContext::new();

    open_color_editor(&mut ui, &mut editor, &mut world, &mut input, &mut history, FrameShape::default());
    assert!(editor.inspector_state.color_editor().is_some(), "the fixture opened the editor");
    inspector_frame_on(&mut ui, &mut editor, &mut world, &input, &mut history, |ui| {
        ui.focus_text_input("some_other_field", "typed");
    });
    let outside = Vec2::new(WINDOW.x - 5.0, WINDOW.y - 5.0);
    click_at(&mut ui, &mut editor, &mut world, &mut input, &mut history, outside);
    assert!(editor.inspector_state.color_editor().is_none(), "the click outside closed the popup");
    assert!(ui.is_focused("some_other_field"), "and left the other field's focus alone");

    open_color_editor(&mut ui, &mut editor, &mut world, &mut input, &mut history, FrameShape::default());
    let with_dialog = FrameShape { dialog_up: true, ..FrameShape::default() };
    frame_on(&mut ui, &mut editor, &mut world, &input, &mut history, with_dialog, |_| {});
    assert!(editor.inspector_state.color_editor().is_none(), "a confirm dialog closes the popup");
}
