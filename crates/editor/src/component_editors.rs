//! Component-specific editable inspectors.
//!
//! Provides pre-built inspectors for common component types:
//! - Transform2D
//! - Sprite
//! - RigidBody
//! - Collider
//! - AudioSource
//!
//! Each `edit_*` function renders the component's fields and returns
//! `Option<ComponentEdit<T>>` — `None` when nothing changed this frame,
//! `Some` with the full new value and a `field_hint` naming the changed
//! field (used to merge continuous slider drags into one undo entry).

use ecs::sprite_components::Sprite;
use common::Transform2D;
use physics::components::{Collider, RigidBody, RigidBodyType, ColliderShape};
use ecs::audio_components::AudioSource;

use crate::editable_inspector::{EditResult, EditableInspector};

/// Value ranges for component field editors, centralized so every editor
/// uses consistent limits and they can be tuned in one place.
mod ranges {
    use std::ops::RangeInclusive;

    /// Position covers most game worlds.
    pub const POSITION: RangeInclusive<f32> = -1000.0..=1000.0;
    /// Rotation in radians.
    pub const ROTATION: RangeInclusive<f32> =
        -std::f32::consts::PI..=std::f32::consts::PI;
    /// Scale prevents negative/zero values.
    pub const SCALE: RangeInclusive<f32> = 0.01..=10.0;
    /// Sprite/collider offsets relative to the entity.
    pub const OFFSET: RangeInclusive<f32> = -100.0..=100.0;
    /// Collider shape dimensions (half-extents, radii) in pixels.
    pub const COLLIDER_EXTENT: RangeInclusive<f32> = 0.5..=1000.0;
    /// Depth sorting range.
    pub const DEPTH: RangeInclusive<f32> = -100.0..=100.0;
    /// Linear velocity.
    pub const VELOCITY: RangeInclusive<f32> = -500.0..=500.0;
    /// Angular velocity in radians per second.
    pub const ANGULAR_VELOCITY: RangeInclusive<f32> = -10.0..=10.0;
    /// Gravity scale (1.0 is normal gravity).
    pub const GRAVITY_SCALE: RangeInclusive<f32> = 0.0..=2.0;
    /// Audio pitch (slow-motion to chipmunk).
    pub const PITCH: RangeInclusive<f32> = 0.1..=3.0;
    /// Spatial audio cutoff distance.
    pub const MAX_DISTANCE: RangeInclusive<f32> = 0.0..=5000.0;
    /// Spatial audio reference distance.
    pub const REFERENCE_DISTANCE: RangeInclusive<f32> = 0.0..=1000.0;
    /// Spatial audio rolloff factor.
    pub const ROLLOFF: RangeInclusive<f32> = 0.0..=5.0;
}

/// A completed single-frame inspector edit on a component.
///
/// Holds the full component value with this frame's change applied, plus a
/// hint naming the changed field. The hint drives undo merging: consecutive
/// edits to the same field on the same entity collapse into a single undo
/// entry (see `Set*Command::try_merge`).
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentEdit<T> {
    /// Full component value with this frame's change applied.
    pub new_value: T,
    /// Name of the field that changed (e.g. `"position"`).
    pub field_hint: &'static str,
}

/// Edit the Name component (the entity's durable address for scene files
/// and the command API).
///
/// Returns `Some(ComponentEdit)` when a new non-empty name is committed —
/// an empty commit is ignored so the inspector can't strand an entity with
/// a blank name (delete the component to unname an entity).
pub fn edit_name(
    inspector: &mut EditableInspector<'_>,
    name: &ecs::Name,
    _extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<ecs::Name>> {
    inspector.header("Name");
    if let EditResult::Changed(v) = inspector.string_edit("Name", name.as_str()) {
        if let Some(new_name) = crate::hierarchy::normalized_rename(Some(name.as_str()), &v) {
            return Some(ComponentEdit {
                new_value: ecs::Name::new(new_name),
                field_hint: "name",
            });
        }
    }
    None
}

/// Edit a Transform2D component.
///
/// Returns `Some(ComponentEdit)` if any field changed this frame.
pub fn edit_transform2d(
    inspector: &mut EditableInspector<'_>,
    transform: &Transform2D,
    _extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<Transform2D>> {
    let mut new = *transform;
    let mut hint = None;

    inspector.header("Transform2D");

    if let EditResult::Changed(v) = inspector.vec2("Position", transform.position, ranges::POSITION) {
        new.position = v;
        hint = Some("position");
    }
    if let EditResult::Changed(v) = inspector.f32("Rotation", transform.rotation, ranges::ROTATION) {
        new.rotation = v;
        hint = Some("rotation");
    }
    if let EditResult::Changed(v) = inspector.vec2("Scale", transform.scale, ranges::SCALE) {
        new.scale = v;
        hint = Some("scale");
    }

    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

/// Edit a Sprite component.
pub fn edit_sprite(
    inspector: &mut EditableInspector<'_>,
    sprite: &Sprite,
    extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<Sprite>> {
    let mut new = sprite.clone();
    let mut hint = None;

    inspector.header("Sprite");

    if let EditResult::Changed(v) = inspector.vec2("Offset", sprite.offset, ranges::OFFSET) {
        new.offset = v;
        hint = Some("offset");
    }
    if let EditResult::Changed(v) = inspector.f32("Rotation", sprite.rotation, ranges::ROTATION) {
        new.rotation = v;
        hint = Some("rotation");
    }
    if let EditResult::Changed(v) = inspector.vec2("Scale", sprite.scale, ranges::SCALE) {
        new.scale = v;
        hint = Some("scale");
    }
    if let EditResult::Changed(v) = inspector.color("Color", sprite.color) {
        new.color = v;
        hint = Some("color");
    }
    if let EditResult::Changed(v) = inspector.f32("Depth", sprite.depth, ranges::DEPTH) {
        new.depth = v;
        hint = Some("depth");
    }

    // Texture slot: shows the resolved path, accepts asset-browser drops
    if let EditResult::Changed(handle) = inspector.texture("Texture", sprite.texture_handle, extras) {
        new.texture_handle = handle;
        hint = Some("texture_handle");
    }

    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

/// Edit a RigidBody component.
pub fn edit_rigid_body(
    inspector: &mut EditableInspector<'_>,
    body: &RigidBody,
    _extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<RigidBody>> {
    let mut new = body.clone();
    let mut hint = None;

    inspector.header("RigidBody");

    // Body type (read-only for now - would need dropdown widget)
    let type_str = match body.body_type {
        RigidBodyType::Dynamic => "Dynamic",
        RigidBodyType::Static => "Static",
        RigidBodyType::Kinematic => "Kinematic",
    };
    inspector.header(&format!("  Type: {}", type_str));

    if let EditResult::Changed(v) = inspector.vec2("Velocity", body.velocity, ranges::VELOCITY) {
        new.velocity = v;
        hint = Some("velocity");
    }
    if let EditResult::Changed(v) = inspector.f32("Ang. Velocity", body.angular_velocity, ranges::ANGULAR_VELOCITY) {
        new.angular_velocity = v;
        hint = Some("angular_velocity");
    }
    if let EditResult::Changed(v) = inspector.f32("Gravity Scale", body.gravity_scale, ranges::GRAVITY_SCALE) {
        new.gravity_scale = v;
        hint = Some("gravity_scale");
    }
    if let EditResult::Changed(v) = inspector.normalized_f32("Linear Damping", body.linear_damping) {
        new.linear_damping = v;
        hint = Some("linear_damping");
    }
    if let EditResult::Changed(v) = inspector.normalized_f32("Angular Damping", body.angular_damping) {
        new.angular_damping = v;
        hint = Some("angular_damping");
    }
    if let EditResult::Changed(v) = inspector.bool("Can Rotate", body.can_rotate) {
        new.can_rotate = v;
        hint = Some("can_rotate");
    }
    if let EditResult::Changed(v) = inspector.bool("CCD Enabled", body.ccd_enabled) {
        new.ccd_enabled = v;
        hint = Some("ccd_enabled");
    }

    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

/// Edit a Collider component.
pub fn edit_collider(
    inspector: &mut EditableInspector<'_>,
    collider: &Collider,
    _extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<Collider>> {
    let mut new = collider.clone();
    let mut hint = None;

    inspector.header("Collider");

    // Shape kind is fixed (changing it would need a dropdown), but its
    // dimensions are editable so colliders can be matched to sprites from
    // the editor. Sizes are absolute pixels — physics ignores
    // Transform2D.scale.
    match &collider.shape {
        ColliderShape::Box { half_extents } => {
            inspector.header("  Shape: Box");
            if let EditResult::Changed(v) =
                inspector.vec2("Half Extents", *half_extents, ranges::COLLIDER_EXTENT)
            {
                new.shape = ColliderShape::Box { half_extents: v };
                hint = Some("half_extents");
            }
        }
        ColliderShape::Circle { radius } => {
            inspector.header("  Shape: Circle");
            if let EditResult::Changed(v) =
                inspector.f32("Radius", *radius, ranges::COLLIDER_EXTENT)
            {
                new.shape = ColliderShape::Circle { radius: v };
                hint = Some("radius");
            }
        }
        ColliderShape::CapsuleY { half_height, radius } => {
            inspector.header("  Shape: CapsuleY");
            if let EditResult::Changed(v) =
                inspector.f32("Half Height", *half_height, ranges::COLLIDER_EXTENT)
            {
                new.shape = ColliderShape::CapsuleY { half_height: v, radius: *radius };
                hint = Some("half_height");
            }
            if let EditResult::Changed(v) =
                inspector.f32("Cap Radius", *radius, ranges::COLLIDER_EXTENT)
            {
                new.shape = ColliderShape::CapsuleY { half_height: *half_height, radius: v };
                hint = Some("radius");
            }
        }
        ColliderShape::CapsuleX { half_height, radius } => {
            inspector.header("  Shape: CapsuleX");
            if let EditResult::Changed(v) =
                inspector.f32("Half Width", *half_height, ranges::COLLIDER_EXTENT)
            {
                new.shape = ColliderShape::CapsuleX { half_height: v, radius: *radius };
                hint = Some("half_height");
            }
            if let EditResult::Changed(v) =
                inspector.f32("Cap Radius", *radius, ranges::COLLIDER_EXTENT)
            {
                new.shape = ColliderShape::CapsuleX { half_height: *half_height, radius: v };
                hint = Some("radius");
            }
        }
    }

    if let EditResult::Changed(v) = inspector.vec2("Offset", collider.offset, ranges::OFFSET) {
        new.offset = v;
        hint = Some("offset");
    }
    if let EditResult::Changed(v) = inspector.bool("Is Sensor", collider.is_sensor) {
        new.is_sensor = v;
        hint = Some("is_sensor");
    }
    if let EditResult::Changed(v) = inspector.normalized_f32("Friction", collider.friction) {
        new.friction = v;
        hint = Some("friction");
    }
    if let EditResult::Changed(v) = inspector.normalized_f32("Restitution", collider.restitution) {
        new.restitution = v;
        hint = Some("restitution");
    }

    // Collision groups/filter (read-only)
    inspector.u32("Groups", collider.collision_groups);
    inspector.u32("Filter", collider.collision_filter);

    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

/// Edit an AudioSource component.
pub fn edit_audio_source(
    inspector: &mut EditableInspector<'_>,
    source: &AudioSource,
    _extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<AudioSource>> {
    let mut new = source.clone();
    let mut hint = None;

    inspector.header("AudioSource");

    // Sound ID (read-only asset reference)
    inspector.u32("Sound ID", source.sound_id);

    if let EditResult::Changed(v) = inspector.normalized_f32("Volume", source.volume) {
        new.volume = v;
        hint = Some("volume");
    }
    if let EditResult::Changed(v) = inspector.f32("Pitch", source.pitch, ranges::PITCH) {
        new.pitch = v;
        hint = Some("pitch");
    }
    if let EditResult::Changed(v) = inspector.bool("Looping", source.looping) {
        new.looping = v;
        hint = Some("looping");
    }
    if let EditResult::Changed(v) = inspector.bool("Play on Spawn", source.play_on_spawn) {
        new.play_on_spawn = v;
        hint = Some("play_on_spawn");
    }
    if let EditResult::Changed(v) = inspector.bool("Spatial", source.spatial) {
        new.spatial = v;
        hint = Some("spatial");
    }

    // Spatial audio parameters (only relevant if spatial is true)
    if source.spatial {
        if let EditResult::Changed(v) = inspector.f32("Max Distance", source.max_distance, ranges::MAX_DISTANCE) {
            new.max_distance = v;
            hint = Some("max_distance");
        }
        if let EditResult::Changed(v) = inspector.f32("Ref Distance", source.reference_distance, ranges::REFERENCE_DISTANCE) {
            new.reference_distance = v;
            hint = Some("reference_distance");
        }
        if let EditResult::Changed(v) = inspector.f32("Rolloff", source.rolloff_factor, ranges::ROLLOFF) {
            new.rolloff_factor = v;
            hint = Some("rolloff_factor");
        }
    }

    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

/// Apply an inspector edit: write the new value to the world (for immediate
/// visual feedback) and record it on the undo stack with merge support, so
/// continuous slider drags collapse into a single undo entry.
pub fn apply_component_edit<T: ecs::Component + Clone>(
    world: &mut ecs::World,
    entity: ecs::EntityId,
    old: &T,
    edit: Option<ComponentEdit<T>>,
    history: &mut crate::commands::CommandHistory,
    make_cmd: impl FnOnce(ecs::EntityId, T, T, &'static str) -> Box<dyn crate::commands::EditorCommand>,
) {
    if let Some(ComponentEdit { new_value, field_hint }) = edit {
        if let Some(c) = world.get_mut::<T>(entity) {
            *c = new_value.clone();
        }
        history.try_merge_or_push(make_cmd(entity, old.clone(), new_value, field_hint));
    }
}

/// Render a small [X] remove button at the header position of a component
/// whose `edit_*()` function renders the header internally — the button is
/// overlaid at the same Y position. Used by the registry-generated
/// `edit_all_components`.
pub(crate) fn remove_button(
    ui: &mut ui::UIContext,
    component_index: usize,
    x: f32,
    header_y: f32,
    width: f32,
) -> bool {
    let btn_size = 18.0;
    let btn_x = crate::row_layout::remove_button_x(x, width, btn_size);
    let btn_bounds = ui::Rect::new(btn_x, header_y, btn_size, btn_size);
    let btn_id = crate::FieldId::new(component_index, 99, 0);
    ui.button(btn_id, "X", btn_bounds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn test_component_edit_carries_value_and_hint() {
        let edit = ComponentEdit {
            new_value: Transform2D::new(Vec2::new(5.0, 6.0)),
            field_hint: "position",
        };
        assert_eq!(edit.new_value.position, Vec2::new(5.0, 6.0));
        assert_eq!(edit.field_hint, "position");
    }

    #[test]
    fn test_component_edit_equality() {
        let a = ComponentEdit { new_value: 1.0_f32, field_hint: "x" };
        let b = ComponentEdit { new_value: 1.0_f32, field_hint: "x" };
        let c = ComponentEdit { new_value: 2.0_f32, field_hint: "x" };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_ranges_are_well_formed() {
        assert!(ranges::POSITION.start() < ranges::POSITION.end());
        assert!(ranges::SCALE.start() > &0.0); // scale must stay positive
        assert!(ranges::PITCH.start() > &0.0); // pitch of zero is silence
        assert!(ranges::ROTATION.contains(&0.0));
        // Collider dimensions must stay positive (rapier rejects zero extents)
        assert!(ranges::COLLIDER_EXTENT.start() > &0.0);
    }

    #[test]
    fn test_transform_default_values() {
        let transform = Transform2D::default();
        assert_eq!(transform.position, Vec2::ZERO);
        assert_eq!(transform.rotation, 0.0);
        assert_eq!(transform.scale, Vec2::ONE);
    }

    #[test]
    fn test_sprite_default_values() {
        let sprite = Sprite::default();
        assert_eq!(sprite.offset, Vec2::ZERO);
        assert_eq!(sprite.rotation, 0.0);
        assert_eq!(sprite.scale, Vec2::ONE);
        assert_eq!(sprite.depth, 0.0);
    }

    #[test]
    fn test_rigid_body_default_values() {
        let body = RigidBody::default();
        assert_eq!(body.body_type, RigidBodyType::Dynamic);
        assert_eq!(body.velocity, Vec2::ZERO);
        assert_eq!(body.gravity_scale, 1.0);
    }

    #[test]
    fn test_collider_default_values() {
        let collider = Collider::default();
        assert_eq!(collider.offset, Vec2::ZERO);
        assert!(!collider.is_sensor);
        assert_eq!(collider.friction, 0.5);
        assert_eq!(collider.restitution, 0.0);
    }

    #[test]
    fn test_audio_source_default_values() {
        let source = AudioSource::default();
        assert_eq!(source.volume, 1.0);
        assert_eq!(source.pitch, 1.0);
        assert!(!source.looping);
        assert!(!source.spatial);
    }
}
