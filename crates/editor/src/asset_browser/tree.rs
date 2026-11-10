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

/// A primary's VFS path, its existing companion and its recovered sibling files, or the
/// folder listing's error — a family read short would rename or delete less than the
/// person confirmed. The caller passes the path under the configured asset root, not a
/// project-relative name.
pub fn asset_family(primary: &str) -> std::io::Result<Vec<String>> {
    let mut family = vec![primary.to_string()];
    family.extend(readable_companion(primary)?);
    let path = Path::new(primary);
    let (Some(stem), Some(name)) = (path.file_stem().and_then(|stem| stem.to_str()), path.file_name().and_then(|name| name.to_str())) else { return Ok(family); };
    let directory = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
    // A recovered sibling is a copy of the asset under `<stem>.recovered-<token><the asset's
    // own extension>`: the extension keeps a `hero.recovered-notes.txt` out of hero.png's
    // family, and a dot-free token keeps the sibling's own sidecar from passing as a
    // sibling — it comes in as that sibling's companion instead.
    // The stem as spelled — `Hero.png` and `hero.png` are two assets where the filesystem
    // tells them apart — and the extension in any case, as the scanner classes it.
    let suffix = name[stem.len()..].to_ascii_lowercase();
    let prefix = format!("{stem}.recovered-");
    let is_recovered = |candidate: &str| {
        let Some(after_prefix) = candidate.strip_prefix(prefix.as_str()) else { return false; };
        // A char boundary is never assumed from another string's byte length.
        let Some(split) = after_prefix.len().checked_sub(suffix.len()).filter(|split| after_prefix.is_char_boundary(*split)) else { return false; };
        let (token, found_suffix) = after_prefix.split_at(split);
        !token.is_empty() && !token.contains('.') && found_suffix.eq_ignore_ascii_case(&suffix)
    };
    let mut recovered: Vec<String> = common::vfs::list_entries(directory)?.into_iter()
        .filter(|entry| !entry.is_dir && entry.path.file_name().and_then(|name| name.to_str()).is_some_and(is_recovered))
        .map(|entry| entry.path.to_string_lossy().into_owned()).collect();
    recovered.sort();
    for sibling in recovered {
        let companion = readable_companion(&sibling)?;
        family.push(sibling);
        family.extend(companion);
    }
    Ok(family)
}

/// The primary's companion when it is a readable file: a folder wearing the sidecar's name
/// is no companion (a rename would move its whole contents beside the image), a missing one
/// is none, and one that cannot be read is an error — a family read short would rename or
/// delete less than the person confirmed.
fn readable_companion(primary: &str) -> std::io::Result<Option<String>> {
    let Some(expected) = companion_path(primary) else { return Ok(None); };
    let spellings = spelled_on_disk(&expected);
    if spellings.len() > 1 {
        return Err(std::io::Error::other(format!("{} has {} spellings beside it ({}); keep one", expected, spellings.len(), spellings.join(", "))));
    }
    let companion = spellings.into_iter().next().unwrap_or(expected);
    if Path::new(&companion).is_dir() { return Ok(None); }
    match common::vfs::read(Path::new(&companion)) {
        Ok(_) => Ok(Some(companion)),
        Err(error) if matches!(error.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::IsADirectory) => Ok(None),
        Err(error) => Err(error),
    }
}

/// The companion at `expected` as the folder spells it: the scan pairs a sidecar in any
/// case of its suffix, so whatever opens one asks for the name on disk, not the derived one.
pub fn companion_on_disk(expected: &str) -> Option<String> {
    spelled_on_disk(expected).into_iter().next()
}

/// Every file the folder holds under `expected`'s name in any case of its suffix
/// (`hero.SHEET.RON` for `hero.sheet.ron`, which the scanner classes a companion all the
/// same): none when there is none, more than one where two spellings sit side by side.
fn spelled_on_disk(expected: &str) -> Vec<String> {
    let path = Path::new(expected);
    let (Some(parent), Some(name)) = (path.parent(), path.file_name().and_then(|name| name.to_str())) else { return Vec::new(); };
    let directory = if parent.as_os_str().is_empty() { Path::new(".") } else { parent };
    common::vfs::list_entries(directory).ok().into_iter().flatten()
        .filter(|entry| !entry.is_dir)
        .filter(|entry| entry.path.file_name().and_then(|found| found.to_str()).is_some_and(|found| same_companion_name(name, found)))
        .map(|entry| entry.path.to_string_lossy().into_owned())
        .collect()
}

/// Whether `found` is the companion `expected` names: the stem — up to the first dot — as
/// spelled, since `HERO.PNG` and `hero.png` are two assets where the filesystem tells them
/// apart, and the suffix in any case, as the scanner classes it.
/// A companion path's key for pairing: its folder and stem as spelled, its suffix lowered.
pub(super) fn companion_key(path: &str) -> String {
    let (folder, name) = path.rsplit_once('/').unwrap_or(("", path));
    let (stem, suffix) = name.split_at(name.find('.').unwrap_or(name.len()));
    format!("{folder}/{stem}{}", suffix.to_ascii_lowercase())
}

pub(crate) fn same_companion_name(expected: &str, found: &str) -> bool {
    let (expected_stem, expected_suffix) = expected.split_at(expected.find('.').unwrap_or(expected.len()));
    let Some(found_suffix) = found.strip_prefix(expected_stem) else { return false; };
    found_suffix.eq_ignore_ascii_case(expected_suffix)
}

/// Recursively scan every depth without following symlinks. Missing or unreadable
/// folders contribute nothing. Only companions without a primary become visible tiles.
pub fn scan_assets(base: &Path, daw_project_extensions: &[String]) -> AssetScan {
    let mut scan = AssetScan::default();
    let mut pending = vec![base.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = common::vfs::list_entries(&directory) else { continue; };
        // The companions this folder holds, keyed by stem and lowered suffix: the web's
        // listing walks every key in the project, so it is taken once per folder.
        let companions_here: std::collections::HashSet<String> = entries.iter()
            .filter(|entry| !entry.is_dir)
            .filter_map(|entry| entry.path.file_name().and_then(|name| name.to_str()).map(companion_key))
            .collect();
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
            let has_sidecar = companion_path(&relative_path)
                .and_then(|path| Path::new(&path).file_name().and_then(|name| name.to_str()).map(companion_key))
                .is_some_and(|key| companions_here.contains(&key));
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
    validate_single_name(name)?;
    let extension = CONVENTIONS.iter().find(|rule| rule.kind == kind).map_or("", |rule| rule.extension);
    if kind == AssetKind::Font {
        return Ok(if name.contains('.') { name.into() } else { format!("{name}{extension}") });
    }
    if kind == AssetKind::Companion {
        return Err("A companion takes its primary file's name".into());
    }
    let stem = name.strip_suffix(extension).unwrap_or(name);
    validate_stem(stem, kind)?;
    Ok(format!("{stem}{extension}"))
}

/// Validate a rename while preserving the current extension, including `.scene.ron`.
/// A font keeps its distributed spelling; authored stems follow the creation convention.
pub fn validate_rename(name: &str, kind: AssetKind, current_extension: &str) -> Result<String, String> {
    validate_single_name(name)?;
    if kind == AssetKind::Companion {
        return Err("an orphan companion is deleted, or paired by adding its primary".into());
    }
    let extension = if current_extension.is_empty() || current_extension.starts_with('.') {
        current_extension.to_string()
    } else { format!(".{current_extension}") };
    let stem = name.strip_suffix(&extension).unwrap_or(name);
    // A font or a SoundFont is distributed under its own spelling ("GeneralUser-GS.sf2"),
    // which the creation rule never governed; the prompt's prefilled name must pass. Not
    // whitespace, though: a LICENSE names files by whitespace-separated tokens.
    if matches!(kind, AssetKind::Font | AssetKind::SoundFont) {
        if name.chars().any(char::is_whitespace) {
            return Err("Use a name without whitespace: a LICENSE names files by whitespace-separated tokens".into());
        }
        if name.contains('.') && !name.ends_with(&extension) {
            return Err(format!("Keep this asset's {extension} extension"));
        }
    } else {
        validate_stem(stem, kind)?;
    }
    let renamed = format!("{stem}{extension}");
    if is_midi_path(Path::new(&renamed)) {
        crate::song::saveable_song_path(&Path::new("music").join(&renamed))?;
    }
    Ok(renamed)
}

fn validate_single_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err("Use one name, without folders or parent-directory components".into());
    }
    Ok(())
}

fn validate_stem(stem: &str, kind: AssetKind) -> Result<(), String> {
    if stem.contains('.') { return Err("Use one dot before the kind's extension chain".into()); }
    if !stem.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        || stem.ends_with('_') || stem.contains("__")
        || !stem.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_') {
        return Err("Use ASCII snake_case: start with a lowercase letter, then lowercase letters, digits or single underscores; no spaces or capitals".into());
    }
    if kind == AssetKind::Sound && stem.starts_with("snd_") {
        return Err("Name a sound for its action without the snd_ prefix".into());
    }
    Ok(())
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
