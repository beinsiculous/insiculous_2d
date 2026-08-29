//! Keyboard shortcuts and play state transitions.


use glam::Vec2;
use winit::keyboard::KeyCode;

use editor::world_snapshot::WorldSnapshot;
use editor::{EditorAction, EditorPlayState, EditorTool, PlayControlAction};
use engine_core::contexts::GameContext;
use engine_core::Game;

use crate::entity_ops;

use super::EditorGame;

impl<G: Game> EditorGame<G> {
    /// Handle a play control action (Play, Pause, Stop).
    ///
    /// Returns `true` if a Stop was performed (world restored from snapshot),
    /// so the caller can notify the inner game via `on_play_stopped`.
    pub(super) fn handle_play_action(&mut self, action: PlayControlAction, world: &mut ecs::World) -> bool {
        // Any play-state transition kills an in-flight viewport gesture:
        // handle_input runs in BOTH play and edit modes since #42, so a
        // button held across a transition could otherwise complete a
        // phantom click/marquee in the new state (kimi #42 F5).
        if !matches!(action, PlayControlAction::ToggleCameraFollow) {
            self.editor.viewport_input.cancel_marquee();
        }
        match action {
            PlayControlAction::Play => {
                if self.editor.is_editing() {
                    // Cancel any in-progress gizmo drag (state only — the
                    // world already holds the dragged values; Play snapshots
                    // them and Stop restores)
                    self.gizmo_drag = None;
                    self.editor.gizmo.cancel();
                    // Dropping a live drag is a gesture boundary too (#56
                    // kimi F1): pre-Play and post-Stop nudges must not merge
                    // into one undo entry across the discarded drag.
                    self.command_history.break_merge();
                    // An open command-API batch commits NOW: its commands
                    // are already applied to the world the snapshot is
                    // about to capture, and a macro pushed after Stop's
                    // restore would undo against the wrong world.
                    if let Some(batch) = self.api_batch.take() {
                        if !batch.commands.is_empty() {
                            self.command_history.push_already_executed(Box::new(
                                editor::commands::MacroCommand::new(batch.name, batch.commands),
                            ));
                        }
                        self.editor.status_bar.show_message("API batch committed by Play");
                    }
                    // Starting a new play session — capture snapshot.
                    // (Resume-from-pause takes the branch below and must
                    // never re-capture: the paused world is mid-simulation.)
                    let snapshot = WorldSnapshot::capture(world);
                    if let Some(warning) = snapshot.loss_warning() {
                        self.editor.status_bar.show_message(warning);
                    }
                    self.world_snapshot = Some(snapshot);
                    // Save the editing pan/zoom and adopt the game camera's
                    // pose — position AND zoom (the ecs Camera carries zoom;
                    // issue #42 stopped dropping it). No main-camera entity:
                    // zoom 1.0, parity with how such a game renders outside
                    // the editor. Follow re-arms at every SESSION START only
                    // (kimi R2-F8: pause→resume preserves a user's toggle).
                    self.editing_camera = Some((
                        self.editor.viewport.camera_position(),
                        self.editor.viewport.camera_zoom(),
                    ));
                    self.editor.set_camera_follow(true);
                    match engine_core::main_camera_pose(world) {
                        Some((pos, zoom)) => {
                            self.editor.viewport.set_camera_position(pos);
                            self.editor.viewport.adopt_camera_zoom(zoom);
                        }
                        None => self.editor.viewport.set_camera_zoom(1.0),
                    }
                    self.editor.set_play_state(EditorPlayState::Playing);
                    self.editor.close_add_component_popup();
                    // Scene-authored UI (UiLabel/UiPanel/UiButton) draws only
                    // while the game actually runs.
                    world.remove_resource::<engine_core::UiElementsHidden>();
                    log::info!("Play: snapshot captured, entering play mode");
                } else if self.editor.is_paused() {
                    // Resuming from pause
                    self.editor.set_play_state(EditorPlayState::Playing);
                    self.editor.close_add_component_popup();
                    log::info!("Play: resumed from pause");
                }
                false
            }
            PlayControlAction::Pause => {
                if self.editor.is_playing() {
                    self.editor.set_play_state(EditorPlayState::Paused);
                    log::info!("Paused");
                }
                false
            }
            PlayControlAction::Stop => {
                if self.editor.in_play_session() {
                    // An API batch opened while Paused holds commands
                    // referencing the mid-simulation world the restore below
                    // discards — a later `batch end` would push a macro that
                    // undoes against the wrong world. Drop it with the
                    // runtime state (kimi F2).
                    if let Some(batch) = self.api_batch.take() {
                        if !batch.commands.is_empty() {
                            self.editor
                                .status_bar
                                .show_message("Open API batch discarded by Stop");
                        }
                    }
                    // Restore world from snapshot
                    if let Some(snapshot) = self.world_snapshot.take() {
                        // The loss happens HERE, so report it here too — the
                        // Play-time warning is easy to miss.
                        let drop_report = snapshot.drop_report();
                        let dropped_full_paths = snapshot.uncaptured_types().join(", ");
                        snapshot.restore(world);
                        // The world was wholesale-replaced: drop the transform
                        // system's propagation baselines so no stale cache
                        // entry survives the restore.
                        self.transform_system.reset();
                        log::info!("Stop: world restored from snapshot");
                        if let Some(report) = drop_report {
                            // Status bar gets display names; the log keeps the
                            // full type paths (matching the capture-time log).
                            log::warn!("Stop: dropped unregistered component type(s): {}", dropped_full_paths);
                            self.editor.status_bar.show_message(report);
                        }
                    }
                    // Restore the pan/zoom the user had while editing, and
                    // re-arm the camera follow for the next session.
                    if let Some((position, zoom)) = self.editing_camera.take() {
                        self.editor.viewport.set_camera_position(position);
                        self.editor.viewport.set_camera_zoom(zoom);
                    }
                    self.editor.set_camera_follow(true);
                    // Re-hide scene-authored UI (the marker was removed when
                    // Play started; resources survive the snapshot restore).
                    world.insert_resource(engine_core::UiElementsHidden);
                    self.editor.set_play_state(EditorPlayState::Editing);
                    true
                } else {
                    false
                }
            }
            PlayControlAction::ToggleCameraFollow => {
                if self.editor.in_play_session() {
                    self.editor.toggle_camera_follow();
                    let message = if self.editor.is_camera_following() {
                        "Following game camera"
                    } else {
                        "Free camera — Ctrl+Shift+F or Follow to re-follow"
                    };
                    self.editor.status_bar.show_message(message);
                }
                false
            }
        }
    }

    /// Top-level key handler: every editor shortcut resolves through the
    /// ONE rebindable table (`EditorInputMapping::resolve` — audit §4.9).
    /// Play controls always work; while Playing the raw key forwards to the
    /// game WITHOUT resolving editor actions (Q must reach the playtested
    /// game); unresolved keys forward to the game while Editing too.
    pub(super) fn handle_editor_key(&mut self, key: KeyCode, ctx: &mut GameContext) {
        // A focused text input (inspector value box) owns the keyboard:
        // Delete/Backspace edit the buffer, they must not delete the entity.
        // Enter/Tab/Escape are handled by the widget itself, which clears focus.
        if ctx.ui.wants_keyboard() {
            return;
        }

        let ctrl = ctx.input.keyboard().is_key_pressed(KeyCode::ControlLeft)
            || ctx.input.keyboard().is_key_pressed(KeyCode::ControlRight);
        let shift = ctx.input.keyboard().is_key_pressed(KeyCode::ShiftLeft)
            || ctx.input.keyboard().is_key_pressed(KeyCode::ShiftRight);
        let action = self.editor.input_mapping.resolve(key, ctrl, shift);

        // Play-control actions (always intercepted, in any play state)
        match action {
            Some(EditorAction::StopPlay) => {
                if self.handle_play_action(PlayControlAction::Stop, ctx.world) {
                    self.inner.on_play_stopped(ctx);
                }
                return;
            }
            Some(EditorAction::TogglePlayPause) => {
                if self.editor.is_playing() {
                    self.handle_play_action(PlayControlAction::Pause, ctx.world);
                } else {
                    self.handle_play_action(PlayControlAction::Play, ctx.world);
                }
                return;
            }
            // Always intercepted (Ctrl+Shift+F must work while Playing);
            // a no-op outside a play session.
            Some(EditorAction::ToggleCameraFollow) => {
                if self.editor.in_play_session() {
                    self.handle_play_action(PlayControlAction::ToggleCameraFollow, ctx.world);
                }
                return;
            }
            _ => {}
        }

        // While Playing the raw key belongs to the game — editor actions
        // are deliberately NOT dispatched here.
        if self.editor.is_playing() {
            self.inner.on_key_pressed(key, ctx);
            return;
        }

        match action {
            Some(action) => self.dispatch_editor_action(action, shift, ctx),
            None => self.inner.on_key_pressed(key, ctx),
        }
    }

    /// Execute one resolved editor action (Editing/Paused only — the caller
    /// has already peeled off play controls and the Playing state).
    ///
    /// Guards: file-replacing actions rely on the #22 `in_play_session`
    /// choke points (refused while Paused too); entity edits run while
    /// Paused by design (warn-don't-block); transform/existence-mutating
    /// actions are suppressed while a gizmo drag is live — a mid-drag nudge
    /// would be silently swallowed by the drag's start→final commit.
    fn dispatch_editor_action(&mut self, action: EditorAction, shift: bool, ctx: &mut GameContext) {
        use EditorAction as A;
        let drag_live = self.gizmo_drag.is_some();
        let drag_guard = |game: &mut Self| {
            if drag_live {
                game.editor
                    .status_bar
                    .show_message("Finish or Escape the drag first");
            }
            drag_live
        };
        match action {
            A::Undo => {
                if drag_guard(self) {
                    return;
                }
                self.command_history.undo(ctx.world);
            }
            A::Redo => {
                if drag_guard(self) {
                    return;
                }
                self.command_history.redo(ctx.world);
            }
            A::Save => {
                if let Err(e) = self.save_scene(ctx.world, ctx.assets) {
                    self.editor.status_bar.show_error(format!("Save failed: {}", e));
                    log::error!("Failed to save: {}", e);
                }
            }
            A::SaveAs => {
                let path = self.default_scene_path();
                if let Err(e) = self.save_scene_as(ctx.world, ctx.assets, path) {
                    self.editor.status_bar.show_error(format!("Save failed: {}", e));
                    log::error!("Failed to save: {}", e);
                }
            }
            A::NewScene => self.new_scene(ctx.world),
            A::OpenScene => {
                let path = self.default_scene_path();
                self.load_scene_with_feedback(ctx.world, ctx.assets, &path);
            }
            A::Duplicate => {
                if drag_guard(self) {
                    return;
                }
                self.duplicate_selected_entities(ctx);
            }
            A::Delete => {
                if drag_guard(self) {
                    return;
                }
                self.delete_selected_entities(ctx);
            }
            A::Copy => self.copy_selection(ctx),
            A::Paste => {
                if drag_guard(self) {
                    return;
                }
                self.paste_clipboard(ctx);
            }
            A::Cut => {
                if drag_guard(self) {
                    return;
                }
                self.cut_selection(ctx);
            }
            A::SelectAll => {
                let all = entity_ops::selectable_entities(ctx.world);
                if !all.is_empty() {
                    let count = all.len();
                    self.editor.selection.select_multiple(all);
                    self.editor
                        .status_bar
                        .show_message(format!("Selected {count} entities"));
                }
            }
            A::Cancel => self.cancel_cascade(ctx.world),
            A::NudgeLeft => self.nudge_selection(ctx.world, Vec2::new(-1.0, 0.0), shift),
            A::NudgeRight => self.nudge_selection(ctx.world, Vec2::new(1.0, 0.0), shift),
            A::NudgeUp => self.nudge_selection(ctx.world, Vec2::new(0.0, 1.0), shift),
            A::NudgeDown => self.nudge_selection(ctx.world, Vec2::new(0.0, -1.0), shift),
            A::ZoomIn => self.editor.zoom_camera(1.1),
            A::ZoomOut => self.editor.zoom_camera(0.9),
            A::ResetZoom => self.editor.reset_camera(),
            A::ToggleGrid => self.editor.toggle_grid(),
            A::ToggleColliders => self.editor.toggle_colliders(),
            A::ToggleSnap => self.toggle_snap_with_feedback(),
            A::RenameSelected => {
                // Inline-rename the primary selection in the hierarchy.
                // The field opens pre-focused with the current name selected;
                // an entity without a Name opens empty and only materializes
                // one on a non-empty commit (Escape stays a true no-op).
                if let Some(entity) = self.editor.selection.primary() {
                    let initial = ctx
                        .world
                        .get::<ecs::Name>(entity)
                        .map(|n| n.as_str().to_string())
                        .unwrap_or_default();
                    self.editor.hierarchy.begin_rename(entity);
                    ctx.ui.focus_text_input(
                        editor::HierarchyPanel::rename_widget_id(entity).as_str(),
                        &initial,
                    );
                }
            }
            A::PlayResume => {
                self.handle_play_action(PlayControlAction::Play, ctx.world);
            }
            A::ToolSelect => self.editor.set_tool(EditorTool::Select),
            A::ToolMove => self.editor.set_tool(EditorTool::Move),
            A::ToolRotate => self.editor.set_tool(EditorTool::Rotate),
            A::ToolScale => self.editor.set_tool(EditorTool::Scale),
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
