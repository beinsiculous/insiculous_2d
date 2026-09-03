//! Localization: locale tables loaded from RON, key lookup with fallback,
//! and per-locale font selection.
//!
//! Locale files live in `assets/locales/*.ron` (the directory is
//! configurable via [`GameConfig::locales_dir`](crate::GameConfig)); the
//! locale id is the file stem (`en.ron` → `"en"`). RON was chosen over JSON
//! so translators can use comments and trailing commas.
//!
//! ```ron
//! // assets/locales/pirate.ron
//! LocaleFile(
//!     version: 1,
//!     display_name: "Pirate",
//!     font: Some("fonts/BlackSamsGold-ej5e.ttf"), // relative to the asset base
//!     strings: {
//!         "menu.play": "Set Sail!",
//!     },
//! )
//! ```
//!
//! Lookup chain: current locale → `"en"` → the key itself (with a log-once
//! warning). Games call `ctx.strings.tr("menu.play")`; data-driven UI text
//! goes through [`Strings::resolve`], where a leading `@` marks a key.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Deserialize;
use ui::FontHandle;

/// The locale every table falls back to when a key is missing.
pub const FALLBACK_LOCALE: &str = "en";

/// On-disk locale file schema (RON, hand-authored).
#[derive(Debug, Clone, Deserialize)]
pub struct LocaleFile {
    /// Schema version; currently always `1`.
    pub version: u32,
    /// Human-readable locale name shown in language pickers.
    pub display_name: String,
    /// Optional font file for this locale, relative to the asset base.
    /// `None` keeps the game's own font.
    #[serde(default)]
    pub font: Option<String>,
    /// Key → translated text.
    pub strings: HashMap<String, String>,
}

/// A loaded locale table.
#[derive(Debug, Clone)]
struct Locale {
    display_name: String,
    font: Option<String>,
    strings: HashMap<String, String>,
}

/// All loaded locales plus the current selection.
///
/// Owned by the engine and exposed to games as `ctx.strings`. Never panics:
/// a missing locales directory or a corrupt file just means lookups fall
/// through to the key itself.
#[derive(Debug, Default)]
pub struct Strings {
    locales: HashMap<String, Locale>,
    /// Locale ids sorted for stable picker/cycle order.
    sorted_ids: Vec<String>,
    current: String,
    /// Set when the locale changed and the engine hasn't applied its font yet.
    font_dirty: bool,
    /// The locale font handle currently applied by the engine (`None` =
    /// the game's own font). Read by the editor to scope fonts.
    active_font: Option<FontHandle>,
    /// Keys already warned about (log-once). RefCell so `tr` can take
    /// `&self` — lookups happen mid-UI-pass where everything is borrowed.
    warned: RefCell<HashSet<String>>,
}

impl Strings {
    /// An empty table: every lookup returns the key itself.
    pub fn empty() -> Self {
        Self {
            current: FALLBACK_LOCALE.to_string(),
            ..Default::default()
        }
    }

    /// Load every `*.ron` locale file in `dir`. Missing directory or corrupt
    /// files are warned about and skipped — never a panic.
    pub fn load_dir(dir: &Path) -> Self {
        let mut strings = Self::empty();
        let files = match common::vfs::list_dir_files_checked(dir, "ron") {
            Ok(files) => files,
            Err(e) => {
                log::warn!("Locales dir {:?} not readable ({}); no translations loaded", dir, e);
                return strings;
            }
        };
        if files.is_empty() {
            log::warn!("No locale files under {:?}; no translations loaded", dir);
            return strings;
        }

        for path in files {
            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            match common::vfs::read_to_string(&path) {
                Ok(content) => strings.insert_locale_source(id, &content),
                Err(e) => log::warn!("Could not read locale file {:?}: {}", path, e),
            }
        }
        strings
    }

    /// Parse RON locale-file source and register it under `id`. Corrupt or
    /// wrong-version sources are warned about and skipped.
    pub fn insert_locale_source(&mut self, id: &str, source: &str) {
        match ron::from_str::<LocaleFile>(source) {
            Ok(file) if file.version == 1 => {
                self.locales.insert(
                    id.to_string(),
                    Locale {
                        display_name: file.display_name,
                        font: file.font,
                        strings: file.strings,
                    },
                );
                self.sorted_ids = self.locales.keys().cloned().collect();
                self.sorted_ids.sort();
            }
            Ok(file) => {
                log::warn!("Locale '{}' has unsupported version {}; skipped", id, file.version);
            }
            Err(e) => log::warn!("Locale '{}' failed to parse: {}; skipped", id, e),
        }
    }

    /// Translate a key: current locale → `en` → the key itself (log-once).
    pub fn tr<'a>(&'a self, key: &'a str) -> &'a str {
        if let Some(text) = self.lookup(&self.current, key) {
            return text;
        }
        if let Some(text) = self.lookup(FALLBACK_LOCALE, key) {
            return text;
        }
        let mut warned = self.warned.borrow_mut();
        if warned.insert(key.to_string()) {
            log::warn!("Missing localization key '{}' (locale '{}')", key, self.current);
        }
        key
    }

    /// Resolve data-driven UI text: a leading `@` marks a localization key
    /// (`"@menu.play"` → `tr("menu.play")`), anything else is a literal.
    pub fn resolve<'a>(&'a self, text: &'a str) -> &'a str {
        match text.strip_prefix('@') {
            Some(key) => self.tr(key),
            None => text,
        }
    }

    fn lookup(&self, locale: &str, key: &str) -> Option<&str> {
        self.locales.get(locale)?.strings.get(key).map(String::as_str)
    }

    /// The current locale id (e.g. `"en"`).
    pub fn current_locale(&self) -> &str {
        &self.current
    }

    /// Switch locale. Unknown ids are allowed (lookups fall back), so a
    /// config default of `"en"` works even without locale files. Marks the
    /// font dirty so the engine re-applies the locale font.
    pub fn set_locale(&mut self, id: impl Into<String>) {
        self.current = id.into();
        self.font_dirty = true;
    }

    /// Switch to the next available locale in sorted order (wraps around).
    /// No-op when fewer than two locales are loaded.
    pub fn cycle_locale(&mut self) {
        if self.sorted_ids.is_empty() {
            return;
        }
        let next = match self.sorted_ids.iter().position(|id| *id == self.current) {
            Some(i) => (i + 1) % self.sorted_ids.len(),
            None => 0,
        };
        let id = self.sorted_ids[next].clone();
        self.set_locale(id);
    }

    /// Available locales as `(id, display_name)`, sorted by id.
    pub fn available_locales(&self) -> Vec<(&str, &str)> {
        self.sorted_ids
            .iter()
            .filter_map(|id| {
                self.locales
                    .get(id)
                    .map(|l| (id.as_str(), l.display_name.as_str()))
            })
            .collect()
    }

    /// Sorted key set of a loaded locale — for diagnostics and locale-file
    /// parity tests (every locale should define the same keys).
    pub fn locale_keys(&self, id: &str) -> Option<Vec<&str>> {
        self.locales.get(id).map(|l| {
            let mut keys: Vec<&str> = l.strings.keys().map(String::as_str).collect();
            keys.sort_unstable();
            keys
        })
    }

    /// Display name of the current locale (falls back to its id).
    pub fn current_display_name(&self) -> &str {
        self.locales
            .get(&self.current)
            .map(|l| l.display_name.as_str())
            .unwrap_or(&self.current)
    }

    /// Font file of the current locale, relative to the asset base
    /// (`None` = keep the game's own font).
    pub fn current_font(&self) -> Option<&str> {
        self.locales.get(&self.current)?.font.as_deref()
    }

    /// Whether the locale changed since the engine last applied its font.
    /// Clears the flag.
    pub fn take_font_dirty(&mut self) -> bool {
        std::mem::take(&mut self.font_dirty)
    }

    /// The locale font handle the engine currently has applied (`None` =
    /// the game's own font is active). Read by the editor to scope the
    /// game view's font without touching editor chrome.
    pub fn active_font(&self) -> Option<FontHandle> {
        self.active_font
    }

    /// Record the font handle the engine applied for the current locale.
    pub fn set_active_font(&mut self, font: Option<FontHandle>) {
        self.active_font = font;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN: &str = r#"LocaleFile(
        version: 1,
        display_name: "English",
        strings: { "menu.play": "Play", "menu.quit": "Quit" },
    )"#;

    const PIRATE: &str = r#"LocaleFile(
        version: 1,
        display_name: "Pirate",
        font: Some("fonts/BlackSamsGold-ej5e.ttf"),
        strings: { "menu.play": "Set Sail!" },
    )"#;

    fn loaded() -> Strings {
        let mut strings = Strings::empty();
        strings.insert_locale_source("en", EN);
        strings.insert_locale_source("pirate", PIRATE);
        strings
    }

    #[test]
    fn tr_falls_back_to_english_then_key() {
        let mut strings = loaded();
        strings.set_locale("pirate");

        // Present in the current locale
        assert_eq!(strings.tr("menu.play"), "Set Sail!");
        // Missing in pirate, present in en
        assert_eq!(strings.tr("menu.quit"), "Quit");
        // Missing everywhere → the key itself, and `@key` text resolves the same way
        assert_eq!(strings.tr("menu.nonexistent"), "menu.nonexistent");
        assert_eq!(strings.resolve("@menu.play"), "Set Sail!");
        assert_eq!(strings.resolve("@bogus"), "bogus");
        assert_eq!(strings.resolve("Literal text"), "Literal text");

        // No locale files at all is the same chain with nothing to hit.
        let empty = Strings::load_dir(Path::new("/nonexistent/locales"));
        assert!(empty.available_locales().is_empty());
        assert_eq!(empty.current_locale(), "en");
        assert_eq!(empty.tr("menu.play"), "menu.play");
    }

    #[test]
    fn corrupt_and_wrong_version_sources_are_skipped() -> Result<(), std::io::Error> {
        // One half-saved translator file among good ones must not take the
        // good ones down with it.
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("en.ron"), EN)?;
        std::fs::write(dir.path().join("bad.ron"), "not ron at all {{{")?;
        std::fs::write(
            dir.path().join("future.ron"),
            r#"LocaleFile(version: 99, display_name: "X", strings: {})"#,
        )?;

        let strings = Strings::load_dir(dir.path());

        assert_eq!(strings.available_locales(), vec![("en", "English")]);
        assert_eq!(strings.tr("menu.play"), "Play");
        Ok(())
    }

    #[test]
    fn load_dir_reads_ron_files_by_stem() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("en.ron"), EN)?;
        std::fs::write(dir.path().join("pirate.ron"), PIRATE)?;
        std::fs::write(dir.path().join("notes.txt"), "ignore me")?;

        let mut strings = Strings::load_dir(dir.path());

        assert_eq!(
            strings.available_locales(),
            vec![("en", "English"), ("pirate", "Pirate")],
            "sorted, keyed by file stem, non-.ron ignored"
        );
        assert_eq!(strings.locale_keys("en"), Some(vec!["menu.play", "menu.quit"]));
        assert_eq!(strings.locale_keys("klingon"), None);

        // Cycling walks the sorted ids and wraps.
        strings.set_locale("en");
        strings.cycle_locale();
        assert_eq!(strings.current_locale(), "pirate");
        assert_eq!(strings.current_display_name(), "Pirate");
        strings.cycle_locale();
        assert_eq!(strings.current_locale(), "en");
        Ok(())
    }

    #[test]
    fn current_font_follows_locale() {
        let mut strings = loaded();
        assert_eq!(strings.current_font(), None);

        strings.set_locale("pirate");

        assert_eq!(strings.current_font(), Some("fonts/BlackSamsGold-ej5e.ttf"));
        // The runner applies the font once per change, not every frame.
        assert!(strings.take_font_dirty());
        assert!(!strings.take_font_dirty());
    }
}
