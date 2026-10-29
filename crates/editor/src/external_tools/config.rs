//! Which tool opens which file, and how a request becomes a command line.
//!
//! Resolution, in order: the file type's override if one is present — whatever it
//! says, `OsDefault` included; else the category's choice; `OsDefault` is the
//! operating system's opener. `$VISUAL` and `$EDITOR` are never read: they almost
//! always name a terminal editor, which cannot run without a terminal around it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::file_type::{FileType, ToolCategory};
use super::presets::PresetId;
use super::template::{CommandTemplate, PlaceholderValues, TemplateError};

/// The tool a category or a file type is opened with.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolChoice {
    Preset(PresetId),
    Custom(CommandTemplate),
    /// The operating system's association
    #[default]
    OsDefault,
}

/// The external tools section of the editor's preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExternalToolsConfig {
    pub code: ToolChoice,
    pub music: ToolChoice,
    pub sample: ToolChoice,
    pub art: ToolChoice,
    /// A present entry is honoured whatever it says; an absent one inherits the category
    pub overrides: BTreeMap<FileType, ToolChoice>,
    /// The extensions a music workstation's project files carry, editable
    pub daw_project_extensions: Vec<String>,
}

impl Default for ExternalToolsConfig {
    fn default() -> Self {
        Self {
            code: ToolChoice::OsDefault,
            music: ToolChoice::OsDefault,
            sample: ToolChoice::OsDefault,
            art: ToolChoice::OsDefault,
            overrides: BTreeMap::new(),
            daw_project_extensions: vec![".evil".to_string(), ".prj".to_string(), ".nsp".to_string()],
        }
    }
}

impl ExternalToolsConfig {
    /// The choice that opens `file_type`: its override if one is present, whatever it
    /// says, else its category's.
    pub fn choice_for(&self, file_type: FileType) -> &ToolChoice {
        self.overrides.get(&file_type).unwrap_or_else(|| self.category_choice(file_type.category()))
    }

    /// What the tool opening `file_type` is called, for the context menu's item and
    /// for `resolve`'s messages alike.
    pub fn tool_name(&self, file_type: FileType) -> ToolName {
        ToolName::of(self.choice_for(file_type))
    }

    /// The choice a category falls back to when its file type has no override.
    pub fn category_choice(&self, category: ToolCategory) -> &ToolChoice {
        match category {
            ToolCategory::Code => &self.code,
            ToolCategory::Music => &self.music,
            ToolCategory::Sample => &self.sample,
            ToolCategory::Art => &self.art,
        }
    }

    /// Carry a preferences file's old `ide_command` string into the code tool, unless
    /// a code tool is already chosen. The old command was split on whitespace, so each
    /// word's text is carried literally — a Windows path keeps its backslashes — while
    /// a `{file}` anywhere in it stays the placeholder; the old command had the file
    /// appended, so one naming no `{file}` gains it at the end and `code --wait` keeps
    /// opening the file.
    pub fn adopt_legacy_ide_command(&mut self, ide_command: &str) {
        let words: Vec<&str> = ide_command.split_whitespace().collect();
        if words.is_empty() || self.code != ToolChoice::OsDefault {
            return;
        }
        let mut template = words.iter().map(|word| carried_word(word)).collect::<Vec<_>>().join(" ");
        if !words.iter().any(|word| word.contains("{file}")) {
            template.push_str(" {file}");
        }
        self.code = ToolChoice::Custom(CommandTemplate(template));
    }
}

/// A legacy word in the template grammar: each `{file}` kept as the placeholder and
/// the text around it carried literally, so `{file}:12` still names the line.
fn carried_word(word: &str) -> String {
    word.split("{file}").map(literal_word).collect::<Vec<_>>().join("{file}")
}

/// `word` written so the template grammar reads it back unchanged: as it is when it
/// holds none of the grammar's characters, else single-quoted, else (it holds a `'`)
/// double-quoted with `"`, `\` and `{` escaped.
fn literal_word(word: &str) -> String {
    if !word.contains(['\\', '[', ']', '{', '}', '"', '\'']) {
        return word.to_string();
    }
    if !word.contains('\'') {
        return format!("'{word}'");
    }
    let mut quoted = String::from('"');
    for character in word.chars() {
        if matches!(character, '"' | '\\' | '{') {
            quoted.push('\\');
        }
        quoted.push(character);
    }
    quoted.push('"');
    quoted
}

/// What a tool choice is called. The one naming the context menu's "Open in …" item
/// and `resolve`'s messages share, so the two never disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolName {
    Preset(&'static str),
    CustomCommand,
    SystemDefault,
}

impl ToolName {
    pub fn of(choice: &ToolChoice) -> Self {
        match choice {
            ToolChoice::Preset(id) => ToolName::Preset(id.preset().name),
            ToolChoice::Custom(_) => ToolName::CustomCommand,
            ToolChoice::OsDefault => ToolName::SystemDefault,
        }
    }

    /// The name as a sentence's subject: "RustRover", "The custom command", "The system default".
    pub fn subject(self) -> &'static str {
        match self {
            ToolName::Preset(name) => name,
            ToolName::CustomCommand => "The custom command",
            ToolName::SystemDefault => "The system default",
        }
    }

    /// The context menu's item: "Open in RustRover", "Open with the custom command",
    /// "Open with the system default".
    pub fn open_item(self) -> String {
        match self {
            ToolName::Preset(name) => format!("Open in {name}"),
            ToolName::CustomCommand => "Open with the custom command".to_string(),
            ToolName::SystemDefault => "Open with the system default".to_string(),
        }
    }

    /// The sound emitter's button for a song's binding: "Edit in MIDI Evil", "Edit with
    /// the custom command", "Edit with the system default".
    pub fn edit_item(self) -> String {
        match self {
            ToolName::Preset(name) => format!("Edit in {name}"),
            ToolName::CustomCommand => "Edit with the custom command".to_string(),
            ToolName::SystemDefault => "Edit with the system default".to_string(),
        }
    }
}

/// A program and its arguments, ready to spawn without a shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
    /// The preset this came from, for its fallback locations and its not-found hint
    pub preset: Option<PresetId>,
}

/// Where in a file to open it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    None,
    /// The project rather than the file: a code tool's project template
    Project,
    Line { line: usize, column: Option<usize> },
    /// A track by its name and its index in the file, so the second of two `drums`
    /// is named exactly; a nameless track opens by index alone
    Track { name: Option<String>, index: usize },
}

/// Why nothing opens on the web, where no program can be launched: shown for a request
/// and beside the context menu's disabled items alike.
pub const DESKTOP_ONLY_HINT: &str = "external tools open it from the desktop editor";

/// A file to open in its external tool. Requests queue on the editor's context and
/// the host drains them once a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenRequest {
    pub path: PathBuf,
    pub location: Location,
}

/// The platform family whose default opener the launcher uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Linux and the other freedesktop systems: `xdg-open`
    Linux,
    /// Linux under the Windows Subsystem for Linux: `wslview` on the Windows path
    Wsl,
    MacOs,
    /// The shell association through `ShellExecuteW`, with no child process
    Windows,
}

/// How the operating system opens a file by its association.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OsDefault {
    /// An opener program (`xdg-open`, `open`) run like any tool
    Program(Launch),
    /// Windows' shell association for this file
    ShellAssociation(PathBuf),
    /// `wslview` on this file's Windows path, translated by `wslpath -w` first
    WslView(PathBuf),
}

/// What a request resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// A configured tool: a preset or a custom command
    Tool(Launch),
    OsDefault(OsDefault),
}

/// One request, with its paths already absolute.
#[derive(Debug, Clone, Copy)]
pub struct LaunchTarget<'a> {
    pub file_type: FileType,
    pub file: &'a Path,
    pub project: &'a Path,
    pub location: &'a Location,
}

/// Why a request names no command line.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResolveError {
    #[error("{tool} has no command for opening a project — set a code tool in Edit → External Tools…")]
    NoProjectCommand { tool: String },
    #[error("{tool}'s command cannot be used: {error}")]
    Template { tool: String, error: TemplateError },
}

/// Resolve `target` to a command line or `platform`'s default opener.
pub fn resolve(config: &ExternalToolsConfig, target: LaunchTarget, platform: Platform) -> Result<Resolution, ResolveError> {
    let choice = config.choice_for(target.file_type);
    let tool = ToolName::of(choice).subject();
    match choice {
        ToolChoice::Preset(id) => {
            let preset = id.preset();
            let template = if *target.location == Location::Project {
                preset.project_template.ok_or_else(|| ResolveError::NoProjectCommand { tool: tool.to_string() })?
            } else {
                preset.file_template
            };
            tool_launch(&CommandTemplate(template.to_string()), target, Some(*id), tool)
        }
        ToolChoice::Custom(template) => {
            if *target.location == Location::Project {
                return Err(ResolveError::NoProjectCommand { tool: tool.to_string() });
            }
            tool_launch(template, target, None, tool)
        }
        ToolChoice::OsDefault => {
            if *target.location == Location::Project {
                return Err(ResolveError::NoProjectCommand { tool: tool.to_string() });
            }
            Ok(Resolution::OsDefault(os_default(platform, target.file)))
        }
    }
}

fn tool_launch(
    template: &CommandTemplate,
    target: LaunchTarget,
    preset: Option<PresetId>,
    tool: &str,
) -> Result<Resolution, ResolveError> {
    let mut argv = template
        .expand(&placeholder_values(target))
        .map_err(|error| ResolveError::Template { tool: tool.to_string(), error })?;
    let program = argv.remove(0);
    Ok(Resolution::Tool(Launch { program, args: argv, preset }))
}

/// How `platform` opens `file` by its association; a folder opens the same way.
pub fn os_default(platform: Platform, file: &Path) -> OsDefault {
    let opener = |program: &str| {
        OsDefault::Program(Launch {
            program: program.to_string(),
            args: vec![file.to_string_lossy().into_owned()],
            preset: None,
        })
    };
    match platform {
        Platform::Linux => opener("xdg-open"),
        Platform::MacOs => opener("open"),
        Platform::Windows => OsDefault::ShellAssociation(file.to_path_buf()),
        Platform::Wsl => OsDefault::WslView(file.to_path_buf()),
    }
}

fn placeholder_values(target: LaunchTarget) -> PlaceholderValues {
    let mut values = PlaceholderValues {
        file: target.file.to_string_lossy().into_owned(),
        project: Some(target.project.to_string_lossy().into_owned()),
        ..Default::default()
    };
    match target.location {
        Location::None | Location::Project => {}
        Location::Line { line, column } => {
            values.line = Some(*line);
            values.column = *column;
        }
        Location::Track { name, index } => {
            values.track = name.clone();
            values.track_index = Some(*index);
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "/game/assets/scripts/paddle.rhai";
    const PROJECT: &str = "/game";

    fn resolve_at(config: &ExternalToolsConfig, file_type: FileType, location: &Location, platform: Platform) -> Result<Resolution, ResolveError> {
        let target = LaunchTarget { file_type, file: Path::new(FILE), project: Path::new(PROJECT), location };
        resolve(config, target, platform)
    }

    fn tool(program: &str, args: &[&str], preset: Option<PresetId>) -> Resolution {
        Resolution::Tool(Launch {
            program: program.to_string(),
            args: args.iter().map(|argument| argument.to_string()).collect(),
            preset,
        })
    }

    fn xdg_open() -> Resolution {
        Resolution::OsDefault(OsDefault::Program(Launch {
            program: "xdg-open".to_string(),
            args: vec![FILE.to_string()],
            preset: None,
        }))
    }

    #[test]
    fn test_an_absent_override_inherits_the_category_and_a_present_one_wins_even_as_os_default() {
        let mut config = ExternalToolsConfig { music: ToolChoice::Preset(PresetId::MidiEvil), ..Default::default() };
        let midi = resolve_at(&config, FileType::Midi, &Location::None, Platform::Linux);
        assert_eq!(midi, Ok(tool("midi-evil", &["--project", PROJECT, FILE], Some(PresetId::MidiEvil))));

        config.overrides.insert(FileType::SoundFont, ToolChoice::OsDefault);
        assert_eq!(
            resolve_at(&config, FileType::SoundFont, &Location::None, Platform::Linux),
            Ok(xdg_open()),
            "an explicit OsDefault override is honoured over the Music category's tool"
        );
        assert_eq!(
            resolve_at(&config, FileType::DawProject, &Location::None, Platform::Linux).map(|resolution| matches!(resolution, Resolution::Tool(_))),
            Ok(true),
            "a type without an override still inherits the category"
        );
    }

    #[test]
    fn test_code_left_at_the_system_default_opens_through_the_platform_opener_at_no_line() {
        let at_line = Location::Line { line: 12, column: None };
        assert_eq!(
            resolve_at(&ExternalToolsConfig::default(), FileType::RhaiScript, &at_line, Platform::Linux),
            Ok(xdg_open()),
            "no terminal editor from the environment stands in for the system default"
        );
    }

    #[test]
    fn test_each_platforms_default_is_its_own_opener() {
        let config = ExternalToolsConfig::default();
        let on = |platform| resolve_at(&config, FileType::Image, &Location::None, platform);
        assert_eq!(on(Platform::Linux), Ok(xdg_open()));
        assert_eq!(
            on(Platform::MacOs),
            Ok(Resolution::OsDefault(OsDefault::Program(Launch { program: "open".to_string(), args: vec![FILE.to_string()], preset: None })))
        );
        assert_eq!(on(Platform::Windows), Ok(Resolution::OsDefault(OsDefault::ShellAssociation(PathBuf::from(FILE)))));
        assert_eq!(on(Platform::Wsl), Ok(Resolution::OsDefault(OsDefault::WslView(PathBuf::from(FILE)))));
    }

    #[test]
    fn test_a_project_opens_through_the_presets_project_template_and_nothing_else() {
        let mut config = ExternalToolsConfig { code: ToolChoice::Preset(PresetId::RustRover), ..Default::default() };
        assert_eq!(
            resolve_at(&config, FileType::RustSource, &Location::Project, Platform::Linux),
            Ok(tool("rustrover", &[PROJECT], Some(PresetId::RustRover)))
        );
        config.code = ToolChoice::Custom(CommandTemplate("edit {file}".to_string()));
        assert!(matches!(
            resolve_at(&config, FileType::RustSource, &Location::Project, Platform::Linux),
            Err(ResolveError::NoProjectCommand { .. })
        ));
        config.code = ToolChoice::OsDefault;
        assert!(matches!(
            resolve_at(&config, FileType::RustSource, &Location::Project, Platform::Linux),
            Err(ResolveError::NoProjectCommand { .. })
        ));
    }

    #[test]
    fn test_a_custom_command_with_an_unknown_placeholder_is_refused_naming_it() {
        let config = ExternalToolsConfig { code: ToolChoice::Custom(CommandTemplate("edit {path}".to_string())), ..Default::default() };
        assert_eq!(
            resolve_at(&config, FileType::RhaiScript, &Location::None, Platform::Linux),
            Err(ResolveError::Template {
                tool: "The custom command".to_string(),
                error: TemplateError::UnknownPlaceholder("path".to_string())
            })
        );
    }

    #[test]
    fn test_a_track_location_fills_the_track_placeholders() {
        let config = ExternalToolsConfig { music: ToolChoice::Preset(PresetId::MidiEvil), ..Default::default() };
        let track = Location::Track { name: Some("drums".to_string()), index: 2 };
        assert_eq!(
            resolve_at(&config, FileType::Midi, &track, Platform::Linux),
            Ok(tool("midi-evil", &["--project", PROJECT, "--track", "drums", "--track-index", "2", FILE], Some(PresetId::MidiEvil)))
        );
    }

    #[test]
    fn test_the_open_item_names_what_the_file_resolves_to_as_resolves_messages_do() {
        let mut config = ExternalToolsConfig {
            code: ToolChoice::Preset(PresetId::RustRover),
            music: ToolChoice::Custom(CommandTemplate("player {file}".to_string())),
            ..Default::default()
        };
        config.overrides.insert(FileType::Image, ToolChoice::Preset(PresetId::Aseprite));
        let items: Vec<String> = [FileType::RhaiScript, FileType::Image, FileType::Midi, FileType::AudioSample]
            .into_iter()
            .map(|file_type| config.tool_name(file_type).open_item())
            .collect();
        assert_eq!(
            items,
            ["Open in RustRover", "Open in Aseprite", "Open with the custom command", "Open with the system default"],
            "a preset, an override, Custom and OsDefault"
        );
        config.music = ToolChoice::Preset(PresetId::MidiEvil);
        assert_eq!(config.tool_name(FileType::Midi).edit_item(), "Edit in MIDI Evil", "the emitter's button, by the same name");

        let refusal = |choice: ToolChoice| {
            let config = ExternalToolsConfig { code: choice.clone(), ..Default::default() };
            let Err(error) = resolve_at(&config, FileType::RustSource, &Location::Project, Platform::Linux) else {
                panic!("{choice:?} opens no project here");
            };
            (error.to_string(), ToolName::of(&choice).subject())
        };
        for choice in [ToolChoice::Custom(CommandTemplate("edit {file}".to_string())), ToolChoice::OsDefault] {
            let (message, subject) = refusal(choice);
            assert!(message.starts_with(subject), "the error names the tool as the menu does: {message}");
        }
    }

    #[test]
    fn test_a_legacy_ide_command_becomes_the_custom_code_tool_with_the_file_appended() {
        let mut config = ExternalToolsConfig::default();
        config.adopt_legacy_ide_command("code --wait");
        assert_eq!(config.code, ToolChoice::Custom(CommandTemplate("code --wait {file}".to_string())));

        let mut placed = ExternalToolsConfig::default();
        placed.adopt_legacy_ide_command(" subl {file} --new-window ");
        assert_eq!(placed.code, ToolChoice::Custom(CommandTemplate("subl {file} --new-window".to_string())));

        let mut windows = ExternalToolsConfig::default();
        windows.adopt_legacy_ide_command(r"C:\tools\subl.exe -w");
        let ToolChoice::Custom(template) = &windows.code else {
            panic!("a legacy command becomes the custom code tool, not {:?}", windows.code);
        };
        let values = PlaceholderValues { file: "a.rhai".to_string(), ..Default::default() };
        assert_eq!(
            template.expand(&values),
            Ok(vec![r"C:\tools\subl.exe".to_string(), "-w".to_string(), "a.rhai".to_string()]),
            "the old split kept every backslash, so the carried command does too"
        );

        let embedded = |command: &str| {
            let mut config = ExternalToolsConfig::default();
            config.adopt_legacy_ide_command(command);
            match &config.code {
                ToolChoice::Custom(template) => template.expand(&values),
                other => panic!("a legacy command becomes the custom code tool, not {other:?}"),
            }
        };
        assert_eq!(
            embedded("code --goto {file}:12"),
            Ok(vec!["code".to_string(), "--goto".to_string(), "a.rhai:12".to_string()]),
            "a placeholder inside a word is still the placeholder, and no second file is appended"
        );
        assert_eq!(embedded("tool --file={file}"), Ok(vec!["tool".to_string(), "--file=a.rhai".to_string()]));
        assert_eq!(
            embedded(r#"tool "{file}""#),
            Ok(vec!["tool".to_string(), r#""a.rhai""#.to_string()]),
            "the old split passed the quote characters through, and so does the carried word"
        );

        let mut blank = ExternalToolsConfig::default();
        blank.adopt_legacy_ide_command("   \t ");
        assert_eq!(blank.code, ToolChoice::OsDefault, "a blank command chooses nothing");

        let mut chosen = ExternalToolsConfig { code: ToolChoice::Preset(PresetId::Zed), ..Default::default() };
        chosen.adopt_legacy_ide_command("code --wait");
        assert_eq!(chosen.code, ToolChoice::Preset(PresetId::Zed), "a chosen code tool is never replaced by the old string");
    }

    #[test]
    fn test_the_config_round_trips_through_json_and_a_partial_one_fills_defaults() -> Result<(), serde_json::Error> {
        let mut config = ExternalToolsConfig { art: ToolChoice::Preset(PresetId::Aseprite), ..Default::default() };
        config.overrides.insert(FileType::SoundFont, ToolChoice::OsDefault);
        config.overrides.insert(FileType::Ron, ToolChoice::Custom(CommandTemplate("hx {file}".to_string())));
        let json = serde_json::to_string(&config)?;
        assert_eq!(serde_json::from_str::<ExternalToolsConfig>(&json)?, config);

        let partial: ExternalToolsConfig = serde_json::from_str(r#"{"code": {"Preset": "Zed"}}"#)?;
        assert_eq!(partial.code, ToolChoice::Preset(PresetId::Zed));
        assert_eq!(partial.daw_project_extensions, ExternalToolsConfig::default().daw_project_extensions);
        Ok(())
    }
}
