//! Editor-wrapped game implementation.
//!
//! `EditorGame<G>` transparently wraps any `Game` implementation, intercepting
//! all trait methods to weave in editor UI orchestration (menu bar, toolbar,
//! dock panels, hierarchy, inspector, gizmo, tool shortcuts, play/pause/stop)
//! and delegating to the inner game.
//!
//! The wrapper is split by feature:
//! - [`menu_actions`] — menu bar rendering and action dispatch
//! - [`scene_io`] — scene save/load/new
//! - [`shortcuts`] — keyboard shortcuts and play state transitions
//! - [`viewport_interaction`] — viewport picking and gizmo dragging

use glam::Vec2;
use winit::keyboard::KeyCode;

use ecs::System;
use editor::EditorContext;
use editor::world_snapshot::WorldSnapshot;
use engine_core::contexts::{GameContext, RenderContext};
use engine_core::scene_data::PhysicsSettings;
use engine_core::Game;
use engine_core::GameConfig;

use crate::constants::{clamp_editor_window_size, EDITOR_PREFS_PATH};
use crate::panel_renderer;

mod api;
mod gizmo_drag;
pub mod headless;
mod menu_actions;
mod play_session;
mod preferences;
mod scene_confirm;
mod scene_io;
mod shortcuts;
mod viewport_interaction;

pub(crate) use viewport_interaction::{build_pickable_entities, chrome_owns_mouse};

/// Wraps a user's `Game` with the full editor UI overlay.
struct EditorGame<G: Game> {
    inner: G,
    editor: EditorContext,
    transform_system: ecs::TransformHierarchySystem,
    font_loaded: bool,
    /// Snapshot of the world state captured when entering play mode.
    world_snapshot: Option<WorldSnapshot>,
    /// Auto-incrementing counter for unique entity names.
    entity_counter: u32,
    /// Undo/redo command history for editor actions.
    command_history: editor::CommandHistory,
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
    pub(super) persist_pending: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

impl<G: Game> EditorGame<G> {
    fn new(game: G) -> Self {
        Self {
            inner: game,
            editor: EditorContext::new(),
            transform_system: ecs::TransformHierarchySystem::new(),
            font_loaded: false,
            world_snapshot: None,
            entity_counter: 0,
            command_history: editor::CommandHistory::new(),
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
            initial_scene: None,
            asset_base: std::path::PathBuf::new(),
            prefs_slot: std::path::PathBuf::from(EDITOR_PREFS_PATH),
            last_saved_prefs: None,
            pending_prefs: None,
            prefs_stable_time: 0.0,
            dirty_flag: None,
            persist_pending: None,
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

    /// While Playing WITH camera-follow armed, mirror the game's main-camera
    /// entity — position AND zoom — onto the editor viewport so
    /// the rendered view (derived from the viewport in `render`) follows the
    /// game camera. Free camera (follow broken by a manual pan/zoom) and
    /// Paused keep the user's view — picking stays truthful either way,
    /// because render always derives from the same viewport.
    pub(super) fn sync_viewport_from_main_camera(&mut self, world: &ecs::World) {
        if !self.editor.is_playing() || !self.editor.is_camera_following() {
            return;
        }
        if let Some((pos, zoom)) = engine_core::main_camera_pose(world) {
            self.editor.viewport.set_camera_position(pos);
            // adopt_ skips the interactive zoom clamp: parity with the
            // shipped game even at extreme authored zooms.
            self.editor.viewport.adopt_camera_zoom(zoom);
        }
    }

    /// Render the toolbar and the play controls next to it.
    fn render_toolbar_and_play_controls(&mut self, ctx: &mut GameContext) {
        // The toolbar floats inside the scene view — follow it as panels
        // hide/collapse/resize.
        if let Some(scene_bounds) = self.editor.scene_view_bounds() {
            self.editor.toolbar.set_position(editor::toolbar_position_for(scene_bounds));
        }

        if let Some(tool) = self.editor.toolbar.render(ctx.ui, &self.editor.theme) {
            // set_tool keeps the gizmo mode in sync with the clicked tool.
            self.editor.set_tool(tool);
        }

        let toolbar_bounds = self.editor.toolbar.bounds();
        self.editor.play_controls.position = Vec2::new(
            toolbar_bounds.x + toolbar_bounds.width + self.editor.play_controls.spacing * 4.0,
            toolbar_bounds.y,
        );
        let play_state = self.editor.play_state();
        let camera_follow = self.editor.is_camera_following();
        let theme = &self.editor.theme;
        if let Some(action) =
            self.editor.play_controls.render(ctx.ui, play_state, camera_follow, theme)
        {
            if self.handle_play_action(action, ctx.world) {
                self.inner.on_play_stopped(ctx);
            }
        }
    }

    /// Render the dock panel frames and their content. Returns the panel
    /// content areas for later viewport/gizmo hit testing.
    fn render_panels(
        &mut self,
        ctx: &mut GameContext,
        pickables: &[editor::PickableEntity],
    ) -> Vec<(editor::PanelId, common::Rect)> {
        let theme = &self.editor.theme;
        let content_areas = self.editor.dock_area.render(ctx.ui, theme);

        for (panel_id, bounds) in content_areas.clone() {
            ctx.ui.push_clip_rect(ui::Rect::new(bounds.x, bounds.y, bounds.width, bounds.height));
            panel_renderer::render_panel_content(
                &mut self.editor,
                ctx,
                panel_id,
                bounds,
                &mut self.command_history,
                pickables,
            );
            ctx.ui.pop_clip_rect();
        }

        // After the content loop so the hover/drag grabber draws on top.
        self.editor.dock_area.handle_resize(ctx.ui, &self.editor.theme);

        content_areas
    }

    /// Delegate the frame to the inner game — only while Playing, clipped to
    /// the scene view and rendered in the game's (or active locale's) font.
    fn update_inner_game(&mut self, ctx: &mut GameContext) {
        if !self.editor.is_playing() {
            return;
        }
        if let Some(scene_bounds) = self.editor.scene_view_bounds() {
            ctx.ui.push_clip_rect(ui::Rect::new(
                scene_bounds.x, scene_bounds.y, scene_bounds.width, scene_bounds.height,
            ));
        }

        // Scope the default font to the game's frame: the locale font when
        // one is active, otherwise the game's own — never the editor's.
        if let Some(game_font) = ctx.strings.active_font().or(self.game_base_font) {
            ctx.ui.set_default_font(game_font);
        }
        self.inner.update(ctx);
        if let Some(editor_font) = self.editor_font {
            ctx.ui.set_default_font(editor_font);
        }

        if self.editor.scene_view_bounds().is_some() {
            ctx.ui.pop_clip_rect();
        }
    }


    /// Update status bar stats and render it.
    fn render_status_bar(&mut self, ctx: &mut GameContext, window_size: Vec2) {
        let fps = if ctx.delta_time > 0.0 { 1.0 / ctx.delta_time } else { 0.0 };
        let smoothed_fps = fps.min(999.0); // Cap for display
        self.editor.status_bar.update_stats(ctx.world.entity_count(), smoothed_fps);
        self.editor.status_bar.update(ctx.delta_time);

        let theme = &self.editor.theme;
        self.editor.status_bar.render(ctx.ui, window_size, theme);
    }

    /// Prepare the frame: freeze engine-side time, re-assert the editor font,
    /// interpolate the viewport camera toward its targets, note the selection
    /// before handlers mutate it, update transform hierarchy, sync viewport
    /// camera from main camera if playing, and update layout.
    fn prepare_frame(&mut self, ctx: &mut GameContext) {
        // Freeze engine-side time unless we're Playing. Set before the
        // inner game runs so a Playing game's own write to `time_scale`
        // (a pause menu, say) is the value that survives the frame.
        ctx.time_scale = self.editor_time_scale(ctx.time_scale);

        // Editor chrome always renders in the editor font — re-asserted
        // every frame because the engine applies locale fonts after update.
        if let Some(editor_font) = self.editor_font {
            ctx.ui.set_default_font(editor_font);
        }

        // Interpolate the viewport camera toward its targets — this is
        // what makes scroll zoom, pan, Home, and focus_on actually move the
        // view (every setter writes target_* only). Runs before the play-mode
        // camera sync, which sets camera and target together, so while
        // Playing this is a no-op and the game camera stays authoritative.
        self.editor.update_viewport(ctx.delta_time);

        // Note the selection BEFORE any handler this frame mutates it: every
        // command recorded later this frame carries it as the before-image
        // undo restores. Delete/Cut clear the selection before
        // pushing, which is exactly why the note happens here.
        self.command_history.note_selection(&self.editor.selection);

        self.transform_system.update(ctx.world, ctx.delta_time);

        self.sync_viewport_from_main_camera(ctx.world);

        self.editor.update_layout(ctx.window_size);
    }

    /// Early modal overlays: confirm dialog scrim must land before the drag ghost
    /// can arm a gesture, so no widget arms under a modal.
    fn render_early_overlays(&mut self, ctx: &mut GameContext) {
        self.render_scene_confirm_dialog(ctx);

        self.editor.drag_drop.begin_frame(
            ctx.ui.mouse_pos(),
            ctx.ui.mouse_down(),
            ctx.ui.mouse_just_released(),
        );
        panel_renderer::render_drag_ghost(&mut self.editor, ctx);
    }

    /// Complete the frame: sync dirty mirror, render status bar, publish
    /// window title on change, and clip engine UI to the scene viewport.
    fn finish_frame(&mut self, ctx: &mut GameContext) {
        self.sync_dirty_mirror();
        self.save_preferences_if_changed(ctx.delta_time);

        self.render_status_bar(ctx, ctx.window_size);

        // Publish the scene name + dirty indicator as the OS window
        // title (title_bar_text() finally has a caller).
        // While Playing the running game owns the title (it may write
        // ctx.set_window_title itself); forgetting ours makes Stop republish
        // even if the game changed the OS title in the meantime.
        if self.editor.is_playing() {
            self.last_window_title = None;
        } else if !ctx.window_title_requested() {
            if let Some(title) = self.pending_title_update() {
                ctx.set_window_title(title);
            }
        }

        // Clip the engine's post-update draws (the frame tail's
        // UiLabel/UiPanel/UiButton pass and toasts run after this method
        // returns and painted over editor chrome). A plain
        // trailing push_clip_rect would poison later-flushed UI layers
        // (Floating menus, the Modal dialog — bands reorder at end_frame),
        // so the engine wraps only its own tail draws in this rect.
        let bounds = self
            .editor
            .scene_view_bounds()
            .unwrap_or(common::Rect::new(0.0, 0.0, 0.0, 0.0));
        ctx.clip_engine_ui(bounds);
    }

    /// Mirror the dirty flag from its source of truth: a command was recorded
    /// in the history ⇒ the scene changed. The single place the mirror is
    /// written; it runs in `finish_frame` and after a save or scene reset, so
    /// anything reading dirtiness earlier in a frame consults the history.
    pub(super) fn sync_dirty_mirror(&mut self) {
        let is_command_dirty = self.command_history.is_dirty();
        let is_persist_pending = self
            .persist_pending
            .as_ref()
            .map(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or(false);
        let dirty = is_command_dirty || is_persist_pending;
        self.editor.set_dirty(dirty);
        if let Some(flag) = &self.dirty_flag {
            flag.store(dirty, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

impl<G: Game> Game for EditorGame<G> {
    fn init(&mut self, ctx: &mut GameContext) {
        // Editor look for generic ui widgets (buttons, sliders, inputs):
        // derive the ui theme from the editor palette once at startup.
        ctx.ui.set_theme(self.editor.theme.ui_theme());

        // Restore camera/grid/panel layout from the previous session
        self.load_preferences();

        // Scene-authored UI elements stay hidden while Editing/Paused —
        // removed on Play, re-inserted on Stop. Standalone games never
        // insert this, so their UI always draws.
        ctx.world.insert_resource(engine_core::UiElementsHidden);

        // Delegate to inner game
        self.inner.init(ctx);
        self.asset_base = std::path::PathBuf::from(ctx.assets.base_path());

        // Whatever font the game set up is the game view's baseline; locale
        // fonts layer on top of it during play (see update_inner_game).
        // Captured BEFORE the editor faces load: the game's font is the
        // first loaded and therefore the auto-claimed default — loading
        // DejaVu first would poison this capture and reskin the game view.
        self.game_base_font = ctx.ui.default_font();

        // The editor's chrome faces ship with the editor crate
        // — the old search started at the GAME's assets/fonts/font.ttf, so
        // an opened project's serif skinned the whole editor.
        let load = |ui: &mut ui::UIContext, name: &str, bytes: &[u8]| match ui.load_font(bytes) {
            Ok(handle) => Some(handle),
            Err(e) => {
                log::error!("editor {name} font failed to load: {e}");
                None
            }
        };
        self.editor.fonts = editor::fonts::EditorFonts {
            regular: load(ctx.ui, "regular", editor::fonts::EDITOR_FONT_REGULAR),
            bold: load(ctx.ui, "bold", editor::fonts::EDITOR_FONT_BOLD),
            mono: load(ctx.ui, "mono", editor::fonts::EDITOR_FONT_MONO),
        };
        self.editor_font = self.editor.fonts.regular;
        self.font_loaded = self.editor_font.is_some();
        if let Some(regular) = self.editor_font {
            // Explicit claim: load_font only auto-claims the FIRST font
            // ever loaded, which is the game's when it loaded one.
            ctx.ui.set_default_font(regular);
        } else {
            log::warn!("No editor font loaded. Text will render as placeholders.");
        }

        // Open the initial scene through the REAL editor load path:
        // dry-run guard, scene_path, physics settings + resource, history
        // reset — an old bypass load recorded none of those, so
        // the title stayed "Untitled" and a save silently dropped physics.
        if let Some(path) = self.initial_scene.take() {
            self.load_scene_with_feedback(ctx.world, ctx.assets, &path);
        }
    }

    fn update(&mut self, ctx: &mut GameContext) {
        let window_size = ctx.window_size;
        self.prepare_frame(ctx);
        self.render_early_overlays(ctx);
        self.handle_menu_bar(ctx, window_size);
        self.render_toolbar_and_play_controls(ctx);
        self.drain_api_requests(ctx);
        // Built once per frame: after the last handler that can delete an
        // entity (menu bar, command API) and before the first consumer
        // (panels, picking). A click must not receive a deleted entity, so
        // panel rendering must never delete one. Moves are harmless: a
        // pickable's position is its GlobalTransform2D, which only the
        // transform system writes, in prepare_frame.
        let pickables = if self.editor.is_playing() {
            Vec::new()
        } else {
            build_pickable_entities(ctx.world)
        };
        let content_areas = self.render_panels(ctx, &pickables);
        self.handle_viewport_picking(ctx.ui, ctx.input, ctx.world, &pickables);
        self.handle_gizmo(ctx, &content_areas);
        self.update_inner_game(ctx);
        self.finish_frame(ctx);
    }

    fn render(&mut self, ctx: &mut RenderContext) {
        self.inner.render(ctx);
        // The editor viewport is the single source of truth for the view:
        // derive the GPU camera from it so sprites land inside the scene
        // panel exactly where the overlay (gizmo, picking, grid) expects
        // them. Games that hand-write `ctx.camera` in a custom `render()`
        // are overridden here — the supported path inside the editor is a
        // main-camera entity (mirrored onto the viewport while Playing).
        *ctx.camera = self.editor.viewport.to_window_render_camera(ctx.window_size);
        // Bound the game-world passes to the scene panel: the
        // game stops painting over editor chrome and the GPU stops shading
        // the whole window. A hidden/collapsed panel yields a zero-size
        // rect — no game world at all — never None (full window).
        *ctx.viewport_scissor = Some(
            self.editor
                .scene_view_bounds()
                .unwrap_or(common::Rect::new(0.0, 0.0, 0.0, 0.0)),
        );
    }

    fn on_key_pressed(&mut self, key: KeyCode, ctx: &mut GameContext) {
        self.handle_editor_key(key, ctx);
    }

    fn on_key_released(&mut self, key: KeyCode, ctx: &mut GameContext) {
        // Seal the arrow-nudge merge window: consecutive repeats of a held
        // arrow merged into one NudgeCommand; releasing the key closes that
        // entry so the next hold starts a fresh undo step.
        if !self.editor.is_playing()
            && matches!(
                key,
                KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown
            )
        {
            self.command_history.break_merge();
        }
        self.inner.on_key_released(key, ctx);
    }

    fn on_resize(&mut self, width: u32, height: u32) {
        self.inner.on_resize(width, height);
    }

    fn on_exit(&mut self) {
        self.save_preferences_now();
        self.inner.on_exit();
    }
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
pub fn run_game_with_editor<G: Game>(game: G, config: GameConfig) -> Result<(), engine_core::EngineError> {
    run_game_with_editor_opts(game, config, EditorRunOptions::default())
}

/// Options for [`run_game_with_editor_opts`].
#[derive(Default)]
pub struct EditorRunOptions {
    /// Command-API request channel.
    pub api_rx: Option<std::sync::mpsc::Receiver<String>>,
    /// A scene to open through the editor's load path right after init —
    /// how the standalone binary hands over its project's first scene.
    pub initial_scene: Option<std::path::PathBuf>,
    /// Command-API response channel (for web bridge FIFO responses).
    pub api_responses: Option<std::sync::mpsc::Sender<String>>,
    /// Path or storage slot key for editor preferences.
    pub prefs_slot: Option<std::path::PathBuf>,
    /// Dirty flag written by `sync_dirty_mirror`.
    pub dirty_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// Persistence-pending flag read by `sync_dirty_mirror`.
    pub persist_pending: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
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
    engine_core::run_game(editor_game, config)
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod api_tests;
#[cfg(test)]
mod camera_follow_tests;
#[cfg(test)]
mod gizmo_drag_tests;
#[cfg(test)]
mod play_session_tests;
#[cfg(test)]
mod preferences_tests;
#[cfg(test)]
mod scene_confirm_tests;
#[cfg(test)]
mod scene_io_tests;
#[cfg(test)]
mod shortcuts_tests;
#[cfg(test)]
mod viewport_interaction_tests;
