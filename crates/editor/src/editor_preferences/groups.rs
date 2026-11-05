//! Saved group layout and migration from the former per-panel preferences.

use serde::{Deserialize, Serialize};
use crate::dock::{DockArea, DockPosition, PanelId, TabGroup};
use super::EditorPreferences;

/// Read-only legacy layout record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelPrefs {
    pub id: u32,
    pub visible: bool,
    pub collapsed: bool,
    pub size: f32,
}

/// A group's saved order, allocation, active member and per-panel visibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupPrefs {
    pub position: DockPosition,
    pub size: f32,
    pub collapsed: bool,
    pub panels: Vec<u32>,
    pub active: u32,
    #[serde(default)]
    pub hidden: Vec<u32>,
}

impl EditorPreferences {
    /// Capture group layout; effective budget corrections and the centre are never saved.
    pub fn capture_panels(&mut self, dock: &DockArea) {
        self.groups = dock.groups().iter().map(|group| GroupPrefs {
            position: group.position, size: group.size, collapsed: group.collapsed,
            panels: group.panels.iter().map(|id| id.0).collect(), active: group.active.0,
            hidden: group.panels.iter().filter(|id| !dock.get_panel(**id).is_some_and(|panel| panel.visible))
                .map(|id| id.0).collect(),
        }).collect();
        self.panels.clear();
    }

    /// Restore known panels, appending panels omitted by an older file to their default groups.
    pub fn apply_panels(&self, dock: &mut DockArea) {
        if self.groups.is_empty() { self.apply_legacy(dock); }
        else { self.apply_groups(dock); }
        dock.recompute_minimums();
        dock.layout();
    }

    fn apply_legacy(&self, dock: &mut DockArea) {
        for preference in &self.panels {
            if dock.group_of(PanelId(preference.id)).is_none() { continue; }
            if let Some(panel) = dock.get_panel_mut(PanelId(preference.id)) { panel.visible = preference.visible; }
        }
        for index in 0..dock.groups.len() {
            let group = &dock.groups[index];
            let first_visible = group.panels.iter().find_map(|id| {
                self.panels.iter().find(|preference| preference.id == id.0 && preference.visible)
            });
            if let Some(preference) = first_visible {
                let group = &mut dock.groups[index];
                group.size = preference.size.max(group.min_size);
                group.collapsed = preference.collapsed;
                group.active = PanelId(preference.id);
            } else if group.panels.iter().all(|id| !dock.get_panel(*id).is_some_and(|panel| panel.visible)) {
                let primary = group.panels[0];
                if let Some(defaults) = dock.defaults.iter().find(|defaults| defaults.id == primary) {
                    let group = &mut dock.groups[index];
                    group.size = defaults.size.max(group.min_size);
                    group.collapsed = false;
                    group.active = primary;
                }
            }
        }
    }

    fn apply_groups(&self, dock: &mut DockArea) {
        dock.close_narrow_overlay();
        dock.close_tab_overflow();
        dock.clear_tab_focus();
        let defaults = dock.defaults.clone();
        let mut used = Vec::new();
        let mut groups = Vec::new();
        for preference in &self.groups {
            if preference.position == DockPosition::Center { continue; }
            let members: Vec<PanelId> = preference.panels.iter().map(|id| PanelId(*id)).filter(|id| {
                if used.contains(id) || !defaults.iter().any(|defaults| defaults.id == *id && defaults.position != DockPosition::Center) { return false; }
                used.push(*id);
                true
            }).collect();
            let Some(first) = members.first().copied() else { continue; };
            let Some(registration) = defaults.iter().find(|defaults| defaults.id == first) else { continue; };
            let mut group = TabGroup::new(registration);
            group.position = preference.position;
            group.size = preference.size;
            group.collapsed = preference.collapsed;
            group.panels = members;
            for id in &group.panels {
                if let Some(panel) = dock.get_panel_mut(*id) { panel.visible = !preference.hidden.contains(&id.0); }
            }
            group.active = group.panels.iter().copied()
                .find(|id| id.0 == preference.active && !preference.hidden.contains(&id.0))
                .or_else(|| group.panels.iter().copied().find(|id| !preference.hidden.contains(&id.0)))
                .unwrap_or(first);
            groups.push(group);
        }
        for registration in &defaults {
            if registration.position == DockPosition::Center || used.contains(&registration.id) { continue; }
            if let Some(panel) = dock.get_panel_mut(registration.id) { panel.visible = registration.visible; }
            let primary = defaults.iter().find(|candidate| candidate.position == registration.position).map(|candidate| candidate.id);
            let target = groups.iter().position(|group| primary.is_some_and(|id| group.panels.contains(&id)))
                .or_else(|| groups.iter().position(|group| group.position == registration.position));
            if let Some(target) = target { groups[target].panels.push(registration.id); }
            else { groups.push(TabGroup::new(registration)); }
        }
        dock.groups = groups;
    }
}
