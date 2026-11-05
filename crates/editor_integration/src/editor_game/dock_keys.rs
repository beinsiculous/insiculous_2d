//! The group strips' keyboard route and focus settlement.

use editor::{EditorAction, Modifiers};
use engine_core::Game;
use winit::keyboard::KeyCode;
use super::{EditorGame, shortcuts::KeyRoute};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DockKey { Cycle, Step(bool), Activate, Menu, Release }

impl<G: Game> EditorGame<G> {
    /// Any pointer press or play session yields the strip's keyboard.
    pub(super) fn settle_dock_focus(&mut self, ui: &ui::UIContext) {
        if ui.mouse_just_pressed() || ui.mouse_right_just_pressed() || self.editor.in_play_session() {
            self.editor.dock_area.clear_tab_focus();
        }
        self.release_inactive_panel_focus();
    }

    /// An inactive tab cannot keep consuming keys while its replacement is drawn.
    pub(super) fn release_inactive_panel_focus(&mut self) {
        use editor::PanelId;
        if !self.editor.dock_area.is_active(PanelId::CONDUCTOR) { self.leave_conductor(); }
        if !self.editor.dock_area.is_active(PanelId::ASSET_BROWSER) { self.editor.asset_browser.release_keyboard(); }
        if !self.editor.dock_area.is_active(PanelId::CONSOLE) { self.editor.console.release_keyboard(); }
        if !self.editor.dock_area.is_active(PanelId::SETTINGS) { self.editor.settings.release_keyboard(); }
    }

    pub(super) fn dock_key_route(&self, key: KeyCode, modifiers: Modifiers) -> Option<KeyRoute> {
        if self.editor.in_play_session() || self.editor.context_menu.is_open() { return None; }
        let action = self.editor.input_mapping.resolve(key, modifiers.ctrl, modifiers.shift);
        if action == Some(EditorAction::FocusDockTabs) { return Some(KeyRoute::DockTabs(DockKey::Cycle)); }
        self.editor.dock_area.focused_tab()?;
        let key = match action {
            Some(EditorAction::NudgeLeft) => DockKey::Step(false),
            Some(EditorAction::NudgeRight) => DockKey::Step(true),
            Some(EditorAction::ActivateDockTab) => DockKey::Activate,
            Some(EditorAction::DockTabMenu) => DockKey::Menu,
            Some(EditorAction::Cancel) => DockKey::Release,
            _ => return None,
        };
        Some(KeyRoute::DockTabs(key))
    }

    pub(super) fn run_dock_key(&mut self, key: DockKey) {
        self.leave_conductor();
        self.editor.asset_browser.release_keyboard();
        self.editor.console.release_keyboard();
        self.editor.settings.release_keyboard();
        match key {
            DockKey::Cycle => self.editor.dock_area.cycle_tab_focus(),
            DockKey::Step(forward) => self.editor.dock_area.step_tab_focus(forward),
            DockKey::Activate => {
                if let Some(id) = self.editor.dock_area.focused_tab() { self.editor.dock_area.show_panel(id); }
            }
            DockKey::Menu => {
                if let Some(id) = self.editor.dock_area.focused_tab() {
                    if let Some(bounds) = self.editor.dock_area.tab_bounds(id) {
                        self.editor.context_menu.open_for_tab(glam::Vec2::new(bounds.x, bounds.bottom()), id, &self.editor.dock_area);
                    }
                }
            }
            DockKey::Release => self.editor.dock_area.clear_tab_focus(),
        }
    }
}
