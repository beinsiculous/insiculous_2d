//! Tab membership and panel visibility; each edge can hold several ordered groups.

use ui::Rect;
use super::{DockArea, DockPosition, PanelDefaults, PanelId, HEADER_HEIGHT, MIN_CENTER_HEIGHT, MIN_CENTER_WIDTH};

/// Panels sharing one allocation and one header strip.
#[derive(Debug, Clone)]
pub struct TabGroup {
    pub position: DockPosition,
    pub panels: Vec<PanelId>,
    pub active: PanelId,
    pub size: f32,
    pub min_size: f32,
    pub collapsed: bool,
    pub(super) bounds: Rect,
}

impl TabGroup {
    pub(crate) fn new(defaults: &PanelDefaults) -> Self {
        Self {
            position: defaults.position, panels: vec![defaults.id], active: defaults.id,
            size: defaults.size.max(defaults.min_size), min_size: defaults.min_size,
            collapsed: false, bounds: Rect::default(),
        }
    }

    /// The group's last allocation; only the active visible panel receives its bounds.
    pub fn bounds(&self) -> Rect { self.bounds }

    pub(super) fn effective_size(&self) -> f32 {
        if self.collapsed { HEADER_HEIGHT } else { self.size.max(self.min_size) }
    }
}

impl DockArea {
    /// Whether the new edge group can fit even when every group along that axis is collapsed.
    pub fn can_move_panel_to_edge(&self, id: PanelId, edge: DockPosition) -> bool {
        let Some(source) = self.group_of(id) else { return false; };
        if edge == DockPosition::Center { return false; }
        let side = Self::is_side(edge);
        let count = self.groups.iter().enumerate().filter(|(index, group)| {
            Self::is_side(group.position) == side && self.group_visible(*index)
                && !(*index == source && group.panels.iter().all(|member| *member == id
                    || !self.get_panel(*member).is_some_and(|panel| panel.visible)))
        }).count() + 1;
        let (available, minimum) = if side { (self.bounds.width, MIN_CENTER_WIDTH) }
            else { (self.bounds.height, MIN_CENTER_HEIGHT) };
        available <= 0.0 || count as f32 * HEADER_HEIGHT + minimum <= available
    }

    /// Move the panel into its own new group at the end of the edge; drop an emptied group.
    /// An edge without room even for collapsed strips refuses the move.
    pub fn move_panel_to_edge(&mut self, id: PanelId, edge: DockPosition) {
        if !self.can_move_panel_to_edge(id, edge) { return; }
        let Some(source) = self.group_of(id) else { return; };
        let mut group = self.groups[source].clone();
        self.detach(id);
        group.position = edge;
        group.panels = vec![id];
        group.active = id;
        self.groups.push(group);
        self.finish_move(id);
    }

    /// Insert the panel immediately after `other`, activate it and keep the target's size.
    pub fn group_panel_with(&mut self, id: PanelId, other: PanelId) {
        if id == other || self.group_of(id).is_none() || self.group_of(other).is_none() { return; }
        self.detach(id);
        let Some(target) = self.group_of(other) else { return; };
        let group = &mut self.groups[target];
        let Some(index) = group.panels.iter().position(|member| *member == other) else { return; };
        group.panels.insert(index + 1, id);
        self.finish_move(id);
    }

    fn detach(&mut self, id: PanelId) {
        for group in &mut self.groups { group.panels.retain(|member| *member != id); }
        self.groups.retain(|group| !group.panels.is_empty());
    }

    /// A moved panel is shown where it landed: a group that is collapsed, or collapsed by
    /// the budget, would otherwise take the panel and show nothing.
    fn finish_move(&mut self, id: PanelId) {
        self.narrow_overlay = None;
        self.overflow_group = None;
        self.recompute_minimums();
        self.reveal(id);
    }

    /// Bring a tab forward. A group already showing content — expanded, or open as the
    /// overlay — only switches its active tab, so a visit to the narrow overlay or a click
    /// in an expanded strip never writes the saved collapse flag; a group showing nothing
    /// is shown as `show_panel` shows it, which is the click's meaning there.
    pub fn reveal(&mut self, id: PanelId) {
        if self.group_shows_content(id) { self.activate(id); } else { self.show_panel(id); }
    }

    /// Whether any member of the panel's group has a content rectangle this frame.
    pub fn group_shows_content(&self, id: PanelId) -> bool {
        self.group_of(id).is_some_and(|index| self.groups[index].panels.iter()
            .any(|member| self.get_panel(*member).is_some_and(|panel| panel.content_bounds().height > 0.0)))
    }

    pub(crate) fn recompute_minimums(&mut self) {
        for group in &mut self.groups {
            group.min_size = self.defaults.iter().filter(|defaults| group.panels.contains(&defaults.id))
                .map(|defaults| defaults.min_size).fold(0.0, f32::max);
            group.size = group.size.max(group.min_size);
        }
    }

    /// Activate a tab and make it visible. Host focus settlement observes this immediately.
    pub fn activate(&mut self, id: PanelId) {
        if let Some(panel) = self.get_panel_mut(id) { panel.visible = true; }
        if let Some(index) = self.group_of(id) { self.groups[index].active = id; }
        self.layout();
    }

    /// Show the panel expanded; under layout pressure open its group as a temporary overlay.
    pub fn show_panel(&mut self, id: PanelId) {
        self.activate(id);
        if let Some(index) = self.group_of(id) {
            self.groups[index].collapsed = false;
            self.layout();
            if self.overlay_eligible(index) {
                self.narrow_overlay = Some(index);
                self.layout();
            }
        }
    }

    /// Show and activate, or hide and select the next visible tab.
    pub fn set_panel_visible(&mut self, id: PanelId, visible: bool) {
        if visible { self.activate(id); }
        else if let Some(panel) = self.get_panel_mut(id) {
            panel.visible = false;
            self.layout();
        }
    }

    /// Toggle this panel's visibility and reflow its group.
    pub fn toggle_panel_visible(&mut self, id: PanelId) {
        if let Some(panel) = self.get_panel(id) { self.set_panel_visible(id, !panel.visible); }
    }

    /// Set the group's persisted collapse state. The centre never collapses.
    pub fn set_panel_collapsed(&mut self, id: PanelId, collapsed: bool) {
        if let Some(index) = self.group_of(id) {
            self.groups[index].collapsed = collapsed;
            if collapsed { self.narrow_overlay = None; }
            self.layout();
        }
    }

    /// Toggle a group's chevron, opening a budget-collapsed group as an overlay.
    pub fn toggle_panel_collapsed(&mut self, id: PanelId) {
        if let Some(index) = self.group_of(id) {
            if self.overlay_eligible(index) { self.open_narrow_overlay(id); }
            else { self.set_panel_collapsed(id, !self.groups[index].collapsed); }
        }
    }

    pub(super) fn settle_active_tabs(&mut self) {
        for index in 0..self.groups.len() {
            let group = &self.groups[index];
            if group.panels.contains(&group.active) && self.get_panel(group.active).is_some_and(|panel| panel.visible) { continue; }
            let start = group.panels.iter().position(|member| *member == group.active).unwrap_or(group.panels.len() - 1);
            let next = (1..=group.panels.len()).map(|offset| group.panels[(start + offset) % group.panels.len()])
                .find(|member| self.get_panel(*member).is_some_and(|panel| panel.visible));
            self.groups[index].active = next.unwrap_or(group.panels[0]);
        }
    }
}
