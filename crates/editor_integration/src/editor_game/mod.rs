//! Editor-wrapped game implementation.
//!
//! `EditorGame<G>` transparently wraps any `Game` implementation, intercepting
//! all trait methods to weave in editor UI orchestration (menu bar, toolbar,
//! dock panels, hierarchy, inspector, gizmo, tool shortcuts, play/pause/stop)
//! and delegating to the inner game.
//!
//! The wrapper is split by feature:
//! - [`game_impl`] — the `Game` trait impl: delegation and the frame's phases
//! - [`menu_actions`] — menu bar rendering and action dispatch
//! - [`scene_io`] — scene save/load/new
//! - [`song_io`] — the song beside the scene: new/open/save/close and its API verbs
//! - [`shortcuts`] — keyboard shortcuts and play state transitions
//! - [`viewport_interaction`] — viewport picking and gizmo dragging

use glam::Vec2;

use editor::EditorContext;
use editor::world_snapshot::WorldSnapshot;
use engine_core::scene_data::PhysicsSettings;
use engine_core::Game;

use crate::constants::EDITOR_PREFS_PATH;

mod api;
mod asset_browser_keys;
mod conductor_keys;
mod console;
mod game_impl;
mod frame;
mod gizmo_drag;
pub mod headless;
mod external_tools;
mod hot_reload;
mod menu_actions;
mod panel_press;
mod play_session;
mod preferences;
mod run_options;
mod scene_confirm;
mod scene_io;
mod settings_keys;
mod snapshot;
mod song_entry;
mod song_io;
mod init_music;
mod song_preview;
mod stop_confirm;
#[cfg(test)]
mod stop_confirm_tests;
mod script_status;
mod shortcuts;
mod viewport_interaction;

pub(crate) use viewport_interaction::{build_pickable_entities, chrome_owns_mouse};
pub use run_options::{run_game_with_editor, run_game_with_editor_opts, EditorRunOptions, SongImport};
pub use snapshot::{Completion, SceneSnapshot, SceneSnapshotRequest};

/// Wraps a user's `Game` with the full editor UI overlay.
struct EditorGame<G: Game> {
    inner: G,
    editor: EditorContext,
    font_loaded: bool,
    ui_scale: editor::editor_preferences::UiScale,
    resolved_ui_scale: f32,
    /// Snapshot of the world state captured when entering play mode.
    world_snapshot: Option<WorldSnapshot>,
    /// Auto-incrementing counter for unique entity names.
    entity_counter: u32,
    /// Undo/redo command history for editor actions.
    command_history: editor::CommandHistory,
    /// The Conductor panel's song: a second document with its own history and save watermark.
    pub(super) song: Option<editor::SongDocument>,
    /// Bumped each time the song is created, opened or closed, so the Conductor panel's state
    /// resets instead of pointing into a song that is gone.
    pub(super) song_generation: u64,
    /// The song preview and the session's music: intents drained once a frame.
    pub(super) song_preview: song_preview::SongPreview,
    /// What the game's `init` asked the audio for, asked again by a session with no song in
    /// the Conductor panel.
    pub(super) init_music: init_music::InitMusic,
    /// The song pending or playing in the music slot, as the song phase last read it.
    pub(super) slot_song: Option<std::path::PathBuf>,
    /// Live gizmo drag: start transforms/colliders for every selection root,
    /// captured when the drag begins (applied idempotently, committed as one
    /// undo entry on release, restored verbatim on Escape).
    gizmo_drag: Option<gizmo_drag::GizmoDragState>,
    /// Entity clipboard (Ctrl+C/X/V): captured selection-root subtrees.
    /// Survives scene changes harmlessly — paste just recreates components.
    clipboard: Vec<editor::ClipboardEntity>,
    /// Physics settings for scene serialization.
    physics_settings: Option<PhysicsSettings>,
    /// Editing pan/zoom saved while a play session runs (restored on Stop).
    editing_camera: Option<(Vec2, f32)>,
    /// The editor chrome's font, pinned at init and re-asserted every frame
    /// so locale font switches never restyle panels/menus.
    editor_font: Option<ui::FontHandle>,
    /// The default font right after the inner game's init — what the game
    /// view uses when no locale font is active.
    game_base_font: Option<ui::FontHandle>,
    /// The game's own `time_scale`, held while the editor freezes engine
    /// time outside Play mode. `None` means time is not currently frozen.
    frozen_time_scale: Option<f32>,
    /// Last OS-window title published via `ctx.set_window_title`, so the
    /// title (a window-system round-trip) is only re-sent on change.
    last_window_title: Option<String>,
    pub(super) api: api::ApiSession,
    pub(super) scene_confirm: scene_confirm::SceneConfirm,
    pub(super) stop_confirm: stop_confirm::StopConfirm,
    /// Scene to open through the editor load path right after `init`
    /// (the standalone binary passes it via `EditorRunOptions` so
    /// scene_path/physics/dirty-state are recorded like any other load).
    initial_scene: Option<std::path::PathBuf>,
    pub(super) asset_base: std::path::PathBuf,
    pub(super) prefs_slot: std::path::PathBuf,
    pub(super) last_saved_prefs: Option<editor::EditorPreferences>,
    pub(super) pending_prefs: Option<editor::EditorPreferences>,
    pub(super) prefs_stable_time: f32,
    pub(super) dirty_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// History-only mirror of `dirty_flag`, written by the same
    /// [`Self::sync_dirty_mirror`] pass. The combined flag lags a
    /// completed put by up to a frame, so a reader asking "are there
    /// unsaved edits?" must consult this one instead.
    pub(super) history_dirty_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub(super) persist_pending: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub(super) script_errors: Option<std::sync::Arc<std::sync::Mutex<Vec<String>>>>,
    pub(super) play_frames: u32,
    pub(super) script_error_watermark: usize,
    /// The preview's snapshot mailbox, when the web bridge installed one.
    pub(super) scene_snapshot: Option<std::sync::Arc<snapshot::SceneSnapshotRequest>>,
    /// Set while a preview window holds the simulation. Play here is refused
    /// so two simulations of one scene never run at once.
    pub(super) preview_open: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// The open song's `.mid` for a wasm host, when the web bridge installed one.
    pub(super) open_song_mirror: Option<std::sync::Arc<std::sync::Mutex<Option<std::path::PathBuf>>>>,
    /// The import of the open song's `.mid` a wasm host filed, drained once a frame.
    pub(super) song_import: Option<std::sync::Arc<std::sync::Mutex<Option<SongImport>>>>,
    /// Opens requested files in their external tools and collects what the launches report.
    launcher: external_tools::Launcher,
    /// The watched project files, and what their changes on disk wait on.
    hot_reload: hot_reload::HotReload,
}

impl<G: Game> EditorGame<G> {
    fn new(game: G) -> Self {
        Self {
            inner: game,
            editor: EditorContext::new(),
            font_loaded: false,
            ui_scale: editor::editor_preferences::UiScale::FollowDisplay,
            resolved_ui_scale: 1.0,
            world_snapshot: None,
            entity_counter: 0,
            command_history: editor::CommandHistory::new(),
            song: None,
            song_generation: 0,
            song_preview: song_preview::SongPreview::default(),
            init_music: init_music::InitMusic::None,
            slot_song: None,
            gizmo_drag: None,
            clipboard: Vec::new(),
            physics_settings: None,
            editing_camera: None,
            editor_font: None,
            game_base_font: None,
            frozen_time_scale: None,
            last_window_title: None,
            api: api::ApiSession::default(),
            scene_confirm: scene_confirm::SceneConfirm::default(),
            stop_confirm: stop_confirm::StopConfirm::default(),
            initial_scene: None,
            asset_base: std::path::PathBuf::new(),
            prefs_slot: std::path::PathBuf::from(EDITOR_PREFS_PATH),
            last_saved_prefs: None,
            pending_prefs: None,
            prefs_stable_time: 0.0,
            dirty_flag: None,
            history_dirty_flag: None,
            persist_pending: None,
            script_errors: None,
            play_frames: 0,
            script_error_watermark: 0,
            scene_snapshot: None,
            preview_open: None,
            open_song_mirror: None,
            song_import: None,
            launcher: external_tools::Launcher::new(),
            hot_reload: hot_reload::HotReload::default(),
        }
    }

    /// The window title to publish this frame, or `None` when unchanged.
    /// Change-gated because `Window::set_title` is a window-system
    /// round-trip that must not run every frame.
    fn pending_title_update(&mut self) -> Option<String> {
        let title = self.editor.title_bar_text();
        if self.last_window_title.as_deref() == Some(title.as_str()) {
            return None;
        }
        self.last_window_title = Some(title.clone());
        Some(title)
    }

    /// The engine time multiplier to run this frame, given the one the game
    /// last asked for.
    ///
    /// Outside Play mode the answer is always `0.0`: the game's `update()`
    /// does not run, so anything the engine steps on its own — particles,
    /// sprite animations — would otherwise drift while the scene sits still
    /// in the editor. The game's own value is held and handed back when Play
    /// resumes, so a game that was running at half speed still is.
    ///
    /// Takes and returns a plain `f32` rather than a `GameContext` so the
    /// gate is testable headless, the same shape as `PauseMenu`.
    fn editor_time_scale(&mut self, game_time_scale: f32) -> f32 {
        if self.editor.is_playing() {
            return self.frozen_time_scale.take().unwrap_or(game_time_scale);
        }
        self.frozen_time_scale.get_or_insert(game_time_scale);
        0.0
    }


    /// Mirror the dirty flags from their sources of truth: a command was
    /// recorded in the history ⇒ the scene changed; the song's own history (or
    /// a song never saved) ⇒ the song did. The single place the mirrors are
    /// written; it runs in `finish_frame` and after a save or a document reset,
    /// so anything reading dirtiness earlier in a frame consults the histories.
    /// The host's two flags carry either document's edits, so the playground's
    /// unsaved-work checks cover the song without a bridge change.
    pub(super) fn sync_dirty_mirror(&mut self) {
        self.editor.conductor.reconcile_active_track(self.song.as_ref());
        let is_command_dirty = self.command_history.is_dirty();
        let is_song_dirty = self.song.as_ref().is_some_and(editor::SongDocument::is_dirty);
        let is_persist_pending = self
            .persist_pending
            .as_ref()
            .map(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or(false);
        let dirty = is_command_dirty || is_persist_pending;
        self.editor.set_dirty(dirty);
        self.editor.set_song_mirror(self.song.as_ref().map(|song| editor::SongMirror {
            name: song
                .path()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Untitled song".to_string()),
            dirty: is_song_dirty,
        }));
        if let Some(flag) = &self.dirty_flag {
            flag.store(dirty || is_song_dirty, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(flag) = &self.history_dirty_flag {
            flag.store(is_command_dirty || is_song_dirty, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(Ok(mut open_song)) = self.open_song_mirror.as_ref().map(|mirror| mirror.lock()) {
            *open_song = self.song.as_ref().and_then(editor::SongDocument::path).map(std::path::Path::to_path_buf);
        }
    }
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod api_tests;
#[cfg(test)]
mod asset_browser_keys_tests;
#[cfg(test)]
mod camera_follow_tests;
#[cfg(test)]
mod conductor_keys_tests;
#[cfg(test)]
mod console_tests;
#[cfg(test)]
mod gizmo_drag_tests;
#[cfg(test)]
mod history_dirty_tests;
#[cfg(test)]
mod hot_reload_tests;
#[cfg(test)]
mod hot_reload_sheet_tests;
#[cfg(test)]
mod hot_reload_sample_tests;
#[cfg(test)]
mod hot_reload_song_tests;
#[cfg(test)]
mod panel_press_tests;
#[cfg(test)]
mod play_session_tests;
#[cfg(test)]
mod preferences_tests;
#[cfg(test)]
mod scene_confirm_tests;
#[cfg(test)]
mod scene_io_tests;
#[cfg(test)]
mod settings_keys_tests;
#[cfg(test)]
mod snapshot_tests;
#[cfg(test)]
mod song_entry_tests;
#[cfg(test)]
mod song_io_tests;
#[cfg(test)]
mod song_preview_session_tests;
#[cfg(test)]
mod song_preview_tests;
#[cfg(test)]
mod shortcuts_tests;
#[cfg(test)]
mod viewport_interaction_tests;
#[cfg(test)]
mod scale_tests;
