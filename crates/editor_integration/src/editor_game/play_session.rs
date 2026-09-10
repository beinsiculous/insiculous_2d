//! Play session transitions: start, pause, resume, stop, and camera follow.

use editor::world_snapshot::WorldSnapshot;
use editor::{EditorPlayState, PlayControlAction};
use engine_core::Game;

use super::EditorGame;

/// What Stop did with the edits recorded since the Play boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct StopOutcome {
    /// Entries still in the history after the stop.
    pub kept: usize,
    /// Edits that could not be applied (a macro counts each lost child).
    pub dropped: usize,
}

/// What Stop does with the edits recorded since the Play boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PausedEdits {
    /// Rebase them onto the restored world.
    Keep,
    /// Truncate the history to the Play boundary and let the restore stand.
    Discard,
}

impl<G: Game> EditorGame<G> {
    /// Start a new play session: capture world snapshot, save editing camera,
    /// switch to Playing state.
    fn start_play_session(&mut self, world: &mut ecs::World) {
        // Cancel any in-progress gizmo drag (state only — the
        // world already holds the dragged values; Play snapshots
        // them and Stop restores)
        self.gizmo_drag = None;
        self.editor.gizmo.cancel();
        // An asset drag armed before the Play keypress dies here: a drop
        // consumed by a hierarchy row after Play would mutate the live world,
        // and Stop's restore would erase the attachment the status bar had
        // just confirmed.
        self.editor.drag_drop = editor::DragDropState::new();
        // Defensive: entering Play drops a pending confirm —
        // unreachable through the blocked UI, cheap insurance.
        self.scene_confirm.pending_action = None;
        // A queued Keep would otherwise restore a session the user has
        // just resumed.
        self.stop_confirm.pending = false;
        self.stop_confirm.pending_choice = None;
        // Dropping a live drag is a gesture boundary too:
        // pre-Play and post-Stop nudges must not merge
        // into one undo entry across the discarded drag.
        self.command_history.break_merge();
        // An open command-API batch commits NOW: its commands
        // are already applied to the world the snapshot is
        // about to capture, and a macro pushed after Stop's
        // restore would undo against the wrong world.
        self.commit_open_api_batch("Play");
        // The Play boundary is taken AFTER the batch commit: the batch's
        // macro belongs to the authored history, not to the session.
        self.command_history.begin_session();
        // Starting a new play session — capture snapshot.
        // (Resume-from-pause takes the branch below and must
        // never re-capture: the paused world is mid-simulation.)
        let snapshot = WorldSnapshot::capture(world);
        if let Some(warning) = snapshot.loss_warning() {
            self.editor.status_bar.show_message(warning);
        }
        self.world_snapshot = Some(snapshot);
        self.adopt_game_camera(world);
        self.editor.set_play_state(EditorPlayState::Playing);
        self.editor.close_add_component_popup();
        self.play_frames = 0;
        self.script_error_watermark = 0;
        // Scene-authored UI (UiLabel/UiPanel/UiButton) draws only
        // while the game actually runs.
        world.remove_resource::<engine_core::UiElementsHidden>();
        log::info!("Play: snapshot captured, entering play mode");
    }

    pub(super) fn commit_open_api_batch(&mut self, by: &str) {
        if let Some(batch) = self.api.batch.take() {
            if !batch.commands.is_empty() {
                // The macro carries the batch's own pre-batch
                // selection snapshot.
                self.command_history.push_already_executed_with_before(
                    Box::new(editor::commands::MacroCommand::new(
                        batch.name,
                        batch.commands,
                    )),
                    batch.selection_before,
                );
            }
            self.editor.status_bar.show_message(format!("API batch committed by {by}"));
        }
    }

    fn adopt_game_camera(&mut self, world: &ecs::World) {
        // Save the editing pan/zoom and adopt the game camera's
        // pose — position AND zoom (the ecs Camera carries zoom;
        // the runtime stopped dropping it). No main-camera entity:
        // zoom 1.0, parity with how such a game renders outside
        // the editor. Follow re-arms at every SESSION START only
        // (pause→resume preserves a user's toggle).
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
    }

    fn resume_from_pause(&mut self) {
        // Resuming under a live modal would run the simulation the dialog
        // is asking about, and a queued Keep would then restore it.
        self.stop_confirm.pending = false;
        self.stop_confirm.pending_choice = None;
        self.editor.set_play_state(EditorPlayState::Playing);
        self.editor.close_add_component_popup();
        log::info!("Play: resumed from pause");
    }

    fn pause(&mut self) {
        if self.editor.is_playing() {
            self.editor.set_play_state(EditorPlayState::Paused);
            log::info!("Paused");
        }
    }

    /// Park the session Paused so the stop dialog is never a modal over a
    /// running simulation.
    pub(super) fn pause_for_dialog(&mut self) {
        self.pause();
    }

    /// Restore the world the snapshot holds, replaying or dropping the
    /// paused edits. Returns how many could not be applied.
    fn restore_snapshot(&mut self, world: &mut ecs::World, paused_edits: PausedEdits) -> usize {
        let mut unapplied = 0;
        if let Some(snapshot) = self.world_snapshot.take() {
            // The loss happens HERE, so report it here too — the
            // Play-time warning is easy to miss.
            let drop_report = snapshot.drop_report();
            let dropped_full_paths = snapshot.uncaptured_types().join(", ");
            match paused_edits {
                PausedEdits::Discard => snapshot.restore(world),
                PausedEdits::Keep => {
                    unapplied = self
                        .command_history
                        .rebase_session_entries(world, |restored| snapshot.restore(restored));
                }
            }
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
        unapplied
    }

    fn restore_editing_camera(&mut self) {
        if let Some((position, zoom)) = self.editing_camera.take() {
            self.editor.viewport.set_camera_position(position);
            self.editor.viewport.set_camera_zoom(zoom);
        }
        self.editor.set_camera_follow(true);
    }

    /// Stop and report what became of the paused edits.
    /// Only the confirm flow calls this — every other Stop path goes
    /// through `request_stop`, which is what raises the dialog.
    pub(super) fn stop_with_paused_edits(
        &mut self,
        world: &mut ecs::World,
        paused_edits: PausedEdits,
    ) -> StopOutcome {
        if !self.editor.in_play_session() {
            return StopOutcome::default();
        }
        if paused_edits == PausedEdits::Discard {
            self.command_history.drop_session_entries();
        }
        let dropped = self.restore_snapshot(world, paused_edits);
        let kept = self.command_history.session_entry_count();
        self.command_history.end_session();
        self.restore_editing_camera();
        // Re-hide scene-authored UI (the marker was removed when
        // Play started; resources survive the snapshot restore).
        world.insert_resource(engine_core::UiElementsHidden);
        // Spring-grid backdrops rebuild at rest: entity ids survive
        // the restore, so without this a grid stopped mid-ripple
        // would stay deformed and frozen.
        engine_core::grid::request_backdrop_reset(world);
        if let Some(errors) = &self.script_errors {
            if let Ok(mut lock) = errors.lock() {
                lock.clear();
            }
        }
        self.editor.set_play_state(EditorPlayState::Editing);
        StopOutcome { kept, dropped }
    }

    /// The no-paused-edits Stop: restore and report that it happened.
    pub(super) fn stop_play_session(&mut self, world: &mut ecs::World, paused_edits: PausedEdits) -> bool {
        if !self.editor.in_play_session() {
            return false;
        }
        self.stop_with_paused_edits(world, paused_edits);
        true
    }

    fn toggle_camera_follow_with_feedback(&mut self) {
        if self.editor.in_play_session() {
            self.editor.toggle_camera_follow();
            let message = if self.editor.is_camera_following() {
                "Following game camera"
            } else {
                "Free camera — Ctrl+Shift+F or Follow to re-follow"
            };
            self.editor.status_bar.show_message(message);
        }
    }

    /// Whether a preview window currently owns the simulation.
    fn preview_is_open(&self) -> bool {
        self.preview_open
            .as_ref()
            .map(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or(false)
    }

    /// Handle a play control action (Play, Pause, Stop, ToggleCameraFollow).
    ///
    /// Returns `true` if a Stop was performed (world restored from snapshot),
    /// so the caller can notify the inner game via `on_play_stopped`.
    pub(super) fn handle_play_action(&mut self, action: PlayControlAction, world: &mut ecs::World) -> bool {
        // Any play-state transition kills an in-flight viewport gesture:
        // handle_input runs in BOTH play and edit modes, so a
        // button held across a transition could otherwise complete a
        // phantom click/marquee in the new state.
        if !matches!(action, PlayControlAction::ToggleCameraFollow) {
            self.editor.viewport_input.cancel_marquee();
        }
        match action {
            PlayControlAction::Play => {
                if self.editor.is_editing() {
                    if self.preview_is_open() {
                        self.editor
                            .status_bar
                            .show_error("A preview window is open — close it to Play here");
                        return false;
                    }
                    self.save_preferences_now();
                    self.start_play_session(world);
                } else if self.editor.is_paused() {
                    self.resume_from_pause();
                }
                false
            }
            PlayControlAction::Pause => {
                self.pause();
                false
            }
            PlayControlAction::Stop => {
                let stopped = self.request_stop(world);
                self.save_preferences_now();
                stopped
            }
            PlayControlAction::ToggleCameraFollow => {
                self.toggle_camera_follow_with_feedback();
                false
            }
        }
    }
}
