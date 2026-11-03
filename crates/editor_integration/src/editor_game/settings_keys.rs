//! The External Tools panel's keyboard, on the console's model.
//!
//! A press of either button inside the panel gives it the keyboard; a press elsewhere in
//! the dock, the panel hidden or collapsed, the Conductor panel taking the keyboard, Escape and
//! a play session's entry take it away. While the panel holds it, Up and Down move its
//! cursor — an arrow never nudges a scene entity behind it — Enter acts on the control at
//! the cursor and Left and Right step a cycle row there, each carried out by the next
//! render, which alone knows which control that is; a key it does not use goes on. A
//! dialog, a focused text field — the panel's own fields included — and the open context
//! menu still come first.

use editor::settings_panel::SettingsRequest;
use editor::PanelId;
use engine_core::Game;
use winit::keyboard::KeyCode;

use super::panel_press::PanelPress;
use super::shortcuts::KeyRoute;
use super::EditorGame;

/// A key the panel answers while it holds the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsKey {
    /// Up (`true`) or Down
    Step(bool),
    /// Enter: press, focus or step forward, by the control at the cursor
    Activate,
    /// Right (`true`) or Left, on a cycle row
    Cycle(bool),
    Release,
}

impl<G: Game> EditorGame<G> {
    /// Frame start, after the console settles and before any chrome handles this frame's
    /// press: take or drop the panel's keyboard, by `panel_press`'s rule.
    pub(super) fn settle_settings(&mut self, ui: &ui::UIContext) {
        let press = match self.panel_press(ui, PanelId::SETTINGS) {
            Some(press) if !self.editor.in_play_session() && !self.conductor_holds_focus() => press,
            _ => return self.editor.settings.release_keyboard(),
        };
        match press {
            PanelPress::Inside => self.editor.settings.holds_keyboard = true,
            PanelPress::Elsewhere => self.editor.settings.release_keyboard(),
            PanelPress::Neither => {}
        }
    }

    /// How a key goes while the panel holds the keyboard; `None` hands it on.
    pub(super) fn settings_key_route(&self, key: KeyCode) -> Option<KeyRoute> {
        if !self.editor.settings.holds_keyboard {
            return None;
        }
        let settings_key = match key {
            KeyCode::ArrowUp => SettingsKey::Step(true),
            KeyCode::ArrowDown => SettingsKey::Step(false),
            KeyCode::Enter | KeyCode::NumpadEnter => SettingsKey::Activate,
            KeyCode::ArrowRight => SettingsKey::Cycle(true),
            KeyCode::ArrowLeft => SettingsKey::Cycle(false),
            KeyCode::Escape => SettingsKey::Release,
            _ => return None,
        };
        Some(KeyRoute::Settings(settings_key))
    }

    /// What one of the panel's keys does.
    pub(super) fn run_settings_key(&mut self, key: SettingsKey) {
        let settings = &mut self.editor.settings;
        match key {
            SettingsKey::Step(up) => settings.step_cursor(up),
            SettingsKey::Activate => settings.request = Some(SettingsRequest::Activate),
            SettingsKey::Cycle(forward) => settings.request = Some(SettingsRequest::Cycle { forward }),
            SettingsKey::Release => settings.release_keyboard(),
        }
    }
}
