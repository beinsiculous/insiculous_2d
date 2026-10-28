//! External tools: which program opens which project file, as data and pure
//! resolution, and which watched file changed on disk. The host (`editor_integration`)
//! finds the program, spawns it and reports, and polls the watch on its own thread;
//! nothing here touches a process or a thread.

mod config;
mod file_type;
mod presets;
mod template;
mod watch;

pub use config::{
    os_default, resolve, ExternalToolsConfig, Launch, LaunchTarget, Location, OpenRequest, OsDefault, Platform,
    Resolution, ResolveError, ToolChoice, ToolName, DESKTOP_ONLY_HINT,
};
pub use file_type::{
    is_daw_project_path, is_midi_path, midi_sidecar_path, FileType, ProjectBase, ReloadPolicy, ToolCategory,
};
pub use presets::{Preset, PresetId, PRESETS};
pub use template::{CommandTemplate, PlaceholderValues, TemplateError};
pub use watch::{FileChange, FileWatch, Observed, WatchEvent};
