//! Opening project files in their external tools: each frame the requests queued on
//! the editor's context are resolved through the configured tools and launched, and
//! what a launched program reported since the last frame reaches the status bar.
//!
//! A launch never blocks the frame and never panics: the program is found and
//! spawned here, and two threads per child read its stderr and reap it.

use std::sync::mpsc;

use editor::external_tools::OpenRequest;
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
    /// Launch every file requested this frame, then show what earlier launches reported.
    pub(super) fn open_requested_files(&mut self) {
        for request in std::mem::take(&mut self.editor.open_requests) {
            self.open_in_external_tool(request);
        }
        while let Ok(report) = self.launcher.finished.try_recv() {
            self.editor.status_bar.show_error(report);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn open_in_external_tool(&mut self, request: OpenRequest) {
        let resolved = self.resolve_asset_path(&request.path);
        self.editor.status_bar.show_message(format!(
            "Source: {} (external tools open it from the desktop editor)",
            resolved.display()
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_in_external_tool(&mut self, request: OpenRequest) {
        use editor::external_tools::{resolve, FileType, LaunchTarget, Location, ProjectBase};

        let asset_base = spawn::absolute(&self.asset_base);
        // A project request opens the code tool on the workspace, so it is resolved
        // as the code type a Cargo project is made of.
        let (file, file_type) = if request.location == Location::Project {
            (spawn::workspace_root(&asset_base), FileType::RustSource)
        } else {
            let file = spawn::absolute(&self.resolve_asset_path(&request.path));
            let Some(file_type) = FileType::of(&file, &self.editor.external_tools.daw_project_extensions) else {
                self.editor
                    .status_bar
                    .show_error(format!("No external tool opens {}: its type is unknown", file.display()));
                return;
            };
            (file, file_type)
        };
        let project = match file_type.project_base() {
            ProjectBase::CargoWorkspace => spawn::workspace_root(&asset_base),
            ProjectBase::AssetBase => asset_base,
        };
        let target = LaunchTarget { file_type, file: &file, project: &project, location: &request.location };
        let launched = resolve(&self.editor.external_tools, target, self.launcher.platform)
            .map_err(|error| error.to_string())
            .and_then(|resolution| spawn::launch(resolution, &self.launcher.reports));
        match launched {
            Ok(tool) => {
                let name = file.file_name().map_or_else(|| file.display().to_string(), |name| name.to_string_lossy().into_owned());
                self.editor.status_bar.show_message(format!("Opened {name} in {tool}"));
            }
            Err(message) => self.editor.status_bar.show_error(message),
        }
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
