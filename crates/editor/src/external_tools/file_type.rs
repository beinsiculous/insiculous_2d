//! The one table of what a file is: its type, the tool category that opens it,
//! the folder `{project}` names for it, and what the engine does when it changes.
//!
//! Users pick tools; the engine knows what a file means — so the reload policy is
//! data here, never a setting.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// What a file is, as far as opening it in another program goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FileType {
    /// `.rs`
    RustSource,
    /// `.rhai`
    RhaiScript,
    /// `.ron`: a scene, a `.sheet.ron` or a song's `.mid.ron` sidecar
    Ron,
    /// `.mid` / `.midi`
    Midi,
    /// `.wav`, `.ogg`, `.flac`, `.mp3`
    AudioSample,
    /// A music workstation's own project file, by the configured extension list
    DawProject,
    /// `.sf2`
    SoundFont,
    /// `.png`, `.jpg`, `.jpeg`, `.bmp`, `.gif`
    Image,
}

/// Which configured tool opens a file type unless an override says otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    Code,
    Music,
    Sample,
    Art,
}

/// The folder a template's `{project}` expands to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectBase {
    /// The nearest Cargo workspace above the asset base, so a code tool opens the crate
    CargoWorkspace,
    /// The game's asset folder
    AssetBase,
}

/// What the editor does when a file of this type changes on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadPolicy {
    /// Nothing: the author knows a Rust edit needs a rebuild, and a DAW project is not an asset
    None,
    RecompileScript,
    ReloadDocument,
    ReloadSheet,
    ReloadSong,
    ReloadSample,
}

impl FileType {
    /// The type of `path` by its extension (case-insensitive), or `None` for a file no
    /// tool opens. `daw_project_extensions` is the configured list, with or without dots.
    pub fn of(path: &Path, daw_project_extensions: &[String]) -> Option<FileType> {
        if is_midi_path(path) {
            return Some(FileType::Midi);
        }
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        let file_type = match extension.as_str() {
            "rs" => FileType::RustSource,
            "rhai" => FileType::RhaiScript,
            "ron" => FileType::Ron,
            "wav" | "ogg" | "flac" | "mp3" => FileType::AudioSample,
            "sf2" => FileType::SoundFont,
            "png" | "jpg" | "jpeg" | "bmp" | "gif" => FileType::Image,
            _ if is_daw_project_path(path, daw_project_extensions) => FileType::DawProject,
            _ => return None,
        };
        Some(file_type)
    }

    /// The category whose configured tool opens this type.
    pub fn category(self) -> ToolCategory {
        match self {
            FileType::RustSource | FileType::RhaiScript | FileType::Ron => ToolCategory::Code,
            FileType::Midi | FileType::DawProject | FileType::SoundFont => ToolCategory::Music,
            FileType::AudioSample => ToolCategory::Sample,
            FileType::Image => ToolCategory::Art,
        }
    }

    /// The folder `{project}` names for this type.
    pub fn project_base(self) -> ProjectBase {
        match self.category() {
            ToolCategory::Code => ProjectBase::CargoWorkspace,
            ToolCategory::Music | ToolCategory::Sample | ToolCategory::Art => ProjectBase::AssetBase,
        }
    }

    /// What a change to `path`, a file of this type, asks the editor to do. Only a
    /// `.ron` consults the path: a sidecar's suffix says whose sidecar it is.
    pub fn reload_policy(self, path: &Path) -> ReloadPolicy {
        match self {
            FileType::RustSource | FileType::DawProject | FileType::Image => ReloadPolicy::None,
            FileType::RhaiScript => ReloadPolicy::RecompileScript,
            FileType::Ron => ron_reload_policy(path),
            FileType::Midi | FileType::SoundFont => ReloadPolicy::ReloadSong,
            FileType::AudioSample => ReloadPolicy::ReloadSample,
        }
    }
}

fn ron_reload_policy(path: &Path) -> ReloadPolicy {
    let described = path.with_extension("");
    if is_midi_path(&described) {
        return ReloadPolicy::ReloadSong;
    }
    let is_sheet = described
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("sheet"));
    if is_sheet {
        ReloadPolicy::ReloadSheet
    } else {
        ReloadPolicy::ReloadDocument
    }
}

/// Whether `path` is a Standard MIDI File by its extension: `.mid` or `.midi`, any case.
/// The one place that rule lives.
pub fn is_midi_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("mid") || extension.eq_ignore_ascii_case("midi"))
}

/// The song sidecar beside a MIDI file: `<name>.<ext>.ron`, the MIDI file's whole name
/// with `.ron` added, so `theme.midi` pairs with `theme.midi.ron`.
pub fn midi_sidecar_path(midi_path: &Path) -> PathBuf {
    let mut sidecar = midi_path.as_os_str().to_owned();
    sidecar.push(".ron");
    PathBuf::from(sidecar)
}

/// Whether `path`'s extension is on the configured DAW project list (entries with or
/// without a leading dot, any case).
pub fn is_daw_project_path(path: &Path, daw_project_extensions: &[String]) -> bool {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };
    daw_project_extensions
        .iter()
        .any(|listed| listed.trim_start_matches('.').eq_ignore_ascii_case(extension))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_daw_extensions() -> Vec<String> {
        vec![".evil".to_string(), ".prj".to_string(), ".nsp".to_string()]
    }

    #[test]
    fn test_every_listed_extension_maps_to_its_row_and_unknown_files_to_none() {
        let daw = default_daw_extensions();
        let rows = [
            ("src/main.rs", FileType::RustSource, ToolCategory::Code),
            ("scripts/paddle.rhai", FileType::RhaiScript, ToolCategory::Code),
            ("scenes/level.ron", FileType::Ron, ToolCategory::Code),
            ("music/theme.mid", FileType::Midi, ToolCategory::Music),
            ("music/theme.MIDI", FileType::Midi, ToolCategory::Music),
            ("sounds/jump.wav", FileType::AudioSample, ToolCategory::Sample),
            ("sounds/jump.ogg", FileType::AudioSample, ToolCategory::Sample),
            ("sounds/jump.FLAC", FileType::AudioSample, ToolCategory::Sample),
            ("sounds/jump.mp3", FileType::AudioSample, ToolCategory::Sample),
            ("music/theme.evil", FileType::DawProject, ToolCategory::Music),
            ("music/theme.PRJ", FileType::DawProject, ToolCategory::Music),
            ("music/theme.nsp", FileType::DawProject, ToolCategory::Music),
            ("soundfonts/cast.sf2", FileType::SoundFont, ToolCategory::Music),
            ("art/ship.png", FileType::Image, ToolCategory::Art),
            ("art/ship.jpeg", FileType::Image, ToolCategory::Art),
            ("art/ship.gif", FileType::Image, ToolCategory::Art),
        ];
        for (path, file_type, category) in rows {
            assert_eq!(FileType::of(Path::new(path), &daw), Some(file_type), "{path}");
            assert_eq!(file_type.category(), category, "{path}");
        }
        assert_eq!(FileType::of(Path::new("notes.txt"), &daw), None);
        assert_eq!(FileType::of(Path::new("Makefile"), &daw), None);
        assert_eq!(
            FileType::of(Path::new("music/theme.evil"), &[]),
            None,
            "a DAW project is known only by the configured list"
        );
    }

    #[test]
    fn test_code_types_name_the_cargo_workspace_and_assets_name_the_asset_base() {
        assert_eq!(FileType::RhaiScript.project_base(), ProjectBase::CargoWorkspace);
        assert_eq!(FileType::Ron.project_base(), ProjectBase::CargoWorkspace);
        assert_eq!(FileType::Midi.project_base(), ProjectBase::AssetBase);
        assert_eq!(FileType::Image.project_base(), ProjectBase::AssetBase);
    }

    #[test]
    fn test_a_ron_file_reloads_by_whose_sidecar_it_is() {
        let ron = FileType::Ron;
        assert_eq!(ron.reload_policy(Path::new("scenes/level.ron")), ReloadPolicy::ReloadDocument);
        assert_eq!(ron.reload_policy(Path::new("art/ship.sheet.ron")), ReloadPolicy::ReloadSheet);
        assert_eq!(ron.reload_policy(Path::new("music/theme.mid.ron")), ReloadPolicy::ReloadSong);
        assert_eq!(ron.reload_policy(Path::new("music/theme.MIDI.ron")), ReloadPolicy::ReloadSong);
    }

    #[test]
    fn test_rust_source_and_images_reload_nothing() {
        assert_eq!(FileType::RustSource.reload_policy(Path::new("src/main.rs")), ReloadPolicy::None);
        assert_eq!(FileType::Image.reload_policy(Path::new("art/ship.png")), ReloadPolicy::None);
        assert_eq!(FileType::DawProject.reload_policy(Path::new("music/theme.evil")), ReloadPolicy::None);
        assert_eq!(FileType::SoundFont.reload_policy(Path::new("cast.sf2")), ReloadPolicy::ReloadSong);
    }

    #[test]
    fn test_midi_helper_accepts_both_extensions_in_any_case_and_names_the_whole_name_sidecar() {
        assert!(is_midi_path(Path::new("theme.mid")));
        assert!(is_midi_path(Path::new("theme.Midi")));
        assert!(!is_midi_path(Path::new("theme.mid.ron")));
        assert!(!is_midi_path(Path::new("mid")));
        assert_eq!(midi_sidecar_path(Path::new("music/theme.midi")), PathBuf::from("music/theme.midi.ron"));
        assert_eq!(midi_sidecar_path(Path::new("music/theme.mid")), PathBuf::from("music/theme.mid.ron"));
    }
}
