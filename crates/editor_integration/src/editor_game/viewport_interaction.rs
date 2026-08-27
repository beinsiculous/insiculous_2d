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

        if self.editor.gizmo_has_priority() {
            return;
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

        // Rectangle selection (drag just completed)
        if !input_result.selection_drag_active
            && input_result.selection_start != Vec2::ZERO
            && !input_result.clicked
        {
            let pickables = build_pickable_entities(ctx.world);
            let pick_result = self.editor.picker.pick_in_screen_rect(
                &self.editor.viewport,
                input_result.selection_start,
                input_result.selection_end,
                &pickables,
            );

            if !input_result.shift_held {
                self.editor.selection.clear();
            }
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
                    self.editor.mark_dirty();
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
                self.editor.mark_dirty();
                self.editor.status_bar.show_message(format!("Created sprite from {path}"));
            }
        }
    }

    /// Render the gizmo for the primary selection and apply drag deltas,
    /// recording a single undo entry per drag.
    pub(super) fn handle_gizmo(&mut self, ctx: &mut GameContext, content_areas: &[(PanelId, common::Rect)]) {
        if self.editor.is_playing() {
            return;
        }
        let Some(entity_id) = self.editor.selection.primary() else {
            return;
        };
        if !content_areas.iter().any(|(id, _)| *id == PanelId::SCENE_VIEW) {
            return;
        }

        let entity_pos = ctx.world
            .get::<ecs::GlobalTransform2D>(entity_id)
            .map(|t| t.position);
        let Some(entity_pos) = entity_pos else {
            return;
        };

        let screen_pos = self.editor.world_to_screen(entity_pos);
        let interaction = self.editor.gizmo.render(ctx.ui, screen_pos);

        // Capture initial transform (and collider, for the scale tool)
        // when gizmo drag starts
        if interaction.handle.is_some() && self.gizmo_drag_start.is_none() {
            if let Some(t) = ctx.world.get::<ecs::sprite_components::Transform2D>(entity_id) {
                self.gizmo_drag_start = Some(*t);
                self.gizmo_drag_start_collider =
                    ctx.world.get::<physics::components::Collider>(entity_id).cloned();
            }
        }

        if interaction.handle.is_some() {
            // Translation
            if interaction.delta != Vec2::ZERO {
                let world_delta = self.editor.gizmo_delta_to_world(interaction.delta);
                let snap_enabled = self.editor.is_snap_to_grid();

                if let Some(transform) = ctx.world.get_mut::<ecs::sprite_components::Transform2D>(entity_id) {
                    transform.position += world_delta;
                    if snap_enabled {
                        transform.position = self.editor.snap_position(transform.position);
                    }
                }
            }

            // Rotation
            if interaction.rotation_delta != 0.0 {
                if let Some(transform) = ctx.world.get_mut::<ecs::sprite_components::Transform2D>(entity_id) {
                    transform.rotation += interaction.rotation_delta;
                }
            }

            // Scale — the tool scales the whole object: physics colliders
            // are absolute-pixel sized (they ignore Transform2D.scale), so
            // the collider must be resized by the same factor or it drifts
            // from the visuals.
            if interaction.scale_delta != Vec2::ZERO {
                let factor = ctx.world
                    .get_mut::<ecs::sprite_components::Transform2D>(entity_id)
                    .map(|transform| {
                        let old_scale = transform.scale;
                        transform.scale += interaction.scale_delta;
                        transform.scale = transform.scale.max(Vec2::splat(MIN_ENTITY_SCALE));
                        transform.scale / old_scale.max(Vec2::splat(f32::EPSILON))
                    });
                if let (Some(factor), Some(collider)) =
                    (factor, ctx.world.get_mut::<physics::components::Collider>(entity_id))
                {
                    scale_collider(collider, factor);
                }
            }
        }

        // Gizmo released — record ONE undo entry for the whole drag
        // (transform, plus the collider when the scale tool resized it)
        if interaction.handle.is_none() && self.gizmo_drag_start.is_some() {
            if let Some(initial) = self.gizmo_drag_start.take() {
                let initial_collider = self.gizmo_drag_start_collider.take();
                if let Some(final_val) = ctx.world.get::<ecs::sprite_components::Transform2D>(entity_id) {
                    let transform_cmd =
                        editor::commands::TransformGizmoCommand::new(entity_id, initial, *final_val);

                    let collider_cmd = initial_collider.and_then(|old| {
                        let new = ctx.world.get::<physics::components::Collider>(entity_id)?;
                        (*new != old).then(|| {
                            editor::commands::SetColliderCommand::new(entity_id, old, new.clone(), "gizmo_scale")
                        })
                    });

                    match collider_cmd {
                        Some(collider_cmd) => {
                            let cmd = editor::commands::MacroCommand::new(
                                "Scale Entity",
                                vec![Box::new(transform_cmd), Box::new(collider_cmd)],
                            );
                            self.command_history.push_already_executed(Box::new(cmd));
                        }
                        None => {
                            self.command_history.push_already_executed(Box::new(transform_cmd));
                        }
                    }
                    self.editor.mark_dirty();
                }
            }
        }
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

/// Whether editor chrome owns the mouse this frame: an open overlay (menu
/// dropdown) swallows input at the cursor, or a widget press (toolbar, play
/// controls, panels) holds the gesture from press through the release frame.
/// Viewport picking must not act while this is true — a click on chrome would
/// otherwise fall through and silently reselect whatever sprite lies beneath.
pub(super) fn chrome_owns_mouse(ui: &ui::UIContext) -> bool {
    ui.is_input_blocked_at(ui.mouse_pos()) || ui.wants_mouse()
}

/// Build the list of pickable entities from the world.
///
/// Queries for entities that have both `GlobalTransform2D` and `Sprite` components,
/// which are required for viewport picking (position + visual size).
pub(super) fn build_pickable_entities(world: &World) -> Vec<PickableEntity> {
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
