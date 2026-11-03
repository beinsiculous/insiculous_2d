//! An imported archive's songs: what a song's file must be for the archive to be taken.
//!
//! A `.mid.ron` must parse as a sidecar and sit beside its `.mid`, and a `.mid` must open
//! with `MThd`; either failing refuses the whole archive, as a bad scene does. Any other
//! name is any other file — a `.new` an older project's save left beside its song included.

use std::collections::HashSet;

use super::ArchiveError;

const MIDI_HEADER: &[u8] = b"MThd";

/// Check every song file among `entries`; the first that fails refuses the archive.
pub(super) fn check_song_entries(entries: &[(String, Vec<u8>)]) -> Result<(), ArchiveError> {
    let names: HashSet<String> = entries.iter().map(|(name, _)| name.clone()).collect();
    for (name, bytes) in entries {
        if let Some(problem) = song_file_problem(name, bytes, &names) {
            return Err(ArchiveError::InvalidSong(format!("{name}: {problem}")));
        }
    }
    Ok(())
}

/// What is wrong with the song file `name` holding `bytes`, if it is one: a sidecar that
/// does not parse or, among the archive's `names`, has no `.mid` beside it; a MIDI file
/// that does not open with `MThd`. `None` for anything that is not a song's file.
fn song_file_problem(name: &str, bytes: &[u8], names: &HashSet<String>) -> Option<String> {
    if let Some(midi_name) = name.strip_suffix(".ron").filter(|stem| stem.ends_with(".mid")) {
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) => return Some(format!("the sidecar is not text: {error}")),
        };
        if let Err(error) = music_midi::parse_song_file(name, text) {
            return Some(error.to_string());
        }
        if !names.contains(midi_name) {
            return Some(format!("its MIDI file {midi_name} is not in the archive"));
        }
        return None;
    }
    if name.ends_with(".mid") && !bytes.starts_with(MIDI_HEADER) {
        return Some("it is not a MIDI file (no MThd header)".to_string());
    }
    None
}
