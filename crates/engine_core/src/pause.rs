//! Shared pause mechanism: state, toggle, and the standard pause menu.
//!
//! The engine owns the mechanism — Menu (Escape / any pad's Start) toggles,
//! [`MenuInput`] navigates, and the menu offers Resume / Restart / Quit to
//! Title / Exit Game. Games own the meaning: they decide which states are pausable,
//! call [`PauseMenu::update`] there, skip their entire gameplay update while
//! paused (no physics step, no timers), and map [`PauseAction::Restart`] /
//! [`PauseAction::QuitToTitle`] onto their own `start_game` / `reset_to_title`.
//!
//! ```
//! use engine_core::{PauseAction, PauseMenu};
//! use glam::Vec2;
//! use input::{InputHandler, InputSettings};
//!
//! let mut pause = PauseMenu::new();
//! let settings = InputSettings::default_two_player();
//! let input = InputHandler::new();
//! let window_size = Vec2::new(800.0, 600.0);
//!
//! // Each frame, from a pausable gameplay state:
//! match pause.update(&settings, &input, window_size) {
//!     PauseAction::Restart => { /* self.start_game(...) */ }
//!     PauseAction::QuitToTitle => { /* self.reset_to_title(...) */ }
//!     PauseAction::ExitGame => { /* ctx.exit_requested = true */ }
//!     PauseAction::Resumed => { /* skip this frame; unfreeze next */ }
//!     PauseAction::Idle => {}
//! }
//! // ctx.time_scale = pause.time_scale();   // freezes engine particles
//! if pause.is_active() { /* skip gameplay; draw the overlay in the UI pass */ }
//! ```

use glam::Vec2;
use input::{GameAction, InputHandler, InputSettings};
use ui::UIContext;

use crate::menu_input::MenuInput;
use crate::menu_panel::{MenuPanel, MenuStyle};

/// Width of the pause window — `update` (hit-testing) and `draw_labeled`
/// (rendering) must agree on the same panel geometry.
const PANEL_WIDTH: f32 = 300.0;

/// What the pause menu decided this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseAction {
    /// Nothing changed: either not paused, or paused and still browsing.
    Idle,
    /// The player unpaused (toggle or the Resume item). Games should skip
    /// the rest of this frame's update so the resuming keypress can't leak
    /// into gameplay; the world unfreezes next frame.
    Resumed,
    /// The player picked Restart — restart the current match. Unpauses.
    Restart,
    /// The player picked Quit to Title. Unpauses.
    QuitToTitle,
    /// The player picked Exit Game — set `ctx.exit_requested = true` for a
    /// clean engine shutdown. Unpauses (moot once the loop exits).
    ExitGame,
}

/// The pause menu's item labels, in selection order.
const ITEMS: [&str; 4] = ["Resume", "Restart", "Quit to Title", "Exit Game"];

/// Default hint footer shown under the pause items.
const HINT: &str = "ESC resumes - SPACE/ENTER or click confirms";

/// Localizable label set for the pause menu. The defaults are the built-in
/// English labels, so `PauseMenu::draw` keeps working unchanged; localized
/// games build one per frame from `ctx.strings.tr(...)` and call
/// [`PauseMenu::draw_labeled`]. Item order matches the selection order:
/// Resume, Restart, Quit to Title, Exit Game.
#[derive(Debug, Clone, Copy)]
pub struct PauseMenuLabels<'a> {
    /// Panel title (default "PAUSED")
    pub title: &'a str,
    /// The four menu items in selection order
    pub items: [&'a str; 4],
    /// Hint footer under the items
    pub hint: &'a str,
}

impl Default for PauseMenuLabels<'_> {
    fn default() -> Self {
        Self {
            title: "PAUSED",
            items: ITEMS,
            hint: HINT,
        }
    }
}

/// Pause state + menu. Embed one per game and drive it from the game's
/// pausable states (see the module docs for the frame pattern).
#[derive(Debug, Default)]
pub struct PauseMenu {
    active: bool,
    selection: u8,
}

impl PauseMenu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the game is currently paused.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// `0.0` while paused, `1.0` otherwise — assign to `ctx.time_scale`
    /// every frame so engine-side particles freeze with the game.
    pub fn time_scale(&self) -> f32 {
        if self.active { 0.0 } else { 1.0 }
    }

    /// Advance the pause state machine one frame.
    ///
    /// Not paused: a Menu edge (Escape / any pad's Start, either player)
    /// pauses. Paused: Menu or back (Escape/Start/B) always resumes —
    /// the same button that pauses unpauses, predictably — confirm
    /// (Space/Enter/A) executes the highlighted item, hovering the mouse
    /// moves the highlight, and a left-click on a row executes that row.
    /// `window_size` locates the menu window for mouse hit-testing (the
    /// same geometry [`draw`](Self::draw) renders at); pass
    /// `ctx.window_size`. The mouse is read here, inside the paused branch
    /// only, so gameplay never sees the clicks.
    pub fn update(
        &mut self,
        players: &InputSettings,
        input: &InputHandler,
        window_size: Vec2,
    ) -> PauseAction {
        let menu_pressed = players.just_activated_any(GameAction::Menu, input);

        if !self.active {
            if menu_pressed {
                self.active = true;
                self.selection = 0;
            }
            return PauseAction::Idle;
        }

        let nav = MenuInput::read(input);
        // Toggle/back wins over confirm so pad Start (bound to both) always
        // resumes rather than executing whatever happens to be highlighted.
        if menu_pressed || nav.back {
            self.active = false;
            return PauseAction::Resumed;
        }

        self.selection = nav.navigate(self.selection, ITEMS.len() as u8);
        let mouse = Self::panel("", window_size).mouse_select(input);
        if let Some(hovered) = mouse.hovered {
            self.selection = hovered;
        }
        if nav.confirm || mouse.clicked.is_some() {
            if let Some(clicked) = mouse.clicked {
                self.selection = clicked;
            }
            self.active = false;
            return match self.selection {
                0 => PauseAction::Resumed,
                1 => PauseAction::Restart,
                2 => PauseAction::QuitToTitle,
                _ => PauseAction::ExitGame,
            };
        }
        PauseAction::Idle
    }

    /// The pause window's layout — shared by `update` (hit-testing) and
    /// `draw_labeled` (rendering) so clicks always land where rows draw.
    fn panel(title: &str, window_size: Vec2) -> MenuPanel {
        MenuPanel::new(title, window_size / 2.0, PANEL_WIDTH, ITEMS.len())
    }

    /// Draw the standard pause overlay (input-blocking, dimmed backdrop,
    /// menu-panel chrome). Call from the game's UI pass while
    /// [`is_active`](Self::is_active); the frozen world stays visible
    /// beneath it.
    pub fn draw(&self, ui: &mut UIContext, window_size: Vec2, style: &MenuStyle) {
        self.draw_labeled(ui, window_size, style, &PauseMenuLabels::default());
    }

    /// [`draw`](Self::draw) with caller-supplied (typically localized)
    /// labels. Item order is fixed: Resume, Restart, Quit to Title, Exit.
    pub fn draw_labeled(
        &self,
        ui: &mut UIContext,
        window_size: Vec2,
        style: &MenuStyle,
        labels: &PauseMenuLabels<'_>,
    ) {
        let panel = Self::panel(labels.title, window_size);
        panel.draw_as_overlay(ui, window_size, style, |panel, ui, mut y| {
            for (i, item) in labels.items.iter().enumerate() {
                y = panel.item(ui, y, item, i as u8 == self.selection, style);
            }
            panel.hint(ui, labels.hint, style);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chaos_mode::ChaosMode;
    use crate::chaos_theme::ChaosTheme;
    use crate::test_support::frame;
    use input::{GamepadButton, InputEvent};
    use winit::event::MouseButton;
    use winit::keyboard::KeyCode;

    /// Window size shared by every test — the menu window centers in it.
    const WIN: Vec2 = Vec2::new(800.0, 600.0);

    fn setup() -> (PauseMenu, InputSettings, InputHandler) {
        (PauseMenu::new(), InputSettings::default_two_player(), InputHandler::new())
    }

    /// Pause with Escape and release it, so the menu is open and browsing.
    fn open_paused(pause: &mut PauseMenu, settings: &InputSettings, input: &mut InputHandler) {
        frame(input, &[InputEvent::KeyPressed(KeyCode::Escape)]);
        pause.update(settings, input, WIN);
        frame(input, &[InputEvent::KeyReleased(KeyCode::Escape)]);
        pause.update(settings, input, WIN);
        assert!(pause.is_active(), "setup: the menu is open");
    }

    #[test]
    fn menu_edge_pauses_and_the_same_button_resumes_from_keyboard_or_any_pad() {
        // Escape (GameAction::Menu) is an EDGE: a held key never flaps the
        // state, and the button that paused is the one that resumes.
        let (mut pause, settings, mut input) = setup();
        frame(&mut input, &[InputEvent::KeyPressed(KeyCode::Escape)]);
        assert_eq!(pause.update(&settings, &input, WIN), PauseAction::Idle);
        assert!(pause.is_active());

        frame(&mut input, &[]);
        assert_eq!(pause.update(&settings, &input, WIN), PauseAction::Idle);
        assert!(pause.is_active(), "a held Escape is not a second edge");

        frame(&mut input, &[InputEvent::KeyReleased(KeyCode::Escape)]);
        pause.update(&settings, &input, WIN);
        frame(&mut input, &[InputEvent::KeyPressed(KeyCode::Escape)]);
        assert_eq!(pause.update(&settings, &input, WIN), PauseAction::Resumed);
        assert!(!pause.is_active());

        // Player 2's pad Start pauses, player 1's pad Start resumes: pause
        // belongs to every player.
        frame(&mut input, &[InputEvent::KeyReleased(KeyCode::Escape)]);
        pause.update(&settings, &input, WIN);
        frame(&mut input, &[InputEvent::GamepadButtonPressed(1, GamepadButton::Start)]);
        assert_eq!(pause.update(&settings, &input, WIN), PauseAction::Idle);
        assert!(pause.is_active());
        frame(&mut input, &[InputEvent::GamepadButtonReleased(1, GamepadButton::Start)]);
        pause.update(&settings, &input, WIN);
        frame(&mut input, &[InputEvent::GamepadButtonPressed(0, GamepadButton::Start)]);
        assert_eq!(pause.update(&settings, &input, WIN), PauseAction::Resumed);

        // The pad's back button (B) resumes too, whatever is highlighted.
        frame(&mut input, &[
            InputEvent::GamepadButtonReleased(0, GamepadButton::Start),
            InputEvent::KeyPressed(KeyCode::Escape),
        ]);
        pause.update(&settings, &input, WIN);
        frame(&mut input, &[
            InputEvent::KeyReleased(KeyCode::Escape),
            InputEvent::GamepadButtonPressed(0, GamepadButton::B),
        ]);
        assert_eq!(pause.update(&settings, &input, WIN), PauseAction::Resumed);
    }

    #[test]
    fn confirm_executes_the_highlighted_row_wraps_and_reopens_at_resume() {
        for (downs, expected) in [
            (0, PauseAction::Resumed),
            (1, PauseAction::Restart),
            (2, PauseAction::QuitToTitle),
            (3, PauseAction::ExitGame),
        ] {
            let (mut pause, settings, mut input) = setup();
            open_paused(&mut pause, &settings, &mut input);

            for _ in 0..downs {
                frame(&mut input, &[InputEvent::KeyPressed(KeyCode::KeyS)]);
                assert_eq!(pause.update(&settings, &input, WIN), PauseAction::Idle);
                frame(&mut input, &[InputEvent::KeyReleased(KeyCode::KeyS)]);
                pause.update(&settings, &input, WIN);
            }

            frame(&mut input, &[InputEvent::KeyPressed(KeyCode::Space)]);
            assert_eq!(pause.update(&settings, &input, WIN), expected, "{downs} downs");
            assert!(!pause.is_active(), "every confirm unpauses");
        }

        // Up from the top wraps to the last row (Exit Game) ...
        let (mut pause, settings, mut input) = setup();
        open_paused(&mut pause, &settings, &mut input);
        frame(&mut input, &[InputEvent::KeyPressed(KeyCode::KeyW)]);
        pause.update(&settings, &input, WIN);
        frame(&mut input, &[
            InputEvent::KeyReleased(KeyCode::KeyW),
            InputEvent::KeyPressed(KeyCode::Space),
        ]);
        assert_eq!(pause.update(&settings, &input, WIN), PauseAction::ExitGame);

        // ... and reopening starts back at Resume, never on a stale row.
        frame(&mut input, &[
            InputEvent::KeyReleased(KeyCode::Space),
            InputEvent::KeyPressed(KeyCode::Escape),
        ]);
        pause.update(&settings, &input, WIN);
        frame(&mut input, &[
            InputEvent::KeyReleased(KeyCode::Escape),
            InputEvent::KeyPressed(KeyCode::Space),
        ]);
        assert_eq!(pause.update(&settings, &input, WIN), PauseAction::Resumed);
    }

    #[test]
    fn click_on_a_row_executes_it_hover_moves_the_highlight_and_stray_clicks_do_nothing() {
        let (mut pause, settings, mut input) = setup();
        open_paused(&mut pause, &settings, &mut input);

        // Prime the mouse position: MouseState suppresses the first-ever
        // move's delta (anti-startup-warp), and hover requires movement.
        frame(&mut input, &[InputEvent::MouseMoved(0.0, 0.0)]);
        pause.update(&settings, &input, WIN);

        // Hover over "Quit to Title" (row 2) moves the highlight ...
        let panel = PauseMenu::panel("", WIN);
        let row2 = panel.row_rect(2);
        frame(&mut input, &[InputEvent::MouseMoved(
            row2.x + row2.width / 2.0,
            row2.y + row2.height / 2.0,
        )]);
        assert_eq!(pause.update(&settings, &input, WIN), PauseAction::Idle);

        // ... so a keyboard confirm executes the hovered row.
        frame(&mut input, &[InputEvent::KeyPressed(KeyCode::Space)]);
        assert_eq!(pause.update(&settings, &input, WIN), PauseAction::QuitToTitle);

        // Reopen; a click outside the window leaves the game paused ...
        frame(&mut input, &[
            InputEvent::KeyReleased(KeyCode::Space),
            InputEvent::KeyPressed(KeyCode::Escape),
        ]);
        pause.update(&settings, &input, WIN);
        frame(&mut input, &[
            InputEvent::KeyReleased(KeyCode::Escape),
            InputEvent::MouseMoved(10.0, 10.0),
            InputEvent::MouseButtonPressed(MouseButton::Left),
        ]);
        assert_eq!(pause.update(&settings, &input, WIN), PauseAction::Idle);
        assert!(pause.is_active(), "stray clicks don't unpause");

        // ... and a click on "Restart" (row 1) executes it with no
        // keyboard navigation at all.
        let row1 = panel.row_rect(1);
        frame(&mut input, &[
            InputEvent::MouseButtonReleased(MouseButton::Left),
            InputEvent::MouseMoved(row1.x + 4.0, row1.y + row1.height / 2.0),
            InputEvent::MouseButtonPressed(MouseButton::Left),
        ]);
        assert_eq!(pause.update(&settings, &input, WIN), PauseAction::Restart);
        assert!(!pause.is_active());
    }

    #[test]
    fn time_scale_is_zero_only_while_paused() {
        let (mut pause, settings, mut input) = setup();
        assert_eq!(pause.time_scale(), 1.0);

        frame(&mut input, &[InputEvent::KeyPressed(KeyCode::Escape)]);
        pause.update(&settings, &input, WIN);
        assert_eq!(pause.time_scale(), 0.0);

        frame(&mut input, &[InputEvent::KeyReleased(KeyCode::Escape)]);
        pause.update(&settings, &input, WIN);
        frame(&mut input, &[InputEvent::KeyPressed(KeyCode::Escape)]);
        pause.update(&settings, &input, WIN);
        assert_eq!(pause.time_scale(), 1.0);
    }

    #[test]
    fn draw_labeled_renders_the_supplied_title_items_and_hint_in_selection_order() {
        // Pong's pirate locale builds a PauseMenuLabels per frame; a wired-
        // wrong label set would ship silently without this.
        let labels = PauseMenuLabels {
            title: "PAUSA",
            items: ["Continuar", "Reiniciar", "Al título", "Salir"],
            hint: "ESC continúa",
        };
        let style = MenuStyle::from_theme(&ChaosTheme::for_mode(ChaosMode::Normal));
        let mut ui = UIContext::new();
        ui.begin_frame(&InputHandler::new(), WIN);

        PauseMenu::new().draw_labeled(&mut ui, WIN, &style, &labels);
        ui.end_frame();

        let drawn: Vec<String> = ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                ui::DrawCommand::TextPlaceholder { text, .. } => Some(text.clone()),
                ui::DrawCommand::Text { data, .. } => Some(data.text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            drawn,
            ["PAUSA", "▶ Continuar", "Reiniciar", "Al título", "Salir", "ESC continúa"],
            "title, the four items in selection order (the highlighted one behind the ▶ cursor), then the hint"
        );
    }
}
