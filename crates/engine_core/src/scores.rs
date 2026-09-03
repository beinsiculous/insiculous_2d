//! High-score persistence: engine owns the storage, games own the meaning.
//!
//! [`Scores`] keeps the top [`MAX_SCORES_PER_MODE`] entries per game-defined
//! mode string (e.g. `"single"` / `"coop"`), persisted through the
//! [`crate::save_store`] seam as pretty JSON — a file like `saves/pong_scores.json`
//! natively, a localStorage key like `beinsiculous.games.pong.scores` on the
//! web (contract: `docs/WEB_SAVES.md`). Wired to
//! `GameConfig::with_score_save_path` and exposed to games as `ctx.scores`.
//!
//! Persistence follows the achievements posture: write-through on
//! [`Scores::submit`], saves merge with what's already in the slot (a tab's
//! save preserves entries another tab persisted earlier; same-instant saves
//! can still race, non-atomically), and every failure is logged, never
//! panicked.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use common::clock::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

use crate::save_store;

/// How many entries each mode's list keeps (score-descending).
pub const MAX_SCORES_PER_MODE: usize = 10;

/// One recorded score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreEntry {
    /// The score value; the game defines its meaning and scale.
    pub score: u64,
    /// Unix seconds when the score was submitted.
    pub at: u64,
}

/// Errors from score persistence.
#[derive(Debug, thiserror::Error)]
pub enum ScoresError {
    #[error("Score IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Score serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// The on-disk / localStorage JSON shape.
#[derive(Debug, Default, Serialize, Deserialize)]
struct ScoresFile {
    modes: HashMap<String, Vec<ScoreEntry>>,
}

/// Top-N high-score lists per game-defined mode, with optional persistence.
pub struct Scores {
    modes: HashMap<String, Vec<ScoreEntry>>,
    /// Persistence slot. `None` disables persistence (useful for tests).
    save_path: Option<PathBuf>,
}

impl Scores {
    /// Create score lists with no persistence (in-memory only).
    pub fn in_memory() -> Self {
        Self {
            modes: HashMap::new(),
            save_path: None,
        }
    }

    /// Create score lists that persist to the given slot, loading any
    /// existing entries. A missing slot is "no scores yet"; a corrupt one
    /// warns and starts fresh (it is replaced on the next qualifying submit).
    pub fn with_save_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut scores = Self::in_memory();
        match save_store::read(&path) {
            Ok(Some(contents)) => match serde_json::from_str::<ScoresFile>(&contents) {
                Ok(file) => {
                    scores.modes = file.modes;
                    for list in scores.modes.values_mut() {
                        sort_and_truncate(list);
                    }
                }
                Err(e) => {
                    log::warn!("Could not parse scores {}: {} — starting fresh", path.display(), e);
                }
            },
            Ok(None) => {}
            Err(e) => {
                log::warn!("Could not read scores {}: {} — starting fresh", path.display(), e);
            }
        }
        scores.save_path = Some(path);
        scores
    }

    /// Record `score` under `mode`. Returns true when it entered the top
    /// [`MAX_SCORES_PER_MODE`] (persisting write-through, errors logged);
    /// a non-qualifying score changes nothing and returns false. On a full
    /// list, matching the lowest entry does NOT qualify — a tie doesn't
    /// displace the score it ties with; among entries that are in, equal
    /// scores rank oldest-first.
    pub fn submit(&mut self, mode: &str, score: u64) -> bool {
        let at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let list = self.modes.entry(mode.to_string()).or_default();
        if list.len() >= MAX_SCORES_PER_MODE
            && list.last().is_some_and(|lowest| score <= lowest.score)
        {
            return false;
        }
        // Equal scores keep the older entry first: insert after existing ties.
        let index = list.partition_point(|e| e.score >= score);
        list.insert(index, ScoreEntry { score, at });
        list.truncate(MAX_SCORES_PER_MODE);

        if let Some(path) = &self.save_path {
            let path = path.clone();
            if let Err(e) = self.save_to(&path, true) {
                log::warn!("Failed to save scores: {}", e);
            }
        }
        true
    }

    /// The best score recorded for `mode`, if any.
    pub fn best(&self, mode: &str) -> Option<u64> {
        self.modes.get(mode).and_then(|l| l.first()).map(|e| e.score)
    }

    /// The top entries for `mode`, best first (empty for an unknown mode).
    pub fn top(&self, mode: &str) -> &[ScoreEntry] {
        self.modes.get(mode).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Persist current score state to the configured slot.
    /// Returns `Ok(false)` with no action if no save path is configured.
    pub fn save(&self) -> Result<bool, ScoresError> {
        let Some(path) = &self.save_path else { return Ok(false); };
        self.save_to(path, true)?;
        Ok(true)
    }

    /// Wipe all score state (and persist the empty state if a slot is set).
    pub fn reset(&mut self) {
        self.modes.clear();
        if let Some(path) = &self.save_path {
            let path = path.clone();
            if let Err(e) = self.save_to(&path, false) {
                log::warn!("Failed to save scores after reset: {}", e);
            }
        }
    }

    /// Persist through the save_store seam. With `merge`, entries already in the
    /// slot are unioned in first (dedup on exact score+timestamp pairs, then
    /// re-sorted and truncated) — the multi-tab posture achievements use.
    /// `reset()` passes `merge: false` so an explicit clear actually clears.
    fn save_to(&self, path: &Path, merge: bool) -> Result<(), ScoresError> {
        let mut modes = self.modes.clone();
        if merge {
            if let Ok(Some(existing)) = save_store::read(path) {
                if let Ok(disk) = serde_json::from_str::<ScoresFile>(&existing) {
                    for (mode, entries) in disk.modes {
                        let list = modes.entry(mode).or_default();
                        for entry in entries {
                            if !list.contains(&entry) {
                                list.push(entry);
                            }
                        }
                        sort_and_truncate(list);
                    }
                }
            }
        }
        let json = serde_json::to_string_pretty(&ScoresFile { modes })?;
        save_store::write(path, &json)?;
        Ok(())
    }
}

/// Score-descending, ties oldest-first, capped at [`MAX_SCORES_PER_MODE`].
fn sort_and_truncate(list: &mut Vec<ScoreEntry>) {
    list.sort_by(|a, b| b.score.cmp(&a.score).then(a.at.cmp(&b.at)));
    list.truncate(MAX_SCORES_PER_MODE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_list_rejects_non_qualifying_and_evicts_lowest() {
        let mut scores = Scores::in_memory();
        assert!(scores.submit("solo", 100));
        assert!(scores.submit("solo", 300));
        assert!(scores.submit("solo", 200));
        let top: Vec<u64> = scores.top("solo").iter().map(|e| e.score).collect();
        assert_eq!(top, [300, 200, 100], "best first");
        assert_eq!(scores.best("solo"), Some(300));
        assert_eq!(scores.best("versus"), None, "unknown mode has no best");

        // Modes are independent lists.
        scores.submit("coop", 999);
        assert_eq!(scores.best("coop"), Some(999));
        assert_eq!(scores.best("solo"), Some(300));

        let mut scores = Scores::in_memory();
        for s in 1..=MAX_SCORES_PER_MODE as u64 {
            assert!(scores.submit("solo", s * 10));
        }
        assert!(!scores.submit("solo", 5), "below the lowest of a full list");
        assert_eq!(scores.top("solo").len(), MAX_SCORES_PER_MODE);

        assert!(scores.submit("solo", 55), "mid-list score qualifies");
        assert_eq!(scores.top("solo").len(), MAX_SCORES_PER_MODE, "list stays capped");
        assert_eq!(scores.best("solo"), Some(100));
        assert!(scores.top("solo").iter().all(|e| e.score != 10), "lowest entry evicted");
    }

    #[test]
    fn test_equal_scores_rank_the_earlier_entry_first() {
        // Exercise the comparator directly with distinct timestamps —
        // submit() in a test stamps both entries in the same unix second,
        // which would make an ordering assertion pass vacuously.
        let mut list = vec![
            ScoreEntry { score: 100, at: 2000 },
            ScoreEntry { score: 200, at: 3000 },
            ScoreEntry { score: 100, at: 1000 },
        ];
        sort_and_truncate(&mut list);
        assert_eq!(
            list,
            [
                ScoreEntry { score: 200, at: 3000 },
                ScoreEntry { score: 100, at: 1000 },
                ScoreEntry { score: 100, at: 2000 },
            ],
            "score-descending, equal scores oldest-first"
        );
    }

    #[test]
    fn test_persistence_round_trip() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("scores.json");
        {
            let mut scores = Scores::with_save_path(&path);
            scores.submit("solo", 42);
        }
        let restored = Scores::with_save_path(&path);
        assert_eq!(restored.best("solo"), Some(42));
        Ok(())
    }

    #[test]
    fn test_corrupt_file_warns_and_starts_fresh() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("scores.json");
        std::fs::write(&path, "not json")?;
        let mut scores = Scores::with_save_path(&path);
        assert_eq!(scores.top("solo").len(), 0);
        assert!(scores.submit("solo", 7), "fresh state accepts scores");
        Ok(())
    }

    #[test]
    fn test_concurrent_stores_merge_instead_of_clobbering() -> Result<(), std::io::Error> {
        // Two stores on the same slot = the browser's two-tabs scenario.
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("scores.json");
        let mut tab_a = Scores::with_save_path(&path);
        let mut tab_b = Scores::with_save_path(&path);
        tab_a.submit("solo", 100);
        tab_b.submit("solo", 200); // writes last, must merge 100 back in

        let restored = Scores::with_save_path(&path);
        let top: Vec<u64> = restored.top("solo").iter().map(|e| e.score).collect();
        assert_eq!(top, [200, 100], "tab A's score must survive tab B's save");

        // A rejected submit never touches the file ...
        let mut full = Scores::with_save_path(&path);
        for s in 1..=MAX_SCORES_PER_MODE as u64 {
            full.submit("solo", 1000 + s * 10);
        }
        let before = std::fs::read_to_string(&path)?;
        assert!(!full.submit("solo", 1));
        assert_eq!(std::fs::read_to_string(&path)?, before);

        // ... and an explicit reset still clears despite merge-on-save.
        full.reset();
        let restored = Scores::with_save_path(&path);
        assert_eq!(restored.best("solo"), None, "an explicit clear must actually clear");
        Ok(())
    }
}
