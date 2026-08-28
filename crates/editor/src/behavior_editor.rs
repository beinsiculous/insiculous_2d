//! Editable inspector for the `Behavior` component.
//!
//! Behaviors are an enum, so the editor shows a variant cycle selector
//! (`< PlayerPlatformer >`) followed by the selected variant's fields.
//! Switching variants replaces the behavior with that variant's defaults.
//!
//! String fields (tags, target names) are editable via `string_edit`
//! (commits on Enter/Tab/click-away); distinct `field_hint`s keep edits to
//! different fields from merging into one undo entry.

use ecs::behavior::Behavior;

use crate::component_editors::ComponentEdit;
use crate::editable_inspector::{EditResult, EditableInspector};

/// Value ranges for behavior field editors.
mod ranges {
    use std::ops::RangeInclusive;

    /// Movement/follow/chase speeds in pixels per second.
    pub const SPEED: RangeInclusive<f32> = 0.0..=1000.0;
    /// Jump impulse strength.
    pub const IMPULSE: RangeInclusive<f32> = 0.0..=2000.0;
    /// Cooldowns and wait times in seconds.
    pub const SECONDS: RangeInclusive<f32> = 0.0..=30.0;
    /// Follow/detection/lose-interest distances in pixels.
    pub const DISTANCE: RangeInclusive<f32> = 0.0..=5000.0;
    /// Patrol points cover most game worlds (matches Transform2D position).
    pub const POSITION: RangeInclusive<f32> = -1000.0..=1000.0;
    /// Unit fractions (CameraFollow lerp speed: 1.0 snaps instantly).
    pub const FRACTION: RangeInclusive<f32> = 0.0..=1.0;
    /// Camera look-ahead magnitude in pixels (the pressed direction supplies
    /// the sign, so only non-negative extents are meaningful).
    pub const LOOK_AHEAD: RangeInclusive<f32> = 0.0..=1000.0;
}

/// Edit a Behavior component.
///
/// Returns `Some(ComponentEdit)` if the variant was switched or a field
/// changed this frame.
pub fn edit_behavior(
    inspector: &mut EditableInspector<'_>,
    behavior: &Behavior,
    _extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<Behavior>> {
    inspector.header("Behavior");

    let variant_count = Behavior::VARIANT_NAMES.len();
    if let EditResult::Changed(new_index) = inspector.cycle(
        "Variant",
        behavior.variant_name(),
        behavior.variant_index(),
        variant_count,
    ) {
        return Some(ComponentEdit {
            new_value: Behavior::default_for_variant(new_index),
            field_hint: "variant",
        });
    }

    let mut new = behavior.clone();
    let mut hint = None;

    match &mut new {
        Behavior::PlayerPlatformer { move_speed, jump_impulse, jump_cooldown, tag } => {
            if let EditResult::Changed(v) = inspector.f32("Move Speed", *move_speed, ranges::SPEED) {
                *move_speed = v;
                hint = Some("move_speed");
            }
            if let EditResult::Changed(v) = inspector.f32("Jump Impulse", *jump_impulse, ranges::IMPULSE) {
                *jump_impulse = v;
                hint = Some("jump_impulse");
            }
            if let EditResult::Changed(v) = inspector.f32("Jump Cooldown", *jump_cooldown, ranges::SECONDS) {
                *jump_cooldown = v;
                hint = Some("jump_cooldown");
            }
            if let EditResult::Changed(v) = inspector.string_edit("Tag", tag) {
                *tag = v;
                hint = Some("tag");
            }
        }
        Behavior::PlayerTopDown { move_speed, tag } => {
            if let EditResult::Changed(v) = inspector.f32("Move Speed", *move_speed, ranges::SPEED) {
                *move_speed = v;
                hint = Some("move_speed");
            }
            if let EditResult::Changed(v) = inspector.string_edit("Tag", tag) {
                *tag = v;
                hint = Some("tag");
            }
        }
        Behavior::FollowEntity { target_name, follow_distance, follow_speed } => {
            if let EditResult::Changed(v) = inspector.string_edit("Target Name", target_name) {
                *target_name = v;
                hint = Some("target_name");
            }
            if let EditResult::Changed(v) = inspector.f32("Distance", *follow_distance, ranges::DISTANCE) {
                *follow_distance = v;
                hint = Some("follow_distance");
            }
            if let EditResult::Changed(v) = inspector.f32("Speed", *follow_speed, ranges::SPEED) {
                *follow_speed = v;
                hint = Some("follow_speed");
            }
        }
        Behavior::FollowTagged { target_tag, follow_distance, follow_speed } => {
            if let EditResult::Changed(v) = inspector.string_edit("Target Tag", target_tag) {
                *target_tag = v;
                hint = Some("target_tag");
            }
            if let EditResult::Changed(v) = inspector.f32("Distance", *follow_distance, ranges::DISTANCE) {
                *follow_distance = v;
                hint = Some("follow_distance");
            }
            if let EditResult::Changed(v) = inspector.f32("Speed", *follow_speed, ranges::SPEED) {
                *follow_speed = v;
                hint = Some("follow_speed");
            }
        }
        Behavior::Patrol { point_a, point_b, speed, wait_time } => {
            if let EditResult::Changed(v) = inspector.vec2(
                "Point A",
                glam::Vec2::new(point_a.0, point_a.1),
                ranges::POSITION,
            ) {
                *point_a = (v.x, v.y);
                hint = Some("point_a");
            }
            if let EditResult::Changed(v) = inspector.vec2(
                "Point B",
                glam::Vec2::new(point_b.0, point_b.1),
                ranges::POSITION,
            ) {
                *point_b = (v.x, v.y);
                hint = Some("point_b");
            }
            if let EditResult::Changed(v) = inspector.f32("Speed", *speed, ranges::SPEED) {
                *speed = v;
                hint = Some("speed");
            }
            if let EditResult::Changed(v) = inspector.f32("Wait Time", *wait_time, ranges::SECONDS) {
                *wait_time = v;
                hint = Some("wait_time");
            }
        }
        Behavior::Collectible { score_value, despawn_on_collect, collector_tag } => {
            inspector.u32("Score Value", *score_value);
            if let EditResult::Changed(v) = inspector.bool("Despawn", *despawn_on_collect) {
                *despawn_on_collect = v;
                hint = Some("despawn_on_collect");
            }
            if let EditResult::Changed(v) = inspector.string_edit("Collector Tag", collector_tag) {
                *collector_tag = v;
                hint = Some("collector_tag");
            }
        }
        Behavior::ChaseTagged { target_tag, detection_range, chase_speed, lose_interest_range } => {
            if let EditResult::Changed(v) = inspector.string_edit("Target Tag", target_tag) {
                *target_tag = v;
                hint = Some("target_tag");
            }
            if let EditResult::Changed(v) = inspector.f32("Detect Range", *detection_range, ranges::DISTANCE) {
                *detection_range = v;
                hint = Some("detection_range");
            }
            if let EditResult::Changed(v) = inspector.f32("Chase Speed", *chase_speed, ranges::SPEED) {
                *chase_speed = v;
                hint = Some("chase_speed");
            }
            if let EditResult::Changed(v) = inspector.f32("Lose Range", *lose_interest_range, ranges::DISTANCE) {
                *lose_interest_range = v;
                hint = Some("lose_interest_range");
            }
        }
        Behavior::CameraFollow {
            target_tag, lerp_speed, offset, dead_zone, look_ahead, look_ahead_lerp,
        } => {
            if let EditResult::Changed(v) = inspector.string_edit("Target Tag", target_tag) {
                *target_tag = v;
                hint = Some("target_tag");
            }
            if let EditResult::Changed(v) = inspector.f32("Lerp Speed", *lerp_speed, ranges::FRACTION) {
                *lerp_speed = v;
                hint = Some("lerp_speed");
            }
            if let EditResult::Changed(v) = inspector.vec2(
                "Offset",
                glam::Vec2::new(offset.0, offset.1),
                ranges::POSITION,
            ) {
                *offset = (v.x, v.y);
                hint = Some("offset");
            }
            // Read-only until the ui crate grows an Option/toggle widget —
            // this is an Option<(f32, f32)>, not a string (issue #34 scope
            // note).
            let dead_zone_label = match dead_zone {
                Some((w, h)) => format!("{w:.0} x {h:.0}"),
                None => "None".to_string(),
            };
            inspector.string("Dead Zone", &dead_zone_label);
            if let EditResult::Changed(v) = inspector.vec2(
                "Look Ahead",
                glam::Vec2::new(look_ahead.0, look_ahead.1),
                ranges::LOOK_AHEAD,
            ) {
                *look_ahead = (v.x, v.y);
                hint = Some("look_ahead");
            }
            if let EditResult::Changed(v) =
                inspector.f32("Look Lerp", *look_ahead_lerp, ranges::FRACTION)
            {
                *look_ahead_lerp = v;
                hint = Some("look_ahead_lerp");
            }
        }
    }

    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ranges_are_well_formed() {
        assert!(ranges::SPEED.start() < ranges::SPEED.end());
        assert!(ranges::IMPULSE.start() < ranges::IMPULSE.end());
        assert!(ranges::SECONDS.contains(&0.3)); // default jump cooldown
        assert!(ranges::DISTANCE.contains(&300.0)); // default lose range
        assert!(ranges::POSITION.contains(&0.0));
        assert!(ranges::LOOK_AHEAD.start() < ranges::LOOK_AHEAD.end());
        assert!(ranges::LOOK_AHEAD.contains(&220.0)); // demo scene look-ahead
        assert!(ranges::FRACTION.contains(&0.08)); // default look_ahead_lerp
    }

    #[test]
    fn test_every_variant_default_is_within_editor_ranges() {
        for index in 0..Behavior::VARIANT_NAMES.len() {
            match Behavior::default_for_variant(index) {
                Behavior::PlayerPlatformer { move_speed, jump_impulse, jump_cooldown, .. } => {
                    assert!(ranges::SPEED.contains(&move_speed));
                    assert!(ranges::IMPULSE.contains(&jump_impulse));
                    assert!(ranges::SECONDS.contains(&jump_cooldown));
                }
                Behavior::PlayerTopDown { move_speed, .. } => {
                    assert!(ranges::SPEED.contains(&move_speed));
                }
                Behavior::FollowEntity { follow_distance, follow_speed, .. }
                | Behavior::FollowTagged { follow_distance, follow_speed, .. } => {
                    assert!(ranges::DISTANCE.contains(&follow_distance));
                    assert!(ranges::SPEED.contains(&follow_speed));
                }
                Behavior::Patrol { point_a, point_b, speed, wait_time } => {
                    assert!(ranges::POSITION.contains(&point_a.0));
                    assert!(ranges::POSITION.contains(&point_b.0));
                    assert!(ranges::SPEED.contains(&speed));
                    assert!(ranges::SECONDS.contains(&wait_time));
                }
                Behavior::Collectible { .. } => {}
                Behavior::ChaseTagged { detection_range, chase_speed, lose_interest_range, .. } => {
                    assert!(ranges::DISTANCE.contains(&detection_range));
                    assert!(ranges::SPEED.contains(&chase_speed));
                    assert!(ranges::DISTANCE.contains(&lose_interest_range));
                }
                Behavior::CameraFollow { lerp_speed, offset, look_ahead, look_ahead_lerp, .. } => {
                    assert!(ranges::FRACTION.contains(&lerp_speed));
                    assert!(ranges::POSITION.contains(&offset.0));
                    assert!(ranges::POSITION.contains(&offset.1));
                    assert!(ranges::LOOK_AHEAD.contains(&look_ahead.0));
                    assert!(ranges::LOOK_AHEAD.contains(&look_ahead.1));
                    assert!(ranges::FRACTION.contains(&look_ahead_lerp));
                }
            }
        }
    }
}
