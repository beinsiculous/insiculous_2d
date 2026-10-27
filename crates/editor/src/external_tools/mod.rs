//! External tools: which program opens which project file, as data and pure
//! resolution. The host (`editor_integration`) finds the program, spawns it and
//! reports; nothing here touches a process.

mod config;
mod file_type;
mod presets;
mod template;

pub use config::{
    os_default, resolve, ExternalToolsConfig, Launch, LaunchTarget, Location, OpenRequest, OsDefault, Platform,
    Resolution, ResolveError, ToolChoice, ToolName, DESKTOP_ONLY_HINT,
};
pub use file_type::{
    is_daw_project_path, is_midi_path, midi_sidecar_path, FileType, ProjectBase, ReloadPolicy, ToolCategory,
};
pub use presets::{Preset, PresetId, PRESETS};
pub use template::{CommandTemplate, PlaceholderValues, TemplateError};
