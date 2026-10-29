//! The project's songs as the external tools see them: every MIDI file in the song folders,
//! a song's two files read for a launch, and where in the MIDI file a part, a binding or
//! the first part lives — the `Location::Track` a music tool opens at.
//!
//! The pair is read straight from disk through `common::vfs`, never through the song
//! document's open, which settles an interrupted save (it may write) and refuses a file
//! outside the song folders. Resolving a track is pure over what was read.

use std::io;
use std::path::{Path, PathBuf};

use music_midi::{parse_song_file, Part, SongData, SongFile, SongFileProblem};

use super::config::Location;
use super::file_type::{is_midi_path, midi_sidecar_path};
use crate::song::HOME_DIRECTORIES;

/// Every MIDI file (`is_midi_path`) under `music/` and `ai/music/` (subfolders included),
/// by its asset-relative path with `/` separators, sorted. A missing folder lists nothing.
pub fn scan_songs(asset_base: &Path) -> Vec<String> {
    let mut songs = Vec::new();
    for home in HOME_DIRECTORIES {
        let listed = match common::vfs::list_files(&asset_base.join(home)) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                log::warn!("listing the songs under {home}/ failed: {error}");
                Vec::new()
            }
            Ok(files) => files,
        };
        for path in listed {
            let Ok(relative) = path.strip_prefix(asset_base) else {
                continue;
            };
            if is_midi_path(&path) {
                songs.push(
                    relative
                        .components()
                        .map(|component| component.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
        }
    }
    songs.sort();
    songs.dedup();
    songs
}

/// Why a song's two files could not be read for a launch.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SongPairError {
    #[error("{} could not be read: {reason}", .path.display())]
    Midi { path: PathBuf, reason: String },
    #[error("{} does not exist", .sidecar.display())]
    SidecarMissing { sidecar: PathBuf },
    #[error("{} could not be read: {reason}", .sidecar.display())]
    Sidecar { sidecar: PathBuf, reason: String },
}

/// Read the song at `midi_path` — already resolved against the asset base — and its
/// sidecar beside it (`midi_sidecar_path`), each parsed as the engine parses it.
pub fn read_song_pair(midi_path: &Path) -> Result<(SongFile, SongData), SongPairError> {
    let midi_error = |reason: String| SongPairError::Midi { path: midi_path.to_path_buf(), reason };
    let bytes = common::vfs::read(midi_path).map_err(|error| midi_error(error.to_string()))?;
    let (data, _) = SongData::from_smf(&bytes).map_err(|error| midi_error(error.to_string()))?;
    let sidecar = midi_sidecar_path(midi_path);
    let text = match common::vfs::read_to_string(&sidecar) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(SongPairError::SidecarMissing { sidecar });
        }
        Err(error) => return Err(SongPairError::Sidecar { sidecar, reason: error.to_string() }),
    };
    let file = parse_song_file(&sidecar.to_string_lossy(), &text)
        .map_err(|error| SongPairError::Sidecar { sidecar: sidecar.clone(), reason: error.to_string() })?;
    Ok((file, data))
}

/// Why a binding names no track. Each sentence follows the sidecar's path.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BindingLookupError {
    #[error("binds no action '{action}' — {}", actions_sentence(.actions))]
    UnknownAction { action: String, actions: Vec<String> },
    #[error("binds '{action}' to the part '{part}', which it does not declare")]
    UnknownPart { action: String, part: String },
    #[error("cannot place '{action}': {problem}")]
    Track { action: String, problem: SongFileProblem },
}

fn actions_sentence(actions: &[String]) -> String {
    if actions.is_empty() {
        "it binds none".to_string()
    } else {
        format!("its actions are {}", actions.join(", "))
    }
}

/// Where `part` lives in the MIDI file: the track `Part::resolve_track` selects, by its
/// name and its index **in the file** — a nameless track by its index alone (`name: None`).
/// A type-0 file is read as one track per channel, all from the file's track 0: the tool
/// is sent that track, and no name, since the channel tracks' names are the reader's.
pub fn part_location(part: &Part, data: &SongData) -> Result<Location, SongFileProblem> {
    let selected = part.resolve_track(data)?;
    let tracks = data.tracks();
    let Some(track) = tracks.get(selected) else {
        return Ok(Location::Track { name: None, index: selected });
    };
    let index = track.source_track();
    let shares_its_file_track = tracks.iter().enumerate().any(|(other, candidate)| other != selected && candidate.source_track() == index);
    let name = if shares_its_file_track { None } else { track.name().map(str::to_string) };
    Ok(Location::Track { name, index })
}

/// Where the part `action` plays lives in `data`: the action's binding, its part, then
/// [`part_location`].
pub fn binding_location(file: &SongFile, data: &SongData, action: &str) -> Result<Location, BindingLookupError> {
    let Some(binding) = file.bindings.iter().find(|binding| binding.action == action) else {
        let actions = file.bindings.iter().map(|binding| binding.action.clone()).collect();
        return Err(BindingLookupError::UnknownAction { action: action.to_string(), actions });
    };
    let Some(part) = file.parts.iter().find(|part| part.name == binding.part) else {
        return Err(BindingLookupError::UnknownPart { action: action.to_string(), part: binding.part.clone() });
    };
    part_location(part, data).map_err(|problem| BindingLookupError::Track { action: action.to_string(), problem })
}

/// Where the sidecar's first part lives in `data`; `Ok(None)` for a sidecar with no parts.
pub fn first_part_location(file: &SongFile, data: &SongData) -> Result<Option<Location>, SongFileProblem> {
    file.parts.first().map(|part| part_location(part, data)).transpose()
}

#[cfg(test)]
#[path = "songs_tests.rs"]
mod songs_tests;
