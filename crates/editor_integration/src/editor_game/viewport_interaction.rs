use glam::Vec2;

use ecs::{GlobalTransform2D, Pair, World};
use editor::PickableEntity;
use engine_core::Game;
use input::InputHandler;
use ui::UIContext;

use crate::entity_ops;

use super::EditorGame;

impl<G: Game> EditorGame<G> {
    /// Handle viewport input: pan/zoom plus click and rectangle selection.
    pub(super) fn handle_viewport_picking(
        &mut self,
        ui: &mut UIContext,
        input: &InputHandler,
        world: &mut World,
        pickables: &[PickableEntity],
    ) {
        if self.editor.is_playing() {
            // While Playing only the CAMERA is live: pan/zoom to
            // inspect anywhere, which breaks the game-camera follow. The
            // explicit return is load-bearing — picking, marquee, asset
            // drops, and framing must never run against a live simulation.
            self.handle_play_mode_camera(ui, input);
            return;
        }

        // Asset drops land before the input-blocked check (there is no ghost
        // overlay on the release frame) and before click handling.
        if let Some(scene_bounds) = self.editor.scene_view_bounds() {
            if let Some((editor::DragPayload::Texture { handle, path }, drop_pos)) =
                self.editor.drag_drop.take_drop_in(scene_bounds)
            {
                self.handle_viewport_texture_drop(world, pickables, handle, &path, drop_pos);
                return;
            }
        }
        // While a drag is in flight (or on its release frame) the viewport
        // must not treat the mouse as a pick/selection click.
        if self.editor.drag_drop.suppresses_click() {
            return;
        }

        self.handle_shared_viewport_input(ui, input, pickables);
    }

    /// Play-mode viewport input: pan/zoom ONLY — no picking, no
    /// marquee, no framing, no asset drops. Any camera input the viewport
    /// consumes breaks the game-camera follow so the user can inspect
    /// anywhere while the simulation runs.
    fn handle_play_mode_camera(&mut self, ui: &mut UIContext, input: &InputHandler) {
        if chrome_owns_mouse(ui) {
            return;
        }
        let input_result = self.editor.viewport_input.handle_input_simple(
            &mut self.editor.viewport,
            &self.editor.input_mapping,
            input,
        );
        if input_result.consumed {
            self.break_camera_follow();
        }
    }

    /// Break the play-session camera follow (manual camera input wins),
    /// announcing it once on the status bar.
    fn break_camera_follow(&mut self) {
        if self.editor.is_camera_following() {
            self.editor.set_camera_follow(false);
            self.editor
                .status_bar
                .show_message("Free camera — Ctrl+Shift+F or Follow to re-follow");
        }
    }

    fn handle_framing(
        &mut self,
        input_result: &editor::ViewportInputResult,
        pickables: &[PickableEntity],
        wants_keyboard: bool,
        ctrl_held: bool,
    ) {
        if !wants_keyboard && !ctrl_held {
            // Reset first so a same-frame F + Home resolves to the more
            // specific intent: framing overwrites the reset targets.
            if input_result.reset_requested {
                self.editor.viewport.reset_camera();
            }
            if input_result.focus_requested || input_result.frame_all_requested {
                if input_result.frame_all_requested {
                    self.editor.frame_all(pickables);
                } else if self.editor.selection.is_empty() {
                    if self.editor.frame_selected(pickables) {
                        self.editor
                            .status_bar
                            .show_message("No selection — framed all entities");
                    }
                } else {
                    self.editor.frame_selected(pickables);
                }
            }
        }
    }

    fn handle_click_pick(
        &mut self,
        input_result: &editor::ViewportInputResult,
        pickables: &[PickableEntity],
    ) {
        if !input_result.clicked {
            return;
        }
        self.editor.close_add_component_popup();
        let pick_result = self.editor.picker.pick_at_screen_pos(
            &self.editor.viewport,
            input_result.click_position,
            pickables,
        );

        if let Some(entity_id) = pick_result.topmost() {
            if input_result.shift_held {
                self.editor.selection.add(entity_id);
            } else if input_result.ctrl_held {
                self.editor.selection.toggle(entity_id);
            } else {
                self.editor.selection.select(entity_id);
            }
        } else if !input_result.shift_held && !input_result.ctrl_held {
            self.editor.selection.clear();
        }
    }

    fn handle_marquee(
        &mut self,
        ui: &mut UIContext,
        input_result: &editor::ViewportInputResult,
        pickables: &[PickableEntity],
    ) {
        // Rubber-band rect, visible while the drag is live (same frame's
        // input — drawn here, not in render_scene_view, so it is never a
        // frame stale)
        if let Some((start, current)) = input_result.marquee_active {
            self.draw_marquee(ui, start, current);
        }

        // Rectangle selection (drag just completed)
        if let Some((start, end)) = input_result.marquee_released {
            self.apply_marquee_selection(
                pickables,
                start,
                end,
                input_result.shift_held,
                input_result.ctrl_held,
            );
        }
    }

    /// Editing/Paused viewport input: pan/zoom, framing, picking, marquee.
    fn handle_shared_viewport_input(
        &mut self,
        ui: &mut UIContext,
        input: &InputHandler,
        pickables: &[PickableEntity],
    ) {
        // Editor chrome owns the mouse — skip picking/pan/zoom so clicks
        // don't pass through it into the scene.
        if chrome_owns_mouse(ui) {
            return;
        }

        let input_result = self.editor.viewport_input.handle_input_simple(
            &mut self.editor.viewport,
            &self.editor.input_mapping,
            input,
        );
        // A manual pan/zoom while Paused breaks the camera follow, exactly
        // like one while Playing — otherwise Resume snaps the view back to
        // the game camera and discards where the user just looked.
        if input_result.consumed && self.editor.in_play_session() {
            self.break_camera_follow();
        }

        if self.editor.gizmo_has_priority() {
            return;
        }

        // Camera shortcuts (F / Shift+F / Home) are requests, consumed here —
        // after the gizmo-priority return (reframing mid-drag would slide the
        // handle under the cursor and corrupt the drag) and only while no
        // text field owns the keyboard (the wants_keyboard footgun). Framing
        // is edit-mode-only by design: while Playing the viewport mirrors the
        // game camera and reframing would fight that sync. Ctrl held skips
        // the framing poll — Ctrl+Shift+F is the follow toggle chord, and
        // the KeyAnyMods(F) poll must not also fire a frame request.
        let modifiers = editor::Modifiers::read(input);
        self.handle_framing(
            &input_result,
            pickables,
            ui.wants_keyboard(),
            modifiers.ctrl,
        );
        self.handle_click_pick(&input_result, pickables);
        self.handle_marquee(ui, &input_result, pickables);
    }

    /// Draw the live marquee rect: theme selection fill (faded — the row
    /// token's alpha is too heavy over the scene) + outline border, clipped
    /// to the scene panel.
    ///
    /// Layer precondition: must be called on the default `Content` layer
    /// (phase 6 of the update loop, outside any overlay scope) — panel
    /// chrome flushes after Content and stays above the rect.
    pub(super) fn draw_marquee(&self, ui: &mut ui::UIContext, start: Vec2, current: Vec2) {
        let Some(bounds) = self.editor.scene_view_bounds() else {
            return;
        };
        // Drags go all four directions — normalize to min/max corners. Skip
        // non-finite coordinates entirely (never feed them to the draw list).
        let min = start.min(current);
        let max = start.max(current);
        if !min.is_finite() || !max.is_finite() {
            return;
        }
        let rect = ui::Rect::new(min.x, min.y, max.x - min.x, max.y - min.y);
        let clip = ui::Rect::new(bounds.x, bounds.y, bounds.width, bounds.height);

        let mut fill = self.editor.theme.selection_fill;
        fill.a *= 0.6;
        ui.push_clip_rect(clip);
        ui.rect(rect, fill);
        ui.rect_border(rect, self.editor.theme.selection_outline, 1.0, 0.0);
        ui.pop_clip_rect();
    }

    /// Apply a completed marquee: Ctrl toggles each hit, Shift adds, and a
    /// plain drag replaces — parity with single-click selection.
    pub(super) fn apply_marquee_selection(
        &mut self,
        pickables: &[PickableEntity],
        start: Vec2,
        end: Vec2,
        shift_held: bool,
        ctrl_held: bool,
    ) {
        // Same guard as the draw path: a corrupted input frame must not
        // feed non-finite corners into the pick rect.
        if !start.is_finite() || !end.is_finite() {
            return;
        }
        let pick_result = self.editor.picker.pick_in_screen_rect(
            &self.editor.viewport,
            start,
            end,
            pickables,
        );

        if ctrl_held {
            for &entity_id in &pick_result.hits {
                self.editor.selection.toggle(entity_id);
            }
        } else if shift_held {
            for &entity_id in &pick_result.hits {
                self.editor.selection.add(entity_id);
            }
        } else {
            self.editor.selection.clear();
            for &entity_id in &pick_result.hits {
                self.editor.selection.add(entity_id);
            }
        }
    }

    /// Handle a texture dropped from the asset browser onto the scene view:
    /// dropping onto an existing sprite reskins it (assign); dropping onto
    /// empty space spawns a new sprite entity at that world position. Both
    /// are single undo entries.
    fn handle_viewport_texture_drop(
        &mut self,
        world: &mut World,
        pickables: &[PickableEntity],
        handle: u32,
        path: &str,
        drop_pos: Vec2,
    ) {
        let hit = self
            .editor
            .picker
            .pick_at_screen_pos(&self.editor.viewport, drop_pos, pickables)
            .topmost();

        match hit {
            Some(entity) => {
                if entity_ops::assign_sprite_texture(world, entity, handle, &mut self.command_history) {
                    self.editor.selection.select(entity);
                    self.editor.status_bar.show_message(format!("Assigned {path}"));
                }
            }
            None => {
                let world_pos = self.editor.screen_to_world(drop_pos);
                let stem = std::path::Path::new(path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Sprite");
                entity_ops::create_sprite_entity_with_texture(
                    world,
                    &mut self.editor.selection,
                    world_pos,
                    handle,
                    stem,
                    &mut self.entity_counter,
                    &mut self.command_history,
                );
                self.editor.status_bar.show_message(format!("Created sprite from {path}"));
            }
        }
    }
}

/// Whether editor chrome owns the mouse this frame: an open overlay (menu
/// dropdown) swallows input at the cursor, or a widget press (toolbar, play
/// controls, panels) holds the gesture from press through the release frame.
/// Viewport picking must not act while this is true — a click on chrome would
/// otherwise fall through and silently reselect whatever sprite lies beneath.
pub(crate) fn chrome_owns_mouse(ui: &ui::UIContext) -> bool {
    ui.is_input_blocked_at(ui.mouse_pos()) || ui.wants_mouse()
}

/// Build the list of pickable entities from the world.
///
/// Queries for entities that have both `GlobalTransform2D` and `Sprite` components,
/// which are required for viewport picking (position + visual size).
pub(crate) fn build_pickable_entities(world: &World) -> Vec<PickableEntity> {
    let entities = world.query_entities::<Pair<GlobalTransform2D, ecs::sprite_components::Sprite>>();
    entities
        .into_iter()
        .filter_map(|entity_id| {
            let global_t = world.get::<GlobalTransform2D>(entity_id)?;
            let sprite = world.get::<ecs::sprite_components::Sprite>(entity_id)?;
            // Visual size must match the render path (engine_core game.rs):
            // sprites draw at scale * sprite.scale * RENDER_UNIT pixels.
            let size = sprite.scale * global_t.scale * engine_core::RENDER_UNIT;
            Some(PickableEntity::new(
                entity_id,
                global_t.position,
                size,
                sprite.depth,
            ))
        })
        .collect()
}
