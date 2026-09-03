//! Game configuration structures and builders.
//!
//! This module provides configuration for game window and engine settings.

use renderer::TextureFilter;
use serde::{Deserialize, Serialize};

use crate::chaos_mode::ChaosMode;

fn default_vsync() -> bool {
    true
}

fn default_locale() -> String {
    "en".to_string()
}

fn default_locales_dir() -> String {
    "locales".to_string()
}

/// Configuration for the game window and engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    /// Window title
    pub title: String,
    /// Window width in pixels
    pub width: u32,
    /// Window height in pixels
    pub height: u32,
    /// Target frames per second (0 = unlimited)
    pub target_fps: u32,
    /// Background clear color (RGBA)
    pub clear_color: [f32; 4],
    /// Whether the window is resizable
    pub resizable: bool,
    /// Present frames with vsync. `true` (default) never tears and caps at
    /// the display refresh rate; `false` requests the lowest-latency present
    /// mode the platform offers.
    #[serde(default = "default_vsync")]
    pub vsync: bool,
    /// Project-wide gameplay intensity theme. Each game decides what a given
    /// variant means; the engine just carries the selection.
    #[serde(default)]
    pub chaos_mode: ChaosMode,
    /// Optional path to persist achievement unlock state (JSON). When set, the
    /// engine creates the `AchievementManager` with this save path so unlocks
    /// survive restarts. When `None`, achievements are in-memory only.
    #[serde(default)]
    pub achievement_save_path: Option<String>,
    /// Optional path to persist high scores (JSON). When set, the engine
    /// creates [`crate::scores::Scores`] with this save path so top scores
    /// survive restarts. When `None`, scores are in-memory only.
    #[serde(default)]
    pub score_save_path: Option<String>,
    /// Base directory for asset loading. Relative asset paths passed to
    /// `GameContext::assets` are resolved against this. When `None`, the
    /// default is `assets` relative to the current working directory.
    #[serde(default)]
    pub asset_base_path: Option<String>,
    /// Optional path to persist player input bindings (JSON). When set, the
    /// engine loads bindings from this file at startup (writing the defaults
    /// if it doesn't exist — the file is hand-editable) and saves on exit.
    /// When `None`, the default two-player bindings are used in memory only.
    #[serde(default)]
    pub input_settings_path: Option<String>,
    /// Starting locale id (default `"en"`). Lookup falls back to `"en"`,
    /// then to the key itself, so an unknown id is harmless.
    #[serde(default = "default_locale")]
    pub locale: String,
    /// Directory containing `*.ron` locale files, relative to the asset
    /// base path (default `"locales"` → `assets/locales/`).
    #[serde(default = "default_locales_dir")]
    pub locales_dir: String,
    /// Default sampling filter for textures loaded through
    /// `GameContext::assets` (default [`TextureFilter::Linear`]). Pixel-art
    /// projects set [`TextureFilter::Nearest`] so texel edges stay hard.
    #[serde(default, with = "crate::texture_filter_serde")]
    pub texture_filter: TextureFilter,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            title: "Insiculous 2D Game".to_string(),
            width: 800,
            height: 600,
            target_fps: 60,
            clear_color: [0.1, 0.1, 0.15, 1.0],
            resizable: true,
            vsync: true,
            chaos_mode: ChaosMode::Normal,
            achievement_save_path: None,
            score_save_path: None,
            asset_base_path: None,
            input_settings_path: None,
            locale: default_locale(),
            locales_dir: default_locales_dir(),
            texture_filter: TextureFilter::Linear,
        }
    }
}

impl GameConfig {
    /// Create a new game configuration with the given title
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    /// Set the window size
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set the clear color
    pub fn with_clear_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.clear_color = [r, g, b, a];
        self
    }

    /// Set target FPS
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.target_fps = fps;
        self
    }

    /// Enable or disable vsync (enabled by default)
    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    /// Set the starting chaos mode. Games that let the player pick at runtime
    /// typically leave this at the default and mutate their own field.
    pub fn with_chaos_mode(mut self, mode: ChaosMode) -> Self {
        self.chaos_mode = mode;
        self
    }

    /// Persist achievement unlocks to this JSON path so they survive restarts.
    /// Parent directories are created on first save.
    pub fn with_achievement_save_path(mut self, path: impl Into<String>) -> Self {
        self.achievement_save_path = Some(path.into());
        self
    }

    /// Persist high scores to this JSON path so they survive restarts.
    /// Parent directories are created on first save.
    pub fn with_score_save_path(mut self, path: impl Into<String>) -> Self {
        self.score_save_path = Some(path.into());
        self
    }

    /// Resolve relative asset paths against this directory instead of
    /// `assets/` under the current working directory. Use an absolute path
    /// so the game runs correctly regardless of where it's launched from.
    pub fn with_asset_base_path(mut self, path: impl Into<String>) -> Self {
        self.asset_base_path = Some(path.into());
        self
    }

    /// Persist player input bindings to this JSON path (loaded at startup,
    /// defaults written if missing, saved on exit). Parent directories are
    /// created on first save.
    pub fn with_input_settings_path(mut self, path: impl Into<String>) -> Self {
        self.input_settings_path = Some(path.into());
        self
    }

    /// Set the starting locale id (default `"en"`).
    pub fn with_locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = locale.into();
        self
    }

    /// Set the locales directory, relative to the asset base path
    /// (default `"locales"`).
    pub fn with_locales_dir(mut self, dir: impl Into<String>) -> Self {
        self.locales_dir = dir.into();
        self
    }

    /// Sample textures loaded through the asset manager with this filter
    /// (default [`TextureFilter::Linear`]). Pixel-art games want
    /// [`TextureFilter::Nearest`].
    ///
    /// Applies to `AssetManager::load_texture` and
    /// `load_texture_from_bytes`. Solid-color, checkerboard and glyph
    /// textures stay Linear regardless; `create_texture_from_rgba` stays
    /// Nearest regardless.
    pub fn with_texture_filter(mut self, filter: TextureFilter) -> Self {
        self.texture_filter = filter;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_config_locale_serde_defaults() {
        // A config JSON from before localization must still deserialize.
        let legacy = r#"{
            "title": "Old",
            "width": 800,
            "height": 600,
            "target_fps": 60,
            "clear_color": [0.0, 0.0, 0.0, 1.0],
            "resizable": true
        }"#;
        let config: GameConfig = serde_json::from_str(legacy).expect("legacy config parses");
        assert_eq!(config.locale, "en");
        assert_eq!(config.locales_dir, "locales");
        // Same for a config written before the texture-filter knob existed.
        assert_eq!(config.texture_filter, TextureFilter::Linear);
        // And before the score-save knob existed.
        assert_eq!(config.score_save_path, None);
    }

    #[test]
    fn test_texture_filter_wire_format_is_stable() {
        let config = GameConfig::new("Test").with_texture_filter(TextureFilter::Nearest);

        let json = serde_json::to_string(&config).expect("config serializes");

        // Written as the variant name ...
        assert!(json.contains(r#""texture_filter":"Nearest""#), "got {json}");
        // ... read back unchanged ...
        let restored: GameConfig = serde_json::from_str(&json).expect("config parses");
        assert_eq!(restored.texture_filter, TextureFilter::Nearest);
        // ... a hand-edited lowercase alias is accepted ...
        let lowered = json.replace(r#""texture_filter":"Nearest""#, r#""texture_filter":"nearest""#);
        let restored: GameConfig = serde_json::from_str(&lowered).expect("lowercase alias parses");
        assert_eq!(restored.texture_filter, TextureFilter::Nearest);
        // ... and a typo fails loudly rather than coercing to Linear.
        let typo = json.replace(r#""texture_filter":"Nearest""#, r#""texture_filter":"Nearset""#);
        assert!(serde_json::from_str::<GameConfig>(&typo).is_err());
    }
}
