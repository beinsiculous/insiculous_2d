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

mod menu_actions;
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
    /// Initial transform captured when gizmo drag starts.
    gizmo_drag_start: Option<common::Transform2D>,
    /// Initial collider captured when gizmo drag starts (scale tool resizes
    /// colliders alongside the transform — physics ignores Transform2D.scale).
    gizmo_drag_start_collider: Option<physics::components::Collider>,
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
            gizmo_drag_start: None,
            gizmo_drag_start_collider: None,
            physics_settings: None,
            editing_camera: None,
            editor_font: None,
            game_base_font: None,
            frozen_time_scale: None,
        }
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

    /// While Playing, mirror the game's main-camera entity onto the editor
    /// viewport so the rendered view (derived from the viewport in `render`)
    /// follows the game camera. Paused keeps the frozen view — the user may
    /// pan/zoom while paused and picking stays truthful.
    pub(super) fn sync_viewport_from_main_camera(&mut self, world: &ecs::World) {
        if !self.editor.is_playing() {
            return;
        }
        if let Some(pos) = engine_core::main_camera_position(world) {
            self.editor.viewport.set_camera_position(pos);
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
        let theme = &self.editor.theme;
        if let Some(action) = self.editor.play_controls.render(ctx.ui, play_state, theme) {
            if self.handle_play_action(action, ctx.world) {
                self.inner.on_play_stopped(ctx);
            }
        }
    }

    /// Render the dock panel frames and their content. Returns the panel
    /// content areas for later viewport/gizmo hit testing.
    fn render_panels(&mut self, ctx: &mut GameContext) -> Vec<(editor::PanelId, common::Rect)> {
        let theme = &self.editor.theme;
        let content_areas = self.editor.dock_area.render(ctx.ui, theme);

        for (panel_id, bounds) in content_areas.clone() {
            ctx.ui.push_clip_rect(ui::Rect::new(bounds.x, bounds.y, bounds.width, bounds.height));
            panel_renderer::render_panel_content(
                &mut self.editor, ctx, panel_id, bounds, &mut self.command_history,
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

    /// Load persisted editor preferences (camera, grid, panel layout).
    fn load_preferences(&mut self) {
        let prefs = editor::EditorPreferences::load(std::path::Path::new(EDITOR_PREFS_PATH));
        self.editor.set_camera_offset(Vec2::new(prefs.camera_position.0, prefs.camera_position.1));
        self.editor.set_camera_zoom(prefs.camera_zoom);
        self.editor.set_snap_to_grid(prefs.snap_to_grid);
        self.editor.set_grid_size(prefs.grid_size);
        prefs.apply_panels(&mut self.editor.dock_area);
    }

    /// Capture and save editor preferences. Failures are logged, not fatal.
    fn save_preferences(&self) {
        let mut prefs = editor::EditorPreferences {
            camera_position: (self.editor.camera_offset().x, self.editor.camera_offset().y),
            camera_zoom: self.editor.camera_zoom(),
            last_scene_path: self
                .editor
                .scene_path()
                .and_then(|p| p.to_str())
                .map(|s| s.to_string()),
            snap_to_grid: self.editor.is_snap_to_grid(),
            grid_size: self.editor.grid_size(),
            panels: Vec::new(),
        };
        prefs.capture_panels(&self.editor.dock_area);
        if let Err(e) = prefs.save(std::path::Path::new(EDITOR_PREFS_PATH)) {
            log::warn!("Failed to save editor preferences: {}", e);
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
}

impl<G: Game> Game for EditorGame<G> {
    fn init(&mut self, ctx: &mut GameContext) {
        // Editor look for generic ui widgets (buttons, sliders, inputs):
        // derive the ui theme from the editor palette once at startup.
        ctx.ui.set_theme(self.editor.theme.ui_theme());

        // Load font from common search paths
        let font_paths = [
            "assets/fonts/font.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
            "C:\\Windows\\Fonts\\arial.ttf",
        ];

        for path in font_paths {
            if let Ok(handle) = ctx.ui.load_font_file(path) {
                self.font_loaded = true;
                self.editor_font = Some(handle);
                log::info!("Editor font loaded from: {}", path);
                break;
            }
        }

        if !self.font_loaded {
            log::warn!("No font loaded. Text will render as placeholders.");
            log::warn!("To enable font rendering, add a .ttf file to examples/assets/fonts/font.ttf");
        }

        // Restore camera/grid/panel layout from the previous session
        self.load_preferences();

        // Scene-authored UI elements stay hidden while Editing/Paused —
        // removed on Play, re-inserted on Stop. Standalone games never
        // insert this, so their UI always draws.
        ctx.world.insert_resource(engine_core::UiElementsHidden);

        // Delegate to inner game
        self.inner.init(ctx);

        // Whatever font the game set up is the game view's baseline; locale
        // fonts layer on top of it during play (see update_inner_game).
        self.game_base_font = ctx.ui.default_font();
    }

    fn update(&mut self, ctx: &mut GameContext) {
        let window_size = ctx.window_size;

        // 0. Freeze engine-side time unless we're Playing. Set before the
        // inner game runs so a Playing game's own write to `time_scale`
        // (a pause menu, say) is the value that survives the frame.
        ctx.time_scale = self.editor_time_scale(ctx.time_scale);

        // 0b. Editor chrome always renders in the editor font — re-asserted
        // every frame because the engine applies locale fonts after update.
        if let Some(editor_font) = self.editor_font {
            ctx.ui.set_default_font(editor_font);
        }

        // 0c. Interpolate the viewport camera toward its targets — this is
        // what makes scroll zoom, pan, Home, and focus_on actually move the
        // view (every setter writes target_* only). Runs before the play-mode
        // camera sync, which sets camera and target together, so while
        // Playing this is a no-op and the game camera stays authoritative.
        self.editor.update_viewport(ctx.delta_time);

        // 1. Run transform hierarchy system
        self.transform_system.update(ctx.world, ctx.delta_time);

        // 1b. While Playing, the game's main camera drives the viewport
        self.sync_viewport_from_main_camera(ctx.world);

        // 2. Editor layout
        self.editor.update_layout(window_size);

        // 2b. Drag-and-drop state machine + ghost. Runs before panels so the
        // ghost's overlay blocking rect makes widgets under the cursor inert
        // for the whole frame (the overlay depth band keeps it on top).
        self.editor.drag_drop.begin_frame(
            ctx.ui.mouse_pos(),
            ctx.ui.mouse_down(),
            ctx.ui.mouse_just_released(),
        );
        panel_renderer::render_drag_ghost(&mut self.editor, ctx);

        // 3. Menu bar + action dispatch
        self.handle_menu_bar(ctx, window_size);

        // 4. Toolbar + play controls
        self.render_toolbar_and_play_controls(ctx);

        // 5. Dock panels + content
        let content_areas = self.render_panels(ctx);

        // 6. Viewport input (pan, zoom, click, rectangle selection)
        self.handle_viewport_picking(ctx);

        // 7. Gizmo interaction for the selected entity
        self.handle_gizmo(ctx, &content_areas);

        // 8. Tool keyboard shortcuts (skip during play)
        if !self.editor.is_playing() {
            self.handle_tool_shortcuts(ctx);
        }

        // 9. Delegate to inner game (only when Playing)
        self.update_inner_game(ctx);

        // 10. Status bar
        self.render_status_bar(ctx, window_size);
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
    }

    fn on_key_pressed(&mut self, key: KeyCode, ctx: &mut GameContext) {
        self.handle_editor_key(key, ctx);
    }

    fn on_key_released(&mut self, key: KeyCode, ctx: &mut GameContext) {
        self.inner.on_key_released(key, ctx);
    }

    fn on_resize(&mut self, width: u32, height: u32) {
        self.inner.on_resize(width, height);
    }

    fn on_exit(&mut self) {
        self.save_preferences();
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
pub fn run_game_with_editor<G: Game>(game: G, config: GameConfig) -> Result<(), Box<dyn std::error::Error>> {
    let config = clamp_editor_window_size(config);
    let editor_game = EditorGame::new(game);
    engine_core::run_game(editor_game, config)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod picking_tests;
#[cfg(test)]
mod play_guard_tests;
#[cfg(test)]
mod time_freeze_tests;
