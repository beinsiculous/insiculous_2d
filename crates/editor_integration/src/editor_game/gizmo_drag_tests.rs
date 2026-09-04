//! Gizmo drag tests: apply / commit / cancel / snapping / collider scaling.

use ecs::World;
use glam::Vec2;
use physics::components::{Collider, ColliderShape};

use super::gizmo_drag::scale_collider;
use super::test_support::{
    drag_state_for, editor_game, position, scale_interaction, spawn_at, translate_interaction,
};

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
