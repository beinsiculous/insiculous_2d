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
//! - [`shortcuts`] — keyboard shortcuts and play state transitions
//! - [`viewport_interaction`] — viewport picking and gizmo dragging

use glam::Vec2;

use ecs::System;
use editor::EditorContext;
use editor::world_snapshot::WorldSnapshot;
use engine_core::contexts::GameContext;
use engine_core::scene_data::PhysicsSettings;
use engine_core::Game;

use crate::constants::EDITOR_PREFS_PATH;
use crate::panel_renderer;

mod api;
mod game_impl;
mod gizmo_drag;
pub mod headless;
mod menu_actions;
mod open_source;
mod play_session;
mod preferences;
mod run_options;
mod scene_confirm;
mod scene_io;
mod snapshot;
mod stop_confirm;
#[cfg(test)]
mod stop_confirm_tests;
mod script_status;
mod shortcuts;
mod viewport_interaction;

pub(crate) use viewport_interaction::{build_pickable_entities, chrome_owns_mouse};
pub use run_options::{run_game_with_editor, run_game_with_editor_opts, EditorRunOptions};
pub use snapshot::{Completion, SceneSnapshot, SceneSnapshotRequest};

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


    /// Render the scene view's toolbar strip: the tools and the play
    /// controls, laid out by the strip and drawn on its band.
    fn render_toolbar_and_play_controls(&mut self, ctx: &mut GameContext) {
        // The strip belongs to the scene panel — no panel, no strip.
        let Some(strip) = self.editor.toolbar_strip_bounds() else {
            return;
        };
        let play_state = self.editor.play_state();
        let strip_layout = editor::toolbar_strip::layout(
            strip,
            &self.editor.toolbar,
            &self.editor.play_controls,
            play_state,
        );
        self.editor.play_controls.position = strip_layout.play_controls_origin;

        // The overflow menu goes first: a blocking rect is consulted at each
        // widget's own interact call, and in a window too short to hang the
        // menu below the strip it shifts up over the strip's buttons, which
        // must find its rect already there. Overlay scopes cannot nest, so
        // the menu's scope closes before the strip's opens.
        let overflow_pick = editor::overflow_menu::render_overflow_menu(
            &mut self.editor.toolbar,
            ctx.ui,
            &self.editor.theme,
            &strip_layout,
            &self.editor.view,
        );

        editor::toolbar_strip::begin(ctx.ui, strip, &self.editor.theme);
        let picked_tool = self.editor.toolbar.render(ctx.ui, &self.editor.theme, &strip_layout);
        let camera_follow = self.editor.is_camera_following();
        let theme = &self.editor.theme;
        let play_action =
            self.editor.play_controls.render(ctx.ui, play_state, camera_follow, theme);
        let view_action = strip_layout.view_group.and_then(|origin| {
            editor::view_toggles::render_group(ctx.ui, theme, origin, &self.editor.view)
        });
        editor::toolbar_strip::end(ctx.ui);

        if let Some(tool) = picked_tool {
            self.editor.set_tool(tool);
        }
        if let Some(action) = view_action {
            match action {
                editor::ViewGroupAction::Toggle(toggle) => {
                    self.dispatch_editor_action(toggle.action(), false, ctx);
                }
                editor::ViewGroupAction::ResetLayout => {
                    self.dispatch_editor_action(editor::EditorAction::ResetLayout, false, ctx);
                }
            }
        }
        if let Some(pick) = overflow_pick {
            match pick {
                editor::OverflowPick::Tool(tool) => self.editor.set_tool(tool),
                editor::OverflowPick::Toggle(toggle) => {
                    self.dispatch_editor_action(toggle.action(), false, ctx);
                }
                editor::OverflowPick::ResetLayout => {
                    self.dispatch_editor_action(editor::EditorAction::ResetLayout, false, ctx);
                }
            }
        }
        if let Some(action) = play_action {
            if self.handle_play_action(action, ctx.world) {
                self.inner.on_play_stopped(ctx);
            }
        }
    }

    /// Render the dock panel frames and their content. Picking and the gizmo
    /// take their rect from `scene_view_bounds()`, not from here: the panel's
    /// content area includes the toolbar strip, the viewport does not.
    fn render_panels(&mut self, ctx: &mut GameContext, pickables: &[editor::PickableEntity]) {
        let theme = &self.editor.theme;
        let content_areas = self.editor.dock_area.render(ctx.ui, theme);
        // A panel the dock leaves out — a narrow-mode tab, a hidden panel —
        // never renders, so its rename bookkeeping runs from here instead;
        // an undrawn rename field would otherwise keep the keyboard.
        if !content_areas.iter().any(|(panel_id, _)| *panel_id == editor::PanelId::HIERARCHY) {
            self.editor.hierarchy.settle_rename_focus(ctx.ui);
        }

        for (panel_id, bounds) in content_areas {
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
        self.track_script_status(ctx.world, ctx.scripts);
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
        if self.render_stop_confirm_dialog(ctx) {
            self.inner.on_play_stopped(ctx);
        }

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
        self.open_pending_source();

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
        if let Some(flag) = &self.history_dirty_flag {
            flag.store(is_command_dirty, std::sync::atomic::Ordering::Relaxed);
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
mod camera_follow_tests;
#[cfg(test)]
mod gizmo_drag_tests;
#[cfg(test)]
mod history_dirty_tests;
#[cfg(test)]
mod play_session_tests;
#[cfg(test)]
mod preferences_tests;
#[cfg(test)]
mod scene_confirm_tests;
#[cfg(test)]
mod scene_io_tests;
#[cfg(test)]
mod snapshot_tests;
#[cfg(test)]
mod shortcuts_tests;
#[cfg(test)]
mod viewport_interaction_tests;
