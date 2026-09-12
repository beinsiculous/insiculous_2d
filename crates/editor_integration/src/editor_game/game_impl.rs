//! `Game for EditorGame<G>` — the delegating half of the wrapper.
//!
//! Every trait method forwards to the inner game, or hands the frame to the
//! editor's named phases. No state lives here: the wrapper's own methods and
//! fields stay in [`super`], which this module reaches as a child of it.

use winit::keyboard::KeyCode;

use engine_core::contexts::{GameContext, RenderContext};
use engine_core::{AchievementManager, Game, Strings};

use super::{build_pickable_entities, EditorGame};
use crate::panel_renderer;

impl<G: Game> Game for EditorGame<G> {
    fn register_achievements(&self, achievements: &mut AchievementManager, strings: &Strings) {
        self.inner.register_achievements(achievements, strings);
    }

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
        // After the confirm dialogs, which own the frame while they are up,
        // and before every other widget: a blocking rect is consulted at
        // each widget's own interact call, so the popup's rect has to exist
        // before the menu bar, the strip or any panel draws one under it.
        let dialog_up = self.scene_confirm.pending_action.is_some() || self.stop_confirm.pending;
        panel_renderer::color_editor::render_color_editor_pass(&mut self.editor, ctx.ui, dialog_up);
        self.handle_menu_bar(ctx, window_size);
        self.render_toolbar_and_play_controls(ctx);
        // Before the API drain, which skips mid-drag: this path only reads
        // the world, and the bridge's caller gives up after five seconds.
        self.answer_scene_snapshot(ctx);
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
        self.render_panels(ctx, &pickables);
        self.handle_viewport_picking(ctx.ui, ctx.input, ctx.world, &pickables);
        self.handle_gizmo(ctx);
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

    fn register_scripts(&mut self, registry: &mut engine_core::scripting::ScriptRegistry) {
        self.inner.register_scripts(registry);
    }
}
