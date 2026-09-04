//! Play session transitions: start, pause, resume, stop, and camera follow.

use editor::world_snapshot::WorldSnapshot;
use editor::{EditorPlayState, PlayControlAction};
use engine_core::Game;

use super::EditorGame;

impl<G: Game> EditorGame<G> {
    /// Start a new play session: capture world snapshot, save editing camera,
    /// switch to Playing state.
    fn start_play_session(&mut self, world: &mut ecs::World) {
        // Cancel any in-progress gizmo drag (state only — the
        // world already holds the dragged values; Play snapshots
        // them and Stop restores)
        self.gizmo_drag = None;
        self.editor.gizmo.cancel();
        // Defensive: entering Play drops a pending confirm —
        // unreachable through the blocked UI, cheap insurance.
        self.scene_confirm.pending_action = None;
        // Dropping a live drag is a gesture boundary too:
        // pre-Play and post-Stop nudges must not merge
        // into one undo entry across the discarded drag.
        self.command_history.break_merge();
        // An open command-API batch commits NOW: its commands
        // are already applied to the world the snapshot is
        // about to capture, and a macro pushed after Stop's
        // restore would undo against the wrong world.
        self.commit_open_api_batch();
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
        // Scene-authored UI (UiLabel/UiPanel/UiButton) draws only
        // while the game actually runs.
        world.remove_resource::<engine_core::UiElementsHidden>();
        log::info!("Play: snapshot captured, entering play mode");
    }

    fn commit_open_api_batch(&mut self) {
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
            self.editor.status_bar.show_message("API batch committed by Play");
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

    fn discard_open_api_batch(&mut self) {
        if let Some(batch) = self.api.batch.take() {
            if !batch.commands.is_empty() {
                self.editor
                    .status_bar
                    .show_message("Open API batch discarded by Stop");
            }
        }
    }

    fn restore_snapshot(&mut self, world: &mut ecs::World) {
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
    }

    fn restore_editing_camera(&mut self) {
        if let Some((position, zoom)) = self.editing_camera.take() {
            self.editor.viewport.set_camera_position(position);
            self.editor.viewport.set_camera_zoom(zoom);
        }
        self.editor.set_camera_follow(true);
    }

    fn stop_play_session(&mut self, world: &mut ecs::World) -> bool {
        if !self.editor.in_play_session() {
            return false;
        }
        // An API batch opened while Paused holds commands
        // referencing the mid-simulation world the restore below
        // discards — a later `batch end` would push a macro that
        // undoes against the wrong world. Drop it with the
        // runtime state.
        self.discard_open_api_batch();
        self.restore_snapshot(world);
        self.restore_editing_camera();
        // Re-hide scene-authored UI (the marker was removed when
        // Play started; resources survive the snapshot restore).
        world.insert_resource(engine_core::UiElementsHidden);
        // Spring-grid backdrops rebuild at rest: entity ids survive
        // the restore, so without this a grid stopped mid-ripple
        // would stay deformed and frozen.
        engine_core::grid::request_backdrop_reset(world);
        self.editor.set_play_state(EditorPlayState::Editing);
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
                let stopped = self.stop_play_session(world);
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
