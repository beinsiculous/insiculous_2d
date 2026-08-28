//! Drag-start state for gizmo manipulation.
//!
//! Captured once when a gizmo drag begins, for every selection root; the
//! drag applies `start + cumulative delta` idempotently each frame (never
//! `+=`, which is what let snapping eat sub-cell drag residuals), commits
//! one undo entry on release, and restores these values verbatim on an
//! Escape cancel.

use ecs::EntityId;

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
