//! Viewport interaction against the real drag state and picking path:
//! a gizmo drag applies idempotently and commits ONE undo entry (transform
//! AND collider — physics ignores `Transform2D.scale`), snapping steps
//! cells without eating residuals, Escape restores everything, chrome owns
//! the mouse through the release frame, and picking AABBs match the
//! `RENDER_UNIT`-scaled render.

use ecs::{GlobalTransform2D, World};
use glam::Vec2;
use physics::components::{Collider, ColliderShape};

use super::test_support::{
    drag_state_for, editor_game, position, scale_interaction, spawn_at, translate_interaction,
};
use super::viewport_interaction::{build_pickable_entities, chrome_owns_mouse, scale_collider};

// ---------------------------------------------------------------------------
// Gizmo drag: apply / commit / cancel
// ---------------------------------------------------------------------------

#[test]
fn test_drag_apply_is_idempotent_and_commit_records_one_entry_restoring_every_root() {
    let mut game = editor_game();
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::new(0.0, 0.0));
    let b = spawn_at(&mut world, Vec2::new(100.0, 0.0));

    // A zero-delta click is a drag that moved nothing: no history entry.
    game.gizmo_drag = Some(drag_state_for(&world, &[a, b]));
    game.commit_gizmo_drag(&world);
    assert!(!game.command_history.can_undo(), "nothing changed, nothing recorded");

    // The same cumulative interaction applied twice lands once: every
    // frame is `start + delta`, never `+=`. Screen +y is down, so a
    // (32, 16) screen drag is a (32, -16) world move at zoom 1.
    game.gizmo_drag = Some(drag_state_for(&world, &[a, b]));
    let interaction = translate_interaction(Vec2::new(32.0, 16.0));
    game.apply_gizmo_drag(&mut world, &interaction, false);
    game.apply_gizmo_drag(&mut world, &interaction, false);
    assert_eq!(position(&world, a), Vec2::new(32.0, -16.0));
    assert_eq!(position(&world, b), Vec2::new(132.0, -16.0));

    game.commit_gizmo_drag(&world);
    assert!(game.gizmo_drag.is_none());

    // ONE undo restores BOTH entities to their drag-start positions.
    assert!(game.command_history.undo(&mut world));
    assert_eq!(position(&world, a), Vec2::new(0.0, 0.0));
    assert_eq!(position(&world, b), Vec2::new(100.0, 0.0));
    assert!(!game.command_history.can_undo(), "a multi-entity drag is exactly one history entry");
}

#[test]
fn test_scale_drag_rebuilds_the_collider_and_undoes_it_with_the_transform_as_one_entry() {
    // Colliders are absolute-pixel sized and ignore Transform2D.scale (the
    // top footgun in CLAUDE.md): the scale tool must resize the collider
    // with the sprite, and one Ctrl+Z must revert both.
    let mut game = editor_game();
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::ZERO);
    let mut original = Collider::box_collider(80.0, 40.0); // half extents (40, 20)
    original.offset = Vec2::new(10.0, -5.0);
    world.add_component(&a, original.clone()).ok();

    game.gizmo_drag = Some(drag_state_for(&world, &[a]));
    let interaction = scale_interaction(Vec2::new(2.0, 3.0));
    game.apply_gizmo_drag(&mut world, &interaction, false);
    game.apply_gizmo_drag(&mut world, &interaction, false);

    assert_eq!(world.get::<common::Transform2D>(a).map(|t| t.scale), Some(Vec2::new(2.0, 3.0)));
    let scaled = world.get::<Collider>(a).cloned().expect("collider");
    assert_eq!(scaled.shape, ColliderShape::Box { half_extents: Vec2::new(80.0, 60.0) });
    assert_eq!(scaled.offset, Vec2::new(20.0, -15.0), "the body-local offset scales too");

    game.commit_gizmo_drag(&world);
    assert!(game.command_history.can_undo(), "the scale drag was recorded");

    assert!(game.command_history.undo(&mut world), "one Ctrl+Z reverts the whole drag");
    assert_eq!(world.get::<common::Transform2D>(a).map(|t| t.scale), Some(Vec2::ONE));
    assert_eq!(world.get::<Collider>(a).cloned(), Some(original));
    assert!(!game.command_history.can_undo(), "transform + collider are ONE entry");
}

#[test]
fn test_collider_scaling_keeps_circles_round_and_capsules_axis_aligned() {
    let mut circle = Collider::circle_collider(10.0);
    scale_collider(&mut circle, Vec2::new(1.5, 2.0));
    assert_eq!(circle.shape, ColliderShape::Circle { radius: 20.0 }, "dominant axis factor");

    let mut capsule = Collider::default();
    capsule.shape = ColliderShape::CapsuleY { half_height: 10.0, radius: 4.0 };
    scale_collider(&mut capsule, Vec2::new(2.0, 3.0));
    assert_eq!(
        capsule.shape,
        ColliderShape::CapsuleY { half_height: 30.0, radius: 8.0 },
        "height follows y, radius follows x"
    );
}

#[test]
fn test_slow_snapped_drag_steps_grid_cells_instead_of_freezing() {
    let mut game = editor_game();
    game.editor.set_snap_to_grid(true);
    game.editor.set_grid_size(32.0);
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::new(5.0, 0.0));
    game.gizmo_drag = Some(drag_state_for(&world, &[a]));

    // 20 frames of a slow +2px/frame drag (cumulative screen x = 2..=40;
    // screen x maps 1:1 to world x at zoom 1). The old writeback snapped
    // the accumulated position every frame, so snap(p + 2) == p forever
    // and the entity NEVER moved.
    for frame in 1..=20 {
        let interaction = translate_interaction(Vec2::new(2.0 * frame as f32, 0.0));
        game.apply_gizmo_drag(&mut world, &interaction, false);
    }

    // The anchor (5 + 40 = 45) snaps to the nearest 32-multiple.
    assert_eq!(position(&world, a), Vec2::new(32.0, 0.0));
}

#[test]
fn test_snapped_multi_drag_keeps_formation_offsets_with_the_pref_or_ctrl() {
    let mut world = World::new();
    let primary = spawn_at(&mut world, Vec2::new(5.0, 0.0));
    let other = spawn_at(&mut world, Vec2::new(17.0, 3.0));

    for (label, pref, ctrl_held) in [("snap pref", true, false), ("Ctrl held", false, true)] {
        let mut game = editor_game();
        game.editor.set_snap_to_grid(pref);
        game.editor.set_grid_size(32.0);
        game.gizmo_drag = Some(drag_state_for(&world, &[primary, other]));

        game.apply_gizmo_drag(&mut world, &translate_interaction(Vec2::new(40.0, 0.0)), ctrl_held);

        let p = position(&world, primary);
        assert_eq!(p, Vec2::new(32.0, 0.0), "{label}: the primary anchor snaps");
        assert_eq!(
            position(&world, other) - p,
            Vec2::new(12.0, 3.0),
            "{label}: formation offsets survive — only the shared delta is snapped"
        );
        assert!(game.cancel_gizmo_drag(&mut world), "{label}: reset for the next case");
    }
}

#[test]
fn test_cancel_restores_starts_and_pushes_no_undo_entry() {
    let mut game = editor_game();
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::new(5.0, 5.0));
    let b = spawn_at(&mut world, Vec2::new(50.0, 5.0));
    world.add_component(&b, Collider::box_collider(16.0, 16.0)).ok();
    game.gizmo_drag = Some(drag_state_for(&world, &[a, b]));

    // Mid-drag: entities moved and scaled, b's collider resized with it.
    game.apply_gizmo_drag(&mut world, &translate_interaction(Vec2::new(300.0, 0.0)), false);
    game.apply_gizmo_drag(&mut world, &scale_interaction(Vec2::splat(2.0)), false);
    assert_ne!(world.get::<Collider>(b).cloned(), Some(Collider::box_collider(16.0, 16.0)));

    // Escape.
    assert!(game.cancel_gizmo_drag(&mut world));
    assert_eq!(position(&world, a), Vec2::new(5.0, 5.0));
    assert_eq!(
        world.get::<common::Transform2D>(b).map(|t| (t.position, t.scale)),
        Some((Vec2::new(50.0, 5.0), Vec2::ONE))
    );
    assert_eq!(
        world.get::<Collider>(b).cloned(),
        Some(Collider::box_collider(16.0, 16.0)),
        "the collider rolls back with the transform"
    );
    assert!(!game.command_history.can_undo(), "a cancelled drag leaves no history");
    assert!(!game.cancel_gizmo_drag(&mut world), "second Escape has nothing to cancel");

    // A commit after the cancel is a no-op (state was taken).
    game.commit_gizmo_drag(&world);
    assert!(!game.command_history.can_undo());
}

#[test]
fn test_gizmo_drag_commit_and_cancel_seal_the_nudge_merge_window() {
    // Mergeable commands on either side of a gizmo drag must never
    // collapse into one undo entry across it — commit AND cancel are
    // gesture boundaries.
    let mut game = editor_game();
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::ZERO);
    game.editor.selection.select(a);

    // Nudge, then a zero-delta drag commit (still a gesture), then nudge.
    game.nudge_selection(&mut world, Vec2::new(1.0, 0.0), false);
    game.gizmo_drag = Some(drag_state_for(&world, &[a]));
    game.commit_gizmo_drag(&world);
    game.nudge_selection(&mut world, Vec2::new(1.0, 0.0), false);

    // A cancelled drag is a boundary too.
    game.gizmo_drag = Some(drag_state_for(&world, &[a]));
    assert!(game.cancel_gizmo_drag(&mut world));
    game.nudge_selection(&mut world, Vec2::new(1.0, 0.0), false);

    // Three separate nudge entries — one per gesture window.
    assert!(game.command_history.undo(&mut world));
    assert!(game.command_history.undo(&mut world));
    assert!(game.command_history.undo(&mut world));
    assert!(
        !game.command_history.can_undo(),
        "each drag boundary sealed the previous nudge into its own entry"
    );
    assert_eq!(position(&world, a), Vec2::ZERO);
}

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
    game.editor.selection.select(far);
    game.apply_marquee_selection(&world, HIT_START, HIT_END, false, false);
    assert!(game.editor.selection.contains(entity));
    assert!(!game.editor.selection.contains(far), "a plain marquee replaces");
    game.apply_marquee_selection(&world, Vec2::new(700.0, 500.0), Vec2::new(780.0, 580.0), false, false);
    assert!(game.editor.selection.is_empty());

    // Shift: adds without clearing.
    game.editor.selection.select(far);
    game.apply_marquee_selection(&world, HIT_START, HIT_END, true, false);
    assert!(game.editor.selection.contains(entity), "shift adds the hits");
    assert!(game.editor.selection.contains(far), "shift keeps the rest");

    // Ctrl: toggles each hit, never destructively.
    game.apply_marquee_selection(&world, HIT_START, HIT_END, false, true);
    assert!(!game.editor.selection.contains(entity), "ctrl deselects a selected hit");
    assert!(game.editor.selection.contains(far));
    game.apply_marquee_selection(&world, HIT_START, HIT_END, false, true);
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
