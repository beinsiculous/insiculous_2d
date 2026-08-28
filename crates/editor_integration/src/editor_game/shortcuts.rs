//! Keyboard shortcuts and play state transitions.

use std::path::PathBuf;

use winit::keyboard::KeyCode;

use editor::{EditorPlayState, EditorTool, PlayControlAction};
use editor::world_snapshot::WorldSnapshot;
use engine_core::contexts::GameContext;
use engine_core::Game;

use crate::constants::DEFAULT_SCENE_PATH;

use super::EditorGame;

impl<G: Game> EditorGame<G> {
    /// Q/W/E/R tool selection shortcuts.
    pub(super) fn handle_tool_shortcuts(&mut self, ctx: &GameContext) {
        // A focused text input owns the keyboard — typing must not switch tools
        if ctx.ui.wants_keyboard() {
            return;
        }

        let kb = ctx.input.keyboard();

        if kb.is_key_just_pressed(KeyCode::KeyQ) {
            self.editor.set_tool(EditorTool::Select);
        } else if kb.is_key_just_pressed(KeyCode::KeyW) {
            self.editor.set_tool(EditorTool::Move);
        } else if kb.is_key_just_pressed(KeyCode::KeyE) {
            self.editor.set_tool(EditorTool::Rotate);
        } else if kb.is_key_just_pressed(KeyCode::KeyR) {
            self.editor.set_tool(EditorTool::Scale);
        }
    }

    /// Handle a play control action (Play, Pause, Stop).
    ///
    /// Returns `true` if a Stop was performed (world restored from snapshot),
    /// so the caller can notify the inner game via `on_play_stopped`.
    pub(super) fn handle_play_action(&mut self, action: PlayControlAction, world: &mut ecs::World) -> bool {
        match action {
            PlayControlAction::Play => {
                if self.editor.is_editing() {
                    // Cancel any in-progress gizmo drag (state only — the
                    // world already holds the dragged values; Play snapshots
                    // them and Stop restores)
                    self.gizmo_drag = None;
                    self.editor.gizmo.cancel();
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
                    // Save the editing pan/zoom; play renders at zoom 1.0
                    // (parity with the game's own camera, which has no zoom
                    // source), position driven by the main-camera entity.
                    self.editing_camera = Some((
                        self.editor.viewport.camera_position(),
                        self.editor.viewport.camera_zoom(),
                    ));
                    self.editor.viewport.set_camera_zoom(1.0);
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
                    // Restore the pan/zoom the user had while editing
                    if let Some((position, zoom)) = self.editing_camera.take() {
                        self.editor.viewport.set_camera_position(position);
                        self.editor.viewport.set_camera_zoom(zoom);
                    }
                    // Re-hide scene-authored UI (the marker was removed when
                    // Play started; resources survive the snapshot restore).
                    world.insert_resource(engine_core::UiElementsHidden);
                    self.editor.set_play_state(EditorPlayState::Editing);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Top-level key handler: play shortcuts always work; editor shortcuts
    /// apply while Editing/Paused; everything else forwards to the game.
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

        // Play state shortcuts (always intercepted)
        if key == KeyCode::KeyP && ctrl && shift {
            // Ctrl+Shift+P → Stop
            if self.handle_play_action(PlayControlAction::Stop, ctx.world) {
                self.inner.on_play_stopped(ctx);
            }
            return;
        }
        if key == KeyCode::KeyP && ctrl {
            // Ctrl+P → Play/Pause toggle
            if self.editor.is_playing() {
                self.handle_play_action(PlayControlAction::Pause, ctx.world);
            } else {
                self.handle_play_action(PlayControlAction::Play, ctx.world);
            }
            return;
        }

        // During play mode, forward keys to inner game (skip editor shortcuts)
        if self.editor.is_playing() {
            self.inner.on_key_pressed(key, ctx);
            return;
        }

        // Editor shortcuts (only during Editing/Paused)
        match key {
            KeyCode::KeyZ if ctrl && !shift => {
                self.command_history.undo(ctx.world);
            }
            KeyCode::KeyZ if ctrl && shift => {
                self.command_history.redo(ctx.world);
            }
            KeyCode::KeyY if ctrl => {
                self.command_history.redo(ctx.world);
            }
            KeyCode::KeyG => self.editor.toggle_grid(),
            KeyCode::KeyC if !ctrl => self.editor.toggle_colliders(),
            KeyCode::KeyS if ctrl && shift => {
                // Ctrl+Shift+S → Save As
                let path = PathBuf::from(DEFAULT_SCENE_PATH);
                if let Err(e) = self.save_scene_as(ctx.world, ctx.assets, path) {
                    self.editor.status_bar.show_error(format!("Save failed: {}", e));
                    log::error!("Failed to save: {}", e);
                }
            }
            KeyCode::KeyS if ctrl => {
                // Ctrl+S → Save
                if let Err(e) = self.save_scene(ctx.world, ctx.assets) {
                    self.editor.status_bar.show_error(format!("Save failed: {}", e));
                    log::error!("Failed to save: {}", e);
                }
            }
            KeyCode::KeyS => self.toggle_snap_with_feedback(),
            KeyCode::KeyN if ctrl => {
                // Ctrl+N → New Scene
                self.new_scene(ctx.world);
            }
            KeyCode::KeyO if ctrl => {
                // Ctrl+O → Open Scene
                let path = PathBuf::from(DEFAULT_SCENE_PATH);
                self.load_scene_with_feedback(ctx.world, ctx.assets, &path);
            }
            KeyCode::KeyD if ctrl => {
                self.duplicate_selected_entities(ctx);
            }
            KeyCode::Delete | KeyCode::Backspace => {
                self.delete_selected_entities(ctx);
            }
            KeyCode::Equal => self.editor.zoom_camera(1.1),
            KeyCode::Minus => self.editor.zoom_camera(0.9),
            KeyCode::Digit0 => self.editor.reset_camera(),
            KeyCode::F2 => {
                // F2 → inline-rename the primary selection in the hierarchy.
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
            KeyCode::F5 => {
                // F5 → Start/Resume play (only from Editing or Paused)
                self.handle_play_action(PlayControlAction::Play, ctx.world);
            }
            KeyCode::Escape => {
                // Escape aborts an in-flight gizmo drag; the fuller cancel
                // cascade (marquee, deselect) lands with the shortcut
                // unification (#40).
                if !self.cancel_gizmo_drag(ctx.world) {
                    self.inner.on_key_pressed(key, ctx);
                }
            }
            _ => self.inner.on_key_pressed(key, ctx),
        }
    }
}
