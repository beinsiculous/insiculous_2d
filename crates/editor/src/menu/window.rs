//! The Window menu and tab context menus share one action tree and typed dispatch.

use glam::Vec2;
use crate::{DockAction, DockArea, DockPosition, PanelId};
use super::{Menu, MenuBar, MenuItem};

const EDGES: [(&str, DockPosition); 4] = [
    ("Left", DockPosition::Left), ("Right", DockPosition::Right),
    ("Top", DockPosition::Top), ("Bottom", DockPosition::Bottom),
];

/// A panel's three items. The last is Show for a hidden panel and Hide for a visible one:
/// opening the submenu itself changes nothing, so browsing the Window menu never
/// rearranges the dock.
fn panel_items(id: PanelId, panels: &[(PanelId, String, bool)]) -> Vec<MenuItem> {
    let visible = panels.iter().any(|(other, _, visible)| *other == id && *visible);
    vec![
        MenuItem::submenu("Move to", EDGES.iter().map(|(label, _)| MenuItem::action(*label)).collect()),
        MenuItem::submenu("Group with", panels.iter().filter(|(other, _, visible)| *other != id && *visible)
            .map(|(_, title, _)| MenuItem::action(title)).collect()),
        MenuItem::action(if visible { "Hide" } else { "Show" }),
    ]
}

impl MenuBar {
    /// Refresh panel rows while preserving the currently open menu path.
    pub fn sync_window(&mut self, dock: &DockArea) {
        self.dock_entries = dock.panels().iter().filter(|panel| dock.group_of(panel.id).is_some())
            .map(|panel| (panel.id, panel.title.clone(), panel.visible)).collect();
        let Some(window) = self.menus.iter_mut().find(|menu| menu.title == "Window") else { return; };
        window.items = self.dock_entries.iter().map(|(id, title, visible)| {
            MenuItem::submenu(title, panel_items(*id, &self.dock_entries)).with_checked(*visible)
        }).collect();
        window.items.extend([MenuItem::separator(), MenuItem::action("Reset Layout")]);
    }

    pub(crate) fn dock_popup(anchor: Vec2, id: PanelId, dock: &DockArea) -> Self {
        let mut popup = Self::new();
        popup.dock_entries = dock.panels().iter().filter(|panel| dock.group_of(panel.id).is_some())
            .map(|panel| (panel.id, panel.title.clone(), panel.visible)).collect();
        let mut menu = Menu::new("dock_context").with_items(panel_items(id, &popup.dock_entries));
        menu.bounds = ui::Rect::new(anchor.x, anchor.y, 0.0, 0.0);
        popup.menus.push(menu);
        popup.open_menu = Some(0);
        popup.popup_panel = Some(id);
        popup
    }

    /// Consume the typed action chosen through a dock menu.
    pub fn take_dock_action(&mut self) -> Option<DockAction> { self.dock_action.take() }

    pub(super) fn record_dock_choice(&mut self, label: &str) {
        let (id, path) = if let Some(id) = self.popup_panel {
            (id, self.submenu_path.as_slice())
        } else {
            if self.open_menu.is_none_or(|menu| self.menus[menu].title != "Window") { return; }
            let Some((&index, rest)) = self.submenu_path.split_first() else { return; };
            let Some((id, _, _)) = self.dock_entries.get(index) else { return; };
            (*id, rest)
        };
        self.dock_action = match (path.first(), label) {
            (None, "Show") => Some(DockAction::Show(id)),
            (None, "Hide") => Some(DockAction::Hide(id)),
            (Some(0), _) => EDGES.iter().find(|(name, _)| *name == label).map(|(_, edge)| DockAction::MoveTo(id, *edge)),
            (Some(1), _) => self.dock_entries.iter().find(|(_, title, visible)| title == label && *visible)
                .map(|(other, _, _)| DockAction::GroupWith(id, *other)),
            _ => None,
        };
    }

    pub(crate) fn render_popup(&mut self, ui: &mut ui::UIContext, theme: &crate::EditorTheme) -> Option<DockAction> {
        self.render_open_dropdown(ui, theme);
        self.take_dock_action()
    }

    pub(crate) fn popup_bounds(&self, ui: &ui::UIContext, theme: &crate::EditorTheme) -> Option<ui::Rect> {
        self.dropdown_levels(ui, theme).first().map(|(_, bounds)| *bounds)
    }
}
