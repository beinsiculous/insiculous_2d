//! A typed JSON document in one `save_store` slot: loads with a warn-and-start-fresh
//! fallback, and merges with the slot on save so concurrent sessions or browser
//! tabs keep each other's entries (`docs/WEB_SAVES.md`).

use std::path::{Path, PathBuf};
use serde::{de::DeserializeOwned, Serialize};

/// Contract for documents that merge on disk re-read (e.g. concurrent tabs).
pub trait MergeOnLoad {
    fn merge_from_disk(&mut self, disk: Self);
}

/// Errors from JSON save-slot persistence.
#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Seconds since UNIX epoch via `common::clock` (safe on wasm).
pub fn unix_seconds() -> u64 {
    common::clock::SystemTime::now()
        .duration_since(common::clock::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A typed JSON document persisted to a save slot.
pub struct JsonSaveSlot<T> {
    path: Option<PathBuf>,
    data: T,
}

impl<T: Clone + Default + Serialize + DeserializeOwned + MergeOnLoad> JsonSaveSlot<T> {
    /// Create an in-memory slot with default data and no persistence target.
    pub fn in_memory() -> Self {
        Self {
            path: None,
            data: T::default(),
        }
    }

    /// Create a slot pointing at `path`. If data exists at the slot, it is loaded;
    /// otherwise it starts with default data.
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut slot = Self {
            path: Some(path.clone()),
            data: T::default(),
        };
        match super::read(&path) {
            Ok(Some(contents)) => match serde_json::from_str::<T>(&contents) {
                Ok(data) => slot.data = data,
                Err(e) => {
                    log::warn!("Could not parse save file {}: {} — starting fresh", path.display(), e);
                }
            },
            Ok(None) => {}
            Err(e) => {
                log::warn!("Could not read save file {}: {} — starting fresh", path.display(), e);
            }
        }
        slot
    }

    /// The target path, if persistence is configured.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Borrow the held data.
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Mutably borrow the held data.
    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }

    /// Persist the held data, merging with any existing data at the slot first
    /// (`MergeOnLoad::merge_from_disk`), so one tab's save preserves what another
    /// tab persisted earlier. The read-merge-write is not atomic: same-instant
    /// saves from two tabs can still race, and the loser's entries return on its
    /// next save. An unreadable or unparsable slot skips the merge — the write
    /// then replaces the corrupt state. `Ok(false)` = no path configured.
    pub fn save_with_merge(&self) -> Result<bool, SaveError> {
        let Some(path) = &self.path else { return Ok(false); };
        let mut outgoing = self.data.clone();
        if let Ok(Some(existing)) = super::read(path) {
            if let Ok(disk) = serde_json::from_str::<T>(&existing) {
                outgoing.merge_from_disk(disk);
            }
        }
        let json = serde_json::to_string_pretty(&outgoing)?;
        super::write(path, &json)?;
        Ok(true)
    }

    /// Persist the held data without merging: for `reset()`, where an explicit
    /// clear must actually clear instead of resurrecting the slot's entries.
    /// `Ok(false)` = no path configured.
    pub fn save_without_merge(&self) -> Result<bool, SaveError> {
        let Some(path) = &self.path else { return Ok(false); };
        let json = serde_json::to_string_pretty(&self.data)?;
        super::write(path, &json)?;
        Ok(true)
    }

    /// Reload data from disk, replacing in-memory state.
    pub fn reload(&mut self) -> Result<(), SaveError> {
        let Some(path) = &self.path else {
            return Err(SaveError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no save path configured",
            )));
        };
        let data = super::read(path)?.ok_or_else(|| {
            SaveError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no save data at {}", path.display()),
            ))
        })?;
        self.data = serde_json::from_str(&data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct NumberSetDoc {
        numbers: HashSet<u32>,
    }

    impl MergeOnLoad for NumberSetDoc {
        fn merge_from_disk(&mut self, disk: Self) {
            self.numbers.extend(disk.numbers);
        }
    }

    #[test]
    fn saving_with_merge_unions_the_slot_and_a_corrupt_slot_is_replaced() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let slot_path = dir.path().join("numbers.json");

        let mut slot_a = JsonSaveSlot::<NumberSetDoc>::with_path(&slot_path);
        slot_a.data_mut().numbers.insert(10);
        slot_a.data_mut().numbers.insert(20);
        assert!(slot_a.save_with_merge()?);

        // Second slot reading the saved file
        let mut slot_b = JsonSaveSlot::<NumberSetDoc>::with_path(&slot_path);
        assert_eq!(slot_b.data().numbers.len(), 2);
        slot_b.data_mut().numbers.insert(30);

        // slot_a adds another number and saves
        slot_a.data_mut().numbers.insert(40);
        assert!(slot_a.save_with_merge()?);

        // slot_b saves with merge — should incorporate 40 from disk
        assert!(slot_b.save_with_merge()?);

        slot_a.reload()?;
        assert!(slot_a.data().numbers.contains(&10));
        assert!(slot_a.data().numbers.contains(&20));
        assert!(slot_a.data().numbers.contains(&30));
        assert!(slot_a.data().numbers.contains(&40));

        // A corrupt slot cannot be merged with: the save replaces it instead
        // of failing, and the next reader sees only what was saved.
        std::fs::write(&slot_path, "{not json")?;
        assert!(slot_b.save_with_merge()?);
        let replaced = JsonSaveSlot::<NumberSetDoc>::with_path(&slot_path);
        assert_eq!(replaced.data().numbers, slot_b.data().numbers);

        // No merge: reset semantics, the slot holds exactly the document.
        slot_a.data_mut().numbers.clear();
        assert!(slot_a.save_without_merge()?);
        let cleared = JsonSaveSlot::<NumberSetDoc>::with_path(&slot_path);
        assert!(cleared.data().numbers.is_empty(), "an explicit clear must actually clear");
        Ok(())
    }
}
