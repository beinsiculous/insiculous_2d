//! Where a song may live: the two song folders, the path rule Save and the web import
//! share, and the sample folder a song's path names.

use std::path::{Component, Path};

/// The folders a song saves into, under the asset folder: the human's, and the agents'
/// (whose songs carry the `ai_` prefix until a human pass promotes them).
pub const HOME_DIRECTORIES: [&str; 2] = ["music", "ai/music"];

/// The folder under each home that holds its songs' samples, one folder a song.
const SAMPLES_FOLDER: &str = "samples";

/// The separator a song's path under its home is flattened by into its sample folder's
/// name; [`saveable_song_path`] keeps it unambiguous.
const FLAT_SEPARATOR: &str = "__";

/// Whether `path` lies in a song folder under `asset_base`. A path not under
/// `asset_base` at all — a relative one included — is outside: the samples are placed
/// under `asset_base`, so a target resolved anywhere else would split a song in two.
pub(in crate::song) fn is_in_home_directory(path: &Path, asset_base: &Path) -> bool {
    path.strip_prefix(asset_base)
        .is_ok_and(|relative| home_of(relative).is_some())
}

/// The song folder `relative` (a path under the asset base) lies in.
pub(super) fn home_of(relative: &Path) -> Option<&'static str> {
    HOME_DIRECTORIES
        .into_iter()
        .find(|home| relative.starts_with(home))
}

/// The path's components as text, the last with its extension dropped.
fn named_components(path: &Path) -> Vec<String> {
    let mut names: Vec<String> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    if let (Some(last), Some(stem)) = (names.last_mut(), path.file_stem()) {
        *last = stem.to_string_lossy().into_owned();
    }
    names
}

/// Whether a song at `relative` (a `.mid` under the asset base) may be saved by name: in a
/// song folder, not under that folder's `samples/` (a song there would sit inside another
/// song's sample folder, which a save prunes), and with no component starting or ending
/// with `_`, no `__` and no whitespace — so the `__` that flattens a song's path into its
/// sample folder's name is never ambiguous, and the provenance gate, which splits a
/// `Files:` field on whitespace, can name its files. The refusal says what to change.
pub fn saveable_song_path(relative: &Path) -> Result<(), String> {
    let shown = relative.display();
    let Some(home) = home_of(relative) else {
        return Err(format!(
            "{shown} is outside the song folders; save the song under music/ or ai/music/"
        ));
    };
    if relative.starts_with(Path::new(home).join(SAMPLES_FOLDER)) {
        return Err(format!(
            "{shown} is inside {home}/{SAMPLES_FOLDER}/, which holds the songs' samples; Save As to a path outside it"
        ));
    }
    for name in named_components(relative) {
        if name.chars().any(char::is_whitespace) {
            return Err(format!(
                "{shown} has whitespace in '{name}'; Save As to a name without it (write _ instead)"
            ));
        }
        if name.starts_with('_') || name.ends_with('_') || name.contains(FLAT_SEPARATOR) {
            return Err(format!(
                "{shown}: '{name}' starts or ends with _ or holds __, which name a song's sample folder; Save As to a name without them"
            ));
        }
    }
    Ok(())
}

/// Whether Save takes `relative` on the older, looser rule: a song directly in its home
/// keys its sample folder by its stem alone, so no flattening is involved — short of
/// whitespace, which the provenance gate cannot name.
pub(super) fn is_grandfathered(relative: &Path) -> bool {
    let flat = home_of(relative).is_some_and(|home| relative.parent() == Some(Path::new(home)));
    flat && !relative.to_string_lossy().chars().any(char::is_whitespace)
}

/// Where the song at `relative` (a `.mid` under the asset base) keeps its samples, under
/// the asset base: `<home>/samples/<flat>`, `<flat>` its path under its home, extension
/// dropped, each `/` made `__` — `music/theme.mid` → `music/samples/theme`,
/// `music/a/theme.mid` → `music/samples/a__theme`. `None` outside the song folders.
pub fn sample_directory(relative: &Path) -> Option<String> {
    let home = home_of(relative)?;
    let under_home = relative.strip_prefix(home).ok()?;
    let flat = named_components(under_home).join(FLAT_SEPARATOR);
    Some(format!("{home}/{SAMPLES_FOLDER}/{flat}"))
}

/// `path` as the sidecar and the `LICENSE` files write it: `/`-separated, whatever the target.
pub(super) fn slash_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}
