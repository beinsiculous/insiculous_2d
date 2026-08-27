//! Viewport picking tests: the chrome-owns-mouse guard and pickable-entity
//! construction (position/size must match the render path).

use ecs::GlobalTransform2D;
use glam::Vec2;

use super::viewport_interaction::{build_pickable_entities, chrome_owns_mouse};

#[test]
fn test_chrome_owns_mouse_while_widget_holds_the_gesture() {
    use input::prelude::MouseButton;

    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    let btn = ui::Rect::new(10.0, 10.0, 80.0, 20.0);

    // No gesture, no overlay: the viewport owns the mouse
    ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
    assert!(!chrome_owns_mouse(&ui));
    ui.end_frame();

    // Press on a chrome widget (toolbar/play-control style button)
    input.mouse_mut().update_position(50.0, 20.0);
    input.mouse_mut().handle_button_press(MouseButton::Left);
    ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
    ui.button("chrome_btn", "Play", btn);
    assert!(chrome_owns_mouse(&ui), "widget press must keep picking away");
    ui.end_frame();

    // Release frame — the frame ViewportInputResult.clicked fires on, so
    // the guard MUST still hold here or the toolbar click repicks beneath.
    input.update();
    input.mouse_mut().handle_button_release(MouseButton::Left);
    ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
    ui.button("chrome_btn", "Play", btn);
    assert!(chrome_owns_mouse(&ui), "release frame is when picking decides");
    ui.end_frame();

    // Gesture over: picking is free again
    input.update();
    ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
    assert!(!chrome_owns_mouse(&ui));
    ui.end_frame();
}

#[test]
fn test_chrome_owns_mouse_under_open_overlay() {
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    input.mouse_mut().update_position(50.0, 50.0);
    ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
    ui.begin_overlay(ui::Rect::new(0.0, 0.0, 100.0, 100.0));
    ui.end_overlay();
    assert!(chrome_owns_mouse(&ui), "an open dropdown swallows viewport clicks");
    ui.end_frame();
}

#[test]
fn test_build_pickable_entities_with_both_components() {
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    world.add_component(&entity, GlobalTransform2D {
        position: Vec2::new(100.0, 200.0),
        scale: Vec2::new(2.0, 2.0),
        ..Default::default()
    }).ok();
    let mut sprite = ecs::sprite_components::Sprite::new(0);
    sprite.scale = Vec2::new(0.5, 0.5);
    sprite.depth = 5.0;
    world.add_component(&entity, sprite).ok();

    let pickables = build_pickable_entities(&world);
    assert_eq!(pickables.len(), 1);
    assert_eq!(pickables[0].entity_id, entity);
    assert_eq!(pickables[0].position, Vec2::new(100.0, 200.0));
    // Size matches the render path: sprite.scale * transform.scale *
    // RENDER_UNIT = (0.5, 0.5) * (2, 2) * 80 = (80, 80) pixels
    assert_eq!(pickables[0].size, Vec2::new(80.0, 80.0));
    assert_eq!(pickables[0].depth, 5.0);
}

#[test]
fn test_pick_hits_sprite_at_rendered_size_with_offset_panel() {
    // Regression for two shipped bugs at once:
    // 1. pick size ignored RENDER_UNIT (AABBs 80x smaller than sprites)
    // 2. picking must work with a NONZERO panel origin (dock chrome)
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    world.add_component(&entity, GlobalTransform2D {
        position: Vec2::new(100.0, 50.0),
        ..Default::default()
    }).ok();
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

#[test]
fn test_build_pickable_entities_skips_without_sprite() {
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    // Only GlobalTransform2D, no Sprite
    world.add_component(&entity, GlobalTransform2D::default()).ok();

    let pickables = build_pickable_entities(&world);
    assert!(pickables.is_empty());
}

#[test]
fn test_build_pickable_entities_skips_without_global_transform() {
    let mut world = ecs::World::new();
    let entity = world.create_entity();
    // Only Sprite, no GlobalTransform2D
    world.add_component(&entity, ecs::sprite_components::Sprite::new(0)).ok();

    let pickables = build_pickable_entities(&world);
    assert!(pickables.is_empty());
}

#[test]
fn test_build_pickable_entities_multiple() {
    let mut world = ecs::World::new();

    // Entity 1
    let e1 = world.create_entity();
    world.add_component(&e1, GlobalTransform2D {
        position: Vec2::new(10.0, 20.0),
        ..Default::default()
    }).ok();
    let mut sprite1 = ecs::sprite_components::Sprite::new(0);
    sprite1.depth = 1.0;
    world.add_component(&e1, sprite1).ok();

    // Entity 2
    let e2 = world.create_entity();
    world.add_component(&e2, GlobalTransform2D {
        position: Vec2::new(50.0, 60.0),
        ..Default::default()
    }).ok();
    let mut sprite2 = ecs::sprite_components::Sprite::new(1);
    sprite2.depth = 3.0;
    world.add_component(&e2, sprite2).ok();

    // Entity 3 — no sprite, should be excluded
    let e3 = world.create_entity();
    world.add_component(&e3, GlobalTransform2D::default()).ok();

    let pickables = build_pickable_entities(&world);
    assert_eq!(pickables.len(), 2);

    let ids: Vec<_> = pickables.iter().map(|p| p.entity_id).collect();
    assert!(ids.contains(&e1));
    assert!(ids.contains(&e2));
    assert!(!ids.contains(&e3));
}

