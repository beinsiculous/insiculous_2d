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
    /// A renamed game's previous slot: read at load and by `reload`, never written.
    former_path: Option<PathBuf>,
    data: T,
}

impl<T: Clone + Default + Serialize + DeserializeOwned + MergeOnLoad> JsonSaveSlot<T> {
    /// Create an in-memory slot with default data and no persistence target.
    pub fn in_memory() -> Self {
        Self {
            path: None,
            former_path: None,
            data: T::default(),
        }
    }

    /// Create a slot pointing at `path`. If data exists at the slot, it is loaded;
    /// otherwise it starts with default data.
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let data = Self::read_document(&path).unwrap_or_default();
        Self {
            path: Some(path),
            former_path: None,
            data,
        }
    }

    /// Create a slot pointing at `path` that also folds in the document at
    /// `former_path` — the slot a renamed game saved to before, whose players
    /// would otherwise lose their progress. The former document is unioned into
    /// the primary's with [`MergeOnLoad::merge_from_disk`]; one that is missing or
    /// does not parse adds nothing. Saves go to `path` only: the former slot is
    /// never written, so an older build still reading it keeps working.
    pub fn with_path_and_former(path: impl Into<PathBuf>, former_path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let former_path = former_path.into();
        let data = Self::fold_former(Self::read_document(&path), Self::read_document(&former_path))
            .unwrap_or_default();
        Self {
            path: Some(path),
            former_path: Some(former_path),
            data,
        }
    }

    /// The documents of a primary and a former slot as one: the primary with the
    /// former folded in by [`MergeOnLoad::merge_from_disk`], the former taken
    /// whole when the primary is empty (a merge into a default document would
    /// keep only what the merge copies), `None` when both are empty.
    fn fold_former(primary: Option<T>, former: Option<T>) -> Option<T> {
        match (primary, former) {
            (Some(mut primary), Some(former)) => {
                primary.merge_from_disk(former);
                Some(primary)
            }
            (Some(primary), None) => Some(primary),
            (None, former) => former,
        }
    }

    /// The parsed document at `path`, or `None` — logged, never fatal — when the
    /// slot is empty, unreadable or unparsable.
    fn read_document(path: &Path) -> Option<T> {
        match super::read(path) {
            Ok(Some(contents)) => match serde_json::from_str::<T>(&contents) {
                Ok(data) => Some(data),
                Err(e) => {
                    log::warn!("Could not parse save file {}: {} — treated as empty", path.display(), e);
                    None
                }
            },
            Ok(None) => None,
            Err(e) => {
                log::warn!("Could not read save file {}: {} — treated as empty", path.display(), e);
                None
            }
        }
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

    /// Reload data from disk, replacing in-memory state with what a boot loads:
    /// the primary slot's document with the former slot's folded in when one is
    /// configured — a reload that read the primary alone would drop the former's
    /// records from memory, and the next save would drop them from disk. The
    /// former slot is read as a boot reads it: one that is missing, unreadable or
    /// unparsable adds nothing, logged, and is read again at the next boot or
    /// reload. The primary is read as `reload` always read it: a read or parse
    /// error is the caller's error and memory is left as it was. `NotFound` when
    /// neither slot holds a document, a former that could not be read or parsed
    /// counting as empty here as at boot.
    pub fn reload(&mut self) -> Result<(), SaveError> {
        let Some(path) = &self.path else {
            return Err(SaveError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no save path configured",
            )));
        };
        let primary = match super::read(path)? {
            Some(contents) => Some(serde_json::from_str::<T>(&contents).map_err(|e| {
                log::warn!("Could not parse save file {}: {}", path.display(), e);
                e
            })?),
            None => None,
        };
        let former = self.former_path.as_deref().and_then(Self::read_document);
        let Some(data) = Self::fold_former(primary, former) else {
            return Err(SaveError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "no save data at {} (former slot: {})",
                    path.display(),
                    self.former_path.as_deref().map_or("none".to_string(), |former| former.display().to_string())
                ),
            )));
        };
        self.data = data;
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

    fn numbers(values: &[u32]) -> NumberSetDoc {
        NumberSetDoc { numbers: values.iter().copied().collect() }
    }

    fn write_numbers(path: &Path, values: &[u32]) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(path, serde_json::to_string(&numbers(values))?)?;
        Ok(())
    }

    fn read_numbers(path: &Path) -> Result<NumberSetDoc, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    #[test]
    fn a_former_slot_is_unioned_in_at_load_and_saves_reach_the_primary_only() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let primary_path = dir.path().join("new.json");
        let former_path = dir.path().join("old.json");
        write_numbers(&primary_path, &[1, 2])?;
        write_numbers(&former_path, &[2, 3])?;
        let former_bytes = std::fs::read(&former_path)?;

        let mut slot = JsonSaveSlot::<NumberSetDoc>::with_path_and_former(&primary_path, &former_path);
        assert_eq!(slot.data(), &numbers(&[1, 2, 3]));
        slot.data_mut().numbers.insert(4);
        assert!(slot.save_with_merge()?);

        assert_eq!(read_numbers(&primary_path)?, numbers(&[1, 2, 3, 4]));
        assert_eq!(std::fs::read(&former_path)?, former_bytes, "the former slot is never written");
        Ok(())
    }

    #[test]
    fn an_absent_former_slot_adds_nothing_and_is_not_created() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let primary_path = dir.path().join("new.json");
        let former_path = dir.path().join("old.json");
        write_numbers(&primary_path, &[1])?;

        let slot = JsonSaveSlot::<NumberSetDoc>::with_path_and_former(&primary_path, &former_path);
        assert_eq!(slot.data(), &numbers(&[1]));
        assert!(slot.save_with_merge()?);
        assert!(!former_path.exists(), "saving must not create the former slot");
        Ok(())
    }

    #[test]
    fn an_absent_primary_loads_the_former_and_the_first_save_writes_the_primary() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let primary_path = dir.path().join("new.json");
        let former_path = dir.path().join("old.json");
        write_numbers(&former_path, &[5])?;

        let slot = JsonSaveSlot::<NumberSetDoc>::with_path_and_former(&primary_path, &former_path);
        assert_eq!(slot.data(), &numbers(&[5]));
        assert!(!primary_path.exists(), "loading never writes");
        assert!(slot.save_with_merge()?);
        assert_eq!(read_numbers(&primary_path)?, numbers(&[5]));
        Ok(())
    }

    #[test]
    fn a_corrupt_former_slot_adds_nothing_and_keeps_its_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let primary_path = dir.path().join("new.json");
        let former_path = dir.path().join("old.json");
        write_numbers(&primary_path, &[1])?;
        std::fs::write(&former_path, "{not json")?;

        let slot = JsonSaveSlot::<NumberSetDoc>::with_path_and_former(&primary_path, &former_path);
        assert_eq!(slot.data(), &numbers(&[1]));
        assert!(slot.save_with_merge()?);
        assert_eq!(std::fs::read_to_string(&former_path)?, "{not json");
        Ok(())
    }

    #[test]
    fn a_corrupt_primary_beside_a_valid_former_loads_the_former_and_the_first_save_repairs_it() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let primary_path = dir.path().join("new.json");
        let former_path = dir.path().join("old.json");
        std::fs::write(&primary_path, "{not json")?;
        write_numbers(&former_path, &[7])?;

        let slot = JsonSaveSlot::<NumberSetDoc>::with_path_and_former(&primary_path, &former_path);
        assert_eq!(slot.data(), &numbers(&[7]));
        assert!(slot.save_with_merge()?);
        assert_eq!(read_numbers(&primary_path)?, numbers(&[7]));
        Ok(())
    }

    #[test]
    fn reload_restores_the_union_of_both_slots() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let primary_path = dir.path().join("new.json");
        let former_path = dir.path().join("old.json");
        write_numbers(&primary_path, &[1])?;
        write_numbers(&former_path, &[2, 3])?;

        let mut slot = JsonSaveSlot::<NumberSetDoc>::with_path_and_former(&primary_path, &former_path);
        slot.data_mut().numbers.clear();
        slot.reload()?;
        assert_eq!(slot.data(), &numbers(&[1, 2, 3]), "a reload holds what a boot would");
        Ok(())
    }

    #[test]
    fn reload_reads_a_former_slot_it_cannot_parse_as_a_boot_does() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let primary_path = dir.path().join("new.json");
        let former_path = dir.path().join("old.json");
        write_numbers(&primary_path, &[1])?;
        write_numbers(&former_path, &[2])?;

        let mut slot = JsonSaveSlot::<NumberSetDoc>::with_path_and_former(&primary_path, &former_path);
        std::fs::write(&former_path, "{not json")?;
        slot.reload()?;
        assert_eq!(slot.data(), &numbers(&[1]), "a former that cannot be parsed adds nothing, as at boot");

        std::fs::write(&primary_path, "{not json")?;
        assert!(slot.reload().is_err(), "a primary that cannot be parsed is the caller's error");
        assert_eq!(slot.data(), &numbers(&[1]), "a failed reload leaves memory as it was");
        Ok(())
    }

    #[test]
    fn a_reset_then_a_reload_folds_the_former_slot_back_in() -> Result<(), Box<dyn std::error::Error>> {
        // The recorded hole: the engine never writes a former slot, so a reset
        // clears the primary only, and the former's records return at the next
        // reload or boot. Pinned so a change to it is a decision, not a drift.
        let dir = tempfile::tempdir()?;
        let primary_path = dir.path().join("new.json");
        let former_path = dir.path().join("old.json");
        write_numbers(&primary_path, &[1])?;
        write_numbers(&former_path, &[2])?;

        let mut slot = JsonSaveSlot::<NumberSetDoc>::with_path_and_former(&primary_path, &former_path);
        slot.data_mut().numbers.clear();
        assert!(slot.save_without_merge()?);
        assert_eq!(read_numbers(&primary_path)?, numbers(&[]), "the reset cleared the primary");

        slot.reload()?;
        assert_eq!(slot.data(), &numbers(&[2]), "the former slot's records are back");
        Ok(())
    }

    #[test]
    fn reload_without_a_primary_reads_the_former_slot() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let primary_path = dir.path().join("new.json");
        let former_path = dir.path().join("old.json");
        write_numbers(&former_path, &[8])?;

        let mut slot = JsonSaveSlot::<NumberSetDoc>::with_path_and_former(&primary_path, &former_path);
        slot.data_mut().numbers.clear();
        slot.reload()?;
        assert_eq!(slot.data(), &numbers(&[8]));

        std::fs::remove_file(&former_path)?;
        assert!(slot.reload().is_err(), "with neither slot present, reload reports NotFound");
        Ok(())
    }
}
