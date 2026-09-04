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
    let hint = variant_editor(&new)(inspector, &mut new);

    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

type VariantEditor = fn(&mut EditableInspector<'_>, &mut Behavior) -> Option<&'static str>;

fn variant_editor(behavior: &Behavior) -> VariantEditor {
    match behavior {
        Behavior::PlayerPlatformer { .. } => edit_player_platformer,
        Behavior::PlayerTopDown { .. } => edit_player_top_down,
        Behavior::FollowEntity { .. } => edit_follow_entity,
        Behavior::FollowTagged { .. } => edit_follow_tagged,
        Behavior::Patrol { .. } => edit_patrol,
        Behavior::Collectible { .. } => edit_collectible,
        Behavior::ChaseTagged { .. } => edit_chase_tagged,
        Behavior::CameraFollow { .. } => edit_camera_follow,
    }
}

fn edit_player_platformer(
    inspector: &mut EditableInspector<'_>,
    behavior: &mut Behavior,
) -> Option<&'static str> {
    let Behavior::PlayerPlatformer { move_speed, jump_impulse, jump_cooldown, tag } = behavior else {
        log::error!("expected PlayerPlatformer behavior");
        return None;
    };
    let mut hint = None;
    inspector.f32("Move Speed", *move_speed, ranges::SPEED).assign(move_speed, &mut hint, "move_speed");
    inspector.f32("Jump Impulse", *jump_impulse, ranges::IMPULSE).assign(jump_impulse, &mut hint, "jump_impulse");
    inspector.f32("Jump Cooldown", *jump_cooldown, ranges::SECONDS).assign(jump_cooldown, &mut hint, "jump_cooldown");
    inspector.string_edit("Tag", tag).assign(tag, &mut hint, "tag");
    hint
}

fn edit_player_top_down(
    inspector: &mut EditableInspector<'_>,
    behavior: &mut Behavior,
) -> Option<&'static str> {
    let Behavior::PlayerTopDown { move_speed, tag } = behavior else {
        log::error!("expected PlayerTopDown behavior");
        return None;
    };
    let mut hint = None;
    inspector.f32("Move Speed", *move_speed, ranges::SPEED).assign(move_speed, &mut hint, "move_speed");
    inspector.string_edit("Tag", tag).assign(tag, &mut hint, "tag");
    hint
}

fn edit_follow_entity(
    inspector: &mut EditableInspector<'_>,
    behavior: &mut Behavior,
) -> Option<&'static str> {
    let Behavior::FollowEntity { target_name, follow_distance, follow_speed } = behavior else {
        log::error!("expected FollowEntity behavior");
        return None;
    };
    let mut hint = None;
    inspector.string_edit("Target Name", target_name).assign(target_name, &mut hint, "target_name");
    inspector.f32("Distance", *follow_distance, ranges::DISTANCE).assign(follow_distance, &mut hint, "follow_distance");
    inspector.f32("Speed", *follow_speed, ranges::SPEED).assign(follow_speed, &mut hint, "follow_speed");
    hint
}

fn edit_follow_tagged(
    inspector: &mut EditableInspector<'_>,
    behavior: &mut Behavior,
) -> Option<&'static str> {
    let Behavior::FollowTagged { target_tag, follow_distance, follow_speed } = behavior else {
        log::error!("expected FollowTagged behavior");
        return None;
    };
    let mut hint = None;
    inspector.string_edit("Target Tag", target_tag).assign(target_tag, &mut hint, "target_tag");
    inspector.f32("Distance", *follow_distance, ranges::DISTANCE).assign(follow_distance, &mut hint, "follow_distance");
    inspector.f32("Speed", *follow_speed, ranges::SPEED).assign(follow_speed, &mut hint, "follow_speed");
    hint
}

fn edit_point(
    inspector: &mut EditableInspector<'_>,
    label: &str,
    point: &mut (f32, f32),
    range: std::ops::RangeInclusive<f32>,
    hint: &mut Option<&'static str>,
    field_hint: &'static str,
) {
    if let EditResult::Changed(vector) = inspector.vec2(
        label,
        glam::Vec2::new(point.0, point.1),
        range,
    ) {
        *point = (vector.x, vector.y);
        *hint = Some(field_hint);
    }
}

fn edit_patrol(
    inspector: &mut EditableInspector<'_>,
    behavior: &mut Behavior,
) -> Option<&'static str> {
    let Behavior::Patrol { point_a, point_b, speed, wait_time } = behavior else {
        log::error!("expected Patrol behavior");
        return None;
    };
    let mut hint = None;
    edit_point(inspector, "Point A", point_a, ranges::POSITION, &mut hint, "point_a");
    edit_point(inspector, "Point B", point_b, ranges::POSITION, &mut hint, "point_b");
    inspector.f32("Speed", *speed, ranges::SPEED).assign(speed, &mut hint, "speed");
    inspector.f32("Wait Time", *wait_time, ranges::SECONDS).assign(wait_time, &mut hint, "wait_time");
    hint
}

fn edit_collectible(
    inspector: &mut EditableInspector<'_>,
    behavior: &mut Behavior,
) -> Option<&'static str> {
    let Behavior::Collectible { score_value, despawn_on_collect, collector_tag } = behavior else {
        log::error!("expected Collectible behavior");
        return None;
    };
    let mut hint = None;
    inspector.u32("Score Value", *score_value);
    inspector.bool("Despawn", *despawn_on_collect).assign(despawn_on_collect, &mut hint, "despawn_on_collect");
    inspector.string_edit("Collector Tag", collector_tag).assign(collector_tag, &mut hint, "collector_tag");
    hint
}

fn edit_chase_tagged(
    inspector: &mut EditableInspector<'_>,
    behavior: &mut Behavior,
) -> Option<&'static str> {
    let Behavior::ChaseTagged { target_tag, detection_range, chase_speed, lose_interest_range } = behavior else {
        log::error!("expected ChaseTagged behavior");
        return None;
    };
    let mut hint = None;
    inspector.string_edit("Target Tag", target_tag).assign(target_tag, &mut hint, "target_tag");
    inspector.f32("Detect Range", *detection_range, ranges::DISTANCE).assign(detection_range, &mut hint, "detection_range");
    inspector.f32("Chase Speed", *chase_speed, ranges::SPEED).assign(chase_speed, &mut hint, "chase_speed");
    inspector.f32("Lose Range", *lose_interest_range, ranges::DISTANCE).assign(lose_interest_range, &mut hint, "lose_interest_range");
    hint
}

fn edit_camera_follow(
    inspector: &mut EditableInspector<'_>,
    behavior: &mut Behavior,
) -> Option<&'static str> {
    let Behavior::CameraFollow {
        target_tag, lerp_speed, offset, dead_zone, look_ahead, look_ahead_lerp,
    } = behavior else {
        log::error!("expected CameraFollow behavior");
        return None;
    };
    let mut hint = None;
    inspector.string_edit("Target Tag", target_tag).assign(target_tag, &mut hint, "target_tag");
    inspector.f32("Lerp Speed", *lerp_speed, ranges::FRACTION).assign(lerp_speed, &mut hint, "lerp_speed");
    edit_point(inspector, "Offset", offset, ranges::POSITION, &mut hint, "offset");
    // Read-only until the ui crate grows an Option/toggle widget —
    // this is an Option<(f32, f32)>, not a string.
    let dead_zone_label = match dead_zone {
        Some((width, height)) => format!("{width:.0} x {height:.0}"),
        None => "None".to_string(),
    };
    inspector.string("Dead Zone", &dead_zone_label);
    edit_point(inspector, "Look Ahead", look_ahead, ranges::LOOK_AHEAD, &mut hint, "look_ahead");
    inspector.f32("Look Lerp", *look_ahead_lerp, ranges::FRACTION).assign(look_ahead_lerp, &mut hint, "look_ahead_lerp");
    hint
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The soft ranges steer the scrub step and the typed-commit warning:
    /// each must be a real interval and must admit the values the
    /// engine's own defaults and demo scenes use, or every fresh Behavior
    /// warns the moment it is typed into.
    #[test]
    fn test_behavior_ranges_are_intervals_that_admit_the_engine_defaults() {
        let intervals = [
            ("SPEED", ranges::SPEED),
            ("IMPULSE", ranges::IMPULSE),
            ("SECONDS", ranges::SECONDS),
            ("DISTANCE", ranges::DISTANCE),
            ("POSITION", ranges::POSITION),
            ("FRACTION", ranges::FRACTION),
            ("LOOK_AHEAD", ranges::LOOK_AHEAD),
        ];
        for (name, range) in intervals {
            assert!(range.start() < range.end(), "{name} must be a non-empty interval, got {range:?}");
        }

        assert!(ranges::SECONDS.contains(&0.3), "default jump cooldown");
        assert!(ranges::DISTANCE.contains(&300.0), "default lose-interest range");
        assert!(ranges::POSITION.contains(&0.0), "the origin is a valid position");
        assert!(ranges::LOOK_AHEAD.contains(&220.0), "the demo scene's look-ahead");
        assert!(ranges::FRACTION.contains(&0.08), "default look_ahead_lerp");
    }

    /// Cycling the variant selector hands the author `default_for_variant`;
    /// every one of those defaults must sit inside the editor's soft ranges
    /// or the fresh variant is flagged as out of range before a single edit.
    #[test]
    fn test_every_variant_default_is_within_editor_ranges() {
        for index in 0..Behavior::VARIANT_NAMES.len() {
            let name = Behavior::VARIANT_NAMES[index];
            match Behavior::default_for_variant(index) {
                Behavior::PlayerPlatformer { move_speed, jump_impulse, jump_cooldown, .. } => {
                    assert!(ranges::SPEED.contains(&move_speed), "{name} move_speed");
                    assert!(ranges::IMPULSE.contains(&jump_impulse), "{name} jump_impulse");
                    assert!(ranges::SECONDS.contains(&jump_cooldown), "{name} jump_cooldown");
                }
                Behavior::PlayerTopDown { move_speed, .. } => {
                    assert!(ranges::SPEED.contains(&move_speed), "{name} move_speed");
                }
                Behavior::FollowEntity { follow_distance, follow_speed, .. }
                | Behavior::FollowTagged { follow_distance, follow_speed, .. } => {
                    assert!(ranges::DISTANCE.contains(&follow_distance), "{name} follow_distance");
                    assert!(ranges::SPEED.contains(&follow_speed), "{name} follow_speed");
                }
                Behavior::Patrol { point_a, point_b, speed, wait_time } => {
                    assert!(ranges::POSITION.contains(&point_a.0), "{name} point_a");
                    assert!(ranges::POSITION.contains(&point_b.0), "{name} point_b");
                    assert!(ranges::SPEED.contains(&speed), "{name} speed");
                    assert!(ranges::SECONDS.contains(&wait_time), "{name} wait_time");
                }
                Behavior::Collectible { .. } => {}
                Behavior::ChaseTagged { detection_range, chase_speed, lose_interest_range, .. } => {
                    assert!(ranges::DISTANCE.contains(&detection_range), "{name} detection_range");
                    assert!(ranges::SPEED.contains(&chase_speed), "{name} chase_speed");
                    assert!(ranges::DISTANCE.contains(&lose_interest_range), "{name} lose_interest_range");
                }
                Behavior::CameraFollow { lerp_speed, offset, look_ahead, look_ahead_lerp, .. } => {
                    assert!(ranges::FRACTION.contains(&lerp_speed), "{name} lerp_speed");
                    assert!(ranges::POSITION.contains(&offset.0), "{name} offset.x");
                    assert!(ranges::POSITION.contains(&offset.1), "{name} offset.y");
                    assert!(ranges::LOOK_AHEAD.contains(&look_ahead.0), "{name} look_ahead.x");
                    assert!(ranges::LOOK_AHEAD.contains(&look_ahead.1), "{name} look_ahead.y");
                    assert!(ranges::FRACTION.contains(&look_ahead_lerp), "{name} look_ahead_lerp");
                }
            }
        }
    }

    #[test]
    fn test_every_variant_renders_without_a_phantom_edit() {
        use crate::test_support::extras;
        use crate::{DragDropState, EditableFieldStyle};

        let mut ui = ui::UIContext::new();
        let input = input::InputHandler::new();
        let mut drag_drop = DragDropState::new();
        let style = EditableFieldStyle::default();
        let window = glam::Vec2::new(800.0, 600.0);

        for index in 0..Behavior::VARIANT_NAMES.len() {
            let behavior = Behavior::default_for_variant(index);
            ui.begin_frame(&input, window);
            let mut inspector = EditableInspector::new(&mut ui, &style, 10.0, 10.0);
            let mut test_extras = extras(&mut drag_drop);
            let edit = edit_behavior(&mut inspector, &behavior, &mut test_extras);
            ui.end_frame();
            assert!(
                edit.is_none(),
                "variant {} produced a phantom edit on inert frame: {:?}",
                Behavior::VARIANT_NAMES[index],
                edit
            );
        }
    }
}
