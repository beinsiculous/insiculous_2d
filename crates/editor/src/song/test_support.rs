//! Fixtures for the song tests: an asset folder on disk, a short melody as MIDI bytes, the
//! Napolitan fixture as a saved song, a sample instrument, a store that stops after a set
//! number of writes, renames and removals, one whose renames fail, and one that rewrites a
//! file behind the save's back.

use std::io;
use std::path::{Path, PathBuf};

use music_midi::midly::num::{u15, u24, u28};
use music_midi::midly::{Format, Header, MetaMessage, Smf, Timing, TrackEvent, TrackEventKind};
use music_midi::{
    Instrument, InstrumentSource, NewNote, NewTrack, Provenance, SamplePlayback, SongData,
};

use super::io::{SongStore, VfsStore};
use super::{default_song_file, Gesture, SongDocument, SongEdit};

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

/// A type-1 song as MIDI bytes, at 480 ticks a quarter: the timing track at 120 BPM in
/// 4/4, then a track named `melody` on channel 1 holding three eighths — C4 at tick 0, E4 at
/// 480 and G4 at 960.
pub(super) fn melody_bytes() -> Vec<u8> {
    let event = |kind| TrackEvent {
        delta: u28::new(0),
        kind,
    };
    let timing_track = vec![
        event(TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000)))),
        event(TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8))),
        event(TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let mut timing = Vec::new();
    Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(480))),
        tracks: vec![timing_track],
    }
    .write_std(&mut timing)
    .expect("the timing track writes");
    let (mut data, _) = SongData::from_smf(&timing).expect("the timing track reads");
    let melody = NewTrack {
        name: "melody".to_string(),
        channel: Some(0),
        program: 0,
    };
    let track = data.add_track(melody).expect("the melody track").track;
    for (tick, key) in [(0, 60), (480, 64), (960, 67)] {
        let note = NewNote {
            tick,
            key,
            velocity: 100,
            duration_ticks: 240,
        };
        data.add_note(track, note).expect("the note lands");
    }
    data.write_smf().expect("the melody writes")
}

/// [`melody_bytes`] as a song never saved, with the author and license Save requires — set
/// as an opened song carries them, not as an edit, so the history is empty.
pub(super) fn melody() -> SongDocument {
    let mut document =
        SongDocument::from_smf(&melody_bytes(), default_song_file()).expect("the melody reads");
    document.file.provenance = Some(song_provenance());
    document
}

/// The song's own author and license in every fixture.
pub(super) fn song_provenance() -> Provenance {
    Provenance {
        author: "2026 Jesse".to_string(),
        license: "CC-BY-4.0".to_string(),
    }
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

/// A store over `common::vfs` that refuses every write, rename and removal after the first
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

    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        self.spend()?;
        VfsStore.rename(from, to)
    }

    fn remove(&mut self, path: &Path) -> io::Result<()> {
        self.spend()?;
        VfsStore.remove(path)
    }

    fn list_files(&self, directory: &Path) -> io::Result<Vec<PathBuf>> {
        VfsStore.list_files(directory)
    }
}

/// A store over `common::vfs` whose every rename fails, as a full disk or a lost mount
/// would stop a save between writing a final's temporary file and moving it into place.
pub(super) struct RenameFailingStore;

impl SongStore for RenameFailingStore {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        VfsStore.read(path)
    }

    fn write(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        VfsStore.write(path, bytes)
    }

    fn rename(&mut self, _from: &Path, _to: &Path) -> io::Result<()> {
        Err(io::Error::other("the rename failed"))
    }

    fn remove(&mut self, path: &Path) -> io::Result<()> {
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
/// right after the save's own write, rename or removal number `after_operation` (counted
/// from 1).
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

    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        VfsStore.rename(from, to)?;
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
