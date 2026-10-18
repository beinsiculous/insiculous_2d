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
    load_or_create_from(path, None)
}

/// [`load_or_create`] for a renamed game, whose players' bindings sit at
/// `former_path`, the slot it saved to before. The former slot is consulted
/// only when `path` is missing: its settings, if they parse at the current
/// version, are written to `path` and returned in place of the defaults, so the
/// two slots stop being two sources of truth at the first boot. A former slot
/// that is missing, corrupt, at another version or unreadable gives the
/// defaults, written to `path` as for a plain missing file — a read fault at
/// that one boot therefore costs the former bindings, since the primary now
/// exists and the former is never looked at again; the engine's exit save
/// would write the same defaults anyway, bindings being last-writer-wins by
/// design. Every other case is [`load_or_create`]'s, and the former slot is
/// never written.
pub fn load_or_create_with_former(path: &Path, former_path: &Path) -> InputSettings {
    load_or_create_from(path, Some(former_path))
}

fn load_or_create_from(path: &Path, former_path: Option<&Path>) -> InputSettings {
    // Absence is queried through the save_store seam, not Path::exists() — the
    // latter is always false on wasm, where the slot is a localStorage key.
    let contents = match crate::save_store::read(path) {
        Ok(Some(contents)) => contents,
        Ok(None) => {
            let settings = former_path
                .and_then(read_former)
                .unwrap_or_else(InputSettings::default_two_player);
            if let Err(e) = save(path, &settings) {
                log::warn!(
                    "Could not write input settings to {}: {}",
                    path.display(),
                    e
                );
            }
            return settings;
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
    parse(path, &contents).unwrap_or_else(InputSettings::default_two_player)
}

/// The settings in a former slot, or `None` — logged — when it is missing,
/// unreadable, corrupt or at another version.
fn read_former(former_path: &Path) -> Option<InputSettings> {
    match crate::save_store::read(former_path) {
        Ok(Some(contents)) => parse(former_path, &contents),
        Ok(None) => None,
        Err(e) => {
            log::warn!(
                "Could not read former input settings {}: {} — using defaults",
                former_path.display(),
                e
            );
            None
        }
    }
}

/// `contents` as settings at the current version, or `None` with a warning.
fn parse(path: &Path, contents: &str) -> Option<InputSettings> {
    match serde_json::from_str::<SettingsFile>(contents) {
        Ok(file) if file.version == SETTINGS_VERSION => Some(file.into()),
        Ok(file) => {
            log::warn!(
                "Input settings {} has version {} (expected {}) — using defaults",
                path.display(),
                file.version,
                SETTINGS_VERSION
            );
            None
        }
        Err(e) => {
            log::warn!(
                "Could not parse input settings {}: {} — using defaults",
                path.display(),
                e
            );
            None
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

    #[test]
    fn round_trip_preserves_pads_and_bindings() -> Result<(), InputSettingsError> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("input.json");
        let mut settings = InputSettings::default_two_player();
        settings.assign_pad(PlayerId::P2, Some(3));
        if let Some(p1) = settings.player_mut(PlayerId::P1) {
            p1.bind(GameAction::Action3, PlayerSource::Keyboard(KeyCode::KeyQ));
        }

        save(&path, &settings)?;
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
        Ok(())
    }

    #[test]
    fn missing_file_returns_defaults_and_creates_hand_editable_file() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("saves/input.json");

        let settings = load_or_create(&path);

        assert_eq!(settings.player_count(), 2);
        assert!(path.exists(), "defaults should be written for hand-editing");
        // The created file itself parses back to the same defaults.
        let reloaded = load_or_create(&path);
        assert_eq!(reloaded.pad_of(PlayerId::P1), Some(0));
        assert_eq!(reloaded.pad_of(PlayerId::P2), Some(1));
        Ok(())
    }

    #[test]
    fn corrupt_or_wrong_version_file_falls_back_to_defaults_without_panicking(
    ) -> Result<(), std::io::Error> {
        for (label, contents) in [
            ("corrupt", "{ not valid json !!!"),
            ("wrong version", r#"{"version": 99, "players": []}"#),
        ] {
            let dir = tempfile::tempdir()?;
            let path = dir.path().join("input.json");
            std::fs::write(&path, contents)?;

            let settings = load_or_create(&path);

            assert_eq!(settings.player_count(), 2, "{label}");
            assert_eq!(settings.pad_of(PlayerId::P1), Some(0), "{label}");
        }
        Ok(())
    }

    fn settings_with_player_two_on_pad(pad: u32) -> InputSettings {
        let mut settings = InputSettings::default_two_player();
        settings.assign_pad(PlayerId::P2, Some(pad));
        settings
    }

    #[test]
    fn a_present_primary_wins_over_the_former() -> Result<(), InputSettingsError> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("new.json");
        let former_path = dir.path().join("old.json");
        save(&path, &settings_with_player_two_on_pad(4))?;
        save(&former_path, &settings_with_player_two_on_pad(3))?;

        let settings = load_or_create_with_former(&path, &former_path);

        assert_eq!(settings.pad_of(PlayerId::P2), Some(4));
        Ok(())
    }

    #[test]
    fn an_absent_primary_takes_the_former_and_writes_it_to_the_primary() -> Result<(), InputSettingsError> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("new.json");
        let former_path = dir.path().join("old.json");
        save(&former_path, &settings_with_player_two_on_pad(3))?;
        let former_bytes = std::fs::read(&former_path)?;

        let settings = load_or_create_with_former(&path, &former_path);

        assert_eq!(settings.pad_of(PlayerId::P2), Some(3));
        assert_eq!(load_or_create(&path).pad_of(PlayerId::P2), Some(3), "the primary now holds the former's bindings");
        assert_eq!(std::fs::read(&former_path)?, former_bytes, "the former slot is never written");
        Ok(())
    }

    #[test]
    fn an_absent_primary_with_an_absent_or_corrupt_former_writes_the_defaults() -> Result<(), std::io::Error> {
        for (label, former_contents) in [
            ("absent", None),
            ("corrupt", Some("{ not valid json !!!")),
            ("wrong version", Some(r#"{"version": 99, "players": []}"#)),
        ] {
            let dir = tempfile::tempdir()?;
            let path = dir.path().join("new.json");
            let former_path = dir.path().join("old.json");
            if let Some(contents) = former_contents {
                std::fs::write(&former_path, contents)?;
            }

            let settings = load_or_create_with_former(&path, &former_path);

            assert_eq!(settings.pad_of(PlayerId::P2), Some(1), "{label}");
            assert!(path.exists(), "{label}: the defaults are written to the primary");
            assert_eq!(load_or_create(&path).pad_of(PlayerId::P2), Some(1), "{label}");
        }
        Ok(())
    }

    #[test]
    fn an_unreadable_former_gives_the_defaults_and_writes_them_like_a_missing_one() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("new.json");
        let former_path = dir.path().join("old.json");
        // A directory reads as an error that is not NotFound — a read fault, not an absent slot.
        std::fs::create_dir(&former_path)?;

        let settings = load_or_create_with_former(&path, &former_path);

        assert_eq!(settings.pad_of(PlayerId::P2), Some(1));
        assert_eq!(load_or_create(&path).pad_of(PlayerId::P2), Some(1), "the defaults are written to the primary");
        Ok(())
    }

    #[test]
    fn a_wrong_version_primary_gives_the_defaults_over_a_valid_former_and_writes_nothing() -> Result<(), InputSettingsError> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("new.json");
        let former_path = dir.path().join("old.json");
        let wrong_version = r#"{"version": 99, "players": []}"#;
        std::fs::write(&path, wrong_version)?;
        save(&former_path, &settings_with_player_two_on_pad(3))?;

        let settings = load_or_create_with_former(&path, &former_path);

        assert_eq!(settings.pad_of(PlayerId::P2), Some(1), "present-but-invalid is not absent");
        assert_eq!(std::fs::read_to_string(&path)?, wrong_version);
        Ok(())
    }
}
