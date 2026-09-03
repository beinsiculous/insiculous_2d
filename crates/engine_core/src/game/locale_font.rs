//! GameRunner's locale-font tail: applying the current locale's font to
//! the UI after `set_locale`/`cycle_locale` calls.
//!
//! Child module of `game` (like `render`) so it can reach the runner's
//! private fields without widening visibility.

use super::{Game, GameRunner};

impl<G: Game> GameRunner<G> {
    /// If the locale changed, load (or fetch from cache) its font and make
    /// it the UI default; a locale without a font restores the game's own.
    pub(super) fn apply_locale_font(&mut self) {
        if !self.strings.take_font_dirty() {
            return;
        }

        let handle = match self.strings.current_font().map(str::to_string) {
            Some(rel) => match self.locale_fonts.get(&rel).copied() {
                Some(handle) => Some(handle),
                None => {
                    let base = self
                        .config
                        .asset_base_path
                        .clone()
                        .unwrap_or_else(|| "assets".to_string());
                    let full = std::path::Path::new(&base).join(&rel);
                    let full = full.to_string_lossy();
                    match self.ui.load_font_file(&full) {
                        Ok(handle) => {
                            self.locale_fonts.insert(rel, handle);
                            Some(handle)
                        }
                        Err(e) => {
                            log::warn!("Locale font '{}' failed to load: {}", full, e);
                            None
                        }
                    }
                }
            },
            None => None,
        };

        match handle {
            Some(handle) => {
                self.ui.set_default_font(handle);
                self.strings.set_active_font(Some(handle));
            }
            None => {
                if let Some(base) = self.base_font {
                    self.ui.set_default_font(base);
                }
                self.strings.set_active_font(None);
            }
        }
    }
}
