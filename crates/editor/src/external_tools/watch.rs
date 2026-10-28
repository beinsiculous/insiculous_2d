//! `FileWatch`: which watched files changed on disk, as pure state over the stamps each
//! poll reads. The host owns the thread that polls it; nothing here sleeps or spawns.
//!
//! A change is reported once its reading has held for one further poll, since a program
//! may write a file in steps and a half-written one must not be read. A path that goes
//! missing stays watched as long as the host registers it, so its return, however late, is
//! a change — and an unlink-then-create save that lands inside the window is one change,
//! never a deletion. A `.mid` and its `.mid.ron` settle together: neither is reported
//! while the other is still moving.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use common::vfs::ChangeStamp;

use super::file_type::{is_midi_path, midi_sidecar_path};

/// What a poll read for one path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Observed {
    Present(ChangeStamp),
    Absent,
}

/// A change [`FileWatch::poll`] reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChange {
    /// The file's stamp changed, or a missing file came back, and it then held a poll.
    Changed(PathBuf),
    /// The file was gone at two polls in a row. It stays watched.
    Deleted(PathBuf),
}

impl FileChange {
    /// The path the change is about, exactly as it was registered.
    pub fn path(&self) -> &Path {
        match self {
            FileChange::Changed(path) | FileChange::Deleted(path) => path,
        }
    }
}

/// What a thread polling a [`FileWatch`] sends its host, in the order it happened. A host
/// that takes both kinds from one channel knows that a change arriving before a path's
/// [`Noted`](WatchEvent::Noted) was read before the host's own write of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// A change [`FileWatch::poll`] reported.
    Change(FileChange),
    /// The thread took the host's note of its own write of this path.
    Noted(PathBuf),
}

/// One registered path's readings.
#[derive(Debug, Clone, Copy, Default)]
struct Observation {
    /// What the path was last known to be: its first reading, its last report, or the
    /// editor's own write. `None` until the first poll reads it.
    settled: Option<Observed>,
    /// A reading that differs from `settled`, and how many polls in a row have read it.
    pending: Option<(Observed, u32)>,
}

impl Observation {
    fn is_moving(&self) -> bool {
        self.pending.is_some_and(|(_, polls)| polls < SETTLED_AFTER_POLLS)
    }
}

/// A reading must be seen this many polls in a row before it is reported.
const SETTLED_AFTER_POLLS: u32 = 2;

/// The registered set of paths and, apart from it, what each poll observed of them.
#[derive(Debug, Clone, Default)]
pub struct FileWatch {
    registered: BTreeSet<PathBuf>,
    observed: BTreeMap<PathBuf, Observation>,
}

impl FileWatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Watch exactly `paths` from now on. A path kept keeps what was observed of it; a new
    /// one is read as it stands at the next poll, which reports nothing for it.
    pub fn set_registered(&mut self, paths: BTreeSet<PathBuf>) {
        self.observed.retain(|path, _| paths.contains(path));
        self.registered = paths;
    }

    /// The editor wrote `path` itself and read `stamp` right after: that is the path's
    /// state now, and a change caught before this note is dropped. A write by another
    /// program after it still differs from `stamp`, so it is reported.
    pub fn note_written(&mut self, path: &Path, stamp: ChangeStamp) {
        if let Some(observation) = self.observed.get_mut(path) {
            observation.settled = Some(Observed::Present(stamp));
            observation.pending = None;
        }
    }

    /// Read every registered path from disk and report what has settled.
    pub fn poll(&mut self) -> Vec<FileChange> {
        self.poll_with(read_from_disk)
    }

    /// [`poll`](Self::poll) over readings from `read`; `None` is a path whose filesystem
    /// keeps no stamp, which is never reported.
    pub fn poll_with(&mut self, mut read: impl FnMut(&Path) -> Option<Observed>) -> Vec<FileChange> {
        let mut settled_now = Vec::new();
        for path in &self.registered {
            let Some(reading) = read(path) else { continue };
            let observation = self.observed.entry(path.clone()).or_default();
            match observation.settled {
                None => observation.settled = Some(reading),
                Some(settled) if settled == reading => observation.pending = None,
                Some(_) => {
                    let polls = match observation.pending {
                        Some((seen, polls)) if seen == reading => polls + 1,
                        _ => 1,
                    };
                    observation.pending = Some((reading, polls));
                    if polls >= SETTLED_AFTER_POLLS {
                        settled_now.push(path.clone());
                    }
                }
            }
        }

        let mut changes = Vec::new();
        for path in settled_now {
            if self.companion_is_moving(&path) {
                continue;
            }
            let Some(observation) = self.observed.get_mut(&path) else { continue };
            let Some((reading, _)) = observation.pending.take() else { continue };
            observation.settled = Some(reading);
            changes.push(match reading {
                Observed::Present(_) => FileChange::Changed(path),
                Observed::Absent => FileChange::Deleted(path),
            });
        }
        changes
    }

    fn companion_is_moving(&self, path: &Path) -> bool {
        companion(path)
            .and_then(|companion| self.observed.get(&companion))
            .is_some_and(Observation::is_moving)
    }
}

/// The file that settles with `path`: a MIDI file's sidecar, or a sidecar's MIDI file.
fn companion(path: &Path) -> Option<PathBuf> {
    if is_midi_path(path) {
        return Some(midi_sidecar_path(path));
    }
    let is_ron = path.extension().is_some_and(|extension| extension == "ron");
    let described = path.with_extension("");
    (is_ron && is_midi_path(&described)).then_some(described)
}

/// A path as it stands: missing is `Absent`; a file whose stamp cannot be read for another
/// reason, or that keeps none, is not read at all, so it is never reported deleted.
fn read_from_disk(path: &Path) -> Option<Observed> {
    match common::vfs::change_stamp(path) {
        Ok(stamp) => stamp.map(Observed::Present),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(Observed::Absent),
        Err(_) => None,
    }
}

#[cfg(test)]
#[path = "watch_tests.rs"]
mod watch_tests;
