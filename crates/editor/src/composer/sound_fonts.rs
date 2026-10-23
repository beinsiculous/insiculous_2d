//! The project's SoundFonts, as the Song section's SoundFont row cycles them and a new song
//! takes its background from: every `.sf2` under the asset folder's `soundfonts/`.
//!
//! The row checks a bank's existence and nothing of its contents: the editor carries no
//! synthesizer and cannot read a preset list, so a bank missing an instrument's preset
//! fails the next preview with the loader's error naming them.

use std::io;
use std::path::Path;

/// The folder under the asset base a project's banks live in.
pub const SOUND_FONT_DIRECTORY: &str = "soundfonts";

/// What a scan of `soundfonts/` found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SoundFontScan {
    /// Every `.sf2`, by its asset-relative path with `/` separators, sorted.
    pub banks: Vec<String>,
    /// The folder holds a symbolic link and no bank was listed: natively the listing skips
    /// links, so a linked bank is invisible to the row and to a new song.
    pub links_only: bool,
}

/// Every `.sf2` under `<asset_base>/soundfonts/` (subfolders included) — natively the real
/// files, on the web the preloaded and imported ones. A missing folder lists nothing.
pub fn scan_sound_fonts(asset_base: &Path) -> SoundFontScan {
    let directory = asset_base.join(SOUND_FONT_DIRECTORY);
    let listed = match common::vfs::list_files(&directory) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(error) => {
            log::warn!("listing the SoundFonts under {SOUND_FONT_DIRECTORY}/ failed: {error}");
            Vec::new()
        }
        Ok(files) => files,
    };
    let mut banks: Vec<String> = listed
        .iter()
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("sf2"))
        })
        .filter_map(|path| path.strip_prefix(asset_base).ok())
        .map(|relative| {
            relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect();
    banks.sort();
    let links_only = banks.is_empty() && directory_holds_a_link(&directory);
    SoundFontScan { banks, links_only }
}

/// Whether the bank a sidecar names, relative to the asset base, is a file there: a regular
/// file natively (a directory named like a bank has a size too), a key on the web.
pub fn sound_font_exists(asset_base: &Path, path: &str) -> bool {
    let bank = asset_base.join(path);
    #[cfg(not(target_arch = "wasm32"))]
    {
        bank.is_file()
    }
    #[cfg(target_arch = "wasm32")]
    {
        common::vfs::file_size(&bank).is_ok()
    }
}

/// Whether `directory` or any folder under it, as deep as the listing itself walks, holds a
/// symbolic link — the one entry the listing skips: a folder of links and no listed bank is
/// told from a folder with a LICENSE and no bank yet.
#[cfg(not(target_arch = "wasm32"))]
fn directory_holds_a_link(directory: &Path) -> bool {
    holds_a_link_within(directory, common::vfs::MAX_LIST_DEPTH)
}

#[cfg(not(target_arch = "wasm32"))]
fn holds_a_link_within(directory: &Path, depth_left: usize) -> bool {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return false;
    };
    entries.flatten().any(|entry| match entry.file_type() {
        Ok(file_type) if file_type.is_symlink() => true,
        Ok(file_type) if file_type.is_dir() => {
            depth_left > 0 && holds_a_link_within(&entry.path(), depth_left - 1)
        }
        _ => false,
    })
}

/// The web's files are the preloaded and imported ones, never links.
#[cfg(target_arch = "wasm32")]
fn directory_holds_a_link(_directory: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scan_lists_every_bank_under_soundfonts_sorted_and_nothing_else() {
        let base = tempfile::tempdir().expect("a temporary asset folder");
        for path in ["soundfonts/b.sf2", "soundfonts/a.SF2", "soundfonts/more/c.sf2", "soundfonts/notes.txt", "music/x.sf2"] {
            common::vfs::write(&base.path().join(path), b"bank").expect("written");
        }

        let scan = scan_sound_fonts(base.path());

        assert_eq!(scan.banks, ["soundfonts/a.SF2", "soundfonts/b.sf2", "soundfonts/more/c.sf2"]);
        assert!(!scan.links_only);
        assert!(sound_font_exists(base.path(), "soundfonts/b.sf2"));
        assert!(!sound_font_exists(base.path(), "soundfonts/cast.sf2"));
    }

    #[test]
    fn a_missing_folder_lists_nothing_and_a_folder_of_links_says_so() {
        let base = tempfile::tempdir().expect("a temporary asset folder");
        assert_eq!(scan_sound_fonts(base.path()), SoundFontScan::default());
        common::vfs::write(&base.path().join("soundfonts/LICENSE"), b"Format: ...").expect("written");
        std::fs::create_dir_all(base.path().join("soundfonts/folder.sf2")).expect("a folder named like a bank");
        assert_eq!(scan_sound_fonts(base.path()), SoundFontScan::default(), "a LICENSE and no bank is no link");
        assert!(!sound_font_exists(base.path(), "soundfonts/folder.sf2"), "a directory is no bank");

        let elsewhere = tempfile::tempdir().expect("a second folder");
        let bank = elsewhere.path().join("real.sf2");
        std::fs::write(&bank, b"bank").expect("written");
        std::fs::create_dir_all(base.path().join(SOUND_FONT_DIRECTORY)).expect("folder");
        #[cfg(unix)]
        {
            std::fs::create_dir_all(base.path().join("soundfonts/nested")).expect("a subfolder");
            std::os::unix::fs::symlink(&bank, base.path().join("soundfonts/nested/linked.sf2")).expect("linked");
            let scan = scan_sound_fonts(base.path());
            assert!(scan.banks.is_empty(), "a link is never listed, however deep");
            assert!(scan.links_only, "and a link under a subfolder still earns the note");
        }
    }
}
