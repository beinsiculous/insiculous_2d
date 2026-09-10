//! Editor run options and runner entry points.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::constants::clamp_editor_window_size;
use engine_core::{Game, GameConfig};

use super::EditorGame;

/// Options for [`run_game_with_editor_opts`].
#[derive(Default)]
pub struct EditorRunOptions {
    /// Command-API request channel.
    pub api_rx: Option<Receiver<String>>,
    /// A scene to open through the editor's load path right after init —
    /// how the standalone binary hands over its project's first scene.
    pub initial_scene: Option<PathBuf>,
    /// Command-API response channel (for web bridge FIFO responses).
    pub api_responses: Option<Sender<String>>,
    /// Path or storage slot key for editor preferences.
    pub prefs_slot: Option<PathBuf>,
    /// Dirty flag written by `sync_dirty_mirror`.
    pub dirty_flag: Option<Arc<AtomicBool>>,
    /// Persistence-pending flag read by `sync_dirty_mirror`.
    pub persist_pending: Option<Arc<AtomicBool>>,
    /// Web error mirror for script errors.
    pub script_errors: Option<Arc<Mutex<Vec<String>>>>,
    /// Mailbox the web bridge files preview snapshot requests in.
    pub scene_snapshot: Option<Arc<super::SceneSnapshotRequest>>,
    /// Raised while a preview window owns the simulation.
    pub preview_open: Option<Arc<AtomicBool>>,
}

/// Run a game with the full editor UI overlay.
///
/// This wraps the given game in `EditorGame`, which intercepts all `Game` trait
/// methods to add editor chrome (menu bar, toolbar, dock panels, hierarchy,
/// inspector, gizmo, tool shortcuts, play/pause/stop) around the user's game.
///
/// # Minimum window size
/// The editor needs at least 1024x720 to be usable. If the provided config
/// specifies a smaller size, it will be enlarged.
pub fn run_game_with_editor<G: Game>(
    game: G,
    config: GameConfig,
) -> Result<(), engine_core::EngineError> {
    run_game_with_editor_opts(game, config, EditorRunOptions::default())
}

/// [`run_game_with_editor`] with the full option set.
pub fn run_game_with_editor_opts<G: Game>(
    game: G,
    config: GameConfig,
    options: EditorRunOptions,
) -> Result<(), engine_core::EngineError> {
    let config = clamp_editor_window_size(config);
    let mut editor_game = EditorGame::new(game);
    editor_game.api.receiver = options.api_rx;
    editor_game.api.responses = options.api_responses;
    editor_game.initial_scene = options.initial_scene;
    if let Some(slot) = options.prefs_slot {
        editor_game.prefs_slot = slot;
    }
    editor_game.dirty_flag = options.dirty_flag;
    editor_game.persist_pending = options.persist_pending;
    editor_game.script_errors = options.script_errors;
    editor_game.scene_snapshot = options.scene_snapshot;
    editor_game.preview_open = options.preview_open;
    engine_core::run_game(editor_game, config)
}
