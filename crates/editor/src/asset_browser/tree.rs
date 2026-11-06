//! Asset discovery, companion classification and the project naming convention.

use std::path::Path;
use super::{AssetEntry, AssetKind};
use crate::external_tools::{is_daw_project_path, is_midi_path};

/// One folder relative to the asset root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetFolder { pub relative_path: String, pub name: String }

impl AssetFolder {
    pub(super) fn new(path: &str) -> Self {
        Self { relative_path: path.into(), name: path.rsplit('/').next().unwrap_or(path).into() }
    }
}

/// A recursive scan keeps folders separate from the files they contain.
#[derive(Debug, Clone, Default)]
pub struct AssetScan { pub files: Vec<AssetEntry>, pub folders: Vec<AssetFolder> }

/// Classify companions before the generic RON scene rule.
pub(super) fn kind_for_path(path: &Path, daw_project_extensions: &[String]) -> Option<AssetKind> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    if name.ends_with(".sheet.ron") || name.strip_suffix(".ron").is_some_and(|stem| is_midi_path(Path::new(stem))) {
        return Some(AssetKind::Companion);
    }
    if is_midi_path(path) { return Some(AssetKind::Song); }
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "bmp" => Some(AssetKind::Image),
        "ron" => Some(AssetKind::Scene),
        "rhai" | "rs" => Some(AssetKind::Script),
        "wav" | "mp3" | "ogg" | "flac" => Some(AssetKind::Sound),
        "sf2" => Some(AssetKind::SoundFont),
        _ if is_daw_project_path(path, daw_project_extensions) => Some(AssetKind::DawProject),
        _ => None,
    }
}

/// The companion beside a PNG or MIDI primary, including a nested primary.
pub fn companion_path(primary: &str) -> Option<String> {
    let path = Path::new(primary);
    if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("png")) {
        Some(path.with_extension("sheet.ron").to_string_lossy().into_owned())
    } else if is_midi_path(path) { Some(format!("{primary}.ron")) } else { None }
}

/// Recursively scan every depth without following symlinks. Missing or unreadable
/// folders contribute nothing. Companions remain data but never become visible tiles.
pub fn scan_assets(base: &Path, daw_project_extensions: &[String]) -> AssetScan {
    let mut scan = AssetScan::default();
    let mut pending = vec![base.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = common::vfs::list_entries(&directory) else { continue; };
        for entry in entries {
            let Ok(relative) = entry.path.strip_prefix(base) else { continue; };
            let relative_path = relative.components().map(|component| component.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
            if entry.is_dir {
                scan.folders.push(AssetFolder::new(&relative_path));
                pending.push(entry.path);
                continue;
            }
            let Some(kind) = kind_for_path(&entry.path, daw_project_extensions) else { continue; };
            let Some(name) = entry.path.file_name().and_then(|name| name.to_str()) else { continue; };
            let has_sidecar = companion_path(&relative_path).is_some_and(|path| common::vfs::exists(&base.join(path)));
            scan.files.push(AssetEntry {
                name: name.into(), relative_path, kind, has_sidecar,
                texture_handle: None, load_failed: false, image_size: None,
            });
        }
    }
    scan.files.sort_by(|first, second| first.kind.cmp(&second.kind).then(first.name.cmp(&second.name)));
    scan.folders.sort_by(|first, second| first.relative_path.cmp(&second.relative_path));
    scan
}

struct Convention { kind: AssetKind, root: &'static str, extension: &'static str }

/// The creation convention, shared by the validator and router:
///
/// | Folder | Holds | Names |
/// | --- | --- | --- |
/// | scenes/ | .scene.ron | snake_case, optional _2p |
/// | scripts/ | .rhai | entity or role |
/// | sprites/ | .png + .sheet.ron | snake_case, size suffix, ai_ for AI cuts |
/// | sounds/ | .wav and other recorded audio + LICENSE | snake_case, no snd_ prefix |
/// | music/ | .mid + .mid.ron + LICENSE, samples/ | song naming rules |
/// | soundfonts/ | .sf2 + LICENSE | game or bank name |
/// | fonts/ | .ttf/.otf + LICENSE | distributed filename, preserved |
/// | locales/ | .ron | locale code |
/// | ai/ | mirrored kind folders | ai_ prefix |
///
/// LICENSE, vendored fonts, generated recovery/collision names and .keep are existing
/// file exceptions. New authored names use ASCII snake_case and one extension chain.
const CONVENTIONS: &[Convention] = &[
    Convention { kind: AssetKind::Scene, root: "scenes/", extension: ".scene.ron" },
    Convention { kind: AssetKind::Script, root: "scripts/", extension: ".rhai" },
    Convention { kind: AssetKind::Image, root: "sprites/", extension: ".png" },
    Convention { kind: AssetKind::Sound, root: "sounds/", extension: ".wav" },
    Convention { kind: AssetKind::Song, root: "music/", extension: ".mid" },
    Convention { kind: AssetKind::SoundFont, root: "soundfonts/", extension: ".sf2" },
    Convention { kind: AssetKind::Font, root: "fonts/", extension: ".ttf" },
    Convention { kind: AssetKind::Locale, root: "locales/", extension: ".ron" },
];

/// The asset root for a file kind; folders and companions are placed by their owner.
pub fn kind_root(kind: AssetKind) -> &'static str {
    CONVENTIONS.iter().find(|rule| rule.kind == kind).map_or("", |rule| rule.root)
}

/// Validate a single authored filename, appending its extension chain when absent.
/// A font's distributed filename is kept; no kind accepts path separators or traversal.
pub fn validate_name(name: &str, kind: AssetKind) -> Result<String, String> {
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err("Use one name, without folders or parent-directory components".into());
    }
    let extension = CONVENTIONS.iter().find(|rule| rule.kind == kind).map_or("", |rule| rule.extension);
    if kind == AssetKind::Font {
        return Ok(if name.contains('.') { name.into() } else { format!("{name}{extension}") });
    }
    if kind == AssetKind::Companion {
        return Err("A companion takes its primary file's name".into());
    }
    let stem = name.strip_suffix(extension).unwrap_or(name);
    if stem.contains('.') { return Err("Use one dot before the kind's extension chain".into()); }
    if !stem.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        || stem.ends_with('_') || stem.contains("__")
        || !stem.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_') {
        return Err("Use ASCII snake_case: start with a lowercase letter, then lowercase letters, digits or single underscores; no spaces or capitals".into());
    }
    if kind == AssetKind::Sound && stem.starts_with("snd_") {
        return Err("Name a sound for its action without the snd_ prefix".into());
    }
    Ok(format!("{stem}{extension}"))
}

/// Validate a new folder in the current one, refusing nested kind roots.
pub fn validate_folder(current_folder: &str, name: &str) -> Result<String, String> {
    let name = validate_name(name, AssetKind::Folder)?;
    let current_kind = current_folder.split('/').find(|segment| CONVENTIONS.iter().any(|rule| rule.root.trim_end_matches('/') == *segment));
    if current_kind.is_some() && CONVENTIONS.iter().any(|rule| rule.root.trim_end_matches('/') == name) {
        return Err("Keep kind folders at the asset root; do not nest one kind folder inside another".into());
    }
    Ok(name)
}

/// Keep the author's subfolder when it is within the kind root, otherwise use that root.
/// A folder stays in the current directory. Companions use [`companion_path`] instead.
pub fn route_new_file(current_folder: &str, kind: AssetKind, name: &str) -> String {
    let root = kind_root(kind).trim_end_matches('/');
    let folder = if kind == AssetKind::Folder || current_folder == root || current_folder.starts_with(&format!("{root}/")) {
        current_folder
    } else { root };
    if folder.is_empty() { name.into() } else { format!("{folder}/{name}") }
}
