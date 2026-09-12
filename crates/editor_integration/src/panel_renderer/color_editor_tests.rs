//! The colour editor's contracts: the swatch opens it, a gesture is one undo
//! entry, a drag over the rows beneath it edits the colour and nothing else,
//! it stays inside the window on the Modal band, it follows its component
//! by name, and its fields take a click-away commit, a typed value and a
//! rejected hex value the way the rest of the inspector does.

use super::inspector_tests::{
    click_at, click_at_in, frame_on, inspector_frame_on, position_of, short_shape, swatch_center,
    world_with_selection, FrameShape, BOUNDS, WINDOW,
};
use ecs::World;
use editor::{CommandHistory, EditorContext};
use glam::Vec2;
use ui::UIContext;

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

#[test]
fn test_a_click_on_a_popup_channel_focuses_it_and_a_typed_value_commits() {
    typed_channel_value_commits(editor::EditorPlayState::Editing);
}

#[test]
fn test_a_paused_session_takes_a_typed_channel_value_too() {
    // Paused is editable: the popup opens and a typed value lands the same
    // way it does while Editing.
    typed_channel_value_commits(editor::EditorPlayState::Paused);
}

/// A channel is a field before it is a slider: a click with no drag
/// focuses it with its value selected, and typing replaces it.
fn typed_channel_value_commits(play_state: editor::EditorPlayState) {
    let mut editor = EditorContext::new();
    editor.set_play_state(play_state);
    let mut world = world_with_selection(&mut editor);
    let mut history = CommandHistory::new();
    let mut input = input::InputHandler::new();
    let mut ui = UIContext::new();

    let entity = editor.selection.primary().expect("a selected entity");
    let swatch = open_color_editor(&mut ui, &mut editor, &mut world, &mut input, &mut history, FrameShape::default());
    let popup = editor::color_editor_bounds(swatch, WINDOW);
    let green = editor::color_editor_channel_bounds(popup, 1);
    let centre = Vec2::new(green.x + green.width / 2.0, green.y + green.height / 2.0);
    click_at(&mut ui, &mut editor, &mut world, &mut input, &mut history, centre);

    let green_id = editor::color_editor_channel_id(1);
    assert!(ui.is_focused(green_id), "a click without a drag focuses the channel");
    assert!(ui.wants_keyboard(), "and the keyboard is the field's");

    let mut typing = input::InputHandler::new();
    typing.keyboard_mut().handle_key_press(input::prelude::KeyCode::Enter);
    inspector_frame_on(&mut ui, &mut editor, &mut world, &typing, &mut history, |ui| {
        ui.focus_text_input(green_id, "0.25");
    });
    let color = world.get::<ecs::sprite_components::Sprite>(entity).expect("a sprite").color;
    assert!((color.y - 0.25).abs() < 1e-6, "the typed value reached the world: {color:?}");
}
