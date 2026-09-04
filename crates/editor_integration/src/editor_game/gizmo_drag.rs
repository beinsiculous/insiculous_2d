//! Drag-start state for gizmo manipulation.
//!
//! Captured once when a gizmo drag begins, for every selection root; the
//! drag applies `start + cumulative delta` idempotently each frame (never
//! `+=`, which is what let snapping eat sub-cell drag residuals), commits
//! one undo entry on release, and restores these values verbatim on an
//! Escape cancel.

use ecs::{EntityId, World};
use editor::PanelId;
use engine_core::contexts::GameContext;
use engine_core::Game;
use glam::Vec2;

use crate::constants::MIN_ENTITY_SCALE;
use crate::entity_ops;

use super::EditorGame;

/// Everything a live gizmo drag needs to apply, commit, or roll back.
pub(super) struct GizmoDragState {
    /// Selection roots at drag start; `[0]` is the primary (the snap anchor).
    pub entities: Vec<DragEntity>,
    /// Total rotation applied so far (rotation deltas are per-frame because
    /// a cumulative angle would wrap at ±π).
    pub accumulated_rotation: f32,
}

/// One dragged entity's captured starting state.
pub(super) struct DragEntity {
    pub id: EntityId,
    /// Transform at drag start — the base every frame's delta applies to.
    pub start: common::Transform2D,
    /// Collider at drag start (the scale tool rebuilds the collider from
    /// this — physics ignores `Transform2D.scale`).
    pub start_collider: Option<physics::components::Collider>,
}

impl<G: Game> EditorGame<G> {
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
            self.capture_drag_start(ctx.world);
        }

        // Apply the live drag (hold-Ctrl snaps even when the pref is off —
        // the repo's Ctrl-snap convention, shared with scrub fields).
        if interaction.handle.is_some() {
            let ctrl_held = editor::Modifiers::read(ctx.input).ctrl;
            self.apply_gizmo_drag(ctx.world, &interaction, ctrl_held);
        }

        // Gizmo released — commit the whole drag as one undo entry.
        if !self.editor.gizmo.is_active() {
            self.commit_gizmo_drag(ctx.world);
        }
    }

    /// Capture starting transforms and colliders for every selection root when a drag starts.
    fn capture_drag_start(&mut self, world: &World) {
        let entities: Vec<DragEntity> =
            entity_ops::selection_roots(world, &self.editor.selection)
                .into_iter()
                .filter_map(|id| {
                    let start = *world.get::<ecs::sprite_components::Transform2D>(id)?;
                    let start_collider =
                        world.get::<physics::components::Collider>(id).cloned();
                    Some(DragEntity { id, start, start_collider })
                })
                .collect();
        if !entities.is_empty() {
            self.gizmo_drag = Some(GizmoDragState {
                entities,
                accumulated_rotation: 0.0,
            });
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

    /// Record the finished drag as ONE undo entry: a SetTransformCommand
    /// under GIZMO_FIELD_HINT per root, plus a SetColliderCommand where the
    /// scale tool resized one,
    /// wrapped in a MacroCommand when there is more than one piece. Pushes
    /// nothing when nothing changed (zero-delta click, or an Escape already
    /// rolled the drag back).
    pub(super) fn commit_gizmo_drag(&mut self, world: &World) {
        let Some(drag) = self.gizmo_drag.take() else {
            return;
        };
        // A drag ending is a gesture boundary regardless of whether it
        // moved anything: mergeable commands (nudges, field-hint
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
                commands.push(Box::new(editor::commands::SetTransformCommand::new(
                    entity.id,
                    entity.start,
                    *final_transform,
                    editor::commands::GIZMO_FIELD_HINT,
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
        self.command_history.push_as_one("Transform Entities", commands);
    }

    /// Roll back an in-flight gizmo drag (Escape): every dragged root's
    /// transform and collider is restored to its drag-start value and NO
    /// undo entry is pushed. Returns whether a drag was cancelled.
    pub(super) fn cancel_gizmo_drag(&mut self, world: &mut World) -> bool {
        let Some(drag) = self.gizmo_drag.take() else {
            return false;
        };
        // An Escape-cancelled drag is still a gesture boundary.
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
