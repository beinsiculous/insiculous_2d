//! GameRunner's locale-font tail: applying the current locale's font to
//! the UI after `set_locale`/`cycle_locale` calls.
//!
//! Child module of `game` (like `render`) so it can reach the runner's
//! private fields without widening visibility.

use std::collections::HashMap;
use super::{Game, GameRunner};

/// The runner's localization state, grouped so the strings table and the
/// fonts that follow it travel together.
pub(super) struct Localization {
    /// Localization tables (loaded from `GameConfig::locales_dir` at startup).
    pub strings: crate::localization::Strings,
    /// The game's own default font, captured after `init()` so locale font
    /// switches can restore it (`None` = game never loaded a font).
    pub base_font: Option<ui::FontHandle>,
    /// Locale font path → loaded handle, so cycling locales doesn't reload
    /// font files.
    pub fonts_by_path: HashMap<String, ui::FontHandle>,
}

impl Localization {
    pub fn new(strings: crate::localization::Strings) -> Self {
        Self {
            strings,
            base_font: None,
            fonts_by_path: HashMap::new(),
        }
    }
}

impl<G: Game> GameRunner<G> {
    /// If the locale changed, load (or fetch from cache) its font and make
    /// it the UI default; a locale without a font restores the game's own.
    pub(super) fn apply_locale_font(&mut self) {
        if !self.localization.strings.take_font_dirty() {
            return;
        }

        let handle = match self.localization.strings.current_font().map(str::to_string) {
            Some(rel) => match self.localization.fonts_by_path.get(&rel).copied() {
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
                            self.localization.fonts_by_path.insert(rel, handle);
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
                self.localization.strings.set_active_font(Some(handle));
            }
            None => {
                if let Some(base) = self.localization.base_font {
                    self.ui.set_default_font(base);
                }
                self.localization.strings.set_active_font(None);
            }
        }
    }
}
