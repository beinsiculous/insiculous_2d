//! Dockable panel system for the editor.
//!
//! Provides a flexible layout system with dockable panels that can be
//! positioned at different edges of the window or floated. Panels can be
//! hidden, collapsed to a slim strip, and resized; rendering lives in
//! [`render`] (chrome, collapse chevrons, resize grabbers).

use ui::{Rect, WidgetId};

use crate::layout::{DEFAULT_PANEL_WIDTH, HEADER_HEIGHT, MIN_PANEL_SIZE, RESIZE_HANDLE_SIZE};

mod render;

#[cfg(test)]
mod tests;

/// Unique identifier for a dock panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelId(pub u32);

impl PanelId {
    /// Scene view panel (main viewport)
    pub const SCENE_VIEW: PanelId = PanelId(0);
    /// Entity inspector panel
    pub const INSPECTOR: PanelId = PanelId(1);
    /// Scene hierarchy panel
    pub const HIERARCHY: PanelId = PanelId(2);
    /// Asset browser panel
    pub const ASSET_BROWSER: PanelId = PanelId(3);
    /// Console/output panel
    pub const CONSOLE: PanelId = PanelId(4);
}

impl From<PanelId> for WidgetId {
    fn from(id: PanelId) -> Self {
        WidgetId::new(id.0 as u64 + 10000) // Offset to avoid collision with other widgets
    }
}

/// Map a View-menu item label to the dock panel it toggles.
///
/// Scene View (always visible) and Console (no panel implementation yet)
/// deliberately return `None`.
pub fn panel_id_for_menu_label(label: &str) -> Option<PanelId> {
    match label {
        "Inspector" => Some(PanelId::INSPECTOR),
        "Hierarchy" => Some(PanelId::HIERARCHY),
        "Asset Browser" => Some(PanelId::ASSET_BROWSER),
        _ => None,
    }
}

/// Position where a panel can be docked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockPosition {
    /// Panel is docked to the left edge
    Left,
    /// Panel is docked to the right edge
    Right,
    /// Panel is docked to the top edge
    Top,
    /// Panel is docked to the bottom edge
    Bottom,
    /// Panel fills the center (main content area)
    #[default]
    Center,
    /// Panel is floating (not docked)
    Floating,
}

/// A dockable panel in the editor.
#[derive(Debug, Clone)]
pub struct DockPanel {
    /// Panel identifier
    pub id: PanelId,
    /// Panel title displayed in the header
    pub title: String,
    /// Where the panel is docked
    pub position: DockPosition,
    /// Panel bounds (updated during layout)
    bounds: Rect,
    /// Panel size (width for Left/Right, height for Top/Bottom)
    pub size: f32,
    /// Minimum size
    pub min_size: f32,
    /// Whether the panel is visible
    pub visible: bool,
    /// Whether the panel can be resized
    pub resizable: bool,
    /// Whether the panel is collapsed to a slim strip. `size` is untouched
    /// while collapsed, so expanding restores the previous size.
    pub collapsed: bool,
}

impl DockPanel {
    /// Create a new dock panel.
    pub fn new(id: PanelId, title: impl Into<String>, position: DockPosition) -> Self {
        Self {
            id,
            title: title.into(),
            position,
            bounds: Rect::default(),
            size: DEFAULT_PANEL_WIDTH,
            min_size: MIN_PANEL_SIZE,
            visible: true,
            resizable: true,
            collapsed: false,
        }
    }

    /// Set the panel size.
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Set the minimum size.
    pub fn with_min_size(mut self, min_size: f32) -> Self {
        self.min_size = min_size;
        self
    }

    /// Set whether the panel is resizable.
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Whether this panel can be collapsed (edge-docked panels only).
    pub fn is_collapsible(&self) -> bool {
        matches!(
            self.position,
            DockPosition::Left | DockPosition::Right | DockPosition::Top | DockPosition::Bottom
        )
    }

    /// The size the panel occupies in the layout: the collapsed strip is
    /// exactly one header tall/wide.
    pub fn effective_size(&self) -> f32 {
        if self.collapsed && self.is_collapsible() {
            HEADER_HEIGHT
        } else {
            self.size
        }
    }

    /// Get the content bounds (excluding header).
    ///
    /// A collapsed panel has no content area (zero rect).
    pub fn content_bounds(&self) -> Rect {
        if self.collapsed && self.is_collapsible() {
            return Rect::default();
        }
        Rect::new(
            self.bounds.x,
            self.bounds.y + HEADER_HEIGHT,
            self.bounds.width,
            (self.bounds.height - HEADER_HEIGHT).max(0.0),
        )
    }
}

/// Manages the layout and rendering of docked panels.
#[derive(Debug, Clone)]
pub struct DockArea {
    /// All panels in the dock area
    panels: Vec<DockPanel>,
    /// Available area for docking
    bounds: Rect,
    /// Resize handle size
    resize_handle_size: f32,
}

impl Default for DockArea {
    fn default() -> Self {
        Self::new()
    }
}

impl DockArea {
    /// Create a new dock area.
    pub fn new() -> Self {
        Self {
            panels: Vec::new(),
            bounds: Rect::default(),
            resize_handle_size: RESIZE_HANDLE_SIZE,
        }
    }

    /// Add a panel to the dock area.
    pub fn add_panel(&mut self, panel: DockPanel) {
        self.panels.push(panel);
    }

    /// Get a panel by ID.
    pub fn get_panel(&self, id: PanelId) -> Option<&DockPanel> {
        self.panels.iter().find(|p| p.id == id)
    }

    /// Get a panel by ID (mutable).
    pub fn get_panel_mut(&mut self, id: PanelId) -> Option<&mut DockPanel> {
        self.panels.iter_mut().find(|p| p.id == id)
    }

    /// Get all panels.
    pub fn panels(&self) -> &[DockPanel] {
        &self.panels
    }

    /// The dock area bounds.
    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Set the available bounds for the dock area.
    pub fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    /// Set a panel's visibility and re-run layout.
    pub fn set_panel_visible(&mut self, id: PanelId, visible: bool) {
        if let Some(panel) = self.get_panel_mut(id) {
            panel.visible = visible;
            self.layout();
        }
    }

    /// Toggle a panel's visibility and re-run layout.
    pub fn toggle_panel_visible(&mut self, id: PanelId) {
        if let Some(panel) = self.get_panel_mut(id) {
            panel.visible = !panel.visible;
            self.layout();
        }
    }

    /// Set a panel's collapsed state (edge panels only) and re-run layout.
    pub fn set_panel_collapsed(&mut self, id: PanelId, collapsed: bool) {
        if let Some(panel) = self.get_panel_mut(id) {
            if panel.is_collapsible() {
                panel.collapsed = collapsed;
                self.layout();
            }
        }
    }

    /// Toggle a panel's collapsed state (edge panels only) and re-run layout.
    pub fn toggle_panel_collapsed(&mut self, id: PanelId) {
        if let Some(panel) = self.get_panel_mut(id) {
            if panel.is_collapsible() {
                panel.collapsed = !panel.collapsed;
                self.layout();
            }
        }
    }

    /// Update panel layouts based on current dock positions.
    pub fn layout(&mut self) {
        let mut remaining = self.bounds;

        // First pass: allocate space for edge-docked panels
        for panel in &mut self.panels {
            if !panel.visible {
                continue;
            }

            let size = panel.effective_size();
            match panel.position {
                DockPosition::Left => {
                    let width = size.min(remaining.width);
                    panel.bounds = Rect::new(remaining.x, remaining.y, width, remaining.height);
                    remaining.x += width;
                    remaining.width -= width;
                }
                DockPosition::Right => {
                    let width = size.min(remaining.width);
                    panel.bounds = Rect::new(
                        remaining.x + remaining.width - width,
                        remaining.y,
                        width,
                        remaining.height,
                    );
                    remaining.width -= width;
                }
                DockPosition::Top => {
                    let height = size.min(remaining.height);
                    panel.bounds = Rect::new(remaining.x, remaining.y, remaining.width, height);
                    remaining.y += height;
                    remaining.height -= height;
                }
                DockPosition::Bottom => {
                    let height = size.min(remaining.height);
                    panel.bounds = Rect::new(
                        remaining.x,
                        remaining.y + remaining.height - height,
                        remaining.width,
                        height,
                    );
                    remaining.height -= height;
                }
                DockPosition::Center | DockPosition::Floating => {
                    // Handled in second pass
                }
            }
        }

        // Second pass: center panels get remaining space
        for panel in &mut self.panels {
            if !panel.visible {
                continue;
            }

            if panel.position == DockPosition::Center {
                panel.bounds = remaining;
            }
        }
    }
}
