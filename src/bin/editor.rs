//! Standalone editor binary for the insiculous_2d engine.
//!
//! Opens the editor UI pointed at a game project directory.
//!
//! Usage:
//!   cargo run --bin editor --features editor -- /path/to/project
//!   cargo run --bin editor --features editor              # defaults to "."
//!   cargo run --bin editor --features editor -- /path/to/project --headless
//!       # no window: line-oriented command API on stdin/stdout (Stage C)

use std::path::PathBuf;

use engine_core::prelude::*;
use editor_integration::{
    find_first_scene, run_game_with_editor_opts, EditorRunOptions, ProjectHost,
};

/// Spawn the stdin reader feeding the command API.
/// The thread only moves bytes; dispatch happens on the frame thread.
/// Ends on stdin EOF/error or when the editor side drops the receiver.
fn spawn_api_stdin_reader() -> std::sync::mpsc::Receiver<String> {
    use std::io::BufRead as _;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break;
            }
        }
        log::info!("command API: stdin closed, reader stopped");
    });
    rx
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Args: [project-path] [--api] [--headless]. `--api` answers
    // line-oriented queries from stdin with JSON on stdout alongside the
    // window; `--headless` implies the API but never opens a window at all
    // (docs/EDITOR_COMMAND_API.md — Stage C). Only the known flags are
    // treated as flags — any other argument is a positional path, even one
    // starting with `--`.
    let (flags, paths): (Vec<String>, Vec<String>) =
        std::env::args().skip(1).partition(|a| a == "--api" || a == "--headless");
    let headless = flags.iter().any(|f| f == "--headless");

    let project_path: PathBuf = paths
        .into_iter()
        .next()
        .unwrap_or_else(|| ".".into())
        .into();

    if headless {
        // Logging stays on stderr (env_logger); stdout is protocol-clean.
        let scene =
            editor_integration::find_first_scene(&project_path.join("assets").join("scenes"));
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        if let Err(e) = editor_integration::run_headless_editor_api(
            Some(project_path.join("assets")),
            scene,
            stdin.lock(),
            stdout.lock(),
        ) {
            log::error!("Headless editor error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let api_rx = (!flags.is_empty()).then(spawn_api_stdin_reader);

    let project_name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Insiculous 2D Editor");

    let title = format!("Insiculous 2D Editor — {}", project_name);

    let config = GameConfig::new(&title)
        .with_size(1280, 720)
        .with_clear_color(0.1, 0.1, 0.15, 1.0);

    // First scene in SORTED order: opened by EditorGame through its
    // real load path, so the title, physics block, and Ctrl+S target are
    // right from frame one.
    let initial_scene = find_first_scene(&project_path.join("assets").join("scenes"));
    if initial_scene.is_none() {
        log::info!("no scene found under assets/scenes — starting with an empty scene");
    }

    let opts = EditorRunOptions {
        api_rx,
        initial_scene,
        ..Default::default()
    };
    if let Err(e) = run_game_with_editor_opts(ProjectHost::new(project_path), config, opts) {
        log::error!("Editor error: {}", e);
        std::process::exit(1);
    }
}
