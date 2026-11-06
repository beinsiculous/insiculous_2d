//! The asset browser's keyboard, on the Conductor panel's model, and the context menu's host.
//!
//! A press inside the Assets panel gives the browser the keyboard; a press elsewhere in
//! the dock, the panel hidden or collapsed, the Conductor panel taking the keyboard, Escape and
//! a play session's entry take it away. While the browser holds it, its keys decide
//! before the editor's — so an arrow steps the tiles and never nudges a scene entity
//! behind them — and a key it does not use goes on. A dialog and a focused text field
//! still come first.
//!
//! The context menu's keys travel the same route, ahead of the browser's: the menu is the
//! browser's and the hierarchy's `Scripts` pseudo-rows'. While it is open it takes every
//! key, as a confirm dialog does — an arrow it does not use would nudge an entity behind
//! it, and Delete would delete one.

use editor::context_menu::ContextMenuKey;
use editor::{Modifiers, PanelId, TileStep};
use engine_core::Game;
use winit::keyboard::KeyCode;

use super::panel_press::PanelPress;
use super::shortcuts::KeyRoute;
use super::EditorGame;

/// A key the context menu or the asset browser answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BrowserKey {
    /// One of the open context menu's keys
    Menu(ContextMenuKey),
    Step(TileStep),
    /// Open the selected tile's file, as its double-click does
    Open,
    /// Open the selected tile's context menu, at the tile
    OpenMenu,
    Release,
    Parent,
}

impl<G: Game> EditorGame<G> {
    /// Frame start, after the Conductor panel settles and before any chrome handles this frame's
    /// press: take or drop the browser's keyboard, by `panel_press`'s rule.
    pub(super) fn settle_asset_browser(&mut self, ui: &ui::UIContext) {
        let press = match self.panel_press(ui, PanelId::ASSET_BROWSER) {
            Some(press) if !self.editor.in_play_session() && !self.conductor_holds_focus() => press,
            _ => return self.editor.asset_browser.release_keyboard(),
        };
        match press {
            PanelPress::Inside => self.editor.asset_browser.holds_keyboard = true,
            PanelPress::Elsewhere => self.editor.asset_browser.release_keyboard(),
            PanelPress::Neither => {}
        }
    }

    /// How a key goes while the context menu is open or the browser holds the keyboard;
    /// `None` hands it on. The open menu swallows every key but its own.
    pub(super) fn asset_browser_key_route(&self, key: KeyCode, modifiers: Modifiers) -> Option<KeyRoute> {
        if self.editor.context_menu.is_open() {
            let menu_key = ContextMenuKey::of(key);
            return Some(menu_key.map_or(KeyRoute::Consumed, |menu_key| KeyRoute::AssetBrowser(BrowserKey::Menu(menu_key))));
        }
        if !self.editor.asset_browser.holds_keyboard {
            return None;
        }
        let browser_key = match key {
            KeyCode::ArrowLeft => BrowserKey::Step(TileStep::Left),
            KeyCode::ArrowRight => BrowserKey::Step(TileStep::Right),
            KeyCode::ArrowUp => BrowserKey::Step(TileStep::Up),
            KeyCode::ArrowDown => BrowserKey::Step(TileStep::Down),
            KeyCode::Enter | KeyCode::NumpadEnter => BrowserKey::Open,
            KeyCode::ContextMenu => BrowserKey::OpenMenu,
            KeyCode::F10 if modifiers.shift => BrowserKey::OpenMenu,
            KeyCode::Escape => BrowserKey::Release,
            KeyCode::Backspace => BrowserKey::Parent,
            _ => return None,
        };
        Some(KeyRoute::AssetBrowser(browser_key))
    }

    /// What one of the browser's or the context menu's keys does.
    pub(super) fn run_asset_browser_key(&mut self, key: BrowserKey) {
        let browser = &mut self.editor.asset_browser;
        match key {
            BrowserKey::Menu(menu_key) => {
                if let Some(action) = self.editor.context_menu.handle_key(menu_key) {
                    self.run_context_menu_action(action);
                }
            }
            BrowserKey::Step(step) => browser.step_selection(step),
            BrowserKey::Open => {
                if let Some(request) = browser.open_selected() {
                    self.editor.open_requests.push(request);
                }
            }
            BrowserKey::OpenMenu => {
                browser.menu_requested = browser.selected.is_some();
                browser.reveal_selected = browser.menu_requested;
            }
            BrowserKey::Release => browser.release_keyboard(),
            BrowserKey::Parent => browser.up(),
        }
    }

    /// The context menu's pass: drawn before the menu bar and every panel, so its blocking
    /// rect reaches each widget it covers, a press outside it included. A confirm dialog
    /// owns the frame: the menu shares its band and would take the presses meant for its
    /// buttons, so a dialog coming up closes it.
    pub(super) fn render_context_menu(&mut self, ui: &mut ui::UIContext, dialog_up: bool) {
        if dialog_up {
            return self.editor.context_menu.close();
        }
        if let Some(action) = self.editor.context_menu.render(ui, &self.editor.theme) {
            self.run_context_menu_action(action);
        }
    }

}
