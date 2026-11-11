//! Old paths for a stored project made before the project layout (`docs/PROJECT_LAYOUT.md`):
//! the five bundled files the layout renamed, laid in memory under their old keys at boot when
//! a stored scene or script still names them.
//!
//! One direction only: the bundle's file goes under the old key, and nothing puts a visitor's
//! old-key file under the new one. Two keys for one file kept in step would be a second store
//! invented at boot — every form of that mirror lost a file a saved scene named. So nothing here
//! is stored or written: the aliases are inserted before the stored records are laid down, a
//! stored record at an old key replaces its alias and a stored tombstone there removes it, and
//! the persistence observers are not installed yet, so an alias queues nothing.

use std::io;
use std::path::Path;

use crate::store::StoredFile;

/// One file the layout renamed in a bundled project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyPath {
    /// The bundled project's slug.
    pub slug: &'static str,
    /// The file's key before the rename, relative to the project root.
    pub old_key: &'static str,
    /// The bundle's file now, relative to the project root.
    pub bundle_file: &'static str,
}

/// Exactly the files the rename moved, never a prefix: a sprite a later bundle adds is not
/// aliased under `images/`, and the table never grows with the bundle. The new sidecars have
/// no row — the old paths had none, so an aliased file behaves as it did before the rename.
pub const LEGACY_PATHS: &[LegacyPath] = &[
    LegacyPath { slug: "examples", old_key: "assets/images/wood_texture.png", bundle_file: "assets/sprites/wood_texture.png" },
    LegacyPath { slug: "examples", old_key: "assets/sounds/snd_jump.wav", bundle_file: "assets/sounds/jump.wav" },
    LegacyPath { slug: "pong", old_key: "assets/images/ball_8px.png", bundle_file: "assets/sprites/ball_8px.png" },
    LegacyPath { slug: "pong", old_key: "assets/images/paddle_16px.png", bundle_file: "assets/sprites/paddle_16px.png" },
    LegacyPath { slug: "game-template", old_key: "assets/images/ball_8px.png", bundle_file: "assets/sprites/ball_8px.png" },
];

/// A scene or a script names an asset relative to the project's asset root.
const ASSET_ROOT: &str = "assets/";

/// The `(old key, bundle file)` pairs of `slug`'s rows a live stored scene or script names: a
/// stored record not tombstoned, whose path ends in `.ron` or `.rhai` in any case, containing
/// the row's old key less its `assets/` as a plain substring. Nothing else is consulted — not
/// the manifest, not the bundle version — since a tab left open across a deploy saves its old
/// scene after a newer tab has stamped the manifest. A plain substring over-selects on purpose:
/// a spare key costs nothing, and a missed one is a scene that will not load. Empty for a slug
/// with no rows and for a project with no stored files.
pub fn selected_aliases(slug: &str, stored_files: &[StoredFile]) -> Vec<(&'static str, &'static str)> {
    let referencing_files: Vec<&StoredFile> =
        stored_files.iter().filter(|file| !file.deleted && can_name_assets(&file.path)).collect();
    LEGACY_PATHS
        .iter()
        .filter(|row| row.slug == slug)
        .filter(|row| {
            let token = row.old_key.strip_prefix(ASSET_ROOT).unwrap_or(row.old_key).as_bytes();
            referencing_files.iter().any(|file| file.bytes.windows(token.len()).any(|window| window == token))
        })
        .map(|row| (row.old_key, row.bundle_file))
        .collect()
}

/// Whether a stored path is a scene, a sidecar or a script — the files that name assets by
/// path — its extension compared without regard to case, as the scene loader lists scenes and
/// the asset browser's reference scan reads them.
fn can_name_assets(path: &str) -> bool {
    let extension = Path::new(path).extension().and_then(|extension| extension.to_str()).map(str::to_ascii_lowercase);
    matches!(extension.as_deref(), Some("ron" | "rhai"))
}

/// Lays each `(old key, bundle file)` pair under the project's `root`: the bundle's bytes are
/// inserted at the old key when the bundle's file is readable and nothing is at the old key yet.
/// `read`, `exists` and `insert` are the filesystem's — the browser's global map at boot, a
/// `common::vfs::MemFs` in a test. Called after the bundle's preload and before the stored
/// records are laid down over the same map.
pub fn lay_aliases(
    root: &str,
    aliases: &[(&str, &str)],
    read: impl Fn(&Path) -> io::Result<Vec<u8>>,
    exists: impl Fn(&Path) -> bool,
    mut insert: impl FnMut(String, Vec<u8>),
) {
    for (old_key, bundle_file) in aliases {
        let old_path = format!("{root}/{old_key}");
        if exists(Path::new(&old_path)) {
            continue;
        }
        match read(Path::new(&format!("{root}/{bundle_file}"))) {
            Ok(bytes) => insert(old_path, bytes),
            Err(error) => log::warn!("{old_path} is not aliased: the bundle's {bundle_file} could not be read ({error})"),
        }
    }
}

#[cfg(test)]
mod tests;
