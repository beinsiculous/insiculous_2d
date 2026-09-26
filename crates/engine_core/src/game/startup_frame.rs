//! GameRunner's startup frames: the frames the game's startup cards own, before
//! the game has run `init`, and the quiet moment after them.
//!
//! Child module of `game` (like `frame_tail`) so it can reach the runner's
//! private fields without widening visibility.

use glam::Vec2;
use ui::DrawCommand;

use super::{Game, GameRunner};

/// Whether nobody can see the game: a hidden browser tab. A native window never
/// counts as hidden.
fn page_hidden() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        crate::web::page_hidden()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

impl<G: Game> GameRunner<G> {
    /// This frame's startup work, after input is processed and before the UI or
    /// the game read it. `Some` is a frame the cards own — the card's clock, its
    /// skip and its drawing, with no `init`, no `update` and no frame tail — and
    /// the caller returns its draw commands. `None` lets the game's frame run,
    /// its presses cleared first while the quiet period lasts.
    pub(super) fn step_startup(&mut self, delta_time: f32, window_size: Vec2) -> Option<Vec<DrawCommand>> {
        if let Some(asset_manager) = self.asset_manager.as_mut() {
            self.startup.load_cards(asset_manager);
        }
        self.startup.leave_handoff();
        if !self.startup.holds_game() {
            // A press in the game's first moments is the tail of a player mashing
            // through the cards. `end_frame` is the whole edge reset — keys, mouse
            // and pad buttons, and the pads' axis crossings — and it runs before
            // the UI and the game read input.
            if self.startup.take_quiet(delta_time) {
                self.input.end_frame();
            }
            return None;
        }

        let skip = self.input.any_just_pressed();
        self.startup.advance(delta_time, skip, page_hidden());
        self.update_ui_begin(window_size, delta_time);
        self.startup.draw(&mut self.ui, window_size);
        let ui_commands = self.update_ui_end();
        self.update_input_end();
        Some(ui_commands)
    }
}
