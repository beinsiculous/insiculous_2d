use super::*;

#[test]
fn test_editor_context_new() {
    let ctx = EditorContext::new();
    assert!(ctx.selection.is_empty());
    assert_eq!(ctx.current_tool(), EditorTool::Select);
    assert_eq!(ctx.camera_offset(), Vec2::ZERO);
    assert_eq!(ctx.camera_zoom(), 1.0);
    assert!(ctx.is_grid_visible());
    assert!(!ctx.is_snap_to_grid());
    assert!(!ctx.in_play_session());
}

#[test]
fn test_editor_context_set_tool() {
    let mut ctx = EditorContext::new();

    ctx.set_tool(EditorTool::Move);
    assert_eq!(ctx.current_tool(), EditorTool::Move);
    assert_eq!(ctx.gizmo.mode(), GizmoMode::Translate);

    ctx.set_tool(EditorTool::Rotate);
    assert_eq!(ctx.current_tool(), EditorTool::Rotate);
    assert_eq!(ctx.gizmo.mode(), GizmoMode::Rotate);

    ctx.set_tool(EditorTool::Scale);
    assert_eq!(ctx.current_tool(), EditorTool::Scale);
    assert_eq!(ctx.gizmo.mode(), GizmoMode::Scale);

    ctx.set_tool(EditorTool::Select);
    assert_eq!(ctx.current_tool(), EditorTool::Select);
    assert_eq!(ctx.gizmo.mode(), GizmoMode::None);
}

#[test]
fn test_editor_context_camera() {
    let mut ctx = EditorContext::new();

    ctx.set_camera_offset(Vec2::new(100.0, 200.0));
    assert_eq!(ctx.camera_offset(), Vec2::new(100.0, 200.0));

    ctx.pan_camera(Vec2::new(50.0, -25.0));
    assert_eq!(ctx.camera_offset(), Vec2::new(150.0, 175.0));

    ctx.set_camera_zoom(2.0);
    assert_eq!(ctx.camera_zoom(), 2.0);

    ctx.zoom_camera(0.5);
    assert_eq!(ctx.camera_zoom(), 1.0);

    ctx.reset_camera();
    assert_eq!(ctx.camera_offset(), Vec2::ZERO);
    assert_eq!(ctx.camera_zoom(), 1.0);
}

#[test]
fn test_editor_context_camera_zoom_clamp() {
    let mut ctx = EditorContext::new();

    ctx.set_camera_zoom(0.01);
    assert_eq!(ctx.camera_zoom(), 0.1); // Clamped to min

    ctx.set_camera_zoom(100.0);
    assert_eq!(ctx.camera_zoom(), 10.0); // Clamped to max
}

#[test]
fn test_editor_context_coordinate_conversion() {
    let mut ctx = EditorContext::new();
    // Set up viewport bounds first
    ctx.update_layout(Vec2::new(800.0, 600.0));

    // Get viewport center
    let viewport_center = ctx.viewport.viewport_center();

    // No offset, no zoom: viewport center = world origin
    let world = ctx.screen_to_world(viewport_center);
    assert!((world.x).abs() < 0.01);
    assert!((world.y).abs() < 0.01);

    let screen = ctx.world_to_screen(Vec2::ZERO);
    assert!((screen.x - viewport_center.x).abs() < 0.01);
    assert!((screen.y - viewport_center.y).abs() < 0.01);

    // With offset
    ctx.set_camera_offset(Vec2::new(100.0, 50.0));
    let world = ctx.screen_to_world(viewport_center);
    assert!((world.x - 100.0).abs() < 0.01);
    assert!((world.y - 50.0).abs() < 0.01);
}

#[test]
fn test_editor_context_grid() {
    let mut ctx = EditorContext::new();

    assert!(ctx.is_grid_visible());
    ctx.toggle_grid();
    assert!(!ctx.is_grid_visible());
    ctx.set_grid_visible(true);
    assert!(ctx.is_grid_visible());

    ctx.set_grid_size(64.0);
    assert_eq!(ctx.grid_size(), 64.0);

    // Grid size minimum
    ctx.set_grid_size(0.5);
    assert_eq!(ctx.grid_size(), 1.0);
}

#[test]
fn test_editor_context_snap() {
    let mut ctx = EditorContext::new();
    ctx.set_grid_size(32.0);

    // Snap disabled
    let pos = Vec2::new(45.0, 78.0);
    assert_eq!(ctx.snap_position(pos), pos);

    // Snap enabled
    ctx.set_snap_to_grid(true);
    let snapped = ctx.snap_position(pos);
    // 45/32 = 1.4 rounds to 1 -> 32
    // 78/32 = 2.4 rounds to 2 -> 64
    assert_eq!(snapped, Vec2::new(32.0, 64.0));
}

#[test]
fn test_editor_context_play_state() {
    let mut ctx = EditorContext::new();

    // Default is editing
    assert!(ctx.is_editing());
    assert!(!ctx.in_play_session());
    assert_eq!(ctx.play_state(), EditorPlayState::Editing);

    // Enter play mode
    ctx.enter_play_mode();
    assert!(ctx.is_playing());
    assert!(ctx.in_play_session());

    // Exit play mode
    ctx.exit_play_mode();
    assert!(ctx.is_editing());
    assert!(!ctx.in_play_session());

    // Toggle into playing
    ctx.toggle_play_mode();
    assert!(ctx.is_playing());

    // Toggle back to editing
    ctx.toggle_play_mode();
    assert!(ctx.is_editing());

    // Set paused state directly
    ctx.set_play_state(EditorPlayState::Paused);
    assert!(ctx.is_paused());
    assert!(ctx.in_play_session());
}

#[test]
fn test_editor_context_default_panels() {
    let ctx = EditorContext::new();

    // Should have 4 default panels
    assert_eq!(ctx.dock_area.panels().len(), 4);

    // Check panel positions
    assert!(ctx.dock_area.get_panel(PanelId::HIERARCHY).is_some());
    assert!(ctx.dock_area.get_panel(PanelId::INSPECTOR).is_some());
    assert!(ctx.dock_area.get_panel(PanelId::SCENE_VIEW).is_some());
    assert!(ctx.dock_area.get_panel(PanelId::ASSET_BROWSER).is_some());
}

#[test]
fn test_editor_context_update_layout() {
    let mut ctx = EditorContext::new();
    ctx.update_layout(Vec2::new(1280.0, 720.0));

    // Scene view should have bounds set
    let scene_bounds = ctx.scene_view_bounds();
    assert!(scene_bounds.is_some());

    let bounds = scene_bounds.unwrap();
    assert!(bounds.width > 0.0);
    assert!(bounds.height > 0.0);
}

#[test]
fn test_gizmo_delta_to_world() {
    let mut ctx = EditorContext::new();

    // At zoom 1.0, X unchanged but Y inverted (screen down = world up)
    let delta = Vec2::new(100.0, 50.0);
    let world_delta = ctx.gizmo_delta_to_world(delta);
    assert_eq!(world_delta, Vec2::new(100.0, -50.0));

    // At zoom 2.0, delta is halved and Y inverted
    ctx.set_camera_zoom(2.0);
    let world_delta = ctx.gizmo_delta_to_world(delta);
    assert_eq!(world_delta, Vec2::new(50.0, -25.0));
}

#[test]
fn test_gizmo_has_priority() {
    let ctx = EditorContext::new();

    // Gizmo not active by default
    assert!(!ctx.gizmo_has_priority());
}

#[test]
fn test_new_context_tool_and_gizmo_consistent() {
    // Startup regression: the toolbar defaults to Select but the gizmo used
    // to default to Translate — a gizmo rendered with no tool selected.
    let ctx = EditorContext::new();
    assert_eq!(ctx.current_tool(), EditorTool::Select);
    assert_eq!(ctx.gizmo.mode(), GizmoMode::None);
}

#[test]
fn test_add_component_popup_default_closed() {
    let ctx = EditorContext::new();
    assert!(!ctx.is_add_component_popup_open());
}

#[test]
fn test_add_component_popup_toggle() {
    let mut ctx = EditorContext::new();
    ctx.toggle_add_component_popup();
    assert!(ctx.is_add_component_popup_open());
    ctx.toggle_add_component_popup();
    assert!(!ctx.is_add_component_popup_open());
    ctx.toggle_add_component_popup();
    ctx.close_add_component_popup();
    assert!(!ctx.is_add_component_popup_open());
}

#[test]
fn test_gizmo_screen_position() {
    let mut ctx = EditorContext::new();
    ctx.update_layout(Vec2::new(800.0, 600.0));

    // Set gizmo at world origin
    ctx.gizmo.set_position(Vec2::ZERO);

    // Should be at viewport center
    let screen_pos = ctx.gizmo_screen_position();
    let viewport_center = ctx.viewport.viewport_center();
    assert!((screen_pos.x - viewport_center.x).abs() < 0.01);
    assert!((screen_pos.y - viewport_center.y).abs() < 0.01);
}

#[test]
fn test_dirty_flag_default_false() {
    let ctx = EditorContext::new();
    assert!(!ctx.is_dirty());
}

#[test]
fn test_mark_dirty_sets_flag() {
    let mut ctx = EditorContext::new();
    ctx.mark_dirty();
    assert!(ctx.is_dirty());
    ctx.set_dirty(false);
    assert!(!ctx.is_dirty());
}

#[test]
fn test_scene_path_default_none() {
    let ctx = EditorContext::new();
    assert!(ctx.scene_path().is_none());
}

#[test]
fn test_scene_display_name_untitled() {
    let ctx = EditorContext::new();
    assert_eq!(ctx.scene_display_name(), "Untitled");
}

#[test]
fn test_scene_display_name_with_path() {
    let mut ctx = EditorContext::new();
    ctx.set_scene_path(Some(std::path::PathBuf::from("/scenes/my_level.ron")));
    assert_eq!(ctx.scene_display_name(), "my_level.ron");
}

#[test]
fn test_title_bar_text_clean() {
    let mut ctx = EditorContext::new();
    ctx.set_scene_path(Some(std::path::PathBuf::from("/scenes/test.ron")));
    assert_eq!(ctx.title_bar_text(), "test.ron - Insiculous Editor");
}

#[test]
fn test_title_bar_text_dirty() {
    let mut ctx = EditorContext::new();
    ctx.set_scene_path(Some(std::path::PathBuf::from("/scenes/test.ron")));
    ctx.mark_dirty();
    assert_eq!(ctx.title_bar_text(), "test.ron* - Insiculous Editor");
}

#[test]
fn test_title_bar_text_untitled_dirty() {
    let mut ctx = EditorContext::new();
    ctx.mark_dirty();
    assert_eq!(ctx.title_bar_text(), "Untitled* - Insiculous Editor");
}

#[test]
fn test_collider_overlay_visible_by_default_and_toggles() {
    let mut ctx = EditorContext::new();
    assert!(ctx.is_colliders_visible());

    ctx.toggle_colliders();
    assert!(!ctx.is_colliders_visible());
    ctx.toggle_colliders();
    assert!(ctx.is_colliders_visible());

    ctx.set_colliders_visible(false);
    assert!(!ctx.is_colliders_visible());
}

// ---- Frame selected / frame all (issue #21) ----

fn pickable(id: u64, pos: Vec2, size: Vec2) -> crate::PickableEntity {
    crate::PickableEntity::new(ecs::EntityId::with_generation(id, 1), pos, size, 0.0)
}

fn framing_ctx() -> EditorContext {
    let mut ctx = EditorContext::new();
    ctx.viewport
        .set_viewport_bounds(common::Rect::new(0.0, 0.0, 800.0, 600.0));
    ctx
}

#[test]
fn test_frame_selected_centers_on_selected_entities_only() {
    let mut ctx = framing_ctx();
    let pickables = vec![
        pickable(1, Vec2::new(200.0, 100.0), Vec2::splat(80.0)),
        pickable(2, Vec2::new(-900.0, -700.0), Vec2::splat(80.0)),
    ];
    ctx.selection.select(pickables[0].entity_id);

    assert!(ctx.frame_selected(&pickables));

    // Only the selected entity's bounds count — the far-away one is ignored.
    assert_eq!(
        ctx.viewport.target_camera_position(),
        Vec2::new(200.0, 100.0)
    );
}

#[test]
fn test_frame_selected_zooms_to_fit_single_entity_extents() {
    let mut ctx = framing_ctx();
    let pickables = vec![pickable(1, Vec2::ZERO, Vec2::splat(80.0))];
    ctx.selection.select(pickables[0].entity_id);

    ctx.frame_selected(&pickables);

    // Corners at ±40 give a 80×80 bounds, so zoom-to-fit engages:
    // min(800/(80+100), 600/(80+100)) = 600/180, clamped to ≤10.
    assert!((ctx.viewport.target_camera_zoom() - 600.0 / 180.0).abs() < 0.001);
}

#[test]
fn test_frame_selected_with_empty_selection_frames_all() {
    let mut ctx = framing_ctx();
    let pickables = vec![
        pickable(1, Vec2::new(-100.0, 0.0), Vec2::splat(80.0)),
        pickable(2, Vec2::new(300.0, 200.0), Vec2::splat(80.0)),
    ];

    assert!(ctx.frame_selected(&pickables));

    assert_eq!(
        ctx.viewport.target_camera_position(),
        Vec2::new(100.0, 100.0)
    );
}

#[test]
fn test_frame_all_covers_every_entity_regardless_of_selection() {
    let mut ctx = framing_ctx();
    let pickables = vec![
        pickable(1, Vec2::new(-200.0, -100.0), Vec2::splat(80.0)),
        pickable(2, Vec2::new(400.0, 300.0), Vec2::splat(80.0)),
    ];
    ctx.selection.select(pickables[0].entity_id);

    assert!(ctx.frame_all(&pickables));

    assert_eq!(
        ctx.viewport.target_camera_position(),
        Vec2::new(100.0, 100.0)
    );
}

#[test]
fn test_framing_empty_scene_leaves_camera_targets_unchanged() {
    let mut ctx = framing_ctx();
    ctx.viewport.set_target_camera_position(Vec2::new(5.0, 6.0));

    assert!(!ctx.frame_selected(&[]));
    assert!(!ctx.frame_all(&[]));

    assert_eq!(ctx.viewport.target_camera_position(), Vec2::new(5.0, 6.0));
    assert_eq!(ctx.viewport.target_camera_zoom(), 1.0);
}

#[test]
fn test_framing_flipped_scale_entity_uses_absolute_extents() {
    let mut ctx = framing_ctx();
    let pickables = vec![pickable(1, Vec2::ZERO, Vec2::new(-80.0, 80.0))];
    ctx.selection.select(pickables[0].entity_id);

    ctx.frame_selected(&pickables);

    // A negative (flipped) scale must not produce inverted bounds.
    assert_eq!(ctx.viewport.target_camera_position(), Vec2::ZERO);
    assert!((ctx.viewport.target_camera_zoom() - 600.0 / 180.0).abs() < 0.001);
}
