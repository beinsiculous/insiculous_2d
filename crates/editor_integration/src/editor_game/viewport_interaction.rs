//! Viewport picking (click + rectangle selection) and gizmo dragging.

use glam::Vec2;

use ecs::{GlobalTransform2D, Pair, World};
use editor::{PanelId, PickableEntity};
use engine_core::contexts::GameContext;
use engine_core::Game;

use crate::constants::MIN_ENTITY_SCALE;
use crate::entity_ops;

use super::EditorGame;

impl<G: Game> EditorGame<G> {
    /// Handle viewport input: pan/zoom plus click and rectangle selection.
    pub(super) fn handle_viewport_picking(&mut self, ctx: &mut GameContext) {
        if self.editor.is_playing() {
            // While Playing only the CAMERA is live (issue #42): pan/zoom to
            // inspect anywhere, which breaks the game-camera follow. The
            // explicit return is load-bearing — picking, marquee, asset
            // drops, and framing must never run against a live simulation.
            self.handle_play_mode_camera(ctx);
            return;
        }

        // Asset drops land before the input-blocked check (there is no ghost
        // overlay on the release frame) and before click handling.
        if let Some(scene_bounds) = self.editor.scene_view_bounds() {
            if let Some((editor::DragPayload::Texture { handle, path }, drop_pos)) =
                self.editor.drag_drop.take_drop_in(scene_bounds)
            {
                self.handle_viewport_texture_drop(ctx, handle, &path, drop_pos);
                return;
            }
        }
        // While a drag is in flight (or on its release frame) the viewport
        // must not treat the mouse as a pick/selection click.
        if self.editor.drag_drop.suppresses_click() {
            return;
        }

        self.handle_shared_viewport_input(ctx);
    }

    /// Play-mode viewport input (issue #42): pan/zoom ONLY — no picking, no
    /// marquee, no framing, no asset drops. Any camera input the viewport
    /// consumes breaks the game-camera follow so the user can inspect
    /// anywhere while the simulation runs.
    fn handle_play_mode_camera(&mut self, ctx: &mut GameContext) {
        if chrome_owns_mouse(ctx.ui) {
            return;
        }
        let input_result = self.editor.viewport_input.handle_input_simple(
            &mut self.editor.viewport,
            &self.editor.input_mapping,
            ctx.input,
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

    /// Editing/Paused viewport input: pan/zoom, framing, picking, marquee.
    fn handle_shared_viewport_input(&mut self, ctx: &mut GameContext) {

        // Editor chrome owns the mouse — skip picking/pan/zoom so clicks
        // don't pass through it into the scene.
        if chrome_owns_mouse(ctx.ui) {
            return;
        }

        let input_result = self.editor.viewport_input.handle_input_simple(
            &mut self.editor.viewport,
            &self.editor.input_mapping,
            ctx.input,
        );
        // A manual pan/zoom while Paused breaks the camera follow, exactly
        // like one while Playing — otherwise Resume snaps the view back to
        // the game camera and discards where the user just looked (#42).
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
        let ctrl_held = ctx.input.keyboard().is_key_pressed(winit::keyboard::KeyCode::ControlLeft)
            || ctx.input.keyboard().is_key_pressed(winit::keyboard::KeyCode::ControlRight);
        if !ctx.ui.wants_keyboard() && !ctrl_held {
            // Reset first so a same-frame F + Home resolves to the more
            // specific intent: framing overwrites the reset targets.
            if input_result.reset_requested {
                self.editor.viewport.reset_camera();
            }
            if input_result.focus_requested || input_result.frame_all_requested {
                let pickables = build_pickable_entities(ctx.world);
                if input_result.frame_all_requested {
                    self.editor.frame_all(&pickables);
                } else if self.editor.selection.is_empty() {
                    if self.editor.frame_selected(&pickables) {
                        self.editor
                            .status_bar
                            .show_message("No selection — framed all entities");
                    }
                } else {
                    self.editor.frame_selected(&pickables);
                }
            }
        }

        if input_result.clicked {
            self.editor.close_add_component_popup();
            let pickables = build_pickable_entities(ctx.world);
            let pick_result = self.editor.picker.pick_at_screen_pos(
                &self.editor.viewport,
                input_result.click_position,
                &pickables,
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

        // Rubber-band rect, visible while the drag is live (same frame's
        // input — drawn here, not in render_scene_view, so it is never a
        // frame stale)
        if let Some((start, current)) = input_result.marquee_active {
            self.draw_marquee(ctx.ui, start, current);
        }

        // Rectangle selection (drag just completed)
        if let Some((start, end)) = input_result.marquee_released {
            self.apply_marquee_selection(
                ctx.world,
                start,
                end,
                input_result.shift_held,
                input_result.ctrl_held,
            );
        }
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
        world: &World,
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
        let pickables = build_pickable_entities(world);
        let pick_result = self.editor.picker.pick_in_screen_rect(
            &self.editor.viewport,
            start,
            end,
            &pickables,
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
        ctx: &mut GameContext,
        handle: u32,
        path: &str,
        drop_pos: Vec2,
    ) {
        let pickables = build_pickable_entities(ctx.world);
        let hit = self
            .editor
            .picker
            .pick_at_screen_pos(&self.editor.viewport, drop_pos, &pickables)
            .topmost();

        match hit {
            Some(entity) => {
                if entity_ops::assign_sprite_texture(ctx.world, entity, handle, &mut self.command_history) {
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
                    ctx.world,
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

    /// Render the gizmo at the primary selection and apply the drag to every
    /// selection root, recording a single undo entry per drag.
    ///
    /// Deltas are cumulative from drag start and applied as `start + delta`
    /// (idempotent per frame) — never accumulated into the live transform,
    /// which is what let snapping annihilate sub-cell drag residuals.
    pub(super) fn handle_gizmo(&mut self, ctx: &mut GameContext, content_areas: &[(PanelId, common::Rect)]) {
        if self.editor.is_playing() {
            return;
        }
        let Some(primary) = self.editor.selection.primary() else {
            return;
        };
        let Some(scene_rect) = content_areas
            .iter()
            .find(|(id, _)| *id == PanelId::SCENE_VIEW)
            .map(|(_, rect)| *rect)
        else {
            return;
        };

        let entity_pos = ctx.world
            .get::<ecs::GlobalTransform2D>(primary)
            .map(|t| t.position);
        let Some(entity_pos) = entity_pos else {
            return;
        };

        // Clip the gizmo to the scene panel (it runs after render_panels has
        // popped every panel clip) and refuse to START drags with the mouse
        // outside it — handles panned off the viewport edge must not draw
        // over, or stay draggable through, the neighboring panels.
        let screen_pos = self.editor.world_to_screen(entity_pos);
        let clip = ui::Rect::new(scene_rect.x, scene_rect.y, scene_rect.width, scene_rect.height);
        let mouse_in_panel = clip.contains(ctx.ui.mouse_pos());
        ctx.ui.push_clip_rect(clip);
        let interaction = self.editor.gizmo.render(ctx.ui, screen_pos, mouse_in_panel);
        ctx.ui.pop_clip_rect();

        // Drag start: capture every selection root's transform (and collider,
        // for the scale tool). Roots only — a selected child of a selected
        // parent would otherwise be moved twice.
        if interaction.handle.is_some() && self.gizmo_drag.is_none() {
            let entities: Vec<super::gizmo_drag::DragEntity> =
                entity_ops::selection_roots(ctx.world, &self.editor.selection)
                    .into_iter()
                    .filter_map(|id| {
                        let start = *ctx.world.get::<ecs::sprite_components::Transform2D>(id)?;
                        let start_collider =
                            ctx.world.get::<physics::components::Collider>(id).cloned();
                        Some(super::gizmo_drag::DragEntity { id, start, start_collider })
                    })
                    .collect();
            if !entities.is_empty() {
                self.gizmo_drag = Some(super::gizmo_drag::GizmoDragState {
                    entities,
                    accumulated_rotation: 0.0,
                });
            }
        }

        // Apply the live drag (hold-Ctrl snaps even when the pref is off —
        // the repo's Ctrl-snap convention, shared with scrub fields).
        if interaction.handle.is_some() {
            let ctrl_held = ctrl_held(ctx.input);
            self.apply_gizmo_drag(ctx.world, &interaction, ctrl_held);
        }

        // Gizmo released — commit the whole drag as one undo entry.
        if !self.editor.gizmo.is_active() {
            self.commit_gizmo_drag(ctx.world);
        }
    }

    /// Apply one frame of a live gizmo drag to every captured root. The
    /// three channels apply uniformly — in any given mode the inactive ones
    /// are identity (translation ZERO, rotation delta 0, scale factor ONE).
    /// Everything derives from the captured drag-start values plus the
    /// CUMULATIVE interaction, so re-applying is idempotent: with snapping
    /// active, a slow drag accumulates in the delta (never quantized away)
    /// and the position steps whole grid cells.
    pub(super) fn apply_gizmo_drag(
        &mut self,
        world: &mut World,
        interaction: &editor::GizmoInteraction,
        ctrl_held: bool,
    ) {
        let Some(drag) = self.gizmo_drag.as_mut() else {
            return;
        };
        let world_delta = self.editor.gizmo_delta_to_world(interaction.translation);
        // Snap the PRIMARY's anchor and share the delta so relative
        // offsets in a multi-selection survive a snapped drag.
        let snap_active = self.editor.is_snap_to_grid() || ctrl_held;
        let effective_delta = if snap_active {
            let anchor = drag.entities[0].start.position;
            self.editor.snap_to_grid_position(anchor + world_delta) - anchor
        } else {
            world_delta
        };
        drag.accumulated_rotation += interaction.rotation_delta;

        for entity in &drag.entities {
            let new_scale = (entity.start.scale * interaction.scale_factor)
                .max(Vec2::splat(MIN_ENTITY_SCALE));
            if let Some(transform) =
                world.get_mut::<ecs::sprite_components::Transform2D>(entity.id)
            {
                transform.position = entity.start.position + effective_delta;
                transform.rotation = entity.start.rotation + drag.accumulated_rotation;
                transform.scale = new_scale;
            }

            // Scale the collider alongside: physics colliders are
            // absolute-pixel sized (they ignore Transform2D.scale).
            // Rebuilt from the DRAG-START collider so per-frame
            // application never accumulates float drift.
            if interaction.scale_factor != Vec2::ONE {
                if let Some(start_collider) = &entity.start_collider {
                    let applied =
                        new_scale / entity.start.scale.max(Vec2::splat(f32::EPSILON));
                    let mut rebuilt = start_collider.clone();
                    scale_collider(&mut rebuilt, applied);
                    if let Some(collider) =
                        world.get_mut::<physics::components::Collider>(entity.id)
                    {
                        *collider = rebuilt;
                    }
                }
            }
        }
    }

    /// Record the finished drag as ONE undo entry: a TransformGizmoCommand
    /// per root, plus a SetColliderCommand where the scale tool resized one,
    /// wrapped in a MacroCommand when there is more than one piece. Pushes
    /// nothing when nothing changed (zero-delta click, or an Escape already
    /// rolled the drag back).
    pub(super) fn commit_gizmo_drag(&mut self, world: &World) {
        let Some(drag) = self.gizmo_drag.take() else {
            return;
        };
        // A drag ending is a gesture boundary regardless of whether it
        // moved anything (#56): mergeable commands (nudges, field-hint
        // Set*Commands) on either side of a drag must never collapse into
        // one undo entry across it.
        self.command_history.break_merge();
        let mut commands: Vec<Box<dyn editor::commands::EditorCommand>> = Vec::new();
        for entity in &drag.entities {
            let Some(final_transform) =
                world.get::<ecs::sprite_components::Transform2D>(entity.id)
            else {
                continue;
            };
            if *final_transform != entity.start {
                commands.push(Box::new(editor::commands::TransformGizmoCommand::new(
                    entity.id,
                    entity.start,
                    *final_transform,
                )));
            }
            if let Some(old) = &entity.start_collider {
                if let Some(new) = world.get::<physics::components::Collider>(entity.id) {
                    if *new != *old {
                        commands.push(Box::new(editor::commands::SetColliderCommand::new(
                            entity.id,
                            old.clone(),
                            new.clone(),
                            "gizmo_scale",
                        )));
                    }
                }
            }
        }
        match commands.len() {
            0 => {}
            1 => {
                if let Some(cmd) = commands.pop() {
                    self.command_history.push_already_executed(cmd);
                }
            }
            _ => {
                self.command_history.push_already_executed(Box::new(
                    editor::commands::MacroCommand::new("Transform Entities", commands),
                ));
            }
        }
    }

    /// Roll back an in-flight gizmo drag (Escape): every dragged root's
    /// transform and collider is restored to its drag-start value and NO
    /// undo entry is pushed. Returns whether a drag was cancelled.
    pub(super) fn cancel_gizmo_drag(&mut self, world: &mut World) -> bool {
        let Some(drag) = self.gizmo_drag.take() else {
            return false;
        };
        // An Escape-cancelled drag is still a gesture boundary (#56).
        self.command_history.break_merge();
        for entity in &drag.entities {
            if let Some(transform) =
                world.get_mut::<ecs::sprite_components::Transform2D>(entity.id)
            {
                *transform = entity.start;
            }
            if let Some(start_collider) = &entity.start_collider {
                if let Some(collider) =
                    world.get_mut::<physics::components::Collider>(entity.id)
                {
                    *collider = start_collider.clone();
                }
            }
        }
        self.editor.gizmo.cancel();
        self.editor.status_bar.show_message("Drag cancelled");
        true
    }
}

/// Scale a collider's shape (and body-local offset) by a per-axis factor —
/// how the editor's scale tool keeps absolute-pixel physics shapes in step
/// with the sprite. Radii use the dominant axis factor (circles stay circles).
pub(super) fn scale_collider(collider: &mut physics::components::Collider, factor: Vec2) {
    use physics::components::ColliderShape;
    collider.offset *= factor;
    match &mut collider.shape {
        ColliderShape::Box { half_extents } => *half_extents *= factor,
        ColliderShape::Circle { radius } => *radius *= factor.x.max(factor.y),
        ColliderShape::CapsuleY { half_height, radius } => {
            *half_height *= factor.y;
            *radius *= factor.x;
        }
        ColliderShape::CapsuleX { half_height, radius } => {
            *half_height *= factor.x;
            *radius *= factor.y;
        }
    }
}

/// Whether either Ctrl key is held — the hold-to-snap modifier for gizmo
/// drags (the same Ctrl-snap convention scrub fields use).
pub(super) fn ctrl_held(input: &input::InputHandler) -> bool {
    use winit::keyboard::KeyCode;
    input.keyboard().is_key_pressed(KeyCode::ControlLeft)
        || input.keyboard().is_key_pressed(KeyCode::ControlRight)
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
