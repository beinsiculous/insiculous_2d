//! Fixtures for the song tests: an asset folder on disk, a short melody, the Napolitan
//! fixture as a saved song, a sample instrument, a store that stops after a set number of
//! writes and removals, and one that rewrites a file behind the save's back.

use std::io;
use std::path::{Path, PathBuf};

use music_midi::{Instrument, InstrumentSource, Provenance, SamplePlayback};

use super::io::{SongStore, VfsStore};
use super::{Gesture, SongDocument, SongEdit};

/// The Napolitan fixture: its export is not its bytes (midly writes running status), so a
/// fingerprint of `write_smf` is not the file's.
pub(super) const NAPOLITAN: &[u8] = include_bytes!("../../../music_midi/fixtures/dansenapolitaine.mid");

/// A temporary asset folder, removed when dropped.
pub(super) struct AssetFolder {
    directory: tempfile::TempDir,
}

impl AssetFolder {
    pub fn new() -> Self {
        Self {
            directory: tempfile::tempdir().expect("a temporary asset folder"),
        }
    }

    pub fn base(&self) -> &Path {
        self.directory.path()
    }

    /// `relative` under the folder.
    pub fn path(&self, relative: &str) -> PathBuf {
        self.base().join(relative)
    }

    pub fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = self.path(relative);
        common::vfs::write(&path, bytes).expect("fixture written");
        path
    }

    pub fn read(&self, relative: &str) -> Option<Vec<u8>> {
        std::fs::read(self.path(relative)).ok()
    }

    /// Every file under `relative`, relative to it, sorted.
    pub fn files_under(&self, relative: &str) -> Vec<String> {
        let directory = self.path(relative);
        common::vfs::list_files(&directory)
            .unwrap_or_default()
            .into_iter()
            .map(|path| {
                path.strip_prefix(&directory)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }
}

/// A new song holding three notes on its melody track, at ticks 0, 480 and 960, with the
/// author and license Save requires — set as an opened song carries them, not as an edit,
/// so the history holds the three notes alone.
pub(super) fn melody() -> SongDocument {
    let mut document = SongDocument::new_song().expect("a blank song");
    document.file.provenance = Some(song_provenance());
    for (tick, key) in [(0, 60), (480, 64), (960, 67)] {
        document
            .apply(add_note(tick, key), Gesture::Discrete)
            .expect("the note lands");
    }
    document
}

/// The song's own author and license in every fixture.
pub(super) fn song_provenance() -> Provenance {
    Provenance {
        author: "2026 Jesse".to_string(),
        license: "CC-BY-4.0".to_string(),
    }
}

/// An eighth note on the melody track (track 1 of a new song).
pub(super) fn add_note(tick: u64, key: u8) -> SongEdit {
    SongEdit::AddNote {
        track: 1,
        tick,
        key,
        velocity: 100,
        duration_ticks: 240,
    }
}

/// The note numbers of every note in the song, in track, tick and chord order.
pub(super) fn note_numbers(document: &SongDocument) -> Vec<u64> {
    document
        .data()
        .tracks()
        .iter()
        .flat_map(|track| track.units())
        .flat_map(|unit| &unit.notes)
        .map(|note| note.id.number())
        .collect()
}

/// A one-shot sample instrument playing `path`, with provenance.
pub(super) fn sample_instrument(name: &str, path: &str) -> Instrument {
    Instrument {
        name: name.to_string(),
        source: InstrumentSource::Sample {
            path: path.to_string(),
            root_note: 60,
            gain: 1.0,
            playback: SamplePlayback::OneShot,
        },
        provenance: Some(Provenance {
            author: "2026 Jesse".to_string(),
            license: "MIT".to_string(),
        }),
    }
}

/// The song's MIDI bytes as a save would write them.
pub(super) fn exported(document: &SongDocument) -> Vec<u8> {
    document.data().write_smf().expect("the song exports")
}

/// A store over `common::vfs` that refuses every write and removal after the first
/// `operations_left`, as a crash would stop a save there.
pub(super) struct InterruptedStore {
    pub operations_left: usize,
}

impl InterruptedStore {
    fn spend(&mut self) -> io::Result<()> {
        if self.operations_left == 0 {
            return Err(io::Error::other("interrupted"));
        }
        self.operations_left -= 1;
        Ok(())
    }
}

impl SongStore for InterruptedStore {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        VfsStore.read(path)
    }

    fn write(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.spend()?;
        VfsStore.write(path, bytes)
    }

    fn remove(&mut self, path: &Path) -> io::Result<()> {
        self.spend()?;
        VfsStore.remove(path)
    }

    fn list_files(&self, directory: &Path) -> io::Result<Vec<PathBuf>> {
        VfsStore.list_files(directory)
    }
}

/// The Napolitan fixture saved at `relative` as a person would map it: the `.mid` dropped in,
/// opened on a scaffold sidecar, its provenance set and saved — a map save, so the `.mid`
/// on disk stays the fixture's own bytes.
pub(super) fn napolitan_song(folder: &AssetFolder, relative: &str) -> SongDocument {
    let path = folder.write(relative, NAPOLITAN);
    let mut document = SongDocument::open(&path, folder.base())
        .expect("an unmapped .mid opens in place")
        .document;
    document
        .apply(
            SongEdit::SetProvenance {
                provenance: song_provenance(),
            },
            Gesture::Discrete,
        )
        .expect("the provenance lands");
    document.save(&path, folder.base()).expect("the map saves");
    document
}

/// A store over `common::vfs` that writes `bytes` at `path`, as another program would,
/// right after the save's own write or removal number `after_operation` (counted from 1).
pub(super) struct RewritingStore {
    pub after_operation: usize,
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub landed: usize,
}

impl RewritingStore {
    fn count(&mut self) {
        self.landed += 1;
        if self.landed == self.after_operation {
            common::vfs::write(&self.path, &self.bytes).expect("the other program's write");
        }
    }
}

impl SongStore for RewritingStore {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        VfsStore.read(path)
    }

    fn write(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        VfsStore.write(path, bytes)?;
        self.count();
        Ok(())
    }

    fn remove(&mut self, path: &Path) -> io::Result<()> {
        VfsStore.remove(path)?;
        self.count();
        Ok(())
    }

    fn list_files(&self, directory: &Path) -> io::Result<Vec<PathBuf>> {
        VfsStore.list_files(directory)
    }
}
