//! Viewport interaction against the real drag state and picking path:
//! chrome owns the mouse through the release frame, and picking AABBs match
//! the `RENDER_UNIT`-scaled render.

use ecs::{GlobalTransform2D, World};
use glam::Vec2;

use super::test_support::editor_game;
use super::viewport_interaction::{build_pickable_entities, chrome_owns_mouse};

// ---------------------------------------------------------------------------
// Picking
// ---------------------------------------------------------------------------

#[test]
fn test_chrome_owns_mouse_through_the_release_frame_and_under_an_overlay() {
    use input::prelude::MouseButton;

    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    let btn = ui::Rect::new(10.0, 10.0, 80.0, 20.0);
    let window = Vec2::new(1280.0, 720.0);

    // No gesture, no overlay: the viewport owns the mouse.
    ui.begin_frame(&input, window);
    assert!(!chrome_owns_mouse(&ui));
    ui.end_frame();

    // Press on a chrome widget (toolbar/play-control style button).
    input.mouse_mut().update_position(50.0, 20.0);
    input.mouse_mut().handle_button_press(MouseButton::Left);
    ui.begin_frame(&input, window);
    ui.button("chrome_btn", "Play", btn);
    assert!(chrome_owns_mouse(&ui), "widget press must keep picking away");
    ui.end_frame();

    // Release frame — the frame ViewportInputResult.clicked fires on, so
    // the guard MUST still hold here or the toolbar click repicks beneath.
    input.update();
    input.mouse_mut().handle_button_release(MouseButton::Left);
    ui.begin_frame(&input, window);
    ui.button("chrome_btn", "Play", btn);
    assert!(chrome_owns_mouse(&ui), "release frame is when picking decides");
    ui.end_frame();

    // Gesture over: picking is free again.
    input.update();
    ui.begin_frame(&input, window);
    assert!(!chrome_owns_mouse(&ui));
    ui.end_frame();

    // An open overlay (menu dropdown) under the cursor swallows the click.
    ui.begin_frame(&input, window);
    ui.begin_overlay(ui::Rect::new(0.0, 0.0, 100.0, 100.0));
    ui.end_overlay();
    assert!(chrome_owns_mouse(&ui), "an open dropdown swallows viewport clicks");
    ui.end_frame();
}

#[test]
fn test_pickables_need_sprite_and_global_transform_and_match_the_rendered_size() {
    let mut world = World::new();
    let both = world.create_entity();
    world
        .add_component(
            &both,
            GlobalTransform2D { position: Vec2::new(100.0, 200.0), scale: Vec2::splat(2.0), ..Default::default() },
        )
        .ok();
    let mut sprite = ecs::sprite_components::Sprite::new(0);
    sprite.scale = Vec2::splat(0.5);
    sprite.depth = 5.0;
    world.add_component(&both, sprite).ok();
    let transform_only = world.create_entity();
    world.add_component(&transform_only, GlobalTransform2D::default()).ok();
    let sprite_only = world.create_entity();
    world.add_component(&sprite_only, ecs::sprite_components::Sprite::new(0)).ok();

    let pickables = build_pickable_entities(&world);

    assert_eq!(pickables.len(), 1, "a sprite without a global transform (or vice versa) is unpickable");
    assert_eq!(pickables[0].entity_id, both);
    assert_eq!(pickables[0].position, Vec2::new(100.0, 200.0));
    // Size matches the render path: sprite.scale * transform.scale *
    // RENDER_UNIT = (0.5, 0.5) * (2, 2) * 80 = (80, 80) pixels.
    assert_eq!(pickables[0].size, Vec2::new(80.0, 80.0));
    assert_eq!(pickables[0].depth, 5.0);
}

#[test]
fn test_pick_hits_sprite_at_rendered_size_with_offset_panel() {
    // Regression for two shipped bugs at once:
    // 1. pick size ignored RENDER_UNIT (AABBs 80x smaller than sprites)
    // 2. picking must work with a NONZERO panel origin (dock chrome)
    let mut world = World::new();
    let entity = world.create_entity();
    world
        .add_component(&entity, GlobalTransform2D { position: Vec2::new(100.0, 50.0), ..Default::default() })
        .ok();
    // Unit transform + unit sprite scale renders as an 80x80px sprite.
    world.add_component(&entity, ecs::sprite_components::Sprite::new(0)).ok();
    let mut viewport = editor::SceneViewport::new();
    viewport.set_viewport_bounds(common::Rect::new(300.0, 100.0, 800.0, 600.0));
    let pickables = build_pickable_entities(&world);
    let mut picker = editor::EntityPicker::new();

    // Click 30px off-center — inside the rendered 80x80 sprite, but a miss
    // with the old 1x1 pick AABB.
    let click = viewport.world_to_screen(Vec2::new(100.0, 50.0)) + Vec2::new(30.0, 30.0);
    let result = picker.pick_at_screen_pos(&viewport, click, &pickables);
    assert_eq!(result.topmost(), Some(entity));

    // A click well outside the sprite still misses.
    let miss = viewport.world_to_screen(Vec2::new(100.0, 50.0)) + Vec2::new(90.0, 0.0);
    let result = picker.pick_at_screen_pos(&viewport, miss, &pickables);
    assert_eq!(result.topmost(), None);
}

// ---------------------------------------------------------------------------
// Marquee
// ---------------------------------------------------------------------------

/// An EditorGame with a laid-out viewport and one pickable sprite at
/// world origin (screen center (400, 300), 80px square).
fn marquee_rig() -> (super::EditorGame<super::test_support::DummyGame>, World, ecs::EntityId) {
    let mut game = editor_game();
    game.editor
        .viewport
        .set_viewport_bounds(common::Rect::new(0.0, 0.0, 800.0, 600.0));
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, GlobalTransform2D::default()).ok();
    world.add_component(&entity, ecs::sprite_components::Sprite::new(0)).ok();
    (game, world, entity)
}

const HIT_START: Vec2 = Vec2::new(350.0, 250.0);
const HIT_END: Vec2 = Vec2::new(450.0, 350.0);

#[test]
fn test_marquee_modifiers_match_click_selection() {
    let (mut game, mut world, entity) = marquee_rig();
    let far = world.create_entity();
    world
        .add_component(&far, GlobalTransform2D { position: Vec2::new(1000.0, 0.0), ..Default::default() })
        .ok();
    world.add_component(&far, ecs::sprite_components::Sprite::new(0)).ok();

    // Plain: replaces. A marquee over empty space clears.
    let pickables = super::viewport_interaction::build_pickable_entities(&world);
    game.editor.selection.select(far);
    game.apply_marquee_selection(&pickables, HIT_START, HIT_END, false, false);
    assert!(game.editor.selection.contains(entity));
    assert!(!game.editor.selection.contains(far), "a plain marquee replaces");
    game.apply_marquee_selection(&pickables, Vec2::new(700.0, 500.0), Vec2::new(780.0, 580.0), false, false);
    assert!(game.editor.selection.is_empty());

    // Shift: adds without clearing.
    game.editor.selection.select(far);
    game.apply_marquee_selection(&pickables, HIT_START, HIT_END, true, false);
    assert!(game.editor.selection.contains(entity), "shift adds the hits");
    assert!(game.editor.selection.contains(far), "shift keeps the rest");

    // Ctrl: toggles each hit, never destructively.
    game.apply_marquee_selection(&pickables, HIT_START, HIT_END, false, true);
    assert!(!game.editor.selection.contains(entity), "ctrl deselects a selected hit");
    assert!(game.editor.selection.contains(far));
    game.apply_marquee_selection(&pickables, HIT_START, HIT_END, false, true);
    assert!(game.editor.selection.contains(entity), "ctrl re-selects it");
}

#[test]
fn test_live_marquee_draws_a_clipped_fill_and_border() {
    let (mut game, _world, _entity) = marquee_rig();
    // Lay out the dock so scene_view_bounds() exists.
    game.editor.update_layout(Vec2::new(1280.0, 720.0));
    assert!(game.editor.scene_view_bounds().is_some());
    let mut ui = ui::UIContext::new();

    // Drag up-and-left: the emitted rect must normalize to min/max.
    game.draw_marquee(&mut ui, Vec2::new(300.0, 300.0), Vec2::new(250.0, 200.0));

    let commands = ui.draw_list().commands();
    let expected = (250.0, 200.0, 50.0, 100.0);
    let fill = commands
        .iter()
        .find_map(|c| match c {
            ui::DrawCommand::Rect { bounds, .. } => Some(*bounds),
            _ => None,
        })
        .expect("the rubber-band fill must render");
    let border = commands
        .iter()
        .find_map(|c| match c {
            ui::DrawCommand::RectBorder { bounds, .. } => Some(*bounds),
            _ => None,
        })
        .expect("the rubber-band border must render");
    assert_eq!(
        (fill.x, fill.y, fill.width, fill.height),
        expected,
        "corners normalize whichever direction the drag went"
    );
    assert_eq!((border.x, border.y, border.width, border.height), expected);
    assert!(commands.iter().any(|c| matches!(c, ui::DrawCommand::PushClipRect { .. })));
    assert!(commands.iter().any(|c| matches!(c, ui::DrawCommand::PopClipRect)));
}

#[test]
fn test_viewport_click_selects_while_editing_and_nothing_while_playing() {
    use super::test_support::{press_mouse, release_mouse, ui_frame};

    let (mut game, mut world, entity) = marquee_rig();
    let window = Vec2::new(800.0, 600.0);
    let click_position = Vec2::new(400.0, 300.0);

    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();

    // The list is rebuilt for every frame, as `update` does.
    // 1. While Editing: press frame then release frame selects the entity.
    press_mouse(&mut input, click_position);
    ui_frame(&mut ui, &input, window, |_| {});
    let pickables = build_pickable_entities(&world);
    game.handle_viewport_picking(&mut ui, &input, &mut world, &pickables);
    assert!(game.editor.selection.is_empty(), "press alone does not select yet");

    release_mouse(&mut input);
    ui_frame(&mut ui, &input, window, |_| {});
    let pickables = build_pickable_entities(&world);
    game.handle_viewport_picking(&mut ui, &input, &mut world, &pickables);
    assert_eq!(game.editor.selection.primary(), Some(entity), "release frame completes the click pick");

    // 2. While Playing: the same two frames leave selection untouched.
    game.editor.selection.clear();
    game.editor.set_play_state(editor::EditorPlayState::Playing);

    press_mouse(&mut input, click_position);
    ui_frame(&mut ui, &input, window, |_| {});
    let pickables = build_pickable_entities(&world);
    game.handle_viewport_picking(&mut ui, &input, &mut world, &pickables);

    release_mouse(&mut input);
    ui_frame(&mut ui, &input, window, |_| {});
    let pickables = build_pickable_entities(&world);
    game.handle_viewport_picking(&mut ui, &input, &mut world, &pickables);
    assert!(game.editor.selection.is_empty(), "picking is completely disabled while playing");
}
