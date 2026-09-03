//! EditorContext contracts an editor user notices: the tool ↔ gizmo pairing,
//! grid snapping, the default dock layout, the window title, the gizmo's
//! screen placement and drag mapping, and F / Shift+F framing.

use super::*;
use crate::test_support::{pickable, test_viewport};
use std::path::PathBuf;

fn framing_ctx() -> EditorContext {
    let mut ctx = EditorContext::new();
    ctx.viewport = test_viewport();
    ctx
}

#[test]
fn test_each_tool_shows_its_gizmo_and_a_fresh_editor_starts_with_move() {
    let mut ctx = EditorContext::new();
    // Startup invariant: the current tool and the gizmo mode agree, and the
    // default is Move so a gizmo shows as soon as something is selected.
    assert_eq!((ctx.current_tool(), ctx.gizmo.mode()), (EditorTool::Move, GizmoMode::Translate));

    for (tool, mode) in [
        (EditorTool::Rotate, GizmoMode::Rotate),
        (EditorTool::Scale, GizmoMode::Scale),
        (EditorTool::Select, GizmoMode::None),
        (EditorTool::Move, GizmoMode::Translate),
    ] {
        ctx.set_tool(tool);
        assert_eq!((ctx.current_tool(), ctx.gizmo.mode()), (tool, mode));
    }
}

#[test]
fn test_snapping_rounds_to_the_nearest_grid_cell_only_while_enabled() {
    let mut ctx = EditorContext::new();
    ctx.set_grid_size(32.0);
    let pos = Vec2::new(45.0, 78.0);

    assert_eq!(ctx.snap_position(pos), pos, "snap off: positions pass through");

    ctx.set_snap_to_grid(true);
    // 45/32 = 1.4 rounds to 1 → 32; 78/32 = 2.4 rounds to 2 → 64.
    assert_eq!(ctx.snap_position(pos), Vec2::new(32.0, 64.0));
}

#[test]
fn test_default_layout_docks_hierarchy_left_inspector_right_scene_center_assets_bottom() {
    let ctx = EditorContext::new();

    let layout: Vec<(PanelId, DockPosition)> = ctx.dock_area.panels().iter().map(|p| (p.id, p.position)).collect();

    assert_eq!(
        layout,
        [
            (PanelId::HIERARCHY, DockPosition::Left),
            (PanelId::INSPECTOR, DockPosition::Right),
            (PanelId::SCENE_VIEW, DockPosition::Center),
            (PanelId::ASSET_BROWSER, DockPosition::Bottom),
        ]
    );
}

#[test]
fn test_title_bar_names_the_scene_and_marks_unsaved_changes_with_a_star() {
    for (path, dirty, expected) in [
        (None, false, "Untitled - Insiculous Editor"),
        (None, true, "Untitled* - Insiculous Editor"),
        (Some("/scenes/test.ron"), false, "test.ron - Insiculous Editor"),
        (Some("/scenes/test.ron"), true, "test.ron* - Insiculous Editor"),
    ] {
        let mut ctx = EditorContext::new();
        ctx.set_scene_path(path.map(PathBuf::from));
        ctx.set_dirty(dirty);
        assert_eq!(ctx.title_bar_text(), expected, "path {path:?} dirty {dirty}");
    }
}

#[test]
fn test_gizmo_sits_over_its_world_position_and_drags_map_screen_y_down_to_world_y_up() {
    let mut ctx = EditorContext::new();
    ctx.update_layout(Vec2::new(800.0, 600.0));
    ctx.gizmo.set_position(Vec2::ZERO);

    let screen = ctx.gizmo_screen_position();
    assert!((screen - ctx.viewport.viewport_center()).length() < 0.01, "world origin draws at the panel center");

    // A screen drag of (100, 50): X unchanged, Y inverted, divided by zoom.
    assert_eq!(ctx.gizmo_delta_to_world(Vec2::new(100.0, 50.0)), Vec2::new(100.0, -50.0));
    ctx.set_camera_zoom(2.0);
    assert_eq!(ctx.gizmo_delta_to_world(Vec2::new(100.0, 50.0)), Vec2::new(50.0, -25.0));
}

#[test]
fn test_frame_selected_centers_on_selected_entities_only_and_empty_selection_frames_all() {
    let mut ctx = framing_ctx();
    let pickables = [
        pickable(1, Vec2::new(200.0, 100.0), Vec2::splat(80.0), 0.0),
        pickable(2, Vec2::new(-900.0, -700.0), Vec2::splat(80.0), 0.0),
    ];
    ctx.selection.select(pickables[0].entity_id);

    assert!(ctx.frame_selected(&pickables));
    assert_eq!(
        ctx.viewport.target_camera_position(),
        Vec2::new(200.0, 100.0),
        "only the selected entity's bounds count"
    );

    // Nothing selected: F is the "take me back to my entities" key.
    ctx.selection.clear();
    assert!(ctx.frame_selected(&pickables));
    assert_eq!(ctx.viewport.target_camera_position(), Vec2::new(-350.0, -300.0));

    // Shift+F covers everything regardless of the selection.
    ctx.selection.select(pickables[0].entity_id);
    assert!(ctx.frame_all(&pickables));
    assert_eq!(ctx.viewport.target_camera_position(), Vec2::new(-350.0, -300.0));
}

#[test]
fn test_framing_zooms_to_fit_the_entity_extents_and_an_empty_scene_leaves_the_camera_alone() {
    let mut ctx = framing_ctx();
    let single = [pickable(1, Vec2::ZERO, Vec2::splat(80.0), 0.0)];
    ctx.selection.select(single[0].entity_id);

    ctx.frame_selected(&single);
    // Corners at ±40 give an 80×80 bounds, so zoom-to-fit engages:
    // min(800/(80+100), 600/(80+100)) = 600/180.
    assert!((ctx.viewport.target_camera_zoom() - 600.0 / 180.0).abs() < 0.001);

    // A negative (flipped) scale must not produce inverted bounds.
    let flipped = [pickable(1, Vec2::ZERO, Vec2::new(-80.0, 80.0), 0.0)];
    ctx.frame_selected(&flipped);
    assert_eq!(ctx.viewport.target_camera_position(), Vec2::ZERO);
    assert!((ctx.viewport.target_camera_zoom() - 600.0 / 180.0).abs() < 0.001);

    ctx.viewport.set_target_camera_position(Vec2::new(5.0, 6.0));
    ctx.viewport.set_target_zoom(1.0);
    assert!(!ctx.frame_selected(&[]), "nothing to frame reports no camera move");
    assert!(!ctx.frame_all(&[]));
    assert_eq!(ctx.viewport.target_camera_position(), Vec2::new(5.0, 6.0));
    assert_eq!(ctx.viewport.target_camera_zoom(), 1.0);
}
