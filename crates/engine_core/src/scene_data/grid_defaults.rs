//! Wire defaults for `ComponentData::GridBackdrop` = the component's
//! `Default` (the playfield preset), so `GridBackdrop()` in RON and the
//! editor's "Add Component" agree (#46). A child module of `scene_data`
//! purely for file size.

pub(super) fn default_grid_cols() -> u32 {
    ecs::GridBackdrop::default().cols
}
pub(super) fn default_grid_rows() -> u32 {
    ecs::GridBackdrop::default().rows
}
pub(super) fn default_grid_spacing() -> f32 {
    ecs::GridBackdrop::default().spacing
}
pub(super) fn default_grid_color() -> (f32, f32, f32, f32) {
    ecs::GridBackdrop::default().color.into()
}
pub(super) fn default_grid_emissive() -> f32 {
    ecs::GridBackdrop::default().emissive
}
pub(super) fn default_grid_stiffness() -> f32 {
    ecs::GridBackdrop::default().stiffness
}
pub(super) fn default_grid_damping() -> f32 {
    ecs::GridBackdrop::default().damping
}
pub(super) fn default_grid_rest_pull() -> f32 {
    ecs::GridBackdrop::default().rest_pull
}
pub(super) fn default_grid_rest_alpha_fraction() -> f32 {
    ecs::GridBackdrop::default().rest_alpha_fraction
}
pub(super) fn default_grid_activity_attack() -> f32 {
    ecs::GridBackdrop::default().activity_attack
}
pub(super) fn default_grid_activity_release() -> f32 {
    ecs::GridBackdrop::default().activity_release
}
pub(super) fn default_grid_activity_displacement_ref() -> f32 {
    ecs::GridBackdrop::default().activity_displacement_ref
}
pub(super) fn default_grid_activity_velocity_ref() -> f32 {
    ecs::GridBackdrop::default().activity_velocity_ref
}
