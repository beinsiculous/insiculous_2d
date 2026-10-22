//! An imported archive's songs: what a final file must be for the archive to be taken, and
//! what becomes of a save's journal leftover that is not what it says.
//!
//! A final `.mid.ron` must parse as a sidecar and sit beside its `.mid`, and a final `.mid`
//! must open with `MThd`; either failing refuses the whole archive, as a bad scene does. A
//! `.mid.new` or `.mid.ron.new` is checked the same way, but one that fails is dropped with
//! a note instead: it is recovery data, not the project, and the song's open-time rules
//! settle whatever journal files survive the next time the song is opened.

use std::collections::HashSet;

use super::ArchiveError;

const MIDI_HEADER: &[u8] = b"MThd";

/// Check every song file among `entries`, dropping a journal leftover that fails; returns a
/// note for each one dropped.
pub(super) fn check_song_entries(entries: &mut Vec<(String, Vec<u8>)>) -> Result<Vec<String>, ArchiveError> {
    let names: HashSet<String> = entries.iter().map(|(name, _)| name.clone()).collect();
    let mut notes = Vec::new();
    let mut refusal = None;
    entries.retain(|(name, bytes)| {
        if refusal.is_some() {
            return true;
        }
        if let Some(final_name) = name.strip_suffix(".new") {
            return match song_file_problem(final_name, bytes, None) {
                Some(problem) => {
                    notes.push(format!("{name} was left out: {problem}"));
                    false
                }
                None => true,
            };
        }
        if let Some(problem) = song_file_problem(name, bytes, Some(&names)) {
            refusal = Some(ArchiveError::InvalidSong(format!("{name}: {problem}")));
        }
        true
    });
    match refusal {
        Some(error) => Err(error),
        None => Ok(notes),
    }
}

/// What is wrong with the song file `name` holding `bytes`, if it is one: a sidecar that
/// does not parse or, given the archive's `names`, has no `.mid` beside it; a MIDI file
/// that does not open with `MThd`. `None` for anything that is not a song's file.
fn song_file_problem(name: &str, bytes: &[u8], names: Option<&HashSet<String>>) -> Option<String> {
    if let Some(midi_name) = name.strip_suffix(".ron").filter(|stem| stem.ends_with(".mid")) {
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) => return Some(format!("the sidecar is not text: {error}")),
        };
        if let Err(error) = music_midi::parse_song_file(name, text) {
            return Some(error.to_string());
        }
        if names.is_some_and(|names| !names.contains(midi_name)) {
            return Some(format!("its MIDI file {midi_name} is not in the archive"));
        }
        return None;
    }
    if name.ends_with(".mid") && !bytes.starts_with(MIDI_HEADER) {
        return Some("it is not a MIDI file (no MThd header)".to_string());
    }
    None
}
