//! The document following its two files: a score MIDI Evil rewrote is taken whole with the
//! map and its unsaved edits kept, and a sidecar rewritten on disk is read anew, its history
//! dropped, since an undo of a map edit would replay a sidecar that is gone.

use music_midi::{parse_song_file, SongData, SongError, SongFileError};

use super::{midi_fingerprint, SongDocument};
use crate::external_tools::{diff_tracks, midi_sidecar_path, TrackListing};

impl SongDocument {
    /// Take `bytes` as the song's score: the map, its unsaved edits and its history kept, the
    /// loaded bytes and their fingerprint replaced. Returns the track diff's lines, the parts'
    /// tracks before against after. A file that will not parse changes nothing.
    pub fn adopt_score(&mut self, bytes: Vec<u8>) -> Result<Vec<String>, SongError> {
        let (data, _) = SongData::from_smf(&bytes)?;
        let diff = diff_tracks(
            &TrackListing::of(&self.file, &self.data),
            &TrackListing::of(&self.file, &data),
        );
        self.data = data;
        self.score_fingerprint = midi_fingerprint(&bytes);
        self.source_bytes = bytes;
        Ok(diff.texts())
    }

    /// Take `text` as the song's sidecar, read from disk: the score kept, the history
    /// dropped. A sidecar that will not parse changes nothing. Its `midi_fingerprint` is
    /// dropped, as an open drops it: it describes a file this document has not written.
    pub fn reload_map(&mut self, text: &str) -> Result<(), SongFileError> {
        let label = self.path.as_deref().map_or_else(
            || "the song's sidecar".to_string(),
            |path| midi_sidecar_path(path).display().to_string(),
        );
        let mut file = parse_song_file(&label, text)?;
        file.midi_fingerprint = None;
        self.file = file;
        self.history.drop_all();
        self.map_fingerprint = Some(midi_fingerprint(text.as_bytes()));
        Ok(())
    }
}
