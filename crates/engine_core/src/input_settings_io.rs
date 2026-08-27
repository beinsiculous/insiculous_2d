//! Load/save for player input bindings ([`InputSettings`]).
//!
//! The on-disk shape is a versioned JSON DTO with bindings stored as sorted
//! entry lists (not maps), so the file diffs cleanly and stays hand-editable.
//! Robustness contract: a missing file writes the defaults (hand-edit
//! rebinding works without a UI); a corrupt or wrong-version file logs a
//! warning and falls back to defaults — loading never panics.

use std::path::Path;

use input::{GameAction, InputSettings, PlayerBindings, PlayerId, PlayerSource};
use serde::{Deserialize, Serialize};

/// Current settings-file schema version.
const SETTINGS_VERSION: u32 = 1;

/// Errors from saving input settings
#[derive(Debug, thiserror::Error)]
pub enum InputSettingsError {
    #[error("Input settings IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Input settings serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize)]
struct BindingEntry {
    action: GameAction,
    sources: Vec<PlayerSource>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlayerEntry {
    pad: Option<u32>,
    bindings: Vec<BindingEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SettingsFile {
    version: u32,
    players: Vec<PlayerEntry>,
}

impl From<&InputSettings> for SettingsFile {
    fn from(settings: &InputSettings) -> Self {
        let players = (0..settings.player_count() as u8)
            .filter_map(|i| settings.player(PlayerId(i)))
            .map(|player| {
                let mut bindings: Vec<BindingEntry> = player
                    .all_bindings()
                    .map(|(action, sources)| BindingEntry {
                        action,
                        sources: sources.to_vec(),
                    })
                    .collect();
                // HashMap iteration order is arbitrary — sort by the action's
                // debug form so the file is stable across saves.
                bindings.sort_by_key(|entry| format!("{:?}", entry.action));
                PlayerEntry {
                    pad: player.pad(),
                    bindings,
                }
            })
            .collect();
        Self {
            version: SETTINGS_VERSION,
            players,
        }
    }
}

impl From<SettingsFile> for InputSettings {
    fn from(file: SettingsFile) -> Self {
        let players = file
            .players
            .into_iter()
            .map(|entry| {
                let mut bindings = PlayerBindings::new();
                bindings.set_pad(entry.pad);
                for binding in entry.bindings {
                    for source in binding.sources {
                        bindings.bind(binding.action, source);
                    }
                }
                bindings
            })
            .collect();
        InputSettings::from_players(players)
    }
}

/// Load input settings from `path`.
///
/// - File exists and parses at the current version → those settings.
/// - File missing → the default two-player settings are written to `path`
///   (so players can hand-edit bindings) and returned.
/// - File unreadable, corrupt, or wrong version → warn + defaults; the file
///   is left untouched.
pub fn load_or_create(path: &Path) -> InputSettings {
    // Absence is queried through the save_store seam, not Path::exists() — the
    // latter is always false on wasm, where the slot is a localStorage key.
    let contents = match crate::save_store::read(path) {
        Ok(Some(contents)) => contents,
        Ok(None) => {
            let defaults = InputSettings::default_two_player();
            if let Err(e) = save(path, &defaults) {
                log::warn!(
                    "Could not write default input settings to {}: {}",
                    path.display(),
                    e
                );
            }
            return defaults;
        }
        Err(e) => {
            log::warn!(
                "Could not read input settings {}: {} — using defaults",
                path.display(),
                e
            );
            return InputSettings::default_two_player();
        }
    };

    match serde_json::from_str::<SettingsFile>(&contents) {
        Ok(file) if file.version == SETTINGS_VERSION => file.into(),
        Ok(file) => {
            log::warn!(
                "Input settings {} has version {} (expected {}) — using defaults",
                path.display(),
                file.version,
                SETTINGS_VERSION
            );
            InputSettings::default_two_player()
        }
        Err(e) => {
            log::warn!(
                "Could not parse input settings {}: {} — using defaults",
                path.display(),
                e
            );
            InputSettings::default_two_player()
        }
    }
}

/// Save input settings to `path` as pretty JSON through the [`crate::save_store`]
/// seam (native: atomic file write, parents created; web: localStorage).
pub fn save(path: &Path, settings: &InputSettings) -> Result<(), InputSettingsError> {
    let file = SettingsFile::from(settings);
    let json = serde_json::to_string_pretty(&file)?;
    crate::save_store::write(path, &json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use input::{AxisDirection, GamepadAxis, GamepadButton};
    use winit::keyboard::KeyCode;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join("insiculous_input_settings_tests")
            .join(format!("{}_{}.json", name, std::process::id()))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn round_trip_preserves_pads_and_bindings() {
        let path = temp_path("round_trip");
        cleanup(&path);

        let mut settings = InputSettings::default_two_player();
        settings.assign_pad(PlayerId::P2, Some(3));
        if let Some(p1) = settings.player_mut(PlayerId::P1) {
            p1.bind(GameAction::Action3, PlayerSource::Keyboard(KeyCode::KeyQ));
        }

        save(&path, &settings).expect("save should succeed");
        let restored = load_or_create(&path);

        assert_eq!(restored.player_count(), 2);
        assert_eq!(restored.pad_of(PlayerId::P1), Some(0));
        assert_eq!(restored.pad_of(PlayerId::P2), Some(3));
        let p1 = restored.player(PlayerId::P1).expect("P1 exists");
        assert!(p1
            .bindings(GameAction::Action3)
            .contains(&PlayerSource::Keyboard(KeyCode::KeyQ)));
        assert!(p1
            .bindings(GameAction::MoveUp)
            .contains(&PlayerSource::PadAxis(GamepadAxis::LeftStickY, AxisDirection::Positive)));
        assert!(p1
            .bindings(GameAction::Action1)
            .contains(&PlayerSource::PadButton(GamepadButton::A)));
        cleanup(&path);
    }

    #[test]
    fn missing_file_returns_defaults_and_creates_hand_editable_file() {
        let path = temp_path("missing");
        cleanup(&path);

        let settings = load_or_create(&path);
        assert_eq!(settings.player_count(), 2);
        assert!(path.exists(), "defaults should be written for hand-editing");

        // The created file itself parses back to the same defaults
        let reloaded = load_or_create(&path);
        assert_eq!(reloaded.pad_of(PlayerId::P1), Some(0));
        assert_eq!(reloaded.pad_of(PlayerId::P2), Some(1));
        cleanup(&path);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults_without_panicking() {
        let path = temp_path("corrupt");
        std::fs::create_dir_all(path.parent().expect("has parent")).expect("mkdir");
        std::fs::write(&path, "{ not valid json !!!").expect("write");

        let settings = load_or_create(&path);
        assert_eq!(settings.player_count(), 2);
        assert_eq!(settings.pad_of(PlayerId::P1), Some(0));
        cleanup(&path);
    }

    #[test]
    fn wrong_version_falls_back_to_defaults() {
        let path = temp_path("wrong_version");
        std::fs::create_dir_all(path.parent().expect("has parent")).expect("mkdir");
        std::fs::write(&path, r#"{"version": 99, "players": []}"#).expect("write");

        let settings = load_or_create(&path);
        assert_eq!(settings.player_count(), 2);
        cleanup(&path);
    }

    #[test]
    fn save_creates_nested_parent_directories() {
        let dir = std::env::temp_dir()
            .join("insiculous_input_settings_tests")
            .join(format!("nested_{}", std::process::id()))
            .join("a")
            .join("b");
        let path = dir.join("settings.json");
        let _ = std::fs::remove_dir_all(&dir);

        save(&path, &InputSettings::default_two_player()).expect("save should mkdir -p");
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
