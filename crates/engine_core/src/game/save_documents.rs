//! The save documents a game runs with, opened from its `GameConfig`: the
//! achievements, the high scores and the player input bindings, each persisted
//! when the config names a save path and in memory otherwise. A config that also
//! names a former path — where a renamed game saved before — folds that slot in.

use std::path::Path;

use input::InputSettings;

use crate::achievements::AchievementManager;
use crate::game_config::GameConfig;
use crate::input_settings_io::{load_or_create, load_or_create_with_former};
use crate::scores::Scores;

/// The three save documents `GameRunner` owns.
pub(super) struct SaveDocuments {
    pub(super) achievements: AchievementManager,
    pub(super) scores: Scores,
    pub(super) player_input: InputSettings,
}

/// Open each document from its save path, with its former path when the config
/// carries one. A former path without a save path is ignored: there is nowhere
/// to fold it into.
pub(super) fn open_save_documents(config: &GameConfig) -> SaveDocuments {
    let achievements = match (&config.achievement_save_path, &config.former_achievement_save_path) {
        (Some(path), Some(former_path)) => AchievementManager::with_save_path_and_former(path, former_path),
        (Some(path), None) => AchievementManager::with_save_path(path),
        (None, _) => AchievementManager::in_memory(),
    };
    let scores = match (&config.score_save_path, &config.former_score_save_path) {
        (Some(path), Some(former_path)) => Scores::with_save_path_and_former(path, former_path),
        (Some(path), None) => Scores::with_save_path(path),
        (None, _) => Scores::in_memory(),
    };
    let player_input = match (&config.input_settings_path, &config.former_input_settings_path) {
        (Some(path), Some(former_path)) => {
            load_or_create_with_former(Path::new(path), Path::new(former_path))
        }
        (Some(path), None) => load_or_create(Path::new(path)),
        (None, _) => InputSettings::default_two_player(),
    };
    SaveDocuments { achievements, scores, player_input }
}

#[cfg(test)]
mod tests {
    use super::*;
    use input::PlayerId;

    fn path_string(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn a_renamed_game_boots_holding_everything_its_former_slots_saved() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let former_achievements = dir.path().join("old_achievements.json");
        let former_scores = dir.path().join("old_scores.json");
        let former_input = dir.path().join("old_input.json");
        std::fs::write(&former_achievements, r#"{"unlocks":{"first_win":{"unlocked_at":5}}}"#)?;
        std::fs::write(&former_scores, r#"{"modes":{"solo":[{"score":120,"at":5}]}}"#)?;
        let mut remapped = InputSettings::default_two_player();
        remapped.assign_pad(PlayerId::P2, Some(3));
        crate::input_settings_io::save(&former_input, &remapped)?;

        let config = GameConfig::new("Renamed")
            .with_achievement_save_path(path_string(&dir.path().join("achievements.json")))
            .with_score_save_path(path_string(&dir.path().join("scores.json")))
            .with_input_settings_path(path_string(&dir.path().join("input.json")))
            .with_former_achievement_save_path(path_string(&former_achievements))
            .with_former_score_save_path(path_string(&former_scores))
            .with_former_input_settings_path(path_string(&former_input));

        let documents = open_save_documents(&config);

        assert!(documents.achievements.is_unlocked("first_win"));
        assert_eq!(documents.scores.best("solo"), Some(120));
        assert_eq!(documents.player_input.pad_of(PlayerId::P2), Some(3));
        Ok(())
    }

    #[test]
    fn a_config_without_former_paths_opens_only_its_own_slots() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let config = GameConfig::new("Plain")
            .with_achievement_save_path(path_string(&dir.path().join("achievements.json")))
            .with_score_save_path(path_string(&dir.path().join("scores.json")));

        let documents = open_save_documents(&config);

        assert!(!documents.achievements.is_unlocked("first_win"));
        assert_eq!(documents.scores.best("solo"), None);
        assert_eq!(documents.player_input.pad_of(PlayerId::P2), Some(1), "no input path: in-memory defaults");
        Ok(())
    }
}
