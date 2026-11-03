//! Opening project files in their external tools: each frame the requests queued on
//! the editor's context are resolved through the configured tools and launched, then
//! each Test the settings panel pressed opens a file of its category's kind, and what a
//! launched program reported since the last frame reaches the status bar and the
//! console, as every launch result does.
//!
//! A launch never blocks the frame and never panics: the program is found and
//! spawned here, and two threads per child read its stderr and reap it.

use std::sync::mpsc;

use editor::console::Severity;
use editor::external_tools::{Location, OpenRequest, ToolCategory};
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

/// What a request opens with, decided on the frame.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Opening {
    /// Resolved on the frame: the file's absolute path and what opens it
    Now(std::path::PathBuf, Resolution),
    /// A Windows program under WSL, which reads Windows paths: resolved and started on a
    /// worker once `wslpath -w` has translated the file and the project
    AfterTranslation(Box<WindowsProgramRequest>),
}

/// A request for a Windows program under WSL, owned by the worker that translates its
/// paths and then resolves it.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, PartialEq, Eq)]
pub(super) struct WindowsProgramRequest {
    pub(super) config: editor::external_tools::ExternalToolsConfig,
    pub(super) choice: editor::external_tools::ToolChoice,
    pub(super) file_type: editor::external_tools::FileType,
    /// Absolute, as Linux names it
    pub(super) file: std::path::PathBuf,
    /// Absolute, as Linux names it
    pub(super) project: std::path::PathBuf,
    pub(super) location: Location,
    /// The program the choice runs, as the settings name it
    pub(super) program: String,
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
        for category in std::mem::take(&mut self.editor.settings.tool_tests) {
            self.test_tool(category);
        }
        while let Ok(report) = self.launcher.finished.try_recv() {
            self.report(Severity::Error, report, None);
        }
    }

    /// Edit → Open Project: the code tool on the workspace, which the launcher finds
    /// itself, so the request names no path.
    pub(super) fn open_project(&mut self) {
        self.editor.open_requests.push(OpenRequest { path: std::path::PathBuf::new(), location: Location::Project });
    }

    #[cfg(target_arch = "wasm32")]
    fn test_tool(&mut self, category: ToolCategory) {
        let line = format!("Test {}: {}", category.name(), editor::external_tools::DESKTOP_ONLY_HINT);
        self.report(Severity::Info, line, None);
    }

    /// A Test press: the category's tool — the row the button sits under, past any
    /// override on the fixture's type — opens the project's first file of its kind, a line
    /// naming the command first so the author sees what ran, then the launch's own.
    #[cfg(not(target_arch = "wasm32"))]
    fn test_tool(&mut self, category: ToolCategory) {
        let Some(fixture) = self.tool_test_fixture(category) else {
            let line = format!("no {} file in this project to test with", category.name().to_lowercase());
            return self.report(Severity::Error, line, None);
        };
        let location = match category {
            ToolCategory::Code => Location::Line { line: 1, column: Some(1) },
            ToolCategory::Music | ToolCategory::Sample | ToolCategory::Art => Location::None,
        };
        let request = self.at_first_part_track(OpenRequest { path: fixture, location });
        match self.resolve_open_request_through(&request, Some(category)) {
            Ok(opening) => {
                let line = match &opening {
                    Opening::Now(file, resolution) => test_line(category, file, resolution),
                    Opening::AfterTranslation(windows) => {
                        // The Windows paths are known only once wslpath answers, so the line
                        // shows the command on the Linux paths and says each is translated.
                        use editor::external_tools::{resolve_with, LaunchTarget, Platform};
                        let target = LaunchTarget {
                            file_type: windows.file_type,
                            file: &windows.file,
                            project: &windows.project,
                            location: &windows.location,
                        };
                        match resolve_with(&windows.config, &windows.choice, target, Platform::Wsl) {
                            Ok(resolution) => format!("{} — each path as wslpath -w translates it", test_line(category, &windows.file, &resolution)),
                            Err(error) => format!("Test {}: {} — {error}", category.name(), windows.program),
                        }
                    }
                };
                self.report(Severity::Info, line, None);
                self.launch_opening(opening);
            }
            Err(message) => self.report(Severity::Error, message, None),
        }
    }

    /// The first file of `category`'s kind in the browser's scan, by its asset-relative
    /// path: Code a script else a scene, Music a song else a DAW project else a SoundFont.
    /// A browser that never scanned is scanned here without touching its state, whose
    /// first scan also builds the script catalog.
    #[cfg(not(target_arch = "wasm32"))]
    fn tool_test_fixture(&self, category: ToolCategory) -> Option<std::path::PathBuf> {
        use editor::AssetKind;

        let scanned;
        let entries = if self.editor.asset_browser.scanned {
            &self.editor.asset_browser.entries
        } else {
            scanned = editor::scan_assets(&self.asset_base, &self.editor.external_tools.daw_project_extensions);
            &scanned
        };
        let kinds: &[AssetKind] = match category {
            ToolCategory::Code => &[AssetKind::Script, AssetKind::Scene],
            ToolCategory::Music => &[AssetKind::Song, AssetKind::DawProject, AssetKind::SoundFont],
            ToolCategory::Sample => &[AssetKind::Sound],
            ToolCategory::Art => &[AssetKind::Image],
        };
        kinds
            .iter()
            .find_map(|kind| entries.iter().find(|entry| entry.kind == *kind))
            .map(|entry| std::path::PathBuf::from(&entry.relative_path))
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
        match self.resolve_open_request(&request) {
            Ok(opening) => self.launch_opening(opening),
            Err(message) => self.report(Severity::Error, message, None),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn launch_opening(&mut self, opening: Opening) {
        let (file, deferred, started) = match opening {
            Opening::Now(file, resolution) => (file, false, spawn::launch(resolution, &self.launcher.reports)),
            Opening::AfterTranslation(windows) => {
                (windows.file.clone(), true, spawn::launch_windows_program(*windows, &self.launcher.reports))
            }
        };
        match started {
            Ok(tool) => {
                let name = file.file_name().map_or_else(|| file.display().to_string(), |name| name.to_string_lossy().into_owned());
                // A deferred launch has started only its worker: nothing is opened yet, and
                // a failure arrives through the reports as an error.
                let line = if deferred {
                    format!("Starting {name} in {tool} once wslpath has translated its paths")
                } else {
                    format!("Opened {name} in {tool}")
                };
                self.report(Severity::Info, line, None);
            }
            Err(message) => self.report(Severity::Error, message, None),
        }
    }

    /// The absolute path a request names and what opens it, or why nothing does. A folder
    /// goes straight to the platform's opener, never through a file type. A Windows
    /// program under WSL is left for its worker to resolve, since its paths are not
    /// known until `wslpath` has translated them; every other request resolves here.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn resolve_open_request(&self, request: &OpenRequest) -> Result<Opening, String> {
        self.resolve_open_request_through(request, None)
    }

    /// [`resolve_open_request`](Self::resolve_open_request) through `category`'s own
    /// tool when one is named, past any override on the file's type: what a Test tries.
    #[cfg(not(target_arch = "wasm32"))]
    fn resolve_open_request_through(&self, request: &OpenRequest, category: Option<ToolCategory>) -> Result<Opening, String> {
        use editor::external_tools::{os_default, program_of, resolve_with, FileType, LaunchTarget, Location, ProjectBase};

        let asset_base = spawn::absolute(&self.asset_base);
        // A project request opens the code tool on the workspace, so it is resolved
        // as the code type a Cargo project is made of.
        let (file, file_type) = if request.location == Location::Project {
            (spawn::workspace_root(&asset_base), FileType::RustSource)
        } else {
            let file = spawn::absolute(&self.resolve_asset_path(&request.path));
            if file.is_dir() {
                let opener = Resolution::OsDefault(os_default(self.launcher.platform, &file));
                return Ok(Opening::Now(file, opener));
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
        let config = &self.editor.external_tools;
        let choice = match category {
            Some(category) => config.category_choice(category),
            None => config.request_choice(file_type, &request.location),
        };
        let windows_program = program_of(config, choice, &request.location)
            .filter(|program| spawn::takes_windows_paths(self.launcher.platform, program));
        if let Some(program) = windows_program {
            return Ok(Opening::AfterTranslation(Box::new(WindowsProgramRequest {
                config: config.clone(),
                choice: choice.clone(),
                file_type,
                file,
                project,
                location: request.location.clone(),
                program,
            })));
        }
        let target = LaunchTarget { file_type, file: &file, project: &project, location: &request.location };
        let resolution = resolve_with(config, choice, target, self.launcher.platform).map_err(|error| error.to_string())?;
        Ok(Opening::Now(file, resolution))
    }
}

/// What a Test runs, before it runs: "Test Code: rustrover --line 1 /game/src/main.rs", a
/// token holding a space in double quotes, or "Test Art: the system default opens <file>".
/// The line is shown, not shell-quoted: a quote inside a token is left as it is.
#[cfg(not(target_arch = "wasm32"))]
fn test_line(category: ToolCategory, file: &std::path::Path, resolution: &Resolution) -> String {
    match resolution {
        Resolution::Tool(launch) => {
            let argv: Vec<String> = std::iter::once(&launch.program)
                .chain(&launch.args)
                .map(|token| if token.contains(char::is_whitespace) { format!("\"{token}\"") } else { token.clone() })
                .collect();
            format!("Test {}: {}", category.name(), argv.join(" "))
        }
        Resolution::OsDefault(_) => format!("Test {}: the system default opens {}", category.name(), file.display()),
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::super::test_support::editor_game;
    use editor::external_tools::{
        CommandTemplate, CustomTool, Launch, Location, OpenRequest, OsDefault, PresetId, Resolution, ToolCategory, ToolChoice,
    };
    use std::path::{Path, PathBuf};

    fn request(path: &str) -> OpenRequest {
        OpenRequest { path: PathBuf::from(path), location: Location::None }
    }

    fn custom(file: &str) -> ToolChoice {
        ToolChoice::Custom(CustomTool::file_only(CommandTemplate(file.to_string())))
    }

    /// A crate with a manifest and its `assets/` holding `files`, each empty.
    fn crate_with_assets(files: &[&str]) -> std::io::Result<tempfile::TempDir> {
        let project = tempfile::tempdir()?;
        std::fs::write(project.path().join("Cargo.toml"), "[package]\nname = \"game\"\n")?;
        for file in files {
            let path = project.path().join("assets").join(file);
            std::fs::create_dir_all(path.parent().unwrap_or(project.path()))?;
            std::fs::write(path, "")?;
        }
        std::fs::create_dir_all(project.path().join("assets"))?;
        Ok(project)
    }

    fn console_lines(session: &super::super::EditorGame<super::super::test_support::DummyGame>) -> Vec<String> {
        session.editor.console.log.entries().iter().map(|entry| entry.text.clone()).collect()
    }

    #[test]
    fn test_open_project_with_rustrover_resolves_to_rustrover_on_the_workspace() -> std::io::Result<()> {
        let project = crate_with_assets(&[])?;
        let mut session = editor_game();
        session.asset_base = project.path().join("assets");
        session.editor.external_tools.code = ToolChoice::Preset(PresetId::RustRover);

        session.open_project();
        let request = session.editor.open_requests.pop().expect("Open Project files a request");
        let workspace = project.path().to_path_buf();
        let rustrover = Launch {
            program: "rustrover".to_string(),
            args: vec![workspace.to_string_lossy().into_owned()],
            preset: Some(PresetId::RustRover),
            named_program: false,
        };
        assert_eq!(session.resolve_open_request(&request), Ok(super::Opening::Now(workspace, Resolution::Tool(rustrover))));
        Ok(())
    }

    #[test]
    fn test_open_project_with_a_custom_tool_without_a_project_command_says_where_to_set_one() {
        let mut session = editor_game();
        session.editor.external_tools.code = custom("edit {file}");

        session.open_project();
        session.open_requested_files();

        assert_eq!(
            session.editor.status_bar.message(),
            Some("The custom command has no command for opening a project — set a code tool in Edit → External Tools…")
        );
        assert!(session.editor.status_bar.is_showing_error());
    }

    #[test]
    fn test_a_code_test_reports_the_command_line_it_runs_before_the_launchs_own_line() -> std::io::Result<()> {
        let project = crate_with_assets(&["scenes/main.ron", "scripts/paddle.rhai"])?;
        let mut session = editor_game();
        session.asset_base = project.path().join("assets");
        session.editor.external_tools.code = custom("echo {project} {file}:{line}");

        session.editor.settings.tool_tests.push(ToolCategory::Code);
        session.open_requested_files();

        let script = project.path().join("assets/scripts/paddle.rhai");
        let lines = console_lines(&session);
        assert_eq!(lines.first(), Some(&format!("Test Code: echo {} {}:1", project.path().display(), script.display())), "a script before a scene");
        if cfg!(unix) {
            assert_eq!(lines.get(1).map(String::as_str), Some("Opened paddle.rhai in echo"));
        }
        assert!(session.editor.settings.tool_tests.is_empty(), "each press is drained once");
        assert!(!session.editor.asset_browser.scanned, "the browser's own first scan, which builds the script catalog, is left to it");
        Ok(())
    }

    #[test]
    fn test_a_test_tries_the_categorys_tool_past_an_override_on_its_fixtures_type() -> std::io::Result<()> {
        let project = crate_with_assets(&["sprites/hero.png"])?;
        let mut session = editor_game();
        session.asset_base = project.path().join("assets");
        session.editor.external_tools.art = custom("echo {file}");
        session.editor.external_tools.overrides.insert(editor::external_tools::FileType::Image, ToolChoice::OsDefault);

        session.editor.settings.tool_tests.push(ToolCategory::Art);
        session.open_requested_files();

        let image = project.path().join("assets/sprites/hero.png");
        let expected = format!("Test Art: echo {}", image.display());
        assert!(console_lines(&session).contains(&expected), "the Art row's own tool, not the Image override: {:?}", console_lines(&session));
        Ok(())
    }

    #[test]
    fn test_a_category_with_no_file_of_its_kind_says_so_and_launches_nothing() -> std::io::Result<()> {
        let project = crate_with_assets(&["scripts/paddle.rhai"])?;
        let mut session = editor_game();
        session.asset_base = project.path().join("assets");

        session.editor.settings.tool_tests.push(ToolCategory::Sample);
        session.open_requested_files();

        assert_eq!(session.editor.status_bar.message(), Some("no sample file in this project to test with"));
        assert!(session.editor.status_bar.is_showing_error());
        assert_eq!(console_lines(&session).len(), 1);
        Ok(())
    }

    #[test]
    fn test_a_music_test_opens_a_song_before_a_sound_font() -> std::io::Result<()> {
        let project = crate_with_assets(&["soundfonts/bank.sf2", "music/theme.mid"])?;
        let mut session = editor_game();
        session.asset_base = project.path().join("assets");
        session.editor.external_tools.music = custom("echo {file}");

        session.editor.settings.tool_tests.push(ToolCategory::Music);
        session.open_requested_files();

        let song = project.path().join("assets/music/theme.mid");
        let expected = format!("Test Music: echo {}", song.display());
        assert!(console_lines(&session).contains(&expected), "{:?}", console_lines(&session));
        Ok(())
    }

    #[test]
    fn test_the_test_line_quotes_a_token_holding_a_space_and_names_the_system_defaults_file() {
        let file = Path::new("/game/assets/sprites/hero one.png");
        let launch = Launch {
            program: "/opt/my tools/aseprite".to_string(),
            args: vec!["--batch".to_string(), file.to_string_lossy().into_owned()],
            preset: None,
            named_program: false,
        };
        assert_eq!(
            super::test_line(ToolCategory::Art, file, &Resolution::Tool(launch)),
            "Test Art: \"/opt/my tools/aseprite\" --batch \"/game/assets/sprites/hero one.png\""
        );
        let opener = Resolution::OsDefault(OsDefault::ShellAssociation(file.to_path_buf()));
        assert_eq!(
            super::test_line(ToolCategory::Art, file, &opener),
            "Test Art: the system default opens /game/assets/sprites/hero one.png"
        );
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
        let opener =
            Launch { program: "xdg-open".to_string(), args: vec![folder.to_string_lossy().into_owned()], preset: None, named_program: false };
        assert_eq!(resolved, Ok(super::Opening::Now(folder, Resolution::OsDefault(OsDefault::Program(opener)))));
        Ok(())
    }

    #[test]
    fn test_a_windows_program_resolves_on_the_frame_under_linux_and_waits_for_its_paths_under_wsl() -> std::io::Result<()> {
        use editor::external_tools::{FileType, Platform};

        let project = crate_with_assets(&["scripts/paddle.rhai"])?;
        let mut session = editor_game();
        session.asset_base = project.path().join("assets");
        session.editor.external_tools.code = custom("notepad.exe {file}[:{line}]");
        let at_line = OpenRequest { path: PathBuf::from("scripts/paddle.rhai"), location: Location::Line { line: 7, column: None } };
        let script = project.path().join("assets/scripts/paddle.rhai");

        session.launcher.platform = Platform::Linux;
        let notepad = Launch { program: "notepad.exe".to_string(), args: vec![format!("{}:7", script.display())], preset: None, named_program: false };
        assert_eq!(
            session.resolve_open_request(&at_line),
            Ok(super::Opening::Now(script.clone(), Resolution::Tool(notepad))),
            "Linux gives a .exe its Linux paths"
        );

        session.launcher.platform = Platform::Wsl;
        let Ok(super::Opening::AfterTranslation(windows)) = session.resolve_open_request(&at_line) else {
            panic!("under WSL a .exe waits for wslpath");
        };
        assert_eq!((windows.program.as_str(), windows.file_type), ("notepad.exe", FileType::RhaiScript));
        assert_eq!((windows.file, windows.project), (script, project.path().to_path_buf()), "the Linux paths, for the worker to translate");

        session.editor.external_tools.code = custom("code.cmd {file}");
        assert!(matches!(session.resolve_open_request(&at_line), Ok(super::Opening::Now(..))), "a .cmd is not translated");
        Ok(())
    }

    #[test]
    fn test_a_program_not_found_names_the_directories_searched_and_where_to_set_the_tool() {
        let mut session = editor_game();
        session.editor.external_tools.code =
            custom("nonexistent_command_that_cannot_be_found_xyz123 {file}");
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
        session.editor.external_tools.code = custom("edit {path}");
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
            custom(r#"sh -c "echo broken setup >&2; exit 3" {file}"#);
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
