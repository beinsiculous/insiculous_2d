//! Wire defaults for `ComponentData::GridBackdrop` = the component's
//! `Default` (the playfield preset), so `GridBackdrop()` in RON and the
//! editor's "Add Component" agree. A child module of `scene_data`
//! purely for file size.

macro_rules! grid_default {
    ($fn_name:ident, $field:ident, $ty:ty) => {
        pub(super) fn $fn_name() -> $ty {
            ecs::GridBackdrop::default().$field.into()
        }
    };
}

grid_default!(default_grid_cols, cols, u32);
grid_default!(default_grid_rows, rows, u32);
grid_default!(default_grid_spacing, spacing, f32);
grid_default!(default_grid_color, color, (f32, f32, f32, f32));
grid_default!(default_grid_emissive, emissive, f32);
grid_default!(default_grid_stiffness, stiffness, f32);
grid_default!(default_grid_damping, damping, f32);
grid_default!(default_grid_rest_pull, rest_pull, f32);
grid_default!(default_grid_rest_alpha_fraction, rest_alpha_fraction, f32);
grid_default!(default_grid_activity_attack, activity_attack, f32);
grid_default!(default_grid_activity_release, activity_release, f32);
grid_default!(default_grid_activity_displacement_ref, activity_displacement_ref, f32);
grid_default!(default_grid_activity_velocity_ref, activity_velocity_ref, f32);
