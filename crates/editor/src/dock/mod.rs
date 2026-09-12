//! Dockable panel system for the editor.
//!
//! Provides a flexible layout system with dockable panels that can be
//! positioned at different edges of the window or floated. Panels can be
//! hidden, collapsed to a slim strip, and resized; rendering lives in
//! [`render`] (chrome, collapse chevrons, resize grabbers).

use ui::{Rect, WidgetId};

use crate::layout::{DEFAULT_PANEL_WIDTH, HEADER_HEIGHT, MIN_PANEL_SIZE, PADDING, RESIZE_HANDLE_SIZE};

/// The narrowest centre the dock will lay out. Below it the side panels stop
/// taking an edge allocation (see [`DockArea::layout`]): the centre holds the
/// toolbar strip, and a strip narrower than its own minimum could not show
/// the play controls at all.
pub const MIN_CENTER_WIDTH: f32 = crate::toolbar_strip::TOOLBAR_STRIP_MIN_WIDTH;

/// Length of a narrow-mode header tab along the edge it sits on.
const NARROW_TAB_LENGTH: f32 = 96.0;

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
    /// One sentence on what the panel is for, shown as its header's tooltip.
    /// Empty means the header raises none.
    pub hint: &'static str,
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
            hint: "",
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

    /// Set the sentence the panel's header explains itself with on hover.
    pub fn with_hint(mut self, hint: &'static str) -> Self {
        self.hint = hint;
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
        self.expanded_content_bounds()
    }

    /// The content bounds the panel has when shown expanded, whatever its
    /// collapse flag says. The narrow-mode overlay renders with these: the
    /// flag is a desktop-layout preference that the preferences autosave
    /// persists, so opening a collapsed panel as an overlay must not clear it.
    pub fn expanded_content_bounds(&self) -> Rect {
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
    /// Whether the last layout went narrow: the side panels left the edge
    /// allocation because the centre would have fallen below
    /// [`MIN_CENTER_WIDTH`].
    narrow: bool,
    /// The one side panel shown as an overlay over the viewport while narrow.
    narrow_overlay: Option<PanelId>,
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
            narrow: false,
            narrow_overlay: None,
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

    /// Whether the dock is in narrow mode: the side panels are off the edge
    /// allocation and reachable one at a time as an overlay.
    pub fn is_narrow(&self) -> bool {
        self.narrow
    }

    /// The side panel currently shown as a narrow-mode overlay, if any. Its
    /// chrome and its content belong on the floating band, above the
    /// viewport it covers.
    pub fn narrow_overlay(&self) -> Option<PanelId> {
        self.narrow.then_some(self.narrow_overlay).flatten()
    }

    /// Show `id` as the narrow-mode overlay, closing whichever panel was
    /// open — two overlays at once would leave no viewport between them.
    /// Opening the panel that is already open closes it.
    pub fn open_narrow_overlay(&mut self, id: PanelId) {
        self.narrow_overlay = (self.narrow_overlay != Some(id)).then_some(id);
        self.layout();
    }

    /// Close the narrow-mode overlay, leaving the tabs at the edges.
    pub fn close_narrow_overlay(&mut self) {
        self.narrow_overlay = None;
        self.layout();
    }

    /// A side panel's header tab while narrow: the edge-anchored strip that
    /// opens it, below the centre's toolbar strip so the play controls stay
    /// clickable.
    fn narrow_tab_bounds(panel: &DockPanel, centre: Rect) -> Rect {
        let y = Self::narrow_top(centre) + PADDING;
        let x = match panel.position {
            DockPosition::Right => centre.right() - HEADER_HEIGHT,
            _ => centre.x,
        };
        Rect::new(x, y, HEADER_HEIGHT, NARROW_TAB_LENGTH)
    }

    /// A side panel's bounds while it is the narrow-mode overlay: its own
    /// width at its edge, from the centre's toolbar strip down to the
    /// DOCK's bottom — over a bottom panel, because a short window with the
    /// assets panel up leaves the centre a few rows tall and a field has to
    /// be editable there — never wider than the centre.
    fn narrow_overlay_bounds(panel: &DockPanel, centre: Rect, dock_bottom: f32) -> Rect {
        let width = panel.size.min(centre.width);
        let x = match panel.position {
            DockPosition::Right => centre.right() - width,
            _ => centre.x,
        };
        let top = Self::narrow_top(centre);
        Rect::new(x, top, width, (dock_bottom - top).max(0.0))
    }

    /// Where a narrow-mode panel may start: the bottom of the centre panel's
    /// toolbar strip. The centre has a header like every panel, and the strip
    /// is the top of the content below it — measuring from the dock's own top
    /// put the tabs and the overlay over the play controls.
    fn narrow_top(centre: Rect) -> f32 {
        let centre_content = Rect::new(
            centre.x,
            centre.y + HEADER_HEIGHT,
            centre.width,
            (centre.height - HEADER_HEIGHT).max(0.0),
        );
        crate::toolbar_strip::split(centre_content).0.bottom()
    }

    /// Whether `position` is a side (left/right) edge — the panels narrow
    /// mode moves off the edge allocation.
    fn is_side(position: DockPosition) -> bool {
        matches!(position, DockPosition::Left | DockPosition::Right)
    }

    /// Whether the centre would fall below [`MIN_CENTER_WIDTH`] once the
    /// visible side panels have taken their width.
    fn would_squeeze_the_center(&self) -> bool {
        let side_width: f32 = self
            .panels
            .iter()
            .filter(|panel| panel.visible && Self::is_side(panel.position))
            .map(DockPanel::effective_size)
            .sum();
        self.bounds.width - side_width < MIN_CENTER_WIDTH
    }

    /// Update panel layouts based on current dock positions.
    ///
    /// Below [`MIN_CENTER_WIDTH`] of centre the dock enters **narrow mode**:
    /// the side panels leave the edge allocation (the centre takes the whole
    /// width) and one of them at a time is shown as an overlay over the
    /// viewport, opened from its header tab at the edge. Collapsing them
    /// instead would thrash — an expanded panel violates the minimum again
    /// and would collapse on the next frame, so the field a visitor came to
    /// edit could never be reached.
    pub fn layout(&mut self) {
        self.narrow = self.would_squeeze_the_center();
        if !self.narrow {
            self.narrow_overlay = None;
        }
        let mut remaining = self.bounds;

        // First pass: allocate space for edge-docked panels
        for index in 0..self.panels.len() {
            if !self.panels[index].visible {
                continue;
            }
            if self.narrow && Self::is_side(self.panels[index].position) {
                continue;
            }

            let panel = &mut self.panels[index];
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

        // The narrow side panels sit against the centre they overlay, so they
        // are placed once the centre is known.
        if self.narrow {
            for index in 0..self.panels.len() {
                let panel = &self.panels[index];
                if !panel.visible || !Self::is_side(panel.position) {
                    continue;
                }
                let bounds = if self.narrow_overlay == Some(panel.id) {
                    Self::narrow_overlay_bounds(panel, remaining, self.bounds.bottom())
                } else {
                    Self::narrow_tab_bounds(panel, remaining)
                };
                self.panels[index].bounds = bounds;
            }
        }
    }
}
