//! Achievement / trophy system.
//!
//! Engine-wide, game-agnostic: games register their own achievements at
//! startup and call `unlock(id)` when conditions are met. Unlocked state is
//! persisted to a JSON file so it survives restarts. A toast pops in via the
//! UI system when an achievement is unlocked for the first time.
//!
//! In a game, the manager is available as `ctx.achievements` inside
//! `Game::init()` / `Game::update()`.
//!
//! # Example
//! ```
//! use engine_core::prelude::*;
//!
//! // In Game::init(): register achievements (ctx.achievements in a real game)
//! let mut achievements = AchievementManager::in_memory();
//! achievements.register(Achievement::new(
//!     "first_blood",
//!     "First Blood",
//!     "Defeat your first enemy",
//! ));
//!
//! // In Game::update(): unlock when the condition is met
//! assert!(!achievements.is_unlocked("first_blood"));
//! achievements.unlock("first_blood");
//! assert!(achievements.is_unlocked("first_blood"));
//! assert_eq!(achievements.unlocked_count(), 1);
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::save_store;
use common::clock::{SystemTime, UNIX_EPOCH};
use glam::Vec2;
use serde::{Deserialize, Serialize};
use ui::UIContext;
use common::{Color, Rect};

/// An achievement definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    /// Stable identifier (e.g. `"first_blood"`). Used for unlocking and persistence.
    pub id: String,
    /// Display name shown on toast and in menus.
    pub name: String,
    /// Longer description of how to earn it.
    pub description: String,
    /// If true, name/description stay hidden until unlocked (secret achievement).
    pub hidden: bool,
}

impl Achievement {
    pub fn new(id: impl Into<String>, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            hidden: false,
        }
    }

    pub fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }
}

/// Per-achievement unlock record (persisted).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnlockRecord {
    /// Unix timestamp in seconds when the achievement was unlocked.
    unlocked_at: u64,
}

/// On-disk save format.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SaveFile {
    unlocks: HashMap<String, UnlockRecord>,
}

/// Active toast being displayed.
#[derive(Debug, Clone)]
struct Toast {
    achievement_id: String,
    name: String,
    description: String,
    remaining: f32,
}

/// Errors from achievement persistence.
#[derive(Debug, thiserror::Error)]
pub enum AchievementError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Default time (seconds) a toast stays visible before fading out.
pub const DEFAULT_TOAST_DURATION: f32 = 4.0;

/// Visual styling for achievement toasts.
///
/// Colors carry their base alpha; the fade-out over a toast's last second
/// multiplies that alpha at draw time. Override via
/// [`AchievementManager::set_toast_style`].
#[derive(Debug, Clone, PartialEq)]
pub struct ToastStyle {
    /// Toast panel width in pixels.
    pub width: f32,
    /// Toast panel height in pixels.
    pub height: f32,
    /// Margin from the window's top-right corner.
    pub margin: f32,
    /// Vertical spacing between stacked toasts.
    pub spacing: f32,
    /// Panel background color.
    pub background: Color,
    /// Panel border color.
    pub border: Color,
    /// Panel border width in pixels.
    pub border_width: f32,
    /// "Achievement Unlocked!" header color.
    pub title_color: Color,
    /// Achievement name color.
    pub name_color: Color,
    /// Achievement description color.
    pub description_color: Color,
    /// Header font size.
    pub title_size: f32,
    /// Achievement name font size.
    pub name_size: f32,
    /// Description font size.
    pub description_size: f32,
}

impl Default for ToastStyle {
    fn default() -> Self {
        Self {
            width: 320.0,
            height: 72.0,
            margin: 16.0,
            spacing: 8.0,
            background: Color::new(0.08, 0.08, 0.12, 0.92),
            border: Color::new(1.0, 0.82, 0.2, 1.0),
            border_width: 2.0,
            title_color: Color::new(1.0, 0.82, 0.2, 1.0),
            name_color: Color::new(1.0, 1.0, 1.0, 1.0),
            description_color: Color::new(0.8, 0.8, 0.85, 1.0),
            title_size: 14.0,
            name_size: 16.0,
            description_size: 12.0,
        }
    }
}

/// Multiply a style color's base alpha by the toast's fade factor.
fn faded(color: Color, fade: f32) -> Color {
    Color::new(color.r, color.g, color.b, color.a * fade)
}

/// Manages achievement registration, unlocking, persistence, and toasts.
pub struct AchievementManager {
    /// Registered achievement definitions, keyed by id.
    registered: HashMap<String, Achievement>,
    /// Unlock records loaded from disk / accumulated this session.
    unlocks: HashMap<String, UnlockRecord>,
    /// Toasts queued for display (FIFO).
    toasts: Vec<Toast>,
    /// Path to persist unlocks to. `None` disables persistence (useful for tests).
    save_path: Option<PathBuf>,
    /// How long each toast stays on screen.
    toast_duration: f32,
    /// Visual styling for toasts (dimensions, colors, font sizes).
    toast_style: ToastStyle,
}

impl AchievementManager {
    /// Create a manager with no persistence (in-memory only).
    pub fn in_memory() -> Self {
        Self {
            registered: HashMap::new(),
            unlocks: HashMap::new(),
            toasts: Vec::new(),
            save_path: None,
            toast_duration: DEFAULT_TOAST_DURATION,
            toast_style: ToastStyle::default(),
        }
    }

    /// Create a manager that persists unlocks to the given JSON file.
    ///
    /// If the file already exists, previously unlocked achievements are loaded.
    /// Missing file is treated as "nothing unlocked yet" (not an error).
    pub fn with_save_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut mgr = Self::in_memory();
        mgr.save_path = Some(path.clone());
        // Absence is queried through the save_store seam, not Path::exists() — the
        // latter is always false on wasm, where the slot is a localStorage key.
        match save_store::read(&path) {
            Ok(Some(_)) => {
                if let Err(e) = mgr.load() {
                    log::warn!("Failed to load achievements from {}: {}", path.display(), e);
                }
            }
            Ok(None) => {} // nothing unlocked yet
            Err(e) => {
                log::warn!("Failed to load achievements from {}: {}", path.display(), e);
            }
        }
        mgr
    }

    /// Override the toast duration (seconds).
    pub fn set_toast_duration(&mut self, seconds: f32) {
        self.toast_duration = seconds;
    }

    /// Override the toast appearance (dimensions, colors, font sizes).
    pub fn set_toast_style(&mut self, style: ToastStyle) {
        self.toast_style = style;
    }

    /// The current toast styling.
    pub fn toast_style(&self) -> &ToastStyle {
        &self.toast_style
    }

    /// Register an achievement definition. Call once per achievement at startup.
    ///
    /// Registering the same id twice overwrites the previous definition.
    pub fn register(&mut self, achievement: Achievement) {
        self.registered.insert(achievement.id.clone(), achievement);
    }

    /// Returns the definition for an id, if registered.
    pub fn get(&self, id: &str) -> Option<&Achievement> {
        self.registered.get(id)
    }

    /// All registered achievements (order not guaranteed).
    pub fn all(&self) -> impl Iterator<Item = &Achievement> {
        self.registered.values()
    }

    /// Number of registered achievements.
    pub fn total(&self) -> usize {
        self.registered.len()
    }

    /// Number of unlocked achievements.
    pub fn unlocked_count(&self) -> usize {
        self.unlocks.len()
    }

    /// True if the achievement with this id is unlocked.
    pub fn is_unlocked(&self, id: &str) -> bool {
        self.unlocks.contains_key(id)
    }

    /// Unlock an achievement by id. Returns true if this call actually unlocked
    /// it (i.e. it wasn't already unlocked). Idempotent — calling repeatedly is
    /// safe and only shows the toast once.
    ///
    /// If the id is not registered, this logs a warning and returns false.
    pub fn unlock(&mut self, id: &str) -> bool {
        if self.unlocks.contains_key(id) {
            return false;
        }
        let Some(def) = self.registered.get(id) else {
            log::warn!("unlock() called for unregistered achievement: {}", id);
            return false;
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.unlocks
            .insert(id.to_string(), UnlockRecord { unlocked_at: now });

        self.toasts.push(Toast {
            achievement_id: id.to_string(),
            name: def.name.clone(),
            description: def.description.clone(),
            remaining: self.toast_duration,
        });

        if let Some(path) = &self.save_path {
            let path = path.clone();
            if let Err(e) = self.save_to(&path, true) {
                log::warn!("Failed to save achievements: {}", e);
            }
        }

        log::info!("Achievement unlocked: {} ({})", def.name, id);
        true
    }

    /// Wipe all unlocked state (and persist the empty state if a save path is set).
    /// Typically used for dev/QA or a "reset progress" menu.
    pub fn reset(&mut self) {
        self.unlocks.clear();
        self.toasts.clear();
        if let Some(path) = &self.save_path {
            let path = path.clone();
            if let Err(e) = self.save_to(&path, false) {
                log::warn!("Failed to save achievements after reset: {}", e);
            }
        }
    }

    /// Advance toast timers. Called once per frame.
    pub fn tick(&mut self, delta_time: f32) {
        for toast in &mut self.toasts {
            toast.remaining -= delta_time;
        }
        self.toasts.retain(|t| t.remaining > 0.0);
    }

    /// Draw any active toasts in the top-right corner of the window.
    ///
    /// Toasts fade out over their last second of life.
    pub fn draw_toasts(&self, ui: &mut UIContext, window_size: Vec2) {
        let style = &self.toast_style;

        for (i, toast) in self.toasts.iter().enumerate() {
            let alpha = (toast.remaining / 1.0).clamp(0.0, 1.0);
            let x = window_size.x - style.width - style.margin;
            let y = style.margin + (style.height + style.spacing) * i as f32;

            let bg = faded(style.background, alpha);
            let border = faded(style.border, alpha);
            ui.panel_styled(Rect::new(x, y, style.width, style.height), bg, border, style.border_width);

            ui.label_styled(
                "Achievement Unlocked!",
                Vec2::new(x + 12.0, y + 10.0),
                faded(style.title_color, alpha),
                style.title_size,
            );
            ui.label_styled(
                &toast.name,
                Vec2::new(x + 12.0, y + 30.0),
                faded(style.name_color, alpha),
                style.name_size,
            );
            ui.label_styled(
                &toast.description,
                Vec2::new(x + 12.0, y + 52.0),
                faded(style.description_color, alpha),
                style.description_size,
            );
            let _ = toast.achievement_id; // reserved for future icon lookup
        }
    }

    /// Persist current unlock state to the configured save path.
    /// Returns `Ok(false)` with no action if no save path is configured.
    pub fn save(&self) -> Result<bool, AchievementError> {
        let Some(path) = &self.save_path else { return Ok(false); };
        self.save_to(path, true)?;
        Ok(true)
    }

    /// Persist through the [`crate::save_store`] seam (native: atomic JSON file;
    /// web: localStorage — see save_store.rs for the temp-file consequences).
    ///
    /// With `merge`, unlock records already in the slot are unioned into the
    /// outgoing set first, keeping the earliest `unlocked_at` per id — so a
    /// browser tab's save preserves unlocks another tab persisted earlier.
    /// (The read-merge-write is not atomic: same-instant saves from two tabs
    /// can still race, and the loser's unlock returns on its next save.)
    /// `reset()` passes `merge: false`: an explicit clear must actually clear.
    /// An unreadable or unparsable existing slot skips the merge (the write
    /// then replaces the corrupt state).
    fn save_to(&self, path: &Path, merge: bool) -> Result<(), AchievementError> {
        let mut unlocks = self.unlocks.clone();
        if merge {
            if let Ok(Some(existing)) = save_store::read(path) {
                if let Ok(disk) = serde_json::from_str::<SaveFile>(&existing) {
                    for (id, record) in disk.unlocks {
                        let entry = unlocks.entry(id).or_insert(UnlockRecord {
                            unlocked_at: record.unlocked_at,
                        });
                        entry.unlocked_at = entry.unlocked_at.min(record.unlocked_at);
                    }
                }
            }
        }
        let json = serde_json::to_string_pretty(&SaveFile { unlocks })?;
        save_store::write(path, &json)?;
        Ok(())
    }

    /// Reload unlock state from the configured save path, discarding any
    /// in-memory unlocks. Errors if no path is set or the slot is absent.
    pub fn load(&mut self) -> Result<(), AchievementError> {
        let Some(path) = &self.save_path else {
            return Err(AchievementError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no save path configured",
            )));
        };
        let data = save_store::read(path)?.ok_or_else(|| {
            AchievementError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no save data at {}", path.display()),
            ))
        })?;
        let save: SaveFile = serde_json::from_str(&data)?;
        self.unlocks = save.unlocks;
        Ok(())
    }
}

impl Default for AchievementManager {
    fn default() -> Self {
        Self::in_memory()
    }
}

#[cfg(test)]
mod tests;
