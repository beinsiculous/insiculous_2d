use super::*;
use editor::external_tools::{CommandTemplate, CustomTool, ExternalToolsConfig, FileType, Location, PresetId, ToolChoice};

fn search_over(directories: &[&Path], home: Option<&Path>) -> ProgramSearch {
    ProgramSearch {
        // An empty PATH still splits into one (empty) directory, so no directories is no PATH.
        path: if directories.is_empty() { None } else { std::env::join_paths(directories).ok() },
        executable_extensions: Vec::new(),
        home: home.map(Path::to_path_buf),
    }
}

#[cfg(unix)]
fn write_executable(path: &Path) -> std::io::Result<()> {
    write_script(path, "")
}

/// A shell script at `path` running `body`, executable.
#[cfg(unix)]
fn write_script(path: &Path, body: &str) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("#!/bin/sh\n{body}"))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

#[test]
#[cfg(unix)]
fn test_a_bare_name_is_found_on_path_and_a_miss_counts_the_directories() -> std::io::Result<()> {
    let root = tempfile::tempdir()?;
    let empty = root.path().join("empty");
    let tools = root.path().join("tools");
    std::fs::create_dir_all(&empty)?;
    write_executable(&tools.join("zed"))?;
    std::fs::write(tools.join("notes"), b"not runnable")?;
    let search = search_over(&[&empty, &tools], None);

    assert_eq!(search.find("zed", &[]), Ok(tools.join("zed")));
    assert_eq!(search.find("rustrover", &[]), Err(NotFound::NotOnPath { directories_searched: 2 }));
    assert_eq!(
        search.find("notes", &[]),
        Err(NotFound::NotOnPath { directories_searched: 2 }),
        "a file without the execute bit is not a program"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn test_a_program_naming_a_path_is_checked_as_that_file_and_never_on_path() -> std::io::Result<()> {
    let root = tempfile::tempdir()?;
    let tool = root.path().join("bin/my tool");
    write_executable(&tool)?;
    let search = search_over(&[root.path()], None);

    assert_eq!(search.find(&tool.to_string_lossy(), &[]), Ok(tool.clone()));
    assert_eq!(search.find(&root.path().join("bin/missing").to_string_lossy(), &[]), Err(NotFound::NoSuchFile));
    Ok(())
}

#[test]
#[cfg(unix)]
fn test_a_preset_fallback_under_home_is_found_after_path() -> std::io::Result<()> {
    let home = tempfile::tempdir()?;
    let steam_install = home.path().join(".steam/root/steamapps/common/Aseprite/aseprite");
    write_executable(&steam_install)?;
    let fallback = PresetId::Aseprite.preset().fallback_locations;

    assert_eq!(search_over(&[], Some(home.path())).find("aseprite", fallback), Ok(steam_install));
    assert_eq!(
        search_over(&[], None).find("aseprite", fallback),
        Err(NotFound::NotOnPath { directories_searched: 0 }),
        "without a home the fallback names nothing"
    );
    Ok(())
}

#[test]
fn test_the_not_found_sentence_names_the_count_the_settings_and_a_flatpak() {
    let not_found = NotFound::NotOnPath { directories_searched: 11 };
    assert_eq!(
        not_found.sentence("rustrover", None),
        "rustrover: not found on PATH (searched 11 directories) — set the tool in Edit → External Tools…"
    );
    assert_eq!(
        not_found.sentence("aseprite", Some("org.aseprite.Aseprite")),
        "aseprite: not found on PATH (searched 11 directories) — set the tool in Edit → External Tools… \
         (a Flatpak install runs as the Custom command `flatpak run org.aseprite.Aseprite {file}`)"
    );
}

#[test]
fn test_an_exit_is_news_only_when_it_failed_inside_the_report_window() {
    let quick = Duration::from_millis(50);
    let later = Duration::from_secs(2);
    assert_eq!(exit_report("zed", None, quick, b"", ExitWording::MaybeTerminal), None, "a success says nothing");
    assert_eq!(
        exit_report("zed", Some("exit status: 1"), Duration::from_secs(3600), b"error", ExitWording::MaybeTerminal),
        None,
        "a window closed an hour later is not news"
    );
    assert_eq!(
        exit_report("hx", Some("exit status: 1"), quick, b"\n  Error: not a terminal\nmore\n", ExitWording::MaybeTerminal),
        Some(
            "hx exited at once — a terminal editor needs a terminal; set the tool in Edit → External Tools… \
             — Error: not a terminal"
                .to_string()
        )
    );
    assert_eq!(
        exit_report("zed", Some("exit status: 2"), later, b"", ExitWording::MaybeTerminal),
        Some("zed failed (exit status: 2)".to_string())
    );
    assert_eq!(
        exit_report("xdg-open", Some("exit status: 3"), quick, b"no method available\n", ExitWording::Plain),
        Some("xdg-open failed (exit status: 3) — no method available".to_string()),
        "an opener that fails at once is not blamed on a terminal"
    );
}

#[test]
fn test_a_program_named_in_a_presets_place_has_no_fallback_locations_and_keeps_the_preset() {
    let own = Launch { program: "aseprite".to_string(), args: Vec::new(), preset: Some(PresetId::Aseprite), named_program: false };
    assert_eq!(fallback_locations(&own), PresetId::Aseprite.preset().fallback_locations);
    assert_eq!(flatpak_hint(&own), Some("org.aseprite.Aseprite"));
    let named = Launch { named_program: true, program: "aseprite-beta".to_string(), ..own.clone() };
    assert!(fallback_locations(&named).is_empty(), "a miss on PATH is a miss, never the preset's own binary under the named program");
    assert_eq!(flatpak_hint(&named), None, "the preset's Flatpak is not the program the person named");
    assert_eq!(exit_wording(&named), ExitWording::Plain, "the preset still words a GUI program's early exit");
    assert_eq!(launched_name(&own), "Aseprite");
    assert_eq!(launched_name(&named), "aseprite-beta", "the line names what ran, as typed");
    let custom = Launch { preset: None, ..own };
    assert!(fallback_locations(&custom).is_empty());
    assert_eq!(flatpak_hint(&custom), None);
    assert_eq!(launched_name(&custom), "aseprite");
}

#[test]
fn test_only_a_custom_command_or_a_terminal_preset_is_suspected_of_needing_a_terminal() {
    let launch = |preset| Launch { program: "tool".to_string(), args: Vec::new(), preset, named_program: false };
    assert_eq!(exit_wording(&launch(None)), ExitWording::MaybeTerminal, "a custom command is a program the editor does not know");
    assert_eq!(exit_wording(&launch(Some(PresetId::Helix))), ExitWording::MaybeTerminal);
    assert_eq!(
        exit_wording(&launch(Some(PresetId::MidiEvil))),
        ExitWording::Plain,
        "a GUI preset that dies at once failed for its own reason, which its stderr line says"
    );
}

#[test]
fn test_stderr_keeps_only_its_last_four_kibibytes() {
    let mut noise = vec![b'a'; 10_000];
    noise.extend_from_slice(b"the end");
    let tail = read_tail(noise.as_slice());
    assert_eq!(tail.len(), STDERR_TAIL_BYTES);
    assert!(tail.ends_with(b"the end"));
}

#[test]
fn test_the_workspace_root_prefers_a_workspace_then_the_nearest_manifest_then_the_parent() -> std::io::Result<()> {
    let root = tempfile::tempdir()?;
    let engine = root.path().join("engine");
    let game = root.path().join("games/pong");
    let data = root.path().join("data/project/assets");
    std::fs::create_dir_all(engine.join("examples/assets"))?;
    std::fs::create_dir_all(game.join("assets"))?;
    std::fs::create_dir_all(&data)?;
    std::fs::write(engine.join("Cargo.toml"), "[workspace]\nmembers = []\n")?;
    std::fs::create_dir_all(engine.join("examples"))?;
    std::fs::write(engine.join("examples/Cargo.toml"), "[package]\nname = \"examples\"\n")?;
    std::fs::write(game.join("Cargo.toml"), "[package]\nname = \"pong\"\n")?;

    assert_eq!(workspace_root(&engine.join("examples/assets")), engine, "a workspace above wins over a nearer manifest");
    assert_eq!(workspace_root(&game.join("assets")), game, "a lone crate is its own project");
    assert_eq!(workspace_root(&data), root.path().join("data/project"), "no manifest: the asset base's parent");
    Ok(())
}

#[test]
#[cfg(unix)]
fn test_a_launched_program_that_fails_at_once_reports_through_the_channel() {
    let (reports, finished) = mpsc::channel();
    let launch = Launch {
        program: "sh".to_string(),
        args: vec!["-c".to_string(), "echo first line >&2; exit 4".to_string()],
        preset: None,
        named_program: false,
    };

    assert_eq!(run_program(&ProgramSearch::from_process(), &launch, ExitWording::MaybeTerminal, &reports), Ok("sh".to_string()));

    let report = finished.recv_timeout(Duration::from_secs(10)).unwrap_or_default();
    assert!(report.starts_with("sh "), "{report}");
    assert!(report.ends_with(" — first line"), "{report}");
}

#[test]
fn test_only_a_program_ending_in_exe_under_wsl_takes_windows_paths() {
    assert!(takes_windows_paths(Platform::Wsl, "notepad.exe"));
    assert!(takes_windows_paths(Platform::Wsl, "/mnt/c/Program Files/Notepad++/NOTEPAD++.EXE"), "in any case, a path included");
    assert!(!takes_windows_paths(Platform::Wsl, "code.cmd"), "a .cmd is not translated");
    assert!(!takes_windows_paths(Platform::Wsl, "build.bat"), "nor a .bat");
    assert!(!takes_windows_paths(Platform::Wsl, "code"));
    assert!(!takes_windows_paths(Platform::Linux, "notepad.exe"), "only WSL runs a Windows program from Linux");
    assert!(!takes_windows_paths(Platform::Windows, "notepad.exe"), "Windows itself names Windows paths");
}

/// What a custom `tool.exe` would open: the script `paddle.rhai` at line 12, in `root`.
fn windows_request(root: &Path) -> super::super::WindowsProgramRequest {
    let template = CommandTemplate("tool.exe --project {project} {file}[:{line}]".to_string());
    super::super::WindowsProgramRequest {
        config: ExternalToolsConfig::default(),
        choice: ToolChoice::Custom(CustomTool::file_only(template)),
        file_type: FileType::RhaiScript,
        file: root.join("game/assets/scripts/paddle.rhai"),
        project: root.join("game"),
        location: Location::Line { line: 12, column: None },
        program: "tool.exe".to_string(),
    }
}

/// How the fake `wslpath` writes a path: `W:` then the path with each `/` a `\`.
fn fake_windows_path(path: &Path) -> String {
    format!("W:{}", path.display()).replace('/', "\\")
}

#[test]
#[cfg(unix)]
fn test_a_windows_program_is_started_on_the_paths_wslpath_gives_with_its_line_glued() -> std::io::Result<()> {
    let root = tempfile::tempdir()?;
    let bin = root.path().join("bin");
    write_script(&bin.join("wslpath"), "[ \"$1\" = -w ] || exit 2\nprintf 'W:%s\\n' \"$2\" | tr / '\\\\'\n")?;
    // The fake program says its argv on stderr and fails, so the reaper's report carries it.
    write_script(&bin.join("tool.exe"), "printf '%s|' \"$@\" >&2\nexit 1\n")?;
    let request = windows_request(root.path());
    let (file, project) = (request.file.clone(), request.project.clone());
    let (reports, finished) = mpsc::channel();

    assert_eq!(start_windows_program(request, search_over(&[&bin], None), reports), Ok("tool.exe".to_string()));

    let report = finished.recv_timeout(Duration::from_secs(10)).unwrap_or_default();
    let argv = format!("--project|{}|{}:12|", fake_windows_path(&project), fake_windows_path(&file));
    assert!(report.starts_with("tool.exe "), "{report}");
    assert!(report.ends_with(&format!(" — {argv}")), "the Windows paths, the line glued to the file: {report}");
    Ok(())
}

#[test]
#[cfg(unix)]
fn test_a_missing_or_failing_wslpath_is_one_sentence_naming_the_path_and_the_program_and_nothing_starts() -> std::io::Result<()> {
    let root = tempfile::tempdir()?;
    let without_wslpath = root.path().join("empty");
    std::fs::create_dir_all(&without_wslpath)?;
    let failing = root.path().join("failing");
    write_script(&failing.join("wslpath"), "echo \"wslpath: $2: No such file or directory\" >&2\nexit 1\n")?;
    for directory in [&without_wslpath, &failing] {
        // The program is on the search too, so a start would be heard from its reaper.
        write_script(&directory.join("tool.exe"), "echo started >&2\nexit 1\n")?;
    }
    let file = windows_request(root.path()).file;

    let (reports, finished) = mpsc::channel();
    assert_eq!(start_windows_program(windows_request(root.path()), search_over(&[&without_wslpath], None), reports), Ok("tool.exe".to_string()));
    assert_eq!(
        finished.recv_timeout(Duration::from_secs(10)),
        Ok(format!("wslpath could not translate {} for tool.exe — wslpath: not found on PATH (searched 1 directories)", file.display()))
    );
    assert_eq!(finished.recv_timeout(Duration::from_secs(10)), Err(mpsc::RecvTimeoutError::Disconnected), "nothing was started");

    let (reports, finished) = mpsc::channel();
    assert_eq!(start_windows_program(windows_request(root.path()), search_over(&[&failing], None), reports), Ok("tool.exe".to_string()));
    assert_eq!(
        finished.recv_timeout(Duration::from_secs(10)),
        Ok(format!("wslpath could not translate {0} for tool.exe — wslpath: {0}: No such file or directory", file.display())),
        "wslpath's own first line says why"
    );
    assert_eq!(finished.recv_timeout(Duration::from_secs(10)), Err(mpsc::RecvTimeoutError::Disconnected), "nothing was started");
    Ok(())
}
