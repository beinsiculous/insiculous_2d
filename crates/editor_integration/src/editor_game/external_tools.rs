//! Opening project files in their external tools: each frame the requests queued on
//! the editor's context are resolved through the configured tools and launched, and
//! what a launched program reported since the last frame reaches the status bar and the
//! console, as every launch result does.
//!
//! A launch never blocks the frame and never panics: the program is found and
//! spawned here, and two threads per child read its stderr and reap it.

use std::sync::mpsc;

use editor::console::Severity;
use editor::external_tools::OpenRequest;
#[cfg(not(target_arch = "wasm32"))]
use editor::external_tools::Resolution;
use engine_core::Game;

#[cfg(not(target_arch = "wasm32"))]
mod spawn;

/// What the launcher keeps between frames.
pub(super) struct Launcher {
    /// Handed to each launch's reaper, which reports an early failed exit through it
    #[cfg(not(target_arch = "wasm32"))]
    reports: mpsc::Sender<String>,
    finished: mpsc::Receiver<String>,
    /// Read once at startup: whose opener is the system default
    #[cfg(not(target_arch = "wasm32"))]
    platform: editor::external_tools::Platform,
}

impl Launcher {
    pub(super) fn new() -> Self {
        let (reports, finished) = mpsc::channel();
        #[cfg(target_arch = "wasm32")]
        drop(reports);
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            reports,
            finished,
            #[cfg(not(target_arch = "wasm32"))]
            platform: spawn::current_platform(),
        }
    }
}

impl<G: Game> super::EditorGame<G> {
    /// Launch every file requested this frame — the emitter's binding among them — then
    /// show what earlier launches reported.
    pub(super) fn open_requested_files(&mut self) {
        self.open_requested_binding();
        for request in std::mem::take(&mut self.editor.open_requests) {
            self.open_in_external_tool(request);
        }
        while let Ok(report) = self.launcher.finished.try_recv() {
            self.report(Severity::Error, report, None);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn open_in_external_tool(&mut self, request: OpenRequest) {
        let request = self.at_first_part_track(request);
        let resolved = self.resolve_asset_path(&request.path);
        let track = match &request.location {
            editor::external_tools::Location::Track { name: Some(name), index } => format!(", track '{name}' (index {index})"),
            editor::external_tools::Location::Track { name: None, index } => format!(", track index {index}"),
            _ => String::new(),
        };
        let line = format!("Source: {}{track} ({})", resolved.display(), editor::external_tools::DESKTOP_ONLY_HINT);
        self.report(Severity::Info, line, None);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_in_external_tool(&mut self, request: OpenRequest) {
        let request = self.at_first_part_track(request);
        let launched = self
            .resolve_open_request(&request)
            .and_then(|(file, resolution)| Ok((file, spawn::launch(resolution, &self.launcher.reports)?)));
        match launched {
            Ok((file, tool)) => {
                let name = file.file_name().map_or_else(|| file.display().to_string(), |name| name.to_string_lossy().into_owned());
                self.report(Severity::Info, format!("Opened {name} in {tool}"), None);
            }
            Err(message) => self.report(Severity::Error, message, None),
        }
    }

    /// The absolute path a request names and what opens it, or why nothing does. A folder
    /// goes straight to the platform's opener, never through a file type.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn resolve_open_request(&self, request: &OpenRequest) -> Result<(std::path::PathBuf, Resolution), String> {
        use editor::external_tools::{os_default, resolve, FileType, LaunchTarget, Location, ProjectBase};

        let asset_base = spawn::absolute(&self.asset_base);
        // A project request opens the code tool on the workspace, so it is resolved
        // as the code type a Cargo project is made of.
        let (file, file_type) = if request.location == Location::Project {
            (spawn::workspace_root(&asset_base), FileType::RustSource)
        } else {
            let file = spawn::absolute(&self.resolve_asset_path(&request.path));
            if file.is_dir() {
                let opener = Resolution::OsDefault(os_default(self.launcher.platform, &file));
                return Ok((file, opener));
            }
            let Some(file_type) = FileType::of(&file, &self.editor.external_tools.daw_project_extensions) else {
                return Err(format!("No external tool opens {}: its type is unknown", file.display()));
            };
            (file, file_type)
        };
        let project = match file_type.project_base() {
            ProjectBase::CargoWorkspace => spawn::workspace_root(&asset_base),
            ProjectBase::AssetBase => asset_base,
        };
        let target = LaunchTarget { file_type, file: &file, project: &project, location: &request.location };
        let resolution = resolve(&self.editor.external_tools, target, self.launcher.platform).map_err(|error| error.to_string())?;
        Ok((file, resolution))
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::super::test_support::editor_game;
    use editor::external_tools::{CommandTemplate, Location, OpenRequest, ToolChoice};
    use std::path::PathBuf;

    fn request(path: &str) -> OpenRequest {
        OpenRequest { path: PathBuf::from(path), location: Location::None }
    }

    #[test]
    fn test_a_request_for_an_unknown_file_type_is_reported_and_dropped() {
        let mut session = editor_game();
        session.asset_base = PathBuf::from("/game/assets");
        session.editor.open_requests.push(request("notes/todo.txt"));

        session.open_requested_files();

        assert_eq!(
            session.editor.status_bar.message(),
            Some("No external tool opens /game/assets/notes/todo.txt: its type is unknown")
        );
        assert!(session.editor.status_bar.is_showing_error());
        assert!(session.editor.open_requests.is_empty(), "a request that cannot open is dropped, not retried");
    }

    #[test]
    fn test_a_folder_request_on_linux_goes_to_the_opener_not_through_a_file_type() -> std::io::Result<()> {
        use editor::external_tools::{Launch, OsDefault, Platform, Resolution};

        let project = tempfile::tempdir()?;
        std::fs::create_dir_all(project.path().join("sprites"))?;
        let mut session = editor_game();
        session.asset_base = project.path().to_path_buf();
        session.launcher.platform = Platform::Linux;

        let resolved = session.resolve_open_request(&request("sprites"));

        let folder = project.path().join("sprites");
        let opener = Launch { program: "xdg-open".to_string(), args: vec![folder.to_string_lossy().into_owned()], preset: None };
        assert_eq!(resolved, Ok((folder, Resolution::OsDefault(OsDefault::Program(opener)))));
        Ok(())
    }

    #[test]
    fn test_a_program_not_found_names_the_directories_searched_and_where_to_set_the_tool() {
        let mut session = editor_game();
        session.editor.external_tools.code =
            ToolChoice::Custom(CommandTemplate("nonexistent_command_that_cannot_be_found_xyz123 {file}".to_string()));
        session.editor.open_requests.push(request("scripts/test.rhai"));

        session.open_requested_files();

        let message = session.editor.status_bar.message().unwrap_or_default();
        assert!(
            message.starts_with("nonexistent_command_that_cannot_be_found_xyz123: not found on PATH (searched "),
            "{message}"
        );
        assert!(message.ends_with("directories) — set the tool in Edit → External Tools…"), "{message}");
        assert!(session.editor.status_bar.is_showing_error());
        assert!(session.editor.open_requests.is_empty());
    }

    #[test]
    fn test_an_unusable_custom_command_is_refused_naming_its_error() {
        let mut session = editor_game();
        session.editor.external_tools.code = ToolChoice::Custom(CommandTemplate("edit {path}".to_string()));
        session.editor.open_requests.push(request("scripts/test.rhai"));

        session.open_requested_files();

        assert!(session.editor.status_bar.is_showing_error());
        assert!(
            session.editor.status_bar.message().unwrap_or_default().contains("unknown placeholder {path}"),
            "{:?}",
            session.editor.status_bar.message()
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_a_tool_that_fails_at_once_reports_its_first_stderr_line_on_a_later_frame() {
        let mut session = editor_game();
        session.asset_base = PathBuf::from("/game/assets");
        session.editor.external_tools.code =
            ToolChoice::Custom(CommandTemplate(r#"sh -c "echo broken setup >&2; exit 3" {file}"#.to_string()));
        session.editor.open_requests.push(request("scripts/paddle.rhai"));

        session.open_requested_files();
        assert_eq!(session.editor.status_bar.message(), Some("Opened paddle.rhai in sh"));

        for _ in 0..200 {
            if session.editor.status_bar.is_showing_error() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
            session.open_requested_files();
        }
        let report = session.editor.status_bar.message().unwrap_or_default();
        assert!(session.editor.status_bar.is_showing_error(), "the reaper reported the exit: {report}");
        assert!(report.starts_with("sh "), "{report}");
        assert!(report.ends_with(" — broken setup"), "the stderr reader's first line rides the report: {report}");
    }
}
