//! External tools: which program opens which project file, as data and pure
//! resolution, which watched file changed on disk, and the project's songs and banks —
//! where in a song a part or a binding lives, and what a new score does to the parts. The host (`editor_integration`) finds the
//! program, spawns it and reports, and polls the watch on its own thread; nothing here
//! touches a process or a thread.

mod config;
mod custom_tool;
mod file_type;
mod presets;
mod songs;
mod sound_fonts;
mod template;
mod track_diff;
mod watch;

pub use config::{
    os_default, resolve, resolve_with, ExternalToolsConfig, Launch, LaunchTarget, Location, OpenRequest, OsDefault, Platform,
    Resolution, ResolveError, ToolChoice, ToolName, DESKTOP_ONLY_HINT,
};
pub use custom_tool::CustomTool;
pub use file_type::{
    is_daw_project_path, is_midi_path, midi_sidecar_path, sidecar_midi_path, FileType, ProjectBase, ReloadPolicy,
    ToolCategory,
};
pub use presets::{Preset, PresetId, PRESETS};
pub use songs::{
    binding_location, first_part_location, part_location, read_song_pair, scan_songs, BindingLookupError, SongPairError,
};
pub use sound_fonts::{scan_sound_fonts, sound_font_exists, SoundFontScan, SOUND_FONT_DIRECTORY};
pub use template::{CommandTemplate, PlaceholderValues, TemplateError};
pub use track_diff::{diff_tracks, DiffLine, ResolvedPart, TrackDiff, TrackListing};
pub use watch::{FileChange, FileWatch, Observed, WatchEvent};
