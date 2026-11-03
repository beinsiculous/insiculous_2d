//! Finding and starting a resolved tool, and the operating system's openers.
//!
//! Argv everywhere, never a shell. A spawned child gets stdout to null and stderr
//! piped; one thread reads stderr into a bounded buffer so a long-lived GUI never
//! blocks on a full pipe, and another waits the child, joins the reader for at most
//! [`READER_JOIN_TIMEOUT`], and reports a failed exit only while it is still news.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use editor::external_tools::{resolve_with, Launch, LaunchTarget, OsDefault, Platform, Resolution};

use super::WindowsProgramRequest;

/// How much of a child's stderr is kept: the last 4 KiB.
const STDERR_TAIL_BYTES: usize = 4096;
/// std has no timed join, so the reaper waits on the reader's channel this long; a
/// descendant still holding the pipe blocks only the reader.
const READER_JOIN_TIMEOUT: Duration = Duration::from_millis(200);
/// A child exiting this soon never got going: a terminal editor without a terminal.
const EXITED_AT_ONCE: Duration = Duration::from_millis(500);
/// A failed exit after this is a window the author closed hours later, not news.
const REPORT_WINDOW: Duration = Duration::from_secs(5);
const SET_THE_TOOL: &str = "set the tool in Edit → External Tools…";

/// The platform family whose opener is the system default, for this process.
pub(super) fn current_platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else if std::fs::read_to_string("/proc/version").is_ok_and(|version| version.to_ascii_lowercase().contains("microsoft")) {
        Platform::Wsl
    } else {
        Platform::Linux
    }
}

/// `path` made absolute against the working directory; a path that cannot be is kept as given.
pub(super) fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The folder a code tool opens for this project: walking up from the asset base,
/// the nearest `Cargo.toml` declaring a `[workspace]`, else the nearest `Cargo.toml`,
/// else the asset base's parent.
pub(super) fn workspace_root(asset_base: &Path) -> PathBuf {
    let mut nearest_manifest = None;
    for directory in asset_base.ancestors() {
        let Ok(manifest) = std::fs::read_to_string(directory.join("Cargo.toml")) else {
            continue;
        };
        if manifest.lines().any(|line| line.trim_start().starts_with("[workspace")) {
            return directory.to_path_buf();
        }
        nearest_manifest.get_or_insert_with(|| directory.to_path_buf());
    }
    nearest_manifest.unwrap_or_else(|| asset_base.parent().unwrap_or(asset_base).to_path_buf())
}

/// Start `resolution`; `Ok` names the tool for the status bar, `Err` is the sentence
/// saying why nothing started.
pub(super) fn launch(resolution: Resolution, reports: &mpsc::Sender<String>) -> Result<String, String> {
    let search = ProgramSearch::from_process();
    match resolution {
        Resolution::Tool(launch) => run_program(&search, &launch, exit_wording(&launch), reports),
        Resolution::OsDefault(OsDefault::Program(launch)) => run_program(&search, &launch, ExitWording::Plain, reports),
        Resolution::OsDefault(OsDefault::ShellAssociation(file)) => open_with_shell_association(&file),
        Resolution::OsDefault(OsDefault::WslView(file)) => open_with_wslview(file, reports.clone()),
    }
}

/// Whether `program`, run under `platform`, is handed Windows paths: under WSL a program
/// whose name ends in `.exe`, in any case, is a Windows program. Nothing else is — a
/// `.bat` or a `.cmd` included — and no other platform translates.
pub(super) fn takes_windows_paths(platform: Platform, program: &str) -> bool {
    platform == Platform::Wsl && Path::new(program).extension().is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
}

/// Start a Windows program under WSL: its request's file and project go through
/// `wslpath -w` on a thread of its own, since each runs a process, and the request is
/// resolved and started there. `Ok` names the program for the status bar; what fails
/// after the thread starts arrives through `reports`, like an exit report.
pub(super) fn launch_windows_program(request: WindowsProgramRequest, reports: &mpsc::Sender<String>) -> Result<String, String> {
    start_windows_program(request, ProgramSearch::from_process(), reports.clone())
}

fn start_windows_program(request: WindowsProgramRequest, search: ProgramSearch, reports: mpsc::Sender<String>) -> Result<String, String> {
    let program = request.program.clone();
    std::thread::Builder::new()
        .name("external-tool-wslpath".to_string())
        .spawn(move || {
            if let Err(report) = translate_and_launch(&request, &search, &reports) {
                let _ = reports.send(report);
            }
        })
        .map_err(|error| format!("Failed to start {program}: {error}"))?;
    Ok(program)
}

/// The worker's half: both paths translated, or one sentence naming the path and the
/// program; then the request resolved on the Windows paths and started.
fn translate_and_launch(request: &WindowsProgramRequest, search: &ProgramSearch, reports: &mpsc::Sender<String>) -> Result<(), String> {
    let untranslated = |path: &Path, reason: String| {
        format!("wslpath could not translate {} for {} — {reason}", path.display(), request.program)
    };
    let wslpath = search
        .find("wslpath", &[])
        .map_err(|not_found| untranslated(&request.file, format!("wslpath: {}", not_found.reason())))?;
    let file = windows_path(&wslpath, &request.file).map_err(|reason| untranslated(&request.file, reason))?;
    let project = if request.project == request.file {
        file.clone()
    } else {
        windows_path(&wslpath, &request.project).map_err(|reason| untranslated(&request.project, reason))?
    };
    let target = LaunchTarget {
        file_type: request.file_type,
        file: Path::new(&file),
        project: Path::new(&project),
        location: &request.location,
    };
    match resolve_with(&request.config, &request.choice, target, Platform::Wsl).map_err(|error| error.to_string())? {
        Resolution::Tool(tool) => run_program(search, &tool, exit_wording(&tool), reports).map(drop),
        opener => launch(opener, reports).map(drop),
    }
}

/// `path` as Windows names it, by `wslpath -w`; else why not: wslpath's first stderr
/// line, or its exit status.
fn windows_path(wslpath: &Path, path: &Path) -> Result<String, String> {
    let output = Command::new(wslpath)
        .arg("-w")
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first_line = stderr.lines().map(str::trim).find(|line| !line.is_empty());
        return Err(first_line.map_or_else(|| output.status.to_string(), str::to_string));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_program(search: &ProgramSearch, launch: &Launch, wording: ExitWording, reports: &mpsc::Sender<String>) -> Result<String, String> {
    let program = search
        .find(&launch.program, fallback_locations(launch))
        .map_err(|not_found| not_found.sentence(&launch.program, flatpak_hint(launch)))?;
    spawn_watched(&program, &launch.args, &launch.program, wording, reports.clone())
        .map_err(|error| format!("Failed to start {}: {error}", launch.program))?;
    Ok(launched_name(launch))
}

/// What "Opened … in <name>" calls the program: the preset's name for its own program,
/// the program as typed for one named in its place or a custom command.
pub(super) fn launched_name(launch: &Launch) -> String {
    match launch.preset {
        Some(preset) if !launch.named_program => preset.preset().name.to_string(),
        _ => launch.program.clone(),
    }
}

/// Where a bare name is looked for after `PATH`: the preset's fallback locations for its
/// own program, and nowhere for a program named in the settings in its place — those
/// locations hold the preset's binary, which would run under the configured name.
pub(super) fn fallback_locations(launch: &Launch) -> &'static [&'static str] {
    match launch.preset {
        Some(preset) if !launch.named_program => preset.preset().fallback_locations,
        _ => &[],
    }
}

/// The Flatpak id the not-found sentence offers: the preset's, for the preset's own
/// program — a program named in its place is the person's, and the preset's Flatpak is
/// not what they asked for.
pub(super) fn flatpak_hint(launch: &Launch) -> Option<&'static str> {
    match launch.preset {
        Some(preset) if !launch.named_program => preset.preset().flatpak_id,
        _ => None,
    }
}

/// Where the launcher looks for a program, captured so a test can name its own.
pub(super) struct ProgramSearch {
    path: Option<OsString>,
    /// `PATHEXT` on Windows: the extensions a bare name may be missing
    executable_extensions: Vec<String>,
    home: Option<PathBuf>,
}

/// Why a program was not found.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum NotFound {
    /// The program named a path, and nothing runnable is there
    NoSuchFile,
    NotOnPath { directories_searched: usize },
}

impl NotFound {
    /// What went unfound, without the advice: "not found on PATH (searched 11 directories)".
    fn reason(&self) -> String {
        match self {
            NotFound::NoSuchFile => "no such file".to_string(),
            NotFound::NotOnPath { directories_searched } => format!("not found on PATH (searched {directories_searched} directories)"),
        }
    }

    fn sentence(&self, program: &str, flatpak_id: Option<&str>) -> String {
        let mut sentence = format!("{program}: {} — {SET_THE_TOOL}", self.reason());
        if let Some(flatpak_id) = flatpak_id {
            sentence.push_str(&format!(" (a Flatpak install runs as the Custom command `flatpak run {flatpak_id} {{file}}`)"));
        }
        sentence
    }
}

impl ProgramSearch {
    fn from_process() -> Self {
        let executable_extensions = if cfg!(windows) {
            std::env::var("PATHEXT")
                .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
                .split(';')
                .filter(|extension| !extension.is_empty())
                .map(str::to_string)
                .collect()
        } else {
            Vec::new()
        };
        Self { path: std::env::var_os("PATH"), executable_extensions, home: std::env::var_os("HOME").map(PathBuf::from) }
    }

    /// A program naming a path is checked as a file and nothing else; a bare name is
    /// looked up on `PATH`, then at the fallback locations (`~/` is the home directory).
    pub(super) fn find(&self, program: &str, fallback_locations: &[&str]) -> Result<PathBuf, NotFound> {
        if Path::new(program).components().count() > 1 {
            return self.runnable(Path::new(program)).ok_or(NotFound::NoSuchFile);
        }
        let directories: Vec<PathBuf> = self.path.as_deref().map(|path| std::env::split_paths(path).collect()).unwrap_or_default();
        let on_path = directories.iter().find_map(|directory| self.runnable(&directory.join(program)));
        let at_fallback = || {
            fallback_locations.iter().find_map(|location| {
                let expanded = match (location.strip_prefix("~/"), &self.home) {
                    (Some(relative), Some(home)) => home.join(relative),
                    (Some(_), None) => return None,
                    (None, _) => PathBuf::from(location),
                };
                self.runnable(&expanded)
            })
        };
        on_path.or_else(at_fallback).ok_or(NotFound::NotOnPath { directories_searched: directories.len() })
    }

    fn runnable(&self, candidate: &Path) -> Option<PathBuf> {
        if is_executable_file(candidate) {
            return Some(candidate.to_path_buf());
        }
        self.executable_extensions.iter().find_map(|extension| {
            let mut with_extension = candidate.as_os_str().to_owned();
            with_extension.push(extension);
            let with_extension = PathBuf::from(with_extension);
            is_executable_file(&with_extension).then_some(with_extension)
        })
    }
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

/// How an early failed exit is worded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExitWording {
    /// Dying at once most likely means it needed a terminal
    MaybeTerminal,
    /// The program's failure is told as it is, with its stderr
    Plain,
}

/// A custom command is a program the editor knows nothing about, and a preset marked
/// `needs_terminal` is a terminal editor, so either dying at once most likely needed a
/// terminal; a known GUI preset that fails at once failed for its own reason.
pub(super) fn exit_wording(launch: &Launch) -> ExitWording {
    if launch.preset.is_none_or(|id| id.preset().needs_terminal) {
        ExitWording::MaybeTerminal
    } else {
        ExitWording::Plain
    }
}

fn spawn_watched(
    program: &Path,
    args: &[String],
    name: &str,
    wording: ExitWording,
    reports: mpsc::Sender<String>,
) -> std::io::Result<()> {
    let started = Instant::now();
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let stderr = child.stderr.take();
    let (tail_sender, tail_receiver) = mpsc::channel();
    let reader = std::thread::Builder::new().name("external-tool-stderr".to_string()).spawn(move || {
        let tail = stderr.map(read_tail).unwrap_or_default();
        let _ = tail_sender.send(tail);
    });
    if let Err(error) = reader {
        log::warn!("{name}'s stderr is not read: {error}");
    }
    let name = name.to_string();
    let reaper = std::thread::Builder::new().name("external-tool-reaper".to_string()).spawn(move || {
        let failure = match child.wait() {
            Ok(status) if status.success() => None,
            Ok(status) => Some(status.to_string()),
            Err(error) => Some(format!("could not be waited on: {error}")),
        };
        let lifetime = started.elapsed();
        let tail = tail_receiver.recv_timeout(READER_JOIN_TIMEOUT).unwrap_or_default();
        if let Some(report) = exit_report(&name, failure.as_deref(), lifetime, &tail, wording) {
            let _ = reports.send(report);
        }
    });
    if let Err(error) = reaper {
        log::warn!("the launched program is not reaped and stays a zombie until the editor exits: {error}");
    }
    Ok(())
}

/// Read `reader` to its end, keeping the last [`STDERR_TAIL_BYTES`].
fn read_tail(mut reader: impl Read) -> Vec<u8> {
    let mut tail = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => {
                tail.extend_from_slice(&chunk[..count]);
                if tail.len() > STDERR_TAIL_BYTES {
                    tail.drain(..tail.len() - STDERR_TAIL_BYTES);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    tail
}

/// The status-bar sentence for a child's exit, or `None` when there is nothing to say:
/// it succeeded, or failed after [`REPORT_WINDOW`]. `failure` is the exit status text.
fn exit_report(name: &str, failure: Option<&str>, lifetime: Duration, stderr_tail: &[u8], wording: ExitWording) -> Option<String> {
    let failure = failure?;
    if lifetime >= REPORT_WINDOW {
        return None;
    }
    let mut report = if lifetime < EXITED_AT_ONCE && wording == ExitWording::MaybeTerminal {
        format!("{name} exited at once — a terminal editor needs a terminal; {SET_THE_TOOL}")
    } else {
        format!("{name} failed ({failure})")
    };
    let stderr_text = String::from_utf8_lossy(stderr_tail);
    if let Some(first_line) = stderr_text.lines().map(str::trim).find(|line| !line.is_empty()) {
        report.push_str(" — ");
        report.push_str(first_line);
    }
    Some(report)
}

#[cfg(windows)]
fn open_with_shell_association(file: &Path) -> Result<String, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide = |text: &std::ffi::OsStr| text.encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let operation = wide(std::ffi::OsStr::new("open"));
    let file_wide = wide(file.as_os_str());
    // SAFETY: both buffers are NUL-terminated UTF-16 that outlive the call; a null
    // window, parameter string and directory are documented as allowed.
    let instance = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW returns a value above 32 on success; at or below it, an error code.
    let code = instance as isize;
    if code > 32 {
        Ok("the system default".to_string())
    } else {
        Err(format!("Windows could not open {} (error {code}) — {SET_THE_TOOL}", file.display()))
    }
}

#[cfg(not(windows))]
fn open_with_shell_association(file: &Path) -> Result<String, String> {
    Err(format!("{}: the shell association exists only on Windows — {SET_THE_TOOL}", file.display()))
}

/// Under WSL a Linux binary cannot reach Windows' associations, so the file's Windows
/// path goes to wslu's `wslview`. The translation runs a process, so it happens on a
/// thread of its own and its failure arrives like an exit report.
fn open_with_wslview(file: PathBuf, reports: mpsc::Sender<String>) -> Result<String, String> {
    let search = ProgramSearch::from_process();
    let wslview = search
        .find("wslview", &[])
        .map_err(|not_found| format!("{} — install wslu for the system default under WSL", not_found.sentence("wslview", None)))?;
    let wslpath = search.find("wslpath", &[]).map_err(|not_found| not_found.sentence("wslpath", None))?;
    std::thread::Builder::new()
        .name("external-tool-wslview".to_string())
        .spawn(move || {
            let report = match windows_path(&wslpath, &file) {
                Ok(windows_path) => spawn_watched(&wslview, &[windows_path], "wslview", ExitWording::Plain, reports.clone())
                    .err()
                    .map(|error| format!("Failed to start wslview: {error}")),
                Err(_) => Some(format!("wslpath could not translate {} to a Windows path", file.display())),
            };
            if let Some(report) = report {
                let _ = reports.send(report);
            }
        })
        .map_err(|error| format!("Failed to start wslview: {error}"))?;
    Ok("the system default".to_string())
}

#[cfg(test)]
#[path = "spawn_tests.rs"]
mod tests;
