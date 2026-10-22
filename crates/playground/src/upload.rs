//! A file the page imports into the open project: a song (`.mid`), a sample (`.wav`) or a
//! SoundFont (`.sf2`), written where its kind lives and persisted by the vfs observer like
//! every other write.
//!
//! **Nothing is written until the file is known good.** The name is held to
//! `[A-Za-z0-9._-]+` first — no quote breaks the `song open "…"` the page sends after, and
//! no whitespace reaches a `LICENSE` field the provenance gate splits on; a sample's or a
//! SoundFont's whitespace becomes `_` as a save's does, and a song's name is held to the
//! path rule Save uses. Then the whole file is refused by its size against the loader's
//! own limit, before any decode, and then decoded by the loader that will read it.
//!
//! **A taken name is replaced only when asked.** The page asks first, naming every song
//! whose sidecar names the file. A song's `.mid` is never replaced while it is the song the
//! composer holds; otherwise its sidecar and its journal pair are removed first and the
//! MIDI written last, so every state the store passes through opens — the old MIDI as an
//! import, then the new one.

use std::path::{Path, PathBuf};

use audio::{decode_sample_pcm, SAMPLE_BYTE_LIMIT, SOUND_FONT_BYTE_LIMIT};
use music_midi::{parse_song_file, parse_sound_font, InstrumentSource, SongData, MIDI_BYTE_LIMIT};
use serde::Serialize;

use crate::bridge::validate_bridge_path;

/// What an imported file is, told by its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum UploadKind {
    Song,
    Sample,
    SoundFont,
}

impl UploadKind {
    fn of(name: &str) -> Option<Self> {
        let extension = Path::new(name).extension()?.to_string_lossy().to_ascii_lowercase();
        match extension.as_str() {
            "mid" => Some(UploadKind::Song),
            "wav" => Some(UploadKind::Sample),
            "sf2" => Some(UploadKind::SoundFont),
            _ => None,
        }
    }

    /// The largest file the loader that reads this kind accepts, in bytes.
    fn byte_limit(self) -> usize {
        match self {
            UploadKind::Song => MIDI_BYTE_LIMIT,
            UploadKind::Sample => SAMPLE_BYTE_LIMIT,
            UploadKind::SoundFont => SOUND_FONT_BYTE_LIMIT,
        }
    }

    /// Where a file of this kind is written, relative to the project root.
    fn folder(self) -> &'static str {
        match self {
            UploadKind::Song => "assets/music",
            UploadKind::Sample => "assets/music/imports",
            UploadKind::SoundFont => "assets/soundfonts",
        }
    }
}

/// Where an imported file goes, once its name, size and contents passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadPlan {
    pub kind: UploadKind,
    /// Relative to the project root: `assets/music/theme.mid`.
    pub destination: String,
}

/// What an import did, as the page reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UploadOutcome {
    /// `false` when the name was taken and no replace was asked: nothing was written.
    pub written: bool,
    pub kind: UploadKind,
    pub destination: String,
    /// Every song whose sidecar names the file, by its `.mid` under the asset folder.
    pub referenced_by: Vec<String>,
    /// For a taken song name: its sidecar, whose bindings and instruments a replace loses.
    pub sidecar_lost: bool,
}

/// Check `file_name` and `bytes` for an import, writing nothing.
pub fn plan_upload(file_name: &str, bytes: &[u8]) -> Result<UploadPlan, String> {
    let kind = UploadKind::of(file_name)
        .ok_or_else(|| format!("{file_name} is not a song, a sample or a SoundFont: import a .mid, a .wav or a .sf2"))?;
    let name = upload_name(kind, file_name)?;
    let destination = format!("{}/{name}", kind.folder());
    if kind == UploadKind::Song {
        let relative = destination.trim_start_matches("assets/");
        editor::song::saveable_song_path(Path::new(relative))?;
    }
    let limit = kind.byte_limit();
    if bytes.len() > limit {
        return Err(format!(
            "{name} is {} bytes, over the {} KiB this kind of file may hold",
            bytes.len(),
            limit / 1024
        ));
    }
    let refusal = |reason: String| format!("{name} cannot be imported: {reason}");
    match kind {
        UploadKind::Song => SongData::from_smf(bytes).map(drop).map_err(|error| refusal(error.to_string()))?,
        UploadKind::Sample => decode_sample_pcm(bytes.to_vec()).map(drop).map_err(|error| refusal(error.to_string()))?,
        UploadKind::SoundFont => parse_sound_font(&name, bytes).map(drop).map_err(|error| refusal(error.to_string()))?,
    }
    Ok(UploadPlan { kind, destination })
}

/// The name a file is written under: a sample's or a SoundFont's whitespace made `_`, then
/// held to `[A-Za-z0-9._-]+`.
fn upload_name(kind: UploadKind, file_name: &str) -> Result<String, String> {
    let name: String = match kind {
        UploadKind::Song => file_name.to_string(),
        UploadKind::Sample | UploadKind::SoundFont => file_name
            .chars()
            .map(|character| if character.is_whitespace() { '_' } else { character })
            .collect(),
    };
    let allowed = |character: char| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-');
    if name.chars().all(allowed) {
        return Ok(name);
    }
    Err(format!(
        "{file_name} cannot be imported under its name: use letters, digits, '.', '_' and '-' only"
    ))
}

/// Every song under the two song folders whose sidecar names the file at `destination`
/// (relative to the project root), by its `.mid` under the asset folder.
pub fn songs_referencing(project_root: &Path, destination: &str) -> Vec<String> {
    let asset_path = destination.trim_start_matches("assets/");
    let mut songs = Vec::new();
    for home in editor::song::HOME_DIRECTORIES {
        let folder = project_root.join("assets").join(home);
        for sidecar in common::vfs::list_files(&folder).unwrap_or_default() {
            let Some(sidecar_name) = sidecar.to_str().filter(|name| name.ends_with(".mid.ron")) else {
                continue;
            };
            let Ok(text) = common::vfs::read_to_string(&sidecar) else {
                continue;
            };
            let Ok(file) = parse_song_file(sidecar_name, &text) else {
                continue;
            };
            let named = file.soundfont == asset_path
                || file.instruments.iter().any(|instrument| match &instrument.source {
                    InstrumentSource::SoundFont { path, .. } | InstrumentSource::Sample { path, .. } => path == asset_path,
                });
            let midi = sidecar_name.trim_end_matches(".ron");
            if let (true, Ok(relative)) = (named, Path::new(midi).strip_prefix(project_root.join("assets"))) {
                songs.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    songs.sort();
    songs.dedup();
    songs
}

/// Import `file_name` into the project at `project_root`: checked whole first, then
/// written — unless its name is taken and `replace` was not asked, or it is the `.mid` of
/// `open_song`, the song the composer holds.
pub fn import_file(
    project_root: &Path,
    file_name: &str,
    bytes: &[u8],
    open_song: Option<&Path>,
    replace: bool,
) -> Result<UploadOutcome, String> {
    let plan = plan_upload(file_name, bytes)?;
    let target = validate_bridge_path(project_root, &plan.destination)?;
    let taken = common::vfs::file_size(&target).is_ok();
    let sidecar = with_suffix(&target, ".ron");
    let outcome = UploadOutcome {
        written: false,
        kind: plan.kind,
        destination: plan.destination.clone(),
        referenced_by: songs_referencing(project_root, &plan.destination),
        sidecar_lost: plan.kind == UploadKind::Song && common::vfs::file_size(&sidecar).is_ok(),
    };
    if plan.kind == UploadKind::Song && open_song.is_some_and(|open| open.components().eq(target.components())) {
        return Err(format!("{} is the song open in the composer: close the song first", plan.destination));
    }
    if taken && !replace {
        return Ok(outcome);
    }
    let write_error = |error: std::io::Error| format!("writing {} failed: {error}", plan.destination);
    if plan.kind == UploadKind::Song {
        // The store keeps the write order, so each removal lands before the new MIDI does.
        for leftover in [sidecar.clone(), with_suffix(&sidecar, ".new"), with_suffix(&target, ".new")] {
            common::vfs::remove_prefix(&leftover).map_err(write_error)?;
        }
    }
    common::vfs::write(&target, bytes).map_err(write_error)?;
    Ok(UploadOutcome { written: true, ..outcome })
}

/// `path` with `suffix` appended to its last component (`theme.mid` → `theme.mid.ron`).
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut joined = path.as_os_str().to_owned();
    joined.push(suffix);
    PathBuf::from(joined)
}

#[cfg(test)]
mod tests;
