//! Which tool opens which file, and how a request becomes a command line.
//!
//! Resolution, in order: the file type's override if one is present — whatever it
//! says, `OsDefault` included; else the category's choice; `OsDefault` is the
//! operating system's opener. `$VISUAL` and `$EDITOR` are never read: they almost
//! always name a terminal editor, which cannot run without a terminal around it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::custom_tool::CustomTool;
use super::file_type::{FileType, ToolCategory};
use super::presets::PresetId;
use super::template::{CommandTemplate, PlaceholderValues, TemplateError};

/// The tool a category or a file type is opened with.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolChoice {
    Preset(PresetId),
    Custom(CustomTool),
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
    /// A preset's program in place of its own: a path, or another name looked up as its
    /// own is; an absent or blank entry keeps the preset's
    pub programs: BTreeMap<PresetId, String>,
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
            programs: BTreeMap::new(),
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
        self.code = ToolChoice::Custom(CustomTool::file_only(CommandTemplate(template)));
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
    /// The preset this came from, for its fallback locations, its not-found hint and how
    /// an early exit is worded
    pub preset: Option<PresetId>,
    /// The program was named in the settings in the preset's place, so the preset's
    /// fallback locations — its own binary's — do not apply to it
    pub named_program: bool,
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

/// Resolve `target` to a command line or `platform`'s default opener: a file through its
/// own override, else its category's choice; a project request through the category's
/// choice alone, since a project is no file and an override says how a file opens.
pub fn resolve(config: &ExternalToolsConfig, target: LaunchTarget, platform: Platform) -> Result<Resolution, ResolveError> {
    let choice = if *target.location == Location::Project {
        config.category_choice(target.file_type.category())
    } else {
        config.choice_for(target.file_type)
    };
    resolve_with(config, choice, target, platform)
}

/// The extensions Windows runs a bare name under, which the preset's own name may carry.
const EXECUTABLE_EXTENSIONS: [&str; 4] = ["exe", "com", "bat", "cmd"];

/// Resolve `target` through `choice`, whatever the config would pick for the file: a
/// category's Test tries the tool its row names. `config` still supplies a preset's program.
pub fn resolve_with(
    config: &ExternalToolsConfig,
    choice: &ToolChoice,
    target: LaunchTarget,
    platform: Platform,
) -> Result<Resolution, ResolveError> {
    let tool = ToolName::of(choice).subject();
    match choice {
        ToolChoice::Preset(id) => {
            let preset = id.preset();
            let template = if *target.location == Location::Project {
                preset.project_template.ok_or_else(|| ResolveError::NoProjectCommand { tool: tool.to_string() })?
            } else {
                preset.file_template
            };
            let mut resolution = tool_launch(&CommandTemplate(template.to_string()), target, Some(*id), tool)?;
            let program = config.programs.get(id).map(|program| program.trim()).filter(|program| !program.is_empty());
            if let (Resolution::Tool(launch), Some(program)) = (&mut resolution, program) {
                // The preset's fallback locations hold its own binary, so another program
                // named in its place either runs or is reported not found — never
                // replaced; the preset stays for its not-found hint and its exit wording.
                // Its own bare name typed there is its own program, fallbacks and all —
                // with or without Windows' executable extension, in any case; any other
                // suffix is another name, and a path is the person's file whatever it is
                // called.
                let own = preset.file_template.split_whitespace().next().unwrap_or_default();
                let bare = Path::new(program).components().count() == 1;
                let without_extension = program
                    .rsplit_once('.')
                    .filter(|(_, extension)| EXECUTABLE_EXTENSIONS.iter().any(|known| known.eq_ignore_ascii_case(extension)))
                    .map_or(program, |(name, _)| name);
                launch.named_program = !(bare && without_extension.eq_ignore_ascii_case(own));
                launch.program = program.to_string();
            }
            Ok(resolution)
        }
        ToolChoice::Custom(custom) => {
            let template = if *target.location == Location::Project {
                custom.project.as_ref().ok_or_else(|| ResolveError::NoProjectCommand { tool: tool.to_string() })?
            } else {
                &custom.file
            };
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
    Ok(Resolution::Tool(Launch { program, args: argv, preset, named_program: false }))
}

/// How `platform` opens `file` by its association; a folder opens the same way.
pub fn os_default(platform: Platform, file: &Path) -> OsDefault {
    let opener = |program: &str| {
        OsDefault::Program(Launch {
            program: program.to_string(),
            args: vec![file.to_string_lossy().into_owned()],
            preset: None,
            named_program: false,
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
#[path = "config_tests.rs"]
mod config_tests;
