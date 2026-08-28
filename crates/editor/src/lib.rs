//! Visual scene editor for the insiculous_2d game engine.
//!
//! This crate provides a visual editor for building game worlds, editing
//! entity properties, and managing scene hierarchies. The editor is built
//! on top of the existing immediate-mode UI system and integrates with
//! the engine's ECS, rendering, and asset systems.
//!
//! # Features
//! - Dockable panel system (Scene view, Inspector, Hierarchy, Asset browser)
//! - Entity selection and manipulation (Select, Move, Rotate, Scale)
//! - Visual transform gizmos with grid snapping
//! - Scene saving/loading with editor state preservation
//! - Component property editing with automatic UI generation
//!
//! # Example
//! ```
//! use editor::EditorContext;
//! use ecs::World;
//!
//! // EditorContext holds all editor state: selection, tools, play state,
//! // theme, and panels. The `editor_integration` crate wires it to a
//! // running game via `run_game_with_editor()`.
//! let mut editor = EditorContext::new();
//! let mut world = World::new();
//! let entity = world.create_entity();
//!
//! editor.selection.select(entity);
//! assert_eq!(editor.selection.primary(), Some(entity));
//! assert!(editor.is_editing()); // starts in Editing play state
//! ```

mod asset_browser;
pub mod command_api;
mod behavior_editor;
mod collider_overlay;
mod drag_drop;
pub mod commands;
mod component_editors;
mod composite_rows;
mod context;
mod dock;
mod editable_inspector;
mod field_style;
mod row_layout;
pub mod fonts;
mod editor_input;
mod gizmo;
mod gizmo_math;
mod grid;
mod hierarchy;
#[cfg(test)]
mod hierarchy_tests;
mod inspector;
mod menu;
mod picking;
mod play_controls;
mod play_state;
mod scroll;
mod selection;
mod selection_outline;
pub mod status_bar;
pub mod stored_component;
mod text_field;
mod texture_field;
mod ui_component_editors;
pub mod theme;
pub mod typography;
mod toolbar;
mod viewport;
mod viewport_input;
pub mod editor_preferences;
pub mod layout;
pub mod world_snapshot;

// Re-export main types
pub use asset_browser::{fit_rect, scan_assets, AssetBrowserState, AssetEntry, AssetKind};
pub use behavior_editor::edit_behavior;
pub use drag_drop::{DragDropState, DragPayload, DRAG_THRESHOLD};
pub use texture_field::{edit_texture_field, InspectorExtras};
pub use collider_overlay::{
    collider_outline_segments, render_collider_overlay, ColliderOverlayColors,
};
pub use commands::{CommandHistory, EditorCommand};
pub use component_editors::{
    apply_component_edit, edit_audio_source, edit_collider, edit_rigid_body, edit_sprite,
    edit_transform2d, ComponentEdit,
};
pub use context::EditorContext;
pub use editor_preferences::{EditorPreferences, PanelPrefs};
pub use dock::{panel_id_for_menu_label, DockArea, DockPanel, DockPosition, PanelId};
pub use editable_inspector::{
    component_header, cycle_step, edit_bool, edit_color, edit_f32,
    edit_normalized_f32, edit_vec2, EditableFieldStyle, EditableInspector, EditResult, FieldId,
};
pub use row_layout::{
    color_block_height, ellipsize, field_row, pair_slots, remove_button_x, PairSlot, RowLayout,
};
pub use text_field::{display_string, display_u32, edit_string};
pub use ui_component_editors::{edit_ui_button, edit_ui_label, edit_ui_panel};
pub use editor_input::{EditorAction, EditorInputMapping, EditorInputState};
pub use gizmo::{Gizmo, GizmoMode, GizmoPalette};
pub use hierarchy::{normalized_rename, HierarchyPanel, HierarchyResponse, NameResolution};
pub use grid::{GridColors, GridConfig, GridRenderer};
pub use inspector::{component_value, inspect_component, InspectorStyle};
pub use menu::{Menu, MenuBar, MenuItem};
pub use picking::{EntityPicker, PickResult, PickableEntity, SelectionRect, AABB};
pub use play_controls::{PlayControlAction, PlayControls};
pub use play_state::EditorPlayState;
pub use scroll::ScrollState;
pub use selection::Selection;
pub use selection_outline::{
    hover_entity_at, outline_segments, render_selection_outline, SelectionOutlineColors,
};
pub use status_bar::{StatusBar, StatusBarStats, STATUS_BAR_HEIGHT};
pub use stored_component::{
    available_components, capture_all_components, categorized_components,
    edit_all_components, inspect_all_components, registered_component_type_ids,
    restore_components, ComponentCategory, ComponentKind, StoredComponent,
};
pub use theme::EditorTheme;
pub use toolbar::{toolbar_position_for, EditorTool, Toolbar};
pub use viewport::SceneViewport;
pub use viewport_input::{ViewportInputConfig, ViewportInputHandler, ViewportInputResult};

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::{
        collider_outline_segments, render_collider_overlay, ColliderOverlayColors,
        available_components, capture_all_components, categorized_components,
        inspect_all_components, restore_components, CommandHistory, ComponentCategory,
        ComponentEdit, ComponentKind, EditorCommand, StoredComponent,
        component_header, display_u32, edit_audio_source, edit_bool, edit_collider, edit_color,
        edit_f32, edit_normalized_f32, edit_rigid_body, edit_sprite, edit_transform2d, edit_vec2,
        inspect_component, panel_id_for_menu_label, toolbar_position_for, DockArea, DockPanel,
        DockPosition, EditorAction, EditorContext, EditorInputMapping, EditorInputState,
        EditorPlayState, EditorPreferences, EditorTool, EditableFieldStyle, EditableInspector,
        EditorTheme, EditResult, EntityPicker, FieldId, Gizmo, GizmoMode, GridRenderer,
        HierarchyPanel, InspectorStyle, Menu, MenuBar, MenuItem, PanelId, PickResult,
        PickableEntity, StatusBar, StatusBarStats, STATUS_BAR_HEIGHT,
        PlayControlAction, PlayControls, SceneViewport, Selection,
        SelectionRect, Toolbar, ViewportInputConfig,
        ViewportInputHandler, ViewportInputResult, AABB,
    };
}
