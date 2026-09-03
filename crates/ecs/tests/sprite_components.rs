//! Public-API contracts of `SpriteAnimation` playback and the sprite
//! components' serde and inspector metadata.

use ecs::sprite_components::*;
use ecs::{ComponentMeta, GridBackdrop, UiButton, UiLabel, UiPanel};

/// A 4x2 sheet carrying a looping 2-frame "walk" and a one-shot 3-frame
/// "hit" — the two shapes every playback test needs.
fn test_animation() -> SpriteAnimation {
    SpriteAnimation::new(SheetGrid::new(4, 2))
        .with_clip("walk", AnimationClip::new(vec![0, 1], 10.0))
        .with_clip("hit", AnimationClip::new(vec![4, 5, 6], 10.0).with_looping(false))
}

#[test]
fn test_play_selects_and_always_restarts_the_named_clip() {
    let mut animation = test_animation();
    assert_eq!(animation.current_uv(), None, "nothing selected: the sprite's region is left alone");

    assert!(animation.play("hit"));
    assert_eq!(animation.current_clip.as_deref(), Some("hit"));
    assert!(animation.playing);
    assert_eq!(animation.current_frame, 0);

    // play() means "start from the beginning", same clip or not...
    animation.update(0.1);
    assert_eq!(animation.current_frame, 1);
    assert!(animation.play("hit"));
    assert_eq!(animation.current_frame, 0);
    assert_eq!(animation.time_accumulator, 0.0);

    // ...including a finished one-shot.
    animation.update(1.0);
    assert!(!animation.playing);
    assert!(animation.play("hit"));
    assert!(animation.playing);
    assert_eq!(animation.current_frame, 0);

    // An unknown name is a no-op that keeps the running clip intact.
    animation.update(0.1);
    assert!(!animation.play("sprint"));
    assert_eq!(animation.current_clip.as_deref(), Some("hit"));
    assert_eq!(animation.current_frame, 1);
    assert!(animation.playing);

    // stop() deselects entirely; resume() then has nothing to resume.
    animation.stop();
    assert_eq!(animation.current_clip, None);
    assert_eq!(animation.current_uv(), None);
    animation.resume();
    assert!(!animation.playing);
}

#[test]
fn test_ensure_playing_restarts_only_a_different_or_stopped_clip() {
    let mut animation = test_animation();

    // The state-machine pattern: re-asserting the clip every update never
    // restarts it, so it advances normally.
    for _ in 0..5 {
        assert!(animation.ensure_playing("walk"));
        animation.update(0.1);
    }
    assert_eq!(animation.current_frame, 1, "five 10 fps steps over a 2-frame loop");

    // A different clip restarts.
    assert!(animation.ensure_playing("hit"));
    assert_eq!(animation.current_clip.as_deref(), Some("hit"));
    assert_eq!(animation.current_frame, 0);

    // The same clip, once it has stopped, restarts.
    animation.update(1.0);
    assert!(!animation.playing);
    assert!(animation.ensure_playing("hit"));
    assert!(animation.playing);
    assert_eq!(animation.current_frame, 0);

    // An unknown clip is still rejected.
    assert!(!animation.ensure_playing("sprint"));
    assert_eq!(animation.current_clip.as_deref(), Some("hit"));
}

#[test]
fn test_switching_to_a_shorter_clip_never_exposes_a_stale_frame() {
    let mut animation = SpriteAnimation::new(SheetGrid::new(4, 4))
        .with_clip("long", AnimationClip::new((0..10).collect::<Vec<_>>(), 10.0))
        .with_clip("short", AnimationClip::new(vec![12, 13], 10.0));
    assert!(animation.play("long"));
    animation.update(0.9);
    assert_eq!(animation.current_frame, 9);

    // Frame 9 does not exist in the 2-frame clip: the switch must reset it.
    assert!(animation.play("short"));

    assert_eq!(animation.current_frame, 0);
    assert_eq!(animation.current_uv(), animation.grid.uv_rect_checked(12));
}

#[test]
fn test_looping_clip_wraps_and_non_looping_clip_clamps_on_its_last_frame() {
    let mut animation = test_animation();
    assert!(animation.play("walk"));

    // Half a frame at 10 fps is not enough; the remainder carries over.
    animation.update(0.05);
    assert_eq!(animation.current_frame, 0);
    animation.update(0.05);
    assert_eq!(animation.current_frame, 1);
    // The loop wraps back to the first frame and keeps playing.
    animation.update(0.1);
    assert_eq!(animation.current_frame, 0);
    assert!(animation.playing);
    // A large delta wraps by modulo: 25 frames over 2 lands on 1.
    animation.update(2.5);
    assert_eq!(animation.current_frame, 1);

    assert!(animation.play("hit"));
    animation.update(0.2);
    assert_eq!(animation.current_frame, 2);
    assert!(animation.playing);
    animation.update(0.2);
    assert_eq!(animation.current_frame, 2, "the one-shot clamps on its last frame");
    assert!(!animation.playing, "and stops for good");
    animation.update(10.0);
    assert_eq!(animation.current_frame, 2);
}

#[test]
fn test_pause_holds_the_frame_and_resume_continues_from_it() {
    let mut animation = test_animation();
    assert!(animation.play("walk"));
    animation.update(0.1);

    animation.pause();
    animation.update(0.5);
    assert!(!animation.playing);
    assert_eq!(animation.current_frame, 1, "a paused clip ignores time");

    // resume() continues where play() would have restarted.
    animation.resume();
    assert!(animation.playing);
    assert_eq!(animation.current_frame, 1);
}

#[test]
fn test_broken_clips_and_deltas_never_panic_or_advance() {
    // Defensive net for programmatically built clips: authored clips are
    // rejected at parse time.
    for fps in [0.0, -5.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        let mut animation = SpriteAnimation::new(SheetGrid::new(2, 1))
            .with_clip("broken", AnimationClip::new(vec![0, 1], fps));
        assert!(animation.play("broken"));
        animation.update(1.0);
        assert_eq!(animation.current_frame, 0, "fps {fps} must not advance");
        // The frame still resolves: only the advance is suppressed.
        assert_eq!(animation.current_uv(), Some([0.0, 0.0, 0.5, 1.0]));
    }

    let mut empty = SpriteAnimation::new(SheetGrid::new(2, 1))
        .with_clip("empty", AnimationClip::new(vec![], 10.0));
    assert!(empty.play("empty"));
    empty.update(1.0);
    assert_eq!(empty.current_frame, 0);
    assert_eq!(empty.current_uv(), None, "an empty clip resolves to nothing");

    let mut animation = test_animation();
    assert!(animation.play("walk"));
    for delta in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
        animation.update(delta);
    }
    assert_eq!(animation.current_frame, 0, "non-finite or negative deltas are ignored");
    assert_eq!(animation.time_accumulator, 0.0);
}

#[test]
fn test_current_uv_maps_the_frame_index_through_the_grid() {
    let mut animation = SpriteAnimation::new(SheetGrid::new(4, 2))
        .with_clip("walk", AnimationClip::new(vec![0, 5], 10.0))
        .with_clip("bad", AnimationClip::new(vec![99], 10.0));
    assert!(animation.play("walk"));

    assert_eq!(animation.current_uv(), Some([0.0, 0.0, 0.25, 0.5]));
    animation.update(0.1);
    // Cell 5 is column 1 of row 1.
    assert_eq!(animation.current_uv(), Some([0.25, 0.5, 0.25, 0.5]));

    assert!(animation.play("bad"));
    assert_eq!(animation.current_uv(), None, "an index past the grid resolves to nothing");
}

#[test]
fn test_sprite_deserializes_omitted_region_and_visibility_to_full_and_visible() -> Result<(), serde_json::Error> {
    // Direct Sprite serde must match the scene-wire semantics: an omitted
    // tex_region is the FULL texture (a plain serde default would be the
    // empty region and render nothing) and an omitted visible is true.
    let sprite: Sprite = serde_json::from_value(serde_json::json!({
        "offset": [0.0, 0.0],
        "rotation": 0.0,
        "scale": [1.0, 1.0],
        "color": [1.0, 1.0, 1.0, 1.0],
        "depth": 0.0,
        "texture_handle": 0
    }))?;

    assert_eq!(sprite.tex_region, [0.0, 0.0, 1.0, 1.0]);
    assert!(sprite.visible);
    assert_eq!(sprite.emissive, 0.0);
    Ok(())
}

#[test]
fn test_component_meta_field_order_matches_the_inspector() {
    // The inspector renders fields in this order, so the order is the contract.
    let expected: [(&str, &[&str]); 8] = [
        (Transform2D::type_name(), &["position", "rotation", "scale"]),
        (
            Sprite::type_name(),
            &["offset", "rotation", "scale", "tex_region", "color", "depth", "visible", "emissive", "texture_handle"],
        ),
        (
            Camera::type_name(),
            &["position", "rotation", "zoom", "viewport_size", "is_main_camera", "near", "far"],
        ),
        (
            SpriteAnimation::type_name(),
            &["grid", "clips", "sheet", "current_clip", "playing", "current_frame", "time_accumulator"],
        ),
        (UiLabel::type_name(), &["text", "anchor", "offset", "font_size", "color", "visible"]),
        (
            UiPanel::type_name(),
            &["anchor", "offset", "size", "background", "border", "border_width", "visible"],
        ),
        (UiButton::type_name(), &["text", "id", "anchor", "offset", "size", "visible"]),
        (
            GridBackdrop::type_name(),
            &[
                "topology", "cols", "rows", "spacing", "color", "emissive", "visible", "stiffness",
                "damping", "rest_pull", "rest_alpha_fraction", "activity_attack", "activity_release",
                "activity_displacement_ref", "activity_velocity_ref",
            ],
        ),
    ];
    let actual = [
        (Transform2D::type_name(), Transform2D::field_names()),
        (Sprite::type_name(), Sprite::field_names()),
        (Camera::type_name(), Camera::field_names()),
        (SpriteAnimation::type_name(), SpriteAnimation::field_names()),
        (UiLabel::type_name(), UiLabel::field_names()),
        (UiPanel::type_name(), UiPanel::field_names()),
        (UiButton::type_name(), UiButton::field_names()),
        (GridBackdrop::type_name(), GridBackdrop::field_names()),
    ];

    assert_eq!(actual, expected);
    assert_eq!(
        actual.map(|(name, _)| name),
        ["Transform2D", "Sprite", "Camera", "SpriteAnimation", "UiLabel", "UiPanel", "UiButton", "GridBackdrop"],
        "the registry keys on these names"
    );
}
