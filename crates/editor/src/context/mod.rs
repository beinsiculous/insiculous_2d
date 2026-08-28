//! Editor context extending the game context.
//!
//! The EditorContext wraps a GameContext and adds editor-specific state
//! like selection, gizmos, and editor camera.

use glam::Vec2;

use crate::{
    editor_input::EditorInputMapping,
    grid::GridRenderer,
    hierarchy::HierarchyPanel,
    picking::EntityPicker,
    play_controls::PlayControls,
    play_state::EditorPlayState,
    status_bar::StatusBar,
    theme::EditorTheme,
    viewport::SceneViewport,
    viewport_input::ViewportInputHandler,
    DockArea, DockPanel, DockPosition, EditorTool, Gizmo, GizmoMode, MenuBar, PanelId, Selection,
    Toolbar,
};

/// Editor-specific state that extends the game context.
#[derive(Debug)]
pub struct EditorContext {
    /// Current entity selection
    pub selection: Selection,
    /// Transform gizmo
    pub gizmo: Gizmo,
    /// Editor toolbar
    pub toolbar: Toolbar,
    /// Menu bar
    pub menu_bar: MenuBar,
    /// Dock area for panels
    pub dock_area: DockArea,
    /// Scene viewport for rendering and navigation
    pub viewport: SceneViewport,
    /// Grid renderer for scene view
    pub grid: GridRenderer,
    /// Entity picker for selection
    pub picker: EntityPicker,
    /// Viewport input handler
    pub viewport_input: ViewportInputHandler,
    /// Editor input mapping
    pub input_mapping: EditorInputMapping,
    /// Hierarchy panel for entity tree view
    pub hierarchy: HierarchyPanel,
    /// Snap to grid enabled
    snap_to_grid: bool,
    /// Whether collider outlines are drawn in the scene view
    show_colliders: bool,
    /// Current play state (Editing / Playing / Paused)
    play_state: EditorPlayState,
    /// Play / Pause / Stop controls widget
    pub play_controls: PlayControls,
    /// Whether the add-component popup is open in the inspector.
    add_component_popup_open: bool,
    /// Whether the scene has unsaved changes
    is_dirty: bool,
    /// Current scene file path (None = untitled/new scene)
    scene_path: Option<std::path::PathBuf>,
    /// Centralized design-system theme
    pub theme: EditorTheme,
    /// Status bar at the bottom of the editor
    pub status_bar: StatusBar,
    /// Cross-panel drag-and-drop coordinator
    pub drag_drop: crate::DragDropState,
    /// Asset browser panel state (scan results, scroll)
    pub asset_browser: crate::AssetBrowserState,
    /// The editor's own UI faces (regular/bold/mono), loaded at init from
    /// the crate-shipped bytes — never from the opened project's assets.
    pub fonts: crate::fonts::EditorFonts,
    /// Vertical scroll for the inspector panel (audit §3.3). Lives here
    /// because the inspector has no panel struct of its own.
    pub inspector_scroll: crate::ScrollState,
    /// The entity the inspector scroll belongs to — a selection change
    /// resets the scroll so each entity opens at the top.
    pub inspector_scroll_entity: Option<ecs::EntityId>,
}

impl Default for EditorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// The editor's default panel layout: Hierarchy left, Inspector right,
/// Scene center, Asset browser bottom.
fn default_dock_area() -> DockArea {
    let mut dock_area = DockArea::new();
    dock_area.add_panel(
        DockPanel::new(PanelId::HIERARCHY, "Hierarchy", DockPosition::Left)
            .with_size(200.0)
            .with_min_size(150.0),
    );
    dock_area.add_panel(
        DockPanel::new(PanelId::INSPECTOR, "Inspector", DockPosition::Right)
            .with_size(280.0)
            .with_min_size(200.0),
    );
    dock_area.add_panel(DockPanel::new(
        PanelId::SCENE_VIEW,
        "Scene",
        DockPosition::Center,
    ));
    dock_area.add_panel(
        DockPanel::new(PanelId::ASSET_BROWSER, "Assets", DockPosition::Bottom)
            .with_size(180.0)
            .with_min_size(100.0),
    );
    dock_area
}

impl EditorContext {
    /// Create a new editor context with default settings.
    pub fn new() -> Self {
        let theme = EditorTheme::default();
        let mut gizmo = Gizmo::new();
        gizmo.apply_theme(&theme);

        let mut editor = Self {
            selection: Selection::new(),
            gizmo,
            // Position is set every frame from the scene view bounds
            // (toolbar_position_for) — the default only covers frame 0.
            toolbar: Toolbar::new().with_position(Vec2::new(220.0, 54.0)),
            menu_bar: MenuBar::editor_default(),
            dock_area: default_dock_area(),
            viewport: SceneViewport::new(),
            grid: GridRenderer::new(),
            picker: EntityPicker::new(),
            viewport_input: ViewportInputHandler::new(),
            input_mapping: EditorInputMapping::new(),
            hierarchy: HierarchyPanel::new(),
            snap_to_grid: false,
            show_colliders: true,
            play_state: EditorPlayState::default(),
            play_controls: PlayControls::new(),
            add_component_popup_open: false,
            is_dirty: false,
            scene_path: None,
            theme,
            status_bar: StatusBar::new(),
            fonts: crate::fonts::EditorFonts::default(),
            inspector_scroll: crate::ScrollState::default(),
            inspector_scroll_entity: None,
            drag_drop: crate::DragDropState::new(),
            asset_browser: crate::AssetBrowserState::default(),
        };
        // Run the tool→gizmo mapping once so startup state is consistent
        // whatever the defaults are (today both are Move/Translate).
        editor.set_tool(editor.current_tool());
        editor
    }

    // ================== Tool Methods ==================

    /// Get the currently selected editor tool.
    pub fn current_tool(&self) -> EditorTool {
        self.toolbar.current_tool()
    }

    /// Set the current editor tool.
    pub fn set_tool(&mut self, tool: EditorTool) {
        self.toolbar.set_tool(tool);

        // Update gizmo mode to match tool
        let gizmo_mode = match tool {
            EditorTool::Select => GizmoMode::None,
            EditorTool::Move => GizmoMode::Translate,
            EditorTool::Rotate => GizmoMode::Rotate,
            EditorTool::Scale => GizmoMode::Scale,
        };
        self.gizmo.set_mode(gizmo_mode);
    }

    // ================== Camera Methods ==================
    // These delegate to the SceneViewport for camera control

    /// Get the camera offset (position).
    pub fn camera_offset(&self) -> Vec2 {
        self.viewport.camera_position()
    }

    /// Set the camera offset (position).
    pub fn set_camera_offset(&mut self, offset: Vec2) {
        self.viewport.set_camera_position(offset);
    }

    /// Pan the camera by a delta.
    pub fn pan_camera(&mut self, delta: Vec2) {
        self.viewport.pan_immediate(delta);
    }

    /// Get the camera zoom level.
    pub fn camera_zoom(&self) -> f32 {
        self.viewport.camera_zoom()
    }

    /// Set the camera zoom level.
    pub fn set_camera_zoom(&mut self, zoom: f32) {
        self.viewport.set_camera_zoom(zoom);
    }

    /// Zoom the camera by a factor.
    pub fn zoom_camera(&mut self, factor: f32) {
        let new_zoom = self.viewport.camera_zoom() * factor;
        self.viewport.set_camera_zoom(new_zoom);
    }

    /// Reset camera to default view.
    pub fn reset_camera(&mut self) {
        self.viewport.reset_camera_immediate();
    }

    /// Convert screen position to world position.
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        self.viewport.screen_to_world(screen_pos)
    }

    /// Convert world position to screen position.
    pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2 {
        self.viewport.world_to_screen(world_pos)
    }

    // ================== Grid Methods ==================
    // These delegate to the GridRenderer

    /// Check if the grid is visible.
    pub fn is_grid_visible(&self) -> bool {
        self.grid.is_visible()
    }

    /// Set grid visibility.
    pub fn set_grid_visible(&mut self, visible: bool) {
        self.grid.set_visible(visible);
    }

    /// Toggle grid visibility.
    pub fn toggle_grid(&mut self) {
        self.grid.toggle_visible();
    }

    /// Get the grid size.
    pub fn grid_size(&self) -> f32 {
        self.grid.grid_size()
    }

    /// Set the grid size.
    pub fn set_grid_size(&mut self, size: f32) {
        self.grid.set_grid_size(size);
    }

    // ================== Collider Overlay Methods ==================

    /// Check if collider outlines are visible in the scene view.
    pub fn is_colliders_visible(&self) -> bool {
        self.show_colliders
    }

    /// Set collider outline visibility.
    pub fn set_colliders_visible(&mut self, visible: bool) {
        self.show_colliders = visible;
    }

    /// Toggle collider outline visibility.
    pub fn toggle_colliders(&mut self) {
        self.show_colliders = !self.show_colliders;
    }

    /// Check if snap to grid is enabled.
    pub fn is_snap_to_grid(&self) -> bool {
        self.snap_to_grid
    }

    /// Set snap to grid.
    pub fn set_snap_to_grid(&mut self, snap: bool) {
        self.snap_to_grid = snap;
    }

    /// Toggle snap to grid.
    pub fn toggle_snap_to_grid(&mut self) {
        self.snap_to_grid = !self.snap_to_grid;
    }

    /// Snap a position to the grid.
    pub fn snap_position(&self, pos: Vec2) -> Vec2 {
        if self.snap_to_grid {
            let grid_size = self.grid.grid_size();
            Vec2::new(
                (pos.x / grid_size).round() * grid_size,
                (pos.y / grid_size).round() * grid_size,
            )
        } else {
            pos
        }
    }

    // ================== Play State Methods ==================

    /// Get the current play state.
    pub fn play_state(&self) -> EditorPlayState {
        self.play_state
    }

    /// Set the play state.
    pub fn set_play_state(&mut self, state: EditorPlayState) {
        self.play_state = state;
    }

    /// Whether the editor is in normal editing mode.
    pub fn is_editing(&self) -> bool {
        self.play_state.is_editing()
    }

    /// Whether the game simulation is actively running.
    pub fn is_playing(&self) -> bool {
        self.play_state.is_playing()
    }

    /// Whether the game is paused.
    pub fn is_paused(&self) -> bool {
        self.play_state.is_paused()
    }

    /// Whether a play session is active (Playing or Paused).
    pub fn in_play_session(&self) -> bool {
        self.play_state.in_play_session()
    }

    /// Enter play mode (sets state to Playing).
    pub fn enter_play_mode(&mut self) {
        self.play_state = EditorPlayState::Playing;
    }

    /// Exit play mode (sets state to Editing).
    pub fn exit_play_mode(&mut self) {
        self.play_state = EditorPlayState::Editing;
    }

    /// Toggle between Editing and Playing.
    pub fn toggle_play_mode(&mut self) {
        self.play_state = if self.play_state.is_editing() {
            EditorPlayState::Playing
        } else {
            EditorPlayState::Editing
        };
    }

    // ================== Add Component Popup ==================

    /// Whether the add-component popup is currently open.
    pub fn is_add_component_popup_open(&self) -> bool {
        self.add_component_popup_open
    }

    /// Toggle the add-component popup open/closed.
    pub fn toggle_add_component_popup(&mut self) {
        self.add_component_popup_open = !self.add_component_popup_open;
    }

    /// Close the add-component popup.
    pub fn close_add_component_popup(&mut self) {
        self.add_component_popup_open = false;
    }

    // ================== Scene State Methods ==================

    /// Whether the scene has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// Set the dirty flag.
    ///
    /// This is a per-frame MIRROR of the source of truth,
    /// `CommandHistory::is_dirty()` (synced by `EditorGame::update`) —
    /// don't set it ad hoc from feature code; record a command instead.
    pub fn set_dirty(&mut self, dirty: bool) {
        self.is_dirty = dirty;
    }

    /// Get the current scene file path, if any.
    pub fn scene_path(&self) -> Option<&std::path::Path> {
        self.scene_path.as_deref()
    }

    /// Set the current scene file path.
    pub fn set_scene_path(&mut self, path: Option<std::path::PathBuf>) {
        self.scene_path = path;
    }

    /// Get a display name for the current scene.
    ///
    /// Returns the file name if a path is set, otherwise "Untitled".
    pub fn scene_display_name(&self) -> String {
        self.scene_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    /// Get the title bar text including dirty indicator.
    ///
    /// Format: "filename.ron* - Insiculous Editor" (with * if dirty)
    pub fn title_bar_text(&self) -> String {
        let name = self.scene_display_name();
        let dirty_indicator = if self.is_dirty { "*" } else { "" };
        format!("{}{} - Insiculous Editor", name, dirty_indicator)
    }

    // ================== Layout Methods ==================

    /// Update layout based on window size.
    ///
    /// This should be called each frame before rendering to ensure
    /// panels are properly sized and viewport bounds are updated.
    pub fn update_layout(&mut self, window_size: Vec2) {
        // Reserve space for menu bar (top) and status bar (bottom)
        let menu_height = self.menu_bar.height();
        let status_bar_height = crate::status_bar::STATUS_BAR_HEIGHT;
        let content_bounds = common::Rect::new(
            0.0,
            menu_height,
            window_size.x,
            window_size.y - menu_height - status_bar_height,
        );

        self.dock_area.set_bounds(content_bounds);
        self.dock_area.layout();

        // Update viewport bounds from scene view panel
        if let Some(scene_bounds) = self.scene_view_bounds() {
            self.viewport.set_viewport_bounds(scene_bounds);
        }

        // Note: The gizmo position should be set by the caller based on entity transform
        // after calling update_layout, using self.selection.primary() to get the selected entity
    }

    /// Update viewport interpolation. Call each frame.
    pub fn update_viewport(&mut self, delta_time: f32) {
        self.viewport.update(delta_time);
    }

    /// Reset the dock layout (panel sizes, visibility, collapse state) to
    /// the editor defaults.
    pub fn reset_layout(&mut self) {
        self.dock_area = default_dock_area();
    }

    /// Get the scene view content bounds (where the game world is rendered).
    ///
    /// Returns `None` when the panel is hidden or collapsed (no content area).
    pub fn scene_view_bounds(&self) -> Option<common::Rect> {
        self.panel_content_bounds(PanelId::SCENE_VIEW)
    }

    /// Get the inspector panel bounds (`None` when hidden or collapsed).
    pub fn inspector_bounds(&self) -> Option<common::Rect> {
        self.panel_content_bounds(PanelId::INSPECTOR)
    }

    /// Get the hierarchy panel bounds (`None` when hidden or collapsed).
    pub fn hierarchy_bounds(&self) -> Option<common::Rect> {
        self.panel_content_bounds(PanelId::HIERARCHY)
    }

    /// Content bounds of a panel, or `None` when it is hidden or collapsed.
    fn panel_content_bounds(&self, id: PanelId) -> Option<common::Rect> {
        self.dock_area
            .get_panel(id)
            .filter(|p| p.visible && !(p.collapsed && p.is_collapsible()))
            .map(|p| p.content_bounds())
    }

    // ================== Gizmo Integration ==================

    /// Get the screen position for the gizmo based on its world position.
    ///
    /// Use this to pass to gizmo.render().
    pub fn gizmo_screen_position(&self) -> Vec2 {
        self.viewport.world_to_screen(self.gizmo.position())
    }

    /// Convert a gizmo delta from screen space to world space.
    ///
    /// The gizmo returns deltas in screen pixels. This converts them
    /// to world units accounting for camera zoom and Y-axis inversion
    /// (screen Y increases downward, world Y increases upward).
    pub fn gizmo_delta_to_world(&self, screen_delta: Vec2) -> Vec2 {
        Vec2::new(
            screen_delta.x / self.viewport.camera_zoom(),
            -screen_delta.y / self.viewport.camera_zoom(), // Negate Y for world coords
        )
    }

    /// Check if the gizmo should take priority over picking.
    ///
    /// Returns true if the gizmo is currently being interacted with,
    /// meaning picking should be skipped.
    pub fn gizmo_has_priority(&self) -> bool {
        self.gizmo.is_active()
    }

    /// Frame the current selection in the viewport (the F shortcut).
    ///
    /// Feeds both AABB corners of every selected pickable into
    /// [`SceneViewport::focus_on_bounds`], so zoom-to-fit accounts for
    /// entity extents — a single selected entity still gets a sensible
    /// zoom instead of a zero-size bounds. With nothing selected this
    /// falls back to framing everything: F is the "I'm lost, take me
    /// back to my entities" key. Returns whether the camera moved.
    pub fn frame_selected(&mut self, pickables: &[crate::PickableEntity]) -> bool {
        let selected: Vec<&crate::PickableEntity> = pickables
            .iter()
            .filter(|p| self.selection.contains(p.entity_id))
            .collect();
        if selected.is_empty() {
            return self.frame_all(pickables);
        }
        self.frame_pickables(&selected)
    }

    /// Frame every pickable entity in the viewport (the Shift+F shortcut).
    /// Returns whether the camera moved (false for an empty scene).
    pub fn frame_all(&mut self, pickables: &[crate::PickableEntity]) -> bool {
        self.frame_pickables(&pickables.iter().collect::<Vec<_>>())
    }

    fn frame_pickables(&mut self, pickables: &[&crate::PickableEntity]) -> bool {
        let corners: Vec<Vec2> = pickables
            .iter()
            .flat_map(|p| {
                let aabb = p.aabb();
                [aabb.min, aabb.max]
            })
            .collect();
        if corners.is_empty() {
            return false;
        }
        self.viewport.focus_on_bounds(&corners);
        true
    }
}

#[cfg(test)]
mod tests;
