//! Editor frame orchestration, chrome, and the inner game boundary.

use glam::Vec2;
use editor::EditorPlayState;
use engine_core::{Game, contexts::GameContext};
use crate::panel_renderer;
use super::EditorGame;

impl<G: Game> EditorGame<G> {
    /// Render the scene view's toolbar strip: the tools and the play
    /// controls, laid out by the strip and drawn on its band.
    pub(super) fn render_toolbar_and_play_controls(&mut self, ctx: &mut GameContext) {
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
    /// A drag's target may sit behind a tab: Assets and the Conductor share a group, so a
    /// sample dragged from one has no content of the other to land on until its tab is
    /// brought forward. A payload that rests on a tab of a group already showing content
    /// (`DRAG_DWELL_FRAMES`, so a drag crossing a strip on its way to the scene switches
    /// nothing) activates that tab; the payload rides on. A collapsed strip or a narrow tab
    /// stays as the user left it: `show_panel` there would expand the group, and the settle
    /// rule would save a layout the user never chose. Runs before the dock draws, so the
    /// strip and the content of one frame come from one layout.
    fn bring_forward_the_tab_under_a_drag(&mut self, ui: &ui::UIContext) {
        if self.editor.drag_drop.dragging_payload().is_none() {
            self.editor.dock_area.end_drag();
            return;
        }
        let Some(id) = self.editor.dock_area.tab_rested_on(ui.mouse_pos()) else { return; };
        let dock = &self.editor.dock_area;
        if !dock.is_active(id) && dock.group_shows_content(id) { self.editor.dock_area.activate(id); }
    }

    /// A dragged tab lands where a Window-menu move does — after the menu bar, before the
    /// toolbar strip and the dock draw — and through that menu's own path, its refusal and its
    /// `show_panel` included, so the release frame draws the strip and the panels in the new
    /// layout; taken after the dock draws, as a click is, that frame would draw the old panels
    /// in the old rectangles. A dialog or a menu ends the drag with nothing moved.
    pub(super) fn drop_a_dragged_tab(&mut self, ui: &ui::UIContext) {
        let chrome_open = self.dialog_up() || self.editor.menu_bar.is_open()
            || self.editor.context_menu.is_open() || self.editor.toolbar.is_overflow_open();
        if chrome_open { self.editor.dock_area.cancel_tab_drag(); }
        if let Some(action) = self.editor.dock_area.take_tab_drop(ui) {
            self.run_context_menu_action(editor::context_menu::ContextMenuAction::Dock(action));
        }
    }

    pub(super) fn render_panels(&mut self, ctx: &mut GameContext, pickables: &[editor::PickableEntity]) {
        self.bring_forward_the_tab_under_a_drag(ctx.ui);
        let theme = &self.editor.theme;
        let content_areas = self.editor.dock_area.render(ctx.ui, theme);
        self.release_inactive_panel_focus();
        if let Some((id, anchor)) = self.editor.dock_area.take_tab_context_request() {
            self.editor.context_menu.open_for_tab(anchor, id, &self.editor.dock_area);
        }
        // A panel the dock leaves out — a narrow-mode tab, a hidden panel —
        // never renders, so its rename bookkeeping runs from here instead;
        // an undrawn rename field would otherwise keep the keyboard.
        if !content_areas.iter().any(|(panel_id, _)| *panel_id == editor::PanelId::HIERARCHY) {
            self.editor.hierarchy.settle_rename_focus(ctx.ui);
        }

        for (panel_id, bounds) in content_areas {
            ctx.ui.push_clip_rect(ui::Rect::new(bounds.x, bounds.y, bounds.width, bounds.height));
            // The Conductor panel renders through its host, which holds the song and the drag.
            if panel_id == editor::PanelId::CONDUCTOR {
                self.render_conductor_panel(ctx, bounds);
            } else {
                panel_renderer::render_panel_content(
                    &mut self.editor,
                    ctx,
                    panel_id,
                    bounds,
                    &mut self.command_history,
                    pickables,
                );
            }
            ctx.ui.pop_clip_rect();
        }

        // After the content loop so the hover/drag grabber draws on top.
        self.editor.dock_area.handle_resize(ctx.ui, &self.editor.theme);
    }

    /// Delegate the frame to the inner game — only while Playing, clipped to
    /// the scene view and rendered in the game's (or active locale's) font.
    pub(super) fn update_inner_game(&mut self, ctx: &mut GameContext) {
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
        ctx.ui.set_pixels_per_point(1.0);
        self.inner.update(ctx);
        ctx.ui.set_pixels_per_point(self.resolved_ui_scale);
        if let Some(editor_font) = self.editor_font {
            ctx.ui.set_default_font(editor_font);
        }

        if self.editor.scene_view_bounds().is_some() {
            ctx.ui.pop_clip_rect();
        }
        self.track_script_status(ctx.world, ctx.scripts);
    }


    /// Update status bar stats and render it.
    pub(super) fn render_status_bar(&mut self, ctx: &mut GameContext, window_size: Vec2) {
        let fps = if ctx.delta_time > 0.0 { 1.0 / ctx.delta_time } else { 0.0 };
        let smoothed_fps = fps.min(999.0); // Cap for display
        self.editor.status_bar.update_stats(ctx.world.entity_count(), smoothed_fps);
        self.editor.status_bar.update(ctx.delta_time);

        self.draw_status_bar(ctx.ui, window_size);
    }

    /// Prepare the frame: freeze engine-side time, re-assert the editor font,
    /// interpolate the viewport camera toward its targets, note the selection
    /// before handlers mutate it, update transform hierarchy, sync viewport
    /// camera from main camera if playing, and update layout.
    pub(super) fn points_window_size(&self, window_physical: Vec2) -> Vec2 {
        window_physical / self.resolved_ui_scale
    }

    pub(super) fn prepare_frame(&mut self, ctx: &mut GameContext) {
        self.resolved_ui_scale = editor::editor_preferences::resolve_ui_scale(&self.ui_scale, ctx.display_scale);
        ctx.ui.set_pixels_per_point(self.resolved_ui_scale);
        // First, so the session's first frame runs at the game's own time scale.
        let editing = !self.editor.in_play_session();
        self.start_armed_play(ctx.world);
        // A tab drag begun before Play ends with nothing moved when the session starts.
        if editing && self.editor.in_play_session() { self.editor.dock_area.cancel_tab_drag(); }
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

        self.sync_viewport_from_main_camera(ctx.world);

        self.editor.update_layout(self.points_window_size(ctx.window_size));
    }

    /// Early modal overlays: confirm dialog scrim must land before the drag ghost
    /// can arm a gesture, so no widget arms under a modal.
    pub(super) fn render_early_overlays(&mut self, ctx: &mut GameContext) {
        if let Some((tile, name)) = panel_renderer::asset_browser::mutate::render_prompt(&mut self.editor, ctx) {
            let texture_path_fn = |handle: u32| -> String { super::scene_io::texture_ref_for_save(handle, ctx.assets.texture_path(handle)) };
            let live = self.unsaved_document_texts(ctx.world, &texture_path_fn);
            self.confirm_asset_rename(tile, name, live);
            if !self.editor.asset_prompt.is_open() { ctx.ui.clear_text_focus(); }
        }
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
    pub(super) fn finish_frame(&mut self, ctx: &mut GameContext) {
        self.apply_song_intents(ctx.audio, ctx.sources);
        self.report_song_replacements(ctx.audio);
        // Nothing on screen moves outside a play session: `prepare_frame`
        // already froze engine time, so no particle or sprite animation is
        // live in Editing or Paused and a still window can stop asking for
        // frames — except a sounding preview, whose playhead moves. A song
        // waiting for the web's first click does not hold the page awake.
        ctx.set_idle_throttle_ok(self.idle_throttle_allowed());

        self.sync_dirty_mirror();
        // After the mirror, so a reload sees this frame's dirty state, and before the bar.
        self.drain_song_import();
        self.drain_file_changes(ctx);
        self.save_preferences_if_changed(ctx.delta_time);
        self.open_requested_files();

        self.render_status_bar(ctx, self.points_window_size(ctx.window_size));

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
        ctx.ui_scale_for_engine_tail = Some(1.0);
    }

    /// Whether the engine may skip frames while the window sits idle: outside a running
    /// session, unless the preview is sounding — its playhead moves.
    pub(super) fn idle_throttle_allowed(&self) -> bool {
        matches!(self.editor.play_state(), EditorPlayState::Editing | EditorPlayState::Paused)
            && !self.song_preview.is_audible()
    }

}
