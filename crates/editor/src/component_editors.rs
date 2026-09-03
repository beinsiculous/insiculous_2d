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

/// The `GridBackdrop` editor (#46), a child module for file size.
pub mod grid_backdrop;
pub use grid_backdrop::edit_grid_backdrop;

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
    /// Scale prevents negative/zero values.
    pub const SCALE: RangeInclusive<f32> = 0.01..=10.0;
    /// Damping is genuinely unbounded in rapier; the soft range covers the
    /// useful span (the old normalized widget LIED — a damping of 2.0
    /// displayed as 1.00 and was uneditable).
    pub const DAMPING: RangeInclusive<f32> = 0.0..=10.0;
    /// Friction above 1.0 is legal in rapier (soft range).
    pub const FRICTION: RangeInclusive<f32> = 0.0..=2.0;
    /// Restitution is conventionally 0..=1 (soft range).
    pub const RESTITUTION: RangeInclusive<f32> = 0.0..=1.0;
    /// Volume is conventionally 0..=1 (soft range).
    pub const VOLUME: RangeInclusive<f32> = 0.0..=1.0;
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

/// Edit an EntityTag component (tag-based gameplay wiring:
/// FollowTagged/ChaseTagged/CameraFollow targets, collectible collectors).
pub fn edit_entity_tag(
    inspector: &mut EditableInspector<'_>,
    tag: &ecs::behavior::EntityTag,
    _extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<ecs::behavior::EntityTag>> {
    inspector.header("EntityTag");
    if let EditResult::Changed(v) = inspector.string_edit("Tag", &tag.0) {
        if v != tag.0 {
            return Some(ComponentEdit {
                new_value: ecs::behavior::EntityTag(v),
                field_hint: "tag",
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
    if let EditResult::Changed(v) = inspector.angle("Rotation", transform.rotation) {
        new.rotation = v;
        hint = Some("rotation");
    }
    if let EditResult::Changed(v) = inspector.vec2("Scale", transform.scale, ranges::SCALE) {
        // Hard physical floor: soft ranges let typing exceed the range, but
        // a zero/negative scale breaks rendering math.
        new.scale = v.max(glam::Vec2::splat(0.01));
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
    if let EditResult::Changed(v) = inspector.angle("Rotation", sprite.rotation) {
        new.rotation = v;
        hint = Some("rotation");
    }
    if let EditResult::Changed(v) = inspector.vec2("Scale", sprite.scale, ranges::SCALE) {
        new.scale = v.max(glam::Vec2::splat(0.01));
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

    // Body type cycles like any other enum. NOTE (engine footgun): a live
    // body_type change still requires the rapier body to be recreated —
    // same as damping/gravity_scale edits, it lands on the next body build.
    if let EditResult::Changed(i) = inspector.cycle(
        "Type",
        body.body_type.label(),
        body.body_type.index(),
        RigidBodyType::ALL.len(),
    ) {
        new.body_type = RigidBodyType::ALL[i];
        hint = Some("body_type");
    }

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
    if let EditResult::Changed(v) = inspector.f32("Linear Damping", body.linear_damping, ranges::DAMPING) {
        new.linear_damping = v.max(0.0);
        hint = Some("linear_damping");
    }
    if let EditResult::Changed(v) = inspector.f32("Angular Damping", body.angular_damping, ranges::DAMPING) {
        new.angular_damping = v.max(0.0);
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

    // Shape variant cycles with best-effort dimension carry-across (a
    // cycled collider keeps its footprint; each cycle is one undo entry).
    // Early-return on change — rendering the OLD variant's fields against
    // the new shape the same frame would show stale rows (Behavior
    // variant-cycle precedent). Sizes are absolute pixels — physics ignores
    // Transform2D.scale.
    if let EditResult::Changed(i) = inspector.cycle(
        "Shape",
        collider.shape.variant_name(),
        collider.shape.variant_index(),
        ColliderShape::VARIANT_NAMES.len(),
    ) {
        new.shape = collider.shape.variant_with_carried_dimensions(i);
        return Some(ComponentEdit { new_value: new, field_hint: "shape" });
    }

    match &collider.shape {
        ColliderShape::Box { half_extents } => {
            if let EditResult::Changed(v) =
                inspector.vec2("Half Extents", *half_extents, ranges::COLLIDER_EXTENT)
            {
                // Hard floor: rapier cannot build a zero-extent collider.
                new.shape = ColliderShape::Box { half_extents: v.max(glam::Vec2::splat(0.5)) };
                hint = Some("half_extents");
            }
        }
        ColliderShape::Circle { radius } => {
            if let EditResult::Changed(v) =
                inspector.f32("Radius", *radius, ranges::COLLIDER_EXTENT)
            {
                new.shape = ColliderShape::Circle { radius: v.max(0.5) };
                hint = Some("radius");
            }
        }
        ColliderShape::CapsuleY { half_height, radius } => {
            if let EditResult::Changed(v) =
                inspector.f32("Half Height", *half_height, ranges::COLLIDER_EXTENT)
            {
                new.shape = ColliderShape::CapsuleY { half_height: v.max(0.0), radius: *radius };
                hint = Some("half_height");
            }
            if let EditResult::Changed(v) =
                inspector.f32("Cap Radius", *radius, ranges::COLLIDER_EXTENT)
            {
                new.shape = ColliderShape::CapsuleY { half_height: *half_height, radius: v.max(0.5) };
                hint = Some("radius");
            }
        }
        ColliderShape::CapsuleX { half_height, radius } => {
            if let EditResult::Changed(v) =
                inspector.f32("Half Width", *half_height, ranges::COLLIDER_EXTENT)
            {
                new.shape = ColliderShape::CapsuleX { half_height: v.max(0.0), radius: *radius };
                hint = Some("half_height");
            }
            if let EditResult::Changed(v) =
                inspector.f32("Cap Radius", *radius, ranges::COLLIDER_EXTENT)
            {
                new.shape = ColliderShape::CapsuleX { half_height: *half_height, radius: v.max(0.5) };
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
    if let EditResult::Changed(v) = inspector.f32("Friction", collider.friction, ranges::FRICTION) {
        new.friction = v.max(0.0);
        hint = Some("friction");
    }
    if let EditResult::Changed(v) = inspector.f32("Restitution", collider.restitution, ranges::RESTITUTION) {
        new.restitution = v.max(0.0);
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

    // Volume/pitch are HARD ranges: the audio runtime clamps playback to
    // volume 0..=1 and speed >= 0.1, so an unclamped inspector value would
    // display parameters that are not actually taking effect (kimi F1).
    if let EditResult::Changed(v) = inspector.f32_hard("Volume", source.volume, ranges::VOLUME) {
        new.volume = v;
        hint = Some("volume");
    }
    if let EditResult::Changed(v) = inspector.f32_hard("Pitch", source.pitch, ranges::PITCH) {
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
mod tests;

#[cfg(test)]
mod range_tests {
    use super::ranges;

    #[test]
    fn test_editor_ranges_admit_every_value_rapier_accepts() {
        // The soft ranges only steer the scrub step and the #55 warning; a
        // range that stops short of a legal value would warn on scenes that
        // are fine (friction 2.0, damping above 1) or let a scrub drive a
        // collider extent to zero, which rapier rejects.
        assert!(ranges::POSITION.start() < ranges::POSITION.end());
        assert!(ranges::SCALE.start() > &0.0, "scale must stay positive");
        assert!(ranges::PITCH.start() > &0.0, "pitch of zero is silence");
        assert!(ranges::DAMPING.end() > &1.0, "damping is unbounded in rapier — the range must exceed 1");
        assert!(ranges::FRICTION.end() > &1.0, "friction above 1.0 is legal in rapier");
        assert!(ranges::COLLIDER_EXTENT.start() > &0.0, "rapier rejects zero extents");
    }
}
