//! The tools the editor knows by name, as data. Each row's doc comment names the
//! vendor page its command line was taken from, so the next verification is a diff
//! against that page.

use serde::{Deserialize, Serialize};

use super::file_type::ToolCategory;

/// A tool the editor ships a command line for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PresetId {
    RustRover,
    VsCode,
    VsCodium,
    Zed,
    Helix,
    MidiEvil,
    Aseprite,
    Audacity,
}

/// One row of the preset table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
    pub id: PresetId,
    /// The name the editor shows ("Open in RustRover")
    pub name: &'static str,
    pub category: ToolCategory,
    /// How a file is opened, in the template grammar
    pub file_template: &'static str,
    /// How the project is opened (Edit → Open project); Code tools only
    pub project_template: Option<&'static str>,
    /// Where to look after `PATH`, `~/` meaning the home directory
    pub fallback_locations: &'static [&'static str],
    /// The Flatpak application id, named when the binary is not found
    pub flatpak_id: Option<&'static str>,
    /// A terminal program: it cannot run without a terminal around it, and the
    /// editor never guesses one
    pub needs_terminal: bool,
}

/// RustRover — jetbrains.com/help/rust/opening-files-from-command-line.html:
/// `rustrover [--line <number>] [--column <number>] <path>`. The launcher is the
/// Toolbox script; none is installed by default. Project-plus-file in one call is
/// not documented, so the project opens by a second launch.
const RUSTROVER: Preset = Preset {
    id: PresetId::RustRover,
    name: "RustRover",
    category: ToolCategory::Code,
    file_template: "rustrover [--line {line}] [--column {column}] {file}",
    project_template: Some("rustrover {project}"),
    fallback_locations: &[],
    flatpak_id: None,
    needs_terminal: false,
};

/// VS Code — code.visualstudio.com/docs/configure/command-line: `--goto
/// file:line{:character}`; a folder beside `--goto` is two documented arguments.
/// macOS needs the Shell Command installed for `code` to exist.
const VS_CODE: Preset = Preset {
    id: PresetId::VsCode,
    name: "VS Code",
    category: ToolCategory::Code,
    file_template: "code {project} --goto {file}[:{line}[:{column}]]",
    project_template: Some("code {project}"),
    fallback_locations: &[],
    flatpak_id: None,
    needs_terminal: false,
};

/// VSCodium — github.com/VSCodium/vscodium/blob/master/docs/usage.md; the flags
/// are VS Code's, which that page does not state.
const VS_CODIUM: Preset = Preset {
    id: PresetId::VsCodium,
    name: "VSCodium",
    category: ToolCategory::Code,
    file_template: "codium {project} --goto {file}[:{line}[:{column}]]",
    project_template: Some("codium {project}"),
    fallback_locations: &[],
    flatpak_id: None,
    needs_terminal: false,
};

/// Zed — zed.dev/docs/reference/cli.md: `zed myfile.txt:42:10`. On Linux the
/// binary is commonly `zed` or `zeditor`.
const ZED: Preset = Preset {
    id: PresetId::Zed,
    name: "Zed",
    category: ToolCategory::Code,
    file_template: "zed {project} {file}[:{line}[:{column}]]",
    project_template: Some("zed {project}"),
    fallback_locations: &[],
    flatpak_id: None,
    needs_terminal: false,
};

/// Helix — helix-term's help text: `file[:row[:col]]`, `-w, --working-dir`.
const HELIX: Preset = Preset {
    id: PresetId::Helix,
    name: "Helix",
    category: ToolCategory::Code,
    file_template: "hx [-w {project}] {file}[:{line}[:{column}]]",
    project_template: Some("hx -w {project}"),
    fallback_locations: &[],
    flatpak_id: None,
    needs_terminal: true,
};

/// MIDI Evil — the crossing's contract this engine defines for the desktop crate's
/// binary; until MIDI Evil reads these flags, a Custom command with `{file}` alone works.
const MIDI_EVIL: Preset = Preset {
    id: PresetId::MidiEvil,
    name: "MIDI Evil",
    category: ToolCategory::Music,
    file_template: "midi-evil [--project {project}] [--track {track}] [--track-index {track_index}] {file}",
    project_template: None,
    fallback_locations: &[],
    flatpak_id: None,
    needs_terminal: false,
};

/// Aseprite — aseprite.org/docs/cli: a file argument opens it in the GUI;
/// `--frame-range` and `--tag` act only in batch mode, so there is no location
/// placeholder. The Steam install is where `deion_assets`' export script finds it.
const ASEPRITE: Preset = Preset {
    id: PresetId::Aseprite,
    name: "Aseprite",
    category: ToolCategory::Art,
    file_template: "aseprite {file}",
    project_template: None,
    fallback_locations: &["~/.steam/root/steamapps/common/Aseprite/aseprite"],
    flatpak_id: Some("org.aseprite.Aseprite"),
    needs_terminal: false,
};

/// Audacity — `AudacityApp::ParseCommandLine` (3.7.8): a file-name parameter and no
/// time-position option.
const AUDACITY: Preset = Preset {
    id: PresetId::Audacity,
    name: "Audacity",
    category: ToolCategory::Sample,
    file_template: "audacity {file}",
    project_template: None,
    fallback_locations: &[],
    flatpak_id: Some("org.audacityteam.Audacity"),
    needs_terminal: false,
};

/// Every preset, in the order a settings panel lists them.
pub const PRESETS: [Preset; 8] = [RUSTROVER, VS_CODE, VS_CODIUM, ZED, HELIX, MIDI_EVIL, ASEPRITE, AUDACITY];

impl PresetId {
    /// This preset's row.
    pub fn preset(self) -> &'static Preset {
        match self {
            PresetId::RustRover => &PRESETS[0],
            PresetId::VsCode => &PRESETS[1],
            PresetId::VsCodium => &PRESETS[2],
            PresetId::Zed => &PRESETS[3],
            PresetId::Helix => &PRESETS[4],
            PresetId::MidiEvil => &PRESETS[5],
            PresetId::Aseprite => &PRESETS[6],
            PresetId::Audacity => &PRESETS[7],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::template::{CommandTemplate, PlaceholderValues};
    use super::*;

    fn expand(template: &str, values: &PlaceholderValues) -> Vec<String> {
        CommandTemplate(template.to_string()).expand(values).expect("every preset template expands")
    }

    fn strings(argv: &[&str]) -> Vec<String> {
        argv.iter().map(|argument| argument.to_string()).collect()
    }

    fn at_line_and_column() -> PlaceholderValues {
        PlaceholderValues {
            file: "/game/src/main.rs".to_string(),
            project: Some("/game".to_string()),
            line: Some(42),
            column: Some(10),
            ..Default::default()
        }
    }

    #[test]
    fn test_every_preset_finds_its_own_row() {
        for preset in &PRESETS {
            assert_eq!(preset.id.preset(), preset, "{}", preset.name);
            assert!(CommandTemplate(preset.file_template.to_string()).names_file(), "{} opens the file", preset.name);
        }
    }

    #[test]
    fn test_rustrover_opens_at_line_and_column_and_the_project_alone() {
        let preset = PresetId::RustRover.preset();
        assert_eq!(
            expand(preset.file_template, &at_line_and_column()),
            strings(&["rustrover", "--line", "42", "--column", "10", "/game/src/main.rs"])
        );
        assert_eq!(expand(preset.project_template.unwrap_or(""), &at_line_and_column()), strings(&["rustrover", "/game"]));
    }

    #[test]
    fn test_vs_code_and_vscodium_open_the_folder_and_go_to_the_line() {
        for (id, program) in [(PresetId::VsCode, "code"), (PresetId::VsCodium, "codium")] {
            let preset = id.preset();
            assert_eq!(
                expand(preset.file_template, &at_line_and_column()),
                strings(&[program, "/game", "--goto", "/game/src/main.rs:42:10"])
            );
            assert_eq!(expand(preset.project_template.unwrap_or(""), &at_line_and_column()), strings(&[program, "/game"]));
        }
    }

    #[test]
    fn test_zed_opens_the_folder_and_the_file_at_its_position() {
        let preset = PresetId::Zed.preset();
        assert_eq!(
            expand(preset.file_template, &at_line_and_column()),
            strings(&["zed", "/game", "/game/src/main.rs:42:10"])
        );
        assert_eq!(expand(preset.project_template.unwrap_or(""), &at_line_and_column()), strings(&["zed", "/game"]));
    }

    #[test]
    fn test_helix_sets_its_working_directory_and_is_marked_a_terminal_editor() {
        let preset = PresetId::Helix.preset();
        assert_eq!(
            expand(preset.file_template, &at_line_and_column()),
            strings(&["hx", "-w", "/game", "/game/src/main.rs:42:10"])
        );
        assert_eq!(expand(preset.project_template.unwrap_or(""), &at_line_and_column()), strings(&["hx", "-w", "/game"]));
        assert!(preset.needs_terminal);
    }

    #[test]
    fn test_midi_evil_names_the_track_by_name_and_index() {
        let values = PlaceholderValues {
            file: "/game/assets/music/theme.mid".to_string(),
            project: Some("/game/assets".to_string()),
            track: Some("drums".to_string()),
            track_index: Some(4),
            ..Default::default()
        };
        assert_eq!(
            expand(PresetId::MidiEvil.preset().file_template, &values),
            strings(&[
                "midi-evil",
                "--project",
                "/game/assets",
                "--track",
                "drums",
                "--track-index",
                "4",
                "/game/assets/music/theme.mid"
            ])
        );
        let nameless = PlaceholderValues { track: None, ..values };
        assert_eq!(
            expand(PresetId::MidiEvil.preset().file_template, &nameless),
            strings(&["midi-evil", "--project", "/game/assets", "--track-index", "4", "/game/assets/music/theme.mid"]),
            "a nameless track opens by index alone"
        );
    }

    #[test]
    fn test_aseprite_and_audacity_open_the_file_alone_and_name_their_flatpaks() {
        let art = PlaceholderValues { file: "/game/assets/art/ship.png".to_string(), ..at_line_and_column() };
        assert_eq!(expand(PresetId::Aseprite.preset().file_template, &art), strings(&["aseprite", "/game/assets/art/ship.png"]));
        assert_eq!(
            PresetId::Aseprite.preset().fallback_locations,
            &["~/.steam/root/steamapps/common/Aseprite/aseprite"]
        );
        assert_eq!(PresetId::Aseprite.preset().flatpak_id, Some("org.aseprite.Aseprite"));
        let sample = PlaceholderValues { file: "/game/assets/sounds/jump.wav".to_string(), ..at_line_and_column() };
        assert_eq!(expand(PresetId::Audacity.preset().file_template, &sample), strings(&["audacity", "/game/assets/sounds/jump.wav"]));
        assert_eq!(PresetId::Audacity.preset().flatpak_id, Some("org.audacityteam.Audacity"));
    }

    #[test]
    fn test_only_code_presets_carry_a_project_template() {
        for preset in &PRESETS {
            assert_eq!(
                preset.project_template.is_some(),
                preset.category == ToolCategory::Code,
                "{}",
                preset.name
            );
        }
    }
}
