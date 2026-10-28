//! The console's host: the one function that reports a line to the status bar and the
//! console together, the status bar's drawing with its link queued, and the console's
//! keyboard on the asset browser's model.
//!
//! A press of either button inside the Console panel gives it the keyboard; a press
//! elsewhere in the dock, the panel hidden or collapsed, the composer taking the keyboard,
//! Escape and a play session's entry take it away. While the console holds it, Up and
//! Down walk its entries — an arrow never nudges a scene entity behind them — Enter opens
//! the focused one, and a key it does not use goes on. A dialog, a focused text field and
//! the open context menu still come first.

use editor::console::{ConsoleStep, Severity};
use editor::external_tools::{Location, OpenRequest};
use editor::PanelId;
use engine_core::scripting::{ScriptError, ScriptErrorKind};
use engine_core::Game;
use winit::keyboard::KeyCode;

use super::panel_press::PanelPress;
use super::shortcuts::KeyRoute;
use super::EditorGame;

/// A key the console answers while it holds the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConsoleKey {
    Step(ConsoleStep),
    /// File the focused entry's request, as a click on it does
    Open,
    Release,
}

impl<G: Game> EditorGame<G> {
    /// Show a line on the status bar and append it to the console, so neither surface
    /// misses what the other shows. An error carries its `location` onto the bar, where
    /// a click on it files the request.
    pub(super) fn report(&mut self, severity: Severity, text: impl Into<String>, location: Option<OpenRequest>) {
        let text = text.into();
        match severity {
            Severity::Error => self.editor.status_bar.show_linked_error(text.clone(), location.clone()),
            Severity::Info => self.editor.status_bar.show_message(text.clone()),
        }
        self.editor.console.push(severity, text, location);
    }

    /// Draw the status bar and queue what a click on its linked error filed. The bar draws
    /// after this frame's drain of the requests, so the next frame's launches it.
    pub(super) fn draw_status_bar(&mut self, ui: &mut ui::UIContext, window_size: glam::Vec2) {
        if let Some(request) = self.editor.status_bar.render(ui, window_size, &self.editor.theme) {
            self.editor.open_requests.push(request);
        }
    }

    /// Frame start, after the asset browser settles and before any chrome handles this
    /// frame's press: take or drop the console's keyboard, by `panel_press`'s rule.
    pub(super) fn settle_console(&mut self, ui: &ui::UIContext) {
        let press = match self.panel_press(ui, PanelId::CONSOLE) {
            Some(press) if !self.editor.in_play_session() && !self.composer_holds_focus() => press,
            _ => return self.editor.console.release_keyboard(),
        };
        match press {
            PanelPress::Inside => self.editor.console.holds_keyboard = true,
            PanelPress::Elsewhere => self.editor.console.release_keyboard(),
            PanelPress::Neither => {}
        }
    }

    /// How a key goes while the console holds the keyboard; `None` hands it on.
    pub(super) fn console_key_route(&self, key: KeyCode) -> Option<KeyRoute> {
        if !self.editor.console.holds_keyboard {
            return None;
        }
        let console_key = match key {
            KeyCode::ArrowUp => ConsoleKey::Step(ConsoleStep::Up),
            KeyCode::ArrowDown => ConsoleKey::Step(ConsoleStep::Down),
            KeyCode::Enter | KeyCode::NumpadEnter => ConsoleKey::Open,
            KeyCode::Escape => ConsoleKey::Release,
            _ => return None,
        };
        Some(KeyRoute::Console(console_key))
    }

    /// What one of the console's keys does.
    pub(super) fn run_console_key(&mut self, key: ConsoleKey) {
        let console = &mut self.editor.console;
        match key {
            ConsoleKey::Step(step) => console.step_focus(step),
            ConsoleKey::Open => {
                if let Some(request) = console.focused_request() {
                    self.editor.open_requests.push(request);
                }
            }
            ConsoleKey::Release => console.release_keyboard(),
        }
    }
}

/// The request a script error's click files: its `.rhai` source at its line, or at no
/// line for a line of 0 — a header error's included, which names its script as a syntax
/// error does. A missing or ambiguous target's `file` is an entity's name and an unresolved
/// script's its id, whatever they end in: neither is a file, so those open nothing.
pub(super) fn script_error_location(error: &ScriptError) -> Option<OpenRequest> {
    let names_its_source = !matches!(
        error.kind,
        ScriptErrorKind::MissingTarget | ScriptErrorKind::AmbiguousTarget | ScriptErrorKind::UnresolvedScript
    );
    (names_its_source && error.file.ends_with(".rhai")).then(|| OpenRequest {
        path: error.file.clone().into(),
        location: if error.line == 0 { Location::None } else { Location::Line { line: error.line, column: None } },
    })
}
