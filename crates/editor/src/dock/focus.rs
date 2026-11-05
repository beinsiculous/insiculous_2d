//! Keyboard focus walks group strips independently of each panel's content focus.

use super::{DockArea, PanelId};

impl DockArea {
    /// The tab the strip keyboard currently addresses.
    pub fn focused_tab(&self) -> Option<PanelId> { self.focused_tab }

    /// Give the next visible group's active tab the strip focus, wrapping at the last group.
    pub fn cycle_tab_focus(&mut self) {
        let groups: Vec<usize> = (0..self.groups.len()).filter(|index| self.group_visible(*index)).collect();
        if groups.is_empty() { self.focused_tab = None; return; }
        let current = self.focused_tab.and_then(|id| self.group_of(id));
        let next = current.and_then(|index| groups.iter().position(|candidate| *candidate == index))
            .map_or(0, |index| (index + 1) % groups.len());
        self.focused_tab = Some(self.groups[groups[next]].active);
    }

    /// Move the strip cursor among visible tabs, including tabs behind the overflow button.
    pub fn step_tab_focus(&mut self, forward: bool) {
        let Some(id) = self.focused_tab else { return; };
        let Some(index) = self.group_of(id) else { return; };
        let tabs = self.visible_tabs(index);
        let Some(current) = tabs.iter().position(|candidate| *candidate == id) else { return; };
        let next = if forward { (current + 1) % tabs.len() } else { (current + tabs.len() - 1) % tabs.len() };
        self.focused_tab = Some(tabs[next]);
    }

    /// Close a tab overflow when a dialog or play session takes over.
    pub fn close_tab_overflow(&mut self) { self.overflow_group = None; }

    /// Yield the strip keyboard to the rest of the editor.
    pub fn clear_tab_focus(&mut self) { self.focused_tab = None; }

    /// Drain a pointer's request to open a tab's action menu.
    pub fn take_tab_context_request(&mut self) -> Option<(PanelId, glam::Vec2)> {
        self.context_request.take()
    }

    pub(super) fn visible_tabs(&self, index: usize) -> Vec<PanelId> {
        self.groups[index].panels.iter().copied()
            .filter(|id| self.get_panel(*id).is_some_and(|panel| panel.visible)).collect()
    }
}
