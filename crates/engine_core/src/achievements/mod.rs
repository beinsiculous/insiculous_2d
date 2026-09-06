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

mod toast;

pub use toast::{ToastStyle, DEFAULT_TOAST_DURATION};
use toast::ToastQueue;

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use crate::save_store::{unix_seconds, JsonSaveSlot, MergeOnLoad, SaveError};
use glam::Vec2;
use serde::{Deserialize, Serialize};
use ui::UIContext;

pub type AchievementError = SaveError;

/// `--achievements-manifest` was given without a usable path — the export was asked for and
/// cannot happen, which must never fall through to opening the game.
#[derive(Debug, thiserror::Error)]
#[error("--achievements-manifest needs a file path")]
pub struct ManifestFlagError;

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

impl MergeOnLoad for SaveFile {
    fn merge_from_disk(&mut self, disk: Self) {
        for (id, record) in disk.unlocks {
            let entry = self.unlocks.entry(id).or_insert(UnlockRecord {
                unlocked_at: record.unlocked_at,
            });
            entry.unlocked_at = entry.unlocked_at.min(record.unlocked_at);
        }
    }
}

/// Manages achievement registration, unlocking, persistence, and toasts.
pub struct AchievementManager {
    /// Registered achievement definitions, keyed by id.
    registered: HashMap<String, Achievement>,
    /// Achievement ids in registration order.
    registration_order: Vec<String>,
    /// Save slot for unlock records.
    slot: JsonSaveSlot<SaveFile>,
    /// Toasts queued for display (FIFO).
    toasts: ToastQueue,
}

impl AchievementManager {
    /// Create a manager with no persistence (in-memory only).
    pub fn in_memory() -> Self {
        Self {
            registered: HashMap::new(),
            registration_order: Vec::new(),
            slot: JsonSaveSlot::in_memory(),
            toasts: ToastQueue::new(),
        }
    }

    /// Create a manager that persists unlocks to the given JSON file.
    ///
    /// If the file already exists, previously unlocked achievements are loaded.
    /// Missing file is treated as "nothing unlocked yet" (not an error).
    pub fn with_save_path(path: impl Into<PathBuf>) -> Self {
        Self {
            registered: HashMap::new(),
            registration_order: Vec::new(),
            slot: JsonSaveSlot::with_path(path),
            toasts: ToastQueue::new(),
        }
    }

    /// Override the toast duration (seconds).
    pub fn set_toast_duration(&mut self, seconds: f32) {
        self.toasts.duration = seconds;
    }

    /// Override the toast appearance (dimensions, colors, font sizes).
    pub fn set_toast_style(&mut self, style: ToastStyle) {
        self.toasts.style = style;
    }

    /// The current toast styling.
    pub fn toast_style(&self) -> &ToastStyle {
        &self.toasts.style
    }

    /// Register an achievement definition. Call once per achievement at startup.
    ///
    /// Registering the same id twice overwrites the previous definition while
    /// keeping its position in registration order.
    pub fn register(&mut self, achievement: Achievement) {
        let id = achievement.id.clone();
        if self.registered.insert(id.clone(), achievement).is_none() {
            self.registration_order.push(id);
        }
    }

    /// Returns the definition for an id, if registered.
    pub fn get(&self, id: &str) -> Option<&Achievement> {
        self.registered.get(id)
    }

    /// All registered achievements in registration order; re-registering an id
    /// keeps its place.
    pub fn all(&self) -> impl Iterator<Item = &Achievement> {
        self.registration_order
            .iter()
            .filter_map(|id| self.registered.get(id))
    }

    /// Number of registered achievements.
    pub fn total(&self) -> usize {
        self.registered.len()
    }

    /// Number of unlocked achievements.
    pub fn unlocked_count(&self) -> usize {
        self.slot.data().unlocks.len()
    }

    /// True if the achievement with this id is unlocked.
    pub fn is_unlocked(&self, id: &str) -> bool {
        self.slot.data().unlocks.contains_key(id)
    }

    /// Unlock an achievement by id. Returns true if this call actually unlocked
    /// it (i.e. it wasn't already unlocked). Idempotent — calling repeatedly is
    /// safe and only shows the toast once.
    ///
    /// If the id is not registered, this logs a warning and returns false.
    pub fn unlock(&mut self, id: &str) -> bool {
        if self.is_unlocked(id) {
            return false;
        }
        let Some(def) = self.registered.get(id) else {
            log::warn!("unlock() called for unregistered achievement: {}", id);
            return false;
        };

        let now = unix_seconds();
        self.slot.data_mut().unlocks.insert(id.to_string(), UnlockRecord { unlocked_at: now });

        self.toasts.push(id.to_string(), def.name.clone(), def.description.clone());

        if let Err(e) = self.slot.save_with_merge() {
            log::warn!("Failed to save achievements: {}", e);
        }

        log::info!("Achievement unlocked: {} ({})", def.name, id);
        true
    }

    /// Wipe all unlocked state (and persist the empty state if a save path is set).
    /// Typically used for dev/QA or a "reset progress" menu.
    pub fn reset(&mut self) {
        self.slot.data_mut().unlocks.clear();
        self.toasts.clear();
        if let Err(e) = self.slot.save_without_merge() {
            log::warn!("Failed to save achievements after reset: {}", e);
        }
    }

    /// Advance toast timers. Called once per frame.
    pub fn tick(&mut self, delta_time: f32) {
        self.toasts.tick(delta_time);
    }

    /// Draw any active toasts in the top-right corner of the window.
    ///
    /// Toasts fade out over their last second of life.
    pub fn draw_toasts(&self, ui: &mut UIContext, window_size: Vec2) {
        self.toasts.draw(ui, window_size);
    }

    /// Persist current unlock state to the configured save path.
    /// Returns `Ok(false)` with no action if no save path is configured.
    pub fn save(&self) -> Result<bool, AchievementError> {
        self.slot.save_with_merge()
    }

    /// Reload unlock state from the configured save path, discarding any
    /// in-memory unlocks. Errors if no path is set or the slot is absent.
    pub fn load(&mut self) -> Result<(), AchievementError> {
        self.slot.reload()
    }

    /// The registry as the site's manifest: a JSON array of every registered achievement, in
    /// registration order, each the serde form of `Achievement` — {id, name, description, hidden}.
    /// Pretty-printed with a trailing newline. This is the file docs/WEB_SAVES.md § The manifest
    /// specifies; the site parses these four fields and nothing else.
    pub fn manifest_json(&self) -> Result<String, AchievementError> {
        let entries: Vec<&Achievement> = self.all().collect();
        let mut json = serde_json::to_string_pretty(&entries)?;
        json.push('\n');
        Ok(json)
    }

    /// Write manifest_json() to `path` through the save store's atomic write (temp file + rename,
    /// parent directories created), so a reader never sees a half-written file and a build
    /// script's output dir need not exist yet. Native only: the flag that asks for it has no web
    /// form.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write_manifest(&self, path: &Path) -> Result<(), AchievementError> {
        let json = self.manifest_json()?;
        crate::save_store::write(path, &json)?;
        Ok(())
    }
}

/// The path after `--achievements-manifest` in `args`, in either form —
/// `--achievements-manifest <path>` or `--achievements-manifest=<path>`: Ok(None) when the flag
/// is absent, Ok(Some(path)) when it carries a value, and Err(ManifestFlagError)
/// when it is the last token, the next token starts with `--`, or the `=` form is empty — an
/// export that was asked for and cannot happen must never fall through to opening the game.
/// The space form refuses a value starting with `--` because that is how a forgotten path
/// looks; a path that really starts with `--` takes the `=` form. Arguments arrive as
/// `OsString` so a non-UTF-8 argument on a native launch is never a panic: the flag is matched
/// through a lossy view and the path keeps its bytes. Pure, so the parse is tested without
/// touching the real command line.
pub fn manifest_export_path_from(
    args: impl Iterator<Item = OsString>,
) -> Result<Option<PathBuf>, ManifestFlagError> {
    let mut args = args;
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy();
        if text == "--achievements-manifest" {
            let next_arg = args.next();
            match next_arg {
                Some(value) if !value.to_string_lossy().starts_with("--") => {
                    return Ok(Some(PathBuf::from(value)))
                }
                _ => return Err(ManifestFlagError),
            }
        } else if let Some(value) = text.strip_prefix("--achievements-manifest=") {
            if value.is_empty() {
                return Err(ManifestFlagError);
            }
            return Ok(Some(PathBuf::from(value)));
        }
    }
    Ok(None)
}

/// Parse `--achievements-manifest` from command-line arguments.
#[cfg(not(target_arch = "wasm32"))]
pub fn manifest_export_path() -> Result<Option<PathBuf>, ManifestFlagError> {
    manifest_export_path_from(std::env::args_os().skip(1))
}

impl Default for AchievementManager {
    fn default() -> Self {
        Self::in_memory()
    }
}

#[cfg(test)]
mod tests;
