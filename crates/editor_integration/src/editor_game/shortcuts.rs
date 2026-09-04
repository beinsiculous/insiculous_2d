//! Keyboard shortcuts and play state transitions.


use glam::Vec2;
use winit::keyboard::KeyCode;

use editor::{EditorAction, EditorTool, PlayControlAction};
use engine_core::contexts::GameContext;
use engine_core::Game;

use crate::entity_ops;

use super::EditorGame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyRoute {
    Consumed,
    PlayControl(PlayControlAction),
    ForwardToGame,
    Editor { action: EditorAction, shift: bool },
}

impl<G: Game> EditorGame<G> {
    /// Apply the selection undo/redo wants restored: platform
    /// convention is that undoing a Delete/Cut brings the selection back.
    pub(super) fn apply_selection_restore(&mut self) {
        if let Some(ids) = self.command_history.take_selection_restore() {
            self.editor.selection.clear();
            self.editor.selection.select_multiple(ids);
        }
    }

    /// Resolve how a keypress should be handled: dialog, text focus,
    /// play control, forward to game, or editor action.
    pub(super) fn route_editor_key(
        &mut self,
        key: KeyCode,
        keyboard_owned: bool,
        modifiers: editor::Modifiers,
    ) -> KeyRoute {
        // A pending confirm dialog owns the keyboard — checked BEFORE the
        // text-focus gate (a focused field must not swallow
        // the modal's keys): Escape cancels, Enter saves, everything else
        // is swallowed.
        if self.confirm_dialog_consumes_key(key) {
            return KeyRoute::Consumed;
        }

        // A focused text input (inspector value box) owns the keyboard:
        // Delete/Backspace edit the buffer, they must not delete the entity.
        // Enter/Tab/Escape are handled by the widget itself, which clears focus.
        if keyboard_owned {
            return KeyRoute::Consumed;
        }

        let action = self.editor.input_mapping.resolve(key, modifiers.ctrl, modifiers.shift);

        // Play-control actions (always intercepted, in any play state)
        match action {
            Some(EditorAction::StopPlay) => {
                return KeyRoute::PlayControl(PlayControlAction::Stop);
            }
            Some(EditorAction::TogglePlayPause) => {
                let play_action = if self.editor.is_playing() {
                    PlayControlAction::Pause
                } else {
                    PlayControlAction::Play
                };
                return KeyRoute::PlayControl(play_action);
            }
            Some(EditorAction::ToggleCameraFollow) => {
                return if self.editor.in_play_session() {
                    KeyRoute::PlayControl(PlayControlAction::ToggleCameraFollow)
                } else {
                    KeyRoute::Consumed
                };
            }
            _ => {}
        }

        // While Playing the raw key belongs to the game — editor actions
        // are deliberately NOT dispatched here.
        if self.editor.is_playing() {
            return KeyRoute::ForwardToGame;
        }

        match action {
            Some(action) => KeyRoute::Editor {
                action,
                shift: modifiers.shift,
            },
            None => KeyRoute::ForwardToGame,
        }
    }

    /// Top-level key handler: every editor shortcut resolves through the
    /// ONE rebindable table (`EditorInputMapping::resolve`).
    /// Play controls always work; while Playing the raw key forwards to the
    /// game WITHOUT resolving editor actions (Q must reach the playtested
    /// game); unresolved keys forward to the game while Editing too.
    pub(super) fn handle_editor_key(&mut self, key: KeyCode, ctx: &mut GameContext) {
        let wants_keyboard = ctx.ui.wants_keyboard();
        let modifiers = editor::Modifiers::read(ctx.input);
        match self.route_editor_key(key, wants_keyboard, modifiers) {
            KeyRoute::Consumed => {}
            KeyRoute::PlayControl(play_action) => {
                let stopped = play_action == PlayControlAction::Stop;
                if self.handle_play_action(play_action, ctx.world) && stopped {
                    self.inner.on_play_stopped(ctx);
                }
            }
            KeyRoute::ForwardToGame => {
                self.inner.on_key_pressed(key, ctx);
            }
            KeyRoute::Editor { action, shift } => {
                self.dispatch_editor_action(action, shift, ctx);
            }
        }
    }

    /// Whether a gizmo drag is live, telling the user to finish or Escape it
    /// first: a transform- or existence-mutating action landing mid-drag
    /// would be silently swallowed by the drag's start→final commit.
    fn refuse_during_drag(&mut self) -> bool {
        let drag_live = self.gizmo_drag.is_some();
        if drag_live {
            self.editor
                .status_bar
                .show_message("Finish or Escape the drag first");
        }
        drag_live
    }

    fn select_all_entities(&mut self, world: &ecs::World) {
        let all = entity_ops::selectable_entities(world);
        if !all.is_empty() {
            let count = all.len();
            self.editor.selection.select_multiple(all);
            self.editor
                .status_bar
                .show_message(format!("Selected {count} entities"));
        }
    }

    fn begin_rename_of_primary(&mut self, world: &ecs::World, ui: &mut ui::UIContext) {
        // Inline-rename the primary selection in the hierarchy.
        // The field opens pre-focused with the current name selected;
        // an entity without a Name opens empty and only materializes
        // one on a non-empty commit (Escape stays a true no-op).
        if let Some(entity) = self.editor.selection.primary() {
            let initial = world
                .get::<ecs::Name>(entity)
                .map(|name| name.as_str().to_string())
                .unwrap_or_default();
            self.editor.hierarchy.begin_rename(entity);
            ui.focus_text_input(
                editor::HierarchyPanel::rename_widget_id(entity).as_str(),
                &initial,
            );
        }
    }

    fn create_entity_at_view_center(&mut self, archetype: editor::Archetype, world: &mut ecs::World) {
        let spawn_pos = self.editor.viewport.camera_position();
        let entity = entity_ops::create_archetype(
            archetype,
            world,
            &mut self.editor.selection,
            spawn_pos,
            &mut self.entity_counter,
        );
        let cmd = editor::commands::CreateEntityCommand::already_created(world, entity);
        self.command_history.push_already_executed(Box::new(cmd));
    }

    fn reset_layout_with_feedback(&mut self) {
        self.editor.reset_layout();
        self.editor.status_bar.show_message("Layout reset to defaults");
    }

    fn cycle_game_locale_with_feedback(&mut self, strings: &mut engine_core::localization::Strings) {
        strings.cycle_locale();
        self.editor.status_bar.show_message(format!(
            "Game locale: {}",
            strings.current_display_name()
        ));
    }

    fn report_save_result(&mut self, result: Result<(), super::scene_io::SceneIoError>) {
        if let Err(error) = result {
            self.editor.status_bar.show_error(format!("Save failed: {error}"));
            log::error!("Failed to save: {error}");
        }
    }

    fn dispatch_edit_action(
        &mut self,
        action: EditorAction,
        shift: bool,
        ctx: &mut GameContext,
    ) {
        use EditorAction as A;
        match action {
            A::Undo => {
                if self.refuse_during_drag() {
                    return;
                }
                self.undo_with_feedback(ctx.world);
            }
            A::Redo => {
                if self.refuse_during_drag() {
                    return;
                }
                self.redo_with_feedback(ctx.world);
            }
            A::Duplicate => {
                if self.refuse_during_drag() {
                    return;
                }
                self.duplicate_selected_entities(ctx.world);
            }
            A::Delete => {
                if self.refuse_during_drag() {
                    return;
                }
                self.delete_selected_entities(ctx.world);
            }
            A::Copy => self.copy_selection(ctx.world),
            A::Paste => {
                if self.refuse_during_drag() {
                    return;
                }
                self.paste_clipboard(ctx.world);
            }
            A::Cut => {
                if self.refuse_during_drag() {
                    return;
                }
                self.cut_selection(ctx.world);
            }
            A::SelectAll => self.select_all_entities(ctx.world),
            A::Cancel => self.cancel_cascade(ctx.world),
            A::NudgeLeft => self.nudge_selection(ctx.world, Vec2::new(-1.0, 0.0), shift),
            A::NudgeRight => self.nudge_selection(ctx.world, Vec2::new(1.0, 0.0), shift),
            A::NudgeUp => self.nudge_selection(ctx.world, Vec2::new(0.0, 1.0), shift),
            A::NudgeDown => self.nudge_selection(ctx.world, Vec2::new(0.0, -1.0), shift),
            A::RenameSelected => self.begin_rename_of_primary(ctx.world, ctx.ui),
            A::CreateEntity(archetype) => self.create_entity_at_view_center(archetype, ctx.world),
            other => log::error!("{other:?} is not an edit action"),
        }
    }

    fn dispatch_file_action(&mut self, action: EditorAction, ctx: &mut GameContext) {
        use EditorAction as A;
        match action {
            A::Save => {
                let result = self.save_scene(ctx.world, ctx.assets);
                self.report_save_result(result);
            }
            A::SaveAs => {
                let path = self.default_scene_path();
                let result = self.save_scene_as(ctx.world, ctx.assets, path);
                self.report_save_result(result);
            }
            A::NewScene => {
                if self.request_scene_replace(super::scene_confirm::PendingSceneAction::NewScene) {
                    self.new_scene(ctx.world);
                }
            }
            A::OpenScene => {
                let path = self.default_scene_path();
                let action = super::scene_confirm::PendingSceneAction::OpenScene(path);
                if self.request_scene_replace(action.clone()) {
                    self.perform_scene_action(ctx, action);
                }
            }
            A::Exit => ctx.request_exit(),
            other => log::error!("{other:?} is not a file action"),
        }
    }

    fn dispatch_view_action(&mut self, action: EditorAction, ctx: &mut GameContext) {
        use EditorAction as A;
        match action {
            A::ZoomIn => self.editor.zoom_camera(1.1),
            A::ZoomOut => self.editor.zoom_camera(0.9),
            A::ResetZoom => self.editor.reset_camera(),
            A::ToggleGrid => self.editor.toggle_grid(),
            A::ToggleColliders => self.editor.toggle_colliders(),
            A::ToggleSnap => self.toggle_snap_with_feedback(),
            A::TogglePanel(id) => self.editor.dock_area.toggle_panel_visible(id),
            A::ResetLayout => self.reset_layout_with_feedback(),
            A::CycleGameLocale => self.cycle_game_locale_with_feedback(ctx.strings),
            other => log::error!("{other:?} is not a view action"),
        }
    }

    fn dispatch_tool_action(&mut self, action: EditorAction) {
        use EditorAction as A;
        match action {
            A::ToolSelect => self.editor.set_tool(EditorTool::Select),
            A::ToolMove => self.editor.set_tool(EditorTool::Move),
            A::ToolRotate => self.editor.set_tool(EditorTool::Rotate),
            A::ToolScale => self.editor.set_tool(EditorTool::Scale),
            other => log::error!("{other:?} is not a tool action"),
        }
    }

    /// Execute one resolved editor action (Editing/Paused only — the caller
    /// has already peeled off play controls and the Playing state).
    ///
    /// Guards: file-replacing actions rely on the `in_play_session`
    /// choke points (refused while Paused too); entity edits run while
    /// Paused by design (warn-don't-block); transform/existence-mutating
    /// actions are suppressed while a gizmo drag is live — a mid-drag nudge
    /// would be silently swallowed by the drag's start→final commit.
    pub(super) fn dispatch_editor_action(&mut self, action: EditorAction, shift: bool, ctx: &mut GameContext) {
        use EditorAction as A;
        match action {
            A::Undo
            | A::Redo
            | A::Duplicate
            | A::Delete
            | A::Copy
            | A::Paste
            | A::Cut
            | A::SelectAll
            | A::Cancel
            | A::NudgeLeft
            | A::NudgeRight
            | A::NudgeUp
            | A::NudgeDown
            | A::RenameSelected
            | A::CreateEntity(_) => self.dispatch_edit_action(action, shift, ctx),

            A::Save | A::SaveAs | A::NewScene | A::OpenScene | A::Exit => {
                self.dispatch_file_action(action, ctx);
            }

            A::ZoomIn
            | A::ZoomOut
            | A::ResetZoom
            | A::ToggleGrid
            | A::ToggleColliders
            | A::ToggleSnap
            | A::TogglePanel(_)
            | A::ResetLayout
            | A::CycleGameLocale => self.dispatch_view_action(action, ctx),

            A::ToolSelect | A::ToolMove | A::ToolRotate | A::ToolScale => {
                self.dispatch_tool_action(action);
            }

            A::PlayResume => {
                self.handle_play_action(PlayControlAction::Play, ctx.world);
            }

            // Poll-only actions: CONSUMED here (no-op) so an editor-bound
            // key never also reaches the inner game — the viewport polling
            // is their authoritative handler.
            A::Pan
            | A::FocusSelection
            | A::ResetCamera
            | A::Select
            | A::AddToSelection
            | A::ToggleSelection => {}

            // Peeled off by the caller before dispatch.
            A::TogglePlayPause | A::StopPlay | A::ToggleCameraFollow => {}
        }
    }

    /// Undo the top entry and name it on the status bar ("Undo: Delete
    /// Entity") — Edit → Undo and Ctrl+Z share this, so both report it.
    pub(super) fn undo_with_feedback(&mut self, world: &mut ecs::World) {
        if let Some(name) = self.command_history.undo_name() {
            self.editor.status_bar.show_message(format!("Undo: {name}"));
        }
        if self.command_history.undo(world) {
            self.apply_selection_restore();
        }
    }

    /// Redo counterpart of [`Self::undo_with_feedback`].
    pub(super) fn redo_with_feedback(&mut self, world: &mut ecs::World) {
        if let Some(name) = self.command_history.redo_name() {
            self.editor.status_bar.show_message(format!("Redo: {name}"));
        }
        if self.command_history.redo(world) {
            self.apply_selection_restore();
        }
    }

    /// Escape: cancel the most specific live thing, exactly one per press —
    /// a gizmo drag, else a marquee, else the selection.
    pub(super) fn cancel_cascade(&mut self, world: &mut ecs::World) {
        if self.cancel_gizmo_drag(world) {
            return;
        }
        // Pending sub-threshold presses cancel too — Escape must suppress
        // the click that press would otherwise become on release.
        if self.editor.viewport_input.has_pending_marquee() {
            self.editor.viewport_input.cancel_marquee();
            return;
        }
        if !self.editor.selection.is_empty() {
            self.editor.selection.clear();
        }
    }

    /// Arrow-key nudge: move every selection root by one world unit (ten
    /// with Shift), as a merging `NudgeCommand` — OS key-repeat machine-guns
    /// `on_key_pressed`, and the merge plus the `break_merge` seal on key
    /// release turns a held arrow into ONE undo entry.
    pub(super) fn nudge_selection(&mut self, world: &mut ecs::World, direction: Vec2, shift: bool) {
        if self.gizmo_drag.is_some() {
            return;
        }
        let step = if shift { 10.0 } else { 1.0 };
        let delta = direction * step;
        let mut moves = Vec::new();
        for id in entity_ops::selection_roots(world, &self.editor.selection) {
            if let Some(transform) = world.get_mut::<common::Transform2D>(id) {
                let old = transform.position;
                transform.position += delta;
                moves.push((id, old, transform.position));
            }
        }
        if !moves.is_empty() {
            self.command_history
                .try_merge_or_push(Box::new(editor::commands::NudgeCommand::new(moves)));
        }
    }
}
