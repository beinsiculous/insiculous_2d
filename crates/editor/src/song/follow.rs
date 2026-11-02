//! The document following its two files: a score another program rewrote is taken whole
//! with the map and its unsaved edits kept, and a sidecar rewritten on disk is read anew
//! with the score and its unsaved edits kept. Each drops the history entries that changed
//! the file it replaced, since an undo of them would replay against a file that is gone.

use music_midi::{parse_song_file, SongData, SongError, SongFileError};

use super::{midi_fingerprint, EditingPolicy, SongDocument, SongEdit, SongSelection};
use crate::external_tools::{diff_tracks, midi_sidecar_path, TrackListing};

impl SongDocument {
    /// Take `bytes` as the song's score: the map and its unsaved edits kept, the
    /// [`EditingPolicy`] recomputed from the new file's report, the history's score
    /// entries dropped and the selection cleared, the loaded bytes and their fingerprint
    /// replaced and the score no longer the engine's. Returns the track diff's lines,
    /// the parts' tracks before against after. A file that will not parse changes nothing.
    pub fn adopt_score(&mut self, bytes: Vec<u8>) -> Result<Vec<String>, SongError> {
        let (data, report) = SongData::from_smf(&bytes)?;
        let diff = diff_tracks(
            &TrackListing::of(&self.file, &self.data),
            &TrackListing::of(&self.file, &data),
        );
        self.data = data;
        let baseline_export = self.data.write_smf().ok();
        self.editing = EditingPolicy::for_score(&report, baseline_export.is_some());
        self.history.drop_entries(SongEdit::changes_score);
        self.selection = SongSelection::new();
        self.take_loaded_score(bytes, baseline_export);
        Ok(diff.texts())
    }

    /// Take `text` as the song's sidecar, read from disk: the score and its unsaved edits
    /// kept, the history's map entries dropped. A sidecar that will not parse changes
    /// nothing. Its `midi_fingerprint` is dropped, as an open drops it: it describes a file
    /// this document has not written.
    pub fn reload_map(&mut self, text: &str) -> Result<(), SongFileError> {
        let label = self.path.as_deref().map_or_else(
            || "the song's sidecar".to_string(),
            |path| midi_sidecar_path(path).display().to_string(),
        );
        let mut file = parse_song_file(&label, text)?;
        file.midi_fingerprint = None;
        self.file = file;
        self.history.drop_entries(SongEdit::changes_map);
        self.map_fingerprint = Some(midi_fingerprint(text.as_bytes()));
        Ok(())
    }

    /// `bytes` become the `.mid` the document follows, as read or written, and the score
    /// is no longer the engine's: the data's export now is the baseline an edit is told from
    /// — `export` when the caller holds it (a save that wrote `write_smf`'s bytes), else made.
    pub(super) fn take_loaded_score(&mut self, bytes: Vec<u8>, export: Option<Vec<u8>>) {
        self.score_fingerprint = midi_fingerprint(&bytes);
        self.source_bytes = bytes;
        self.baseline_export = export.or_else(|| self.data.write_smf().ok());
    }

    /// The `.mid` bytes a save writes: `write_smf`'s once the engine edited the score, else
    /// the bytes it loaded, which a re-encoding could change.
    pub(super) fn score_bytes(&self) -> Result<Vec<u8>, SongError> {
        if self.is_score_authored() {
            self.data.write_smf()
        } else {
            Ok(self.source_bytes.clone())
        }
    }
}
