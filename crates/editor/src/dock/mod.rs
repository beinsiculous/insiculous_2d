//! Dockable panel system for the editor.
//!
//! Provides a flexible layout system with dockable panels that can be
//! positioned in tab groups at the edges of the window. Panels can be
//! hidden, collapsed to a slim strip, and resized. Visibility and collapse
//! operations live in [`groups`], allocation in [`layout`], and chrome in [`render`].

use ui::{Rect, WidgetId};

use crate::layout::{DEFAULT_PANEL_WIDTH, HEADER_HEIGHT, MIN_PANEL_SIZE, RESIZE_HANDLE_SIZE};

pub use groups::TabGroup;

/// The narrowest centre the dock will lay out. Below it the side panels stop
/// taking an edge allocation (see [`DockArea::layout`]): the centre holds the
/// toolbar strip, and a strip narrower than its own minimum could not show
/// the play controls at all.
pub const MIN_CENTER_WIDTH: f32 = crate::toolbar_strip::TOOLBAR_STRIP_MIN_WIDTH;

/// Minimum centre height, in points: the scene's header, its toolbar strip and 120 points
/// of viewport under them.
pub const MIN_CENTER_HEIGHT: f32 = HEADER_HEIGHT + crate::layout::TOOLBAR_STRIP_HEIGHT + 120.0;

/// Length of a narrow-mode header tab along the edge it sits on.
const NARROW_TAB_LENGTH: f32 = 96.0;

/// Frames a dragged payload rests on a tab before that tab comes forward: long enough that
/// a drag crossing a strip on its way to the scene switches nothing, short enough to feel
/// immediate at 60 Hz.
pub const DRAG_DWELL_FRAMES: u32 = 12;

mod focus;
mod groups;
mod layout;
mod render;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod group_tests;

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
    /// The Conductor panel: the song's transport and sidebar
    pub const CONDUCTOR: PanelId = PanelId(5);
    /// The External Tools settings, shown by Edit → External Tools… (no View item)
    pub const SETTINGS: PanelId = PanelId(6);
}

impl From<PanelId> for WidgetId {
    fn from(id: PanelId) -> Self {
        WidgetId::new(id.0 as u64 + 10000) // Offset to avoid collision with other widgets
    }
}

/// Map a View-menu item label to the dock panel it toggles.
///
/// Scene View, which is always visible, deliberately returns `None`.
pub fn panel_id_for_menu_label(label: &str) -> Option<PanelId> {
    match label {
        "Inspector" => Some(PanelId::INSPECTOR),
        "Hierarchy" => Some(PanelId::HIERARCHY),
        "Asset Browser" => Some(PanelId::ASSET_BROWSER),
        "Conductor" => Some(PanelId::CONDUCTOR),
        "Console" => Some(PanelId::CONSOLE),
        _ => None,
    }
}

/// Position where a panel can be docked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
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
}

/// An action shared by the Window menu and each tab's context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockAction {
    MoveTo(PanelId, DockPosition),
    GroupWith(PanelId, PanelId),
    Show(PanelId),
    Hide(PanelId),
}

/// Immutable registration defaults, used when restoring incomplete preferences.
#[derive(Debug, Clone)]
pub(crate) struct PanelDefaults {
    pub id: PanelId,
    pub position: DockPosition,
    pub size: f32,
    pub min_size: f32,
    pub visible: bool,
}

/// A panel's identity and visibility. Layout belongs to its tab group.
#[derive(Debug, Clone)]
pub struct DockPanel {
    pub id: PanelId,
    pub title: String,
    pub hint: &'static str,
    pub visible: bool,
    bounds: Rect,
    content: Rect,
    // Consumed on registration; no panel owns mutable layout after it joins the dock.
    registration: Option<PanelDefaults>,
}

impl DockPanel {
    /// Register a panel at its default edge.
    pub fn new(id: PanelId, title: impl Into<String>, position: DockPosition) -> Self {
        Self {
            id, title: title.into(), hint: "", visible: true,
            bounds: Rect::default(), content: Rect::default(),
            registration: Some(PanelDefaults {
                id, position, size: DEFAULT_PANEL_WIDTH, min_size: MIN_PANEL_SIZE, visible: true,
            }),
        }
    }

    /// Set the preferred size used when this panel starts a default group.
    pub fn with_size(mut self, size: f32) -> Self {
        if let Some(defaults) = &mut self.registration { defaults.size = size; }
        self
    }

    /// Set the minimum contributed to any group containing this panel.
    pub fn with_min_size(mut self, min_size: f32) -> Self {
        if let Some(defaults) = &mut self.registration { defaults.min_size = min_size; }
        self
    }

    /// Set the header's explanatory tooltip.
    pub fn with_hint(mut self, hint: &'static str) -> Self {
        self.hint = hint;
        self
    }

    /// The active panel's content, or an empty rectangle for a hidden, inactive or closed tab.
    pub fn content_bounds(&self) -> Rect { self.content }

    /// The panel's allocated bounds below its header, including a temporary overlay.
    pub fn expanded_content_bounds(&self) -> Rect {
        if self.bounds == Rect::default() { return Rect::default(); }
        Rect::new(self.bounds.x, self.bounds.y + HEADER_HEIGHT, self.bounds.width,
            (self.bounds.height - HEADER_HEIGHT).max(0.0))
    }
}

/// The dock's groups, layout and tab interaction state.
#[derive(Debug, Clone)]
pub struct DockArea {
    panels: Vec<DockPanel>,
    pub(crate) groups: Vec<TabGroup>,
    pub(crate) defaults: Vec<PanelDefaults>,
    bounds: Rect,
    resize_handle_size: f32,
    narrow: bool,
    narrow_overlay: Option<usize>,
    budget_collapsed: Vec<usize>,
    focused_tab: Option<PanelId>,
    context_request: Option<(PanelId, glam::Vec2)>,
    overflow_group: Option<usize>,
    /// Each title's measured width in points, taken as the strips draw, so a tab's
    /// natural width is known to layout calls that have no ui at hand.
    title_widths: Vec<(PanelId, f32)>,
    /// The tab a dragged payload rests on and for how many frames.
    drag_dwell: Option<(PanelId, u32)>,
}

impl Default for DockArea {
    fn default() -> Self { Self::new() }
}

impl DockArea {
    /// Create an empty dock; only the scene occupies the centre.
    pub fn new() -> Self {
        Self {
            panels: Vec::new(), groups: Vec::new(), defaults: Vec::new(),
            bounds: Rect::default(), resize_handle_size: RESIZE_HANDLE_SIZE,
            narrow: false, narrow_overlay: None, budget_collapsed: Vec::new(),
            focused_tab: None, context_request: None, overflow_group: None,
            title_widths: Vec::new(), drag_dwell: None,
        }
    }

    /// Register a panel, appending it to its edge's default group.
    pub fn add_panel(&mut self, mut panel: DockPanel) {
        let Some(mut defaults) = panel.registration.take() else { return; };
        defaults.visible = panel.visible;
        if defaults.position != DockPosition::Center {
            if let Some(group) = self.groups.iter_mut().find(|group| group.position == defaults.position) {
                group.panels.push(panel.id);
                group.min_size = group.min_size.max(defaults.min_size);
                group.size = group.size.max(group.min_size);
            } else {
                self.groups.push(TabGroup::new(&defaults));
            }
        }
        self.defaults.push(defaults);
        self.panels.push(panel);
    }

    /// Read a panel; its bounds are empty unless it is the group's visible active panel.
    pub fn get_panel(&self, id: PanelId) -> Option<&DockPanel> {
        self.panels.iter().find(|panel| panel.id == id)
    }

    /// Mutate panel metadata. Use the dock's visibility methods to reflow layout immediately.
    pub fn get_panel_mut(&mut self, id: PanelId) -> Option<&mut DockPanel> {
        self.panels.iter_mut().find(|panel| panel.id == id)
    }

    /// All registered panels in registration order.
    pub fn panels(&self) -> &[DockPanel] { &self.panels }
    /// Edge groups in allocation order; the scene is excluded.
    pub fn groups(&self) -> &[TabGroup] { &self.groups }
    pub fn group_of(&self, id: PanelId) -> Option<usize> {
        self.groups.iter().position(|group| group.panels.contains(&id))
    }
    /// Available docking rectangle in points.
    pub fn bounds(&self) -> Rect { self.bounds }
    pub fn set_bounds(&mut self, bounds: Rect) { self.bounds = bounds; }
    /// Whether side groups currently use edge tabs instead of taking width.
    pub fn is_narrow(&self) -> bool { self.narrow }

    /// The active panel of the temporary overlay, whether opened by width or height pressure.
    pub fn narrow_overlay(&self) -> Option<PanelId> {
        self.narrow_overlay.and_then(|index| self.groups.get(index)).map(|group| group.active)
    }

    /// Toggle the panel's group overlay without changing the saved collapse preference.
    pub fn open_narrow_overlay(&mut self, id: PanelId) {
        let Some(index) = self.group_of(id) else { return; };
        if !self.overlay_eligible(index) { return; }
        self.narrow_overlay = (self.narrow_overlay != Some(index)).then_some(index);
        self.layout();
    }

    /// Close the temporary overlay, leaving its group tab reachable.
    pub fn close_narrow_overlay(&mut self) {
        self.narrow_overlay = None;
        self.layout();
    }

    /// Whether this panel is its group's active visible member (the scene is always active).
    pub fn is_active(&self, id: PanelId) -> bool {
        self.get_panel(id).is_some_and(|panel| panel.visible)
            && self.group_of(id).is_none_or(|index| self.groups[index].active == id)
    }

    fn group_visible(&self, index: usize) -> bool {
        self.groups[index].panels.iter().any(|id| self.get_panel(*id).is_some_and(|panel| panel.visible))
    }

    fn overlay_eligible(&self, index: usize) -> bool {
        (self.narrow && Self::is_side(self.groups[index].position)) || self.budget_collapsed.contains(&index)
    }

    /// Whether the group draws a horizontal strip of tabs: a narrow or collapsed side group
    /// is one vertical tab instead, unless it is open as the overlay.
    fn has_tab_strip(&self, index: usize) -> bool {
        let group = &self.groups[index];
        !(Self::is_side(group.position) && (self.narrow || group.collapsed) && self.narrow_overlay != Some(index))
    }
}
