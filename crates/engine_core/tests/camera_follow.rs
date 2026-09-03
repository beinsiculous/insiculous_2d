//! Acceptance tests for `Behavior::CameraFollow` (Phase B, Gap 1) and its
//! input-driven look-ahead.
//!
//! All headless: a `World`, a `BehaviorRunner`, and fixed 60 FPS steps —
//! no physics, so position commands write `Transform2D` directly.
//!
//! Input lifecycle: ONE `InputHandler` lives for the whole simulation (a
//! fresh handler per frame would lose held-key state). Holding a key is a
//! single `KeyPressed` on the first frame whose state persists until the
//! matching `KeyReleased` — `test_support::frame` ends the previous frame,
//! queues that frame's events and processes them before the runner update.
//!
//! Exponential lerps never exactly reach their asymptote, so convergence
//! assertions run to settle and compare within `EPS`.

use ecs::behavior::{Behavior, EntityTag};
use ecs::sprite_components::Transform2D;
use ecs::{EntityId, World};
use engine_core::behavior_runner::BehaviorRunner;
use engine_core::test_support::frame;
use glam::Vec2;
use input::{InputEvent, InputHandler};
use winit::keyboard::KeyCode;

const DT: f32 = 1.0 / 60.0;
/// Frames to run before asserting a settled position.
const SETTLE_FRAMES: usize = 600;
/// Tolerance for settled-position assertions, in pixels.
const EPS: f32 = 0.5;

/// Behavior fields for a camera-follow test entity.
struct Follow {
    lerp_speed: f32,
    offset: (f32, f32),
    dead_zone: Option<(f32, f32)>,
    look_ahead: (f32, f32),
}

impl Follow {
    /// Plain follow at the given lerp speed: no offset, dead zone, or lead.
    fn plain(lerp_speed: f32) -> Self {
        Self { lerp_speed, offset: (0.0, 0.0), dead_zone: None, look_ahead: (0.0, 0.0) }
    }

    fn with_offset(mut self, offset: (f32, f32)) -> Self {
        self.offset = offset;
        self
    }

    fn with_dead_zone(mut self, dead_zone: (f32, f32)) -> Self {
        self.dead_zone = Some(dead_zone);
        self
    }

    fn with_look_ahead(mut self, look_ahead: (f32, f32)) -> Self {
        self.look_ahead = look_ahead;
        self
    }
}

/// A world with a "player"-tagged target at `target_pos` (when given) and a
/// camera-follow entity at `camera_start`, plus a runner and an input handler.
struct Rig {
    world: World,
    runner: BehaviorRunner,
    input: InputHandler,
    camera: EntityId,
}

impl Rig {
    fn new(target_pos: Option<Vec2>, camera_start: Vec2, follow: Follow) -> Self {
        let mut world = World::new();
        if let Some(target_pos) = target_pos {
            let target = world.create_entity();
            world.add_component(&target, Transform2D::new(target_pos)).ok();
            world.add_component(&target, EntityTag::new("player")).ok();
        }
        let camera = world.create_entity();
        world.add_component(&camera, Transform2D::new(camera_start)).ok();
        world
            .add_component(
                &camera,
                Behavior::CameraFollow {
                    target_tag: "player".to_string(),
                    lerp_speed: follow.lerp_speed,
                    offset: follow.offset,
                    dead_zone: follow.dead_zone,
                    look_ahead: follow.look_ahead,
                    look_ahead_lerp: 0.08,
                },
            )
            .ok();
        Self { world, runner: BehaviorRunner::new(), input: InputHandler::new(), camera }
    }

    fn with_target(target_pos: Vec2, follow: Follow) -> Self {
        Self::new(Some(target_pos), Vec2::ZERO, follow)
    }

    fn camera_position(&self) -> Vec2 {
        self.world.get::<Transform2D>(self.camera).expect("camera transform").position
    }

    /// Advance `frames` frames; `first_frame_events` are queued on the first
    /// one only (a press or release persists on the shared handler).
    fn step(&mut self, first_frame_events: &[InputEvent], frames: usize) {
        for index in 0..frames {
            frame(&mut self.input, if index == 0 { first_frame_events } else { &[] });
            self.runner.update(&mut self.world, &self.input, DT, None);
        }
    }

    fn hold(&mut self, keys: &[KeyCode], frames: usize) {
        let presses: Vec<InputEvent> = keys.iter().map(|key| InputEvent::KeyPressed(*key)).collect();
        self.step(&presses, frames);
    }

    fn release(&mut self, keys: &[KeyCode], frames: usize) {
        let releases: Vec<InputEvent> = keys.iter().map(|key| InputEvent::KeyReleased(*key)).collect();
        self.step(&releases, frames);
    }
}

fn assert_near(actual: Vec2, expected: Vec2, what: &str) {
    assert!((actual - expected).length() < EPS, "{what}: expected ~{expected}, got {actual}");
}

#[test]
fn test_camera_converges_on_the_target_plus_offset_at_the_lerp_speed() {
    // 0.5 per frame over 10 frames leaves 0.5^10 ≈ 0.1% of the distance.
    let target_pos = Vec2::new(400.0, 300.0);
    let mut rig = Rig::with_target(target_pos, Follow::plain(0.5));
    rig.step(&[], 10);
    let remaining = (target_pos - rig.camera_position()).length();
    assert!(
        remaining < target_pos.length() * 0.01,
        "camera should be within 1% of target after 10 frames, {remaining} px left"
    );

    // Lerp 1.0 snaps in a single frame, and the offset shifts the point it
    // converges on.
    let mut rig = Rig::with_target(Vec2::new(100.0, 100.0), Follow::plain(1.0).with_offset((0.0, 50.0)));
    rig.step(&[], 1);
    assert_eq!(rig.camera_position(), Vec2::new(100.0, 150.0));
}

#[test]
fn test_dead_zone_ignores_targets_inside_the_box() {
    // Target 40 px away, dead zone half-extent (100, 60) — inside the box.
    let mut rig = Rig::with_target(Vec2::new(40.0, 30.0), Follow::plain(0.5).with_dead_zone((200.0, 120.0)));
    rig.step(&[], 30);
    assert_eq!(rig.camera_position(), Vec2::ZERO);
}

#[test]
fn test_dead_zone_converges_with_target_on_the_box_edge_and_no_target_stays_put() {
    // Target 400 px right of camera, dead zone 200 px wide (100 half-extent):
    // camera moves right until the target sits on the box's right edge.
    let mut rig = Rig::with_target(Vec2::new(400.0, 0.0), Follow::plain(0.5).with_dead_zone((200.0, 200.0)));
    rig.step(&[], 40);
    let pos = rig.camera_position();
    assert!((pos - Vec2::new(300.0, 0.0)).length() < 1.0, "camera should stop with target on box edge (300, 0), got {pos}");

    // Nothing carries the tag: the camera does not drift toward the origin.
    let mut rig = Rig::new(None, Vec2::new(5.0, 5.0), Follow::plain(0.5).with_look_ahead((220.0, 140.0)));
    rig.step(&[], 5);
    assert_eq!(rig.camera_position(), Vec2::new(5.0, 5.0));
}

#[test]
fn test_holding_a_direction_leads_the_camera_by_look_ahead_ramping_in_and_decaying_out() {
    // Settled lead for each held direction (+y = up), including the cases
    // that must produce no lead at all and the dead zone absorbing its
    // half-width of it (the lead applies to the focus point BEFORE the
    // dead-zone clamp, so a stationary player leads by 220 − 80).
    let look_ahead = (220.0, 140.0);
    let rows: [(&[KeyCode], Vec2, Follow, Vec2, &str); 6] = [
        (&[KeyCode::KeyD], Vec2::ZERO, Follow::plain(0.5).with_look_ahead(look_ahead), Vec2::new(220.0, 0.0), "holding right leads by look_ahead.x"),
        (&[KeyCode::KeyW], Vec2::ZERO, Follow::plain(0.5).with_look_ahead(look_ahead), Vec2::new(0.0, 140.0), "holding up leads by +look_ahead.y"),
        (&[KeyCode::KeyS], Vec2::ZERO, Follow::plain(0.5).with_look_ahead(look_ahead), Vec2::new(0.0, -140.0), "holding down leads by -look_ahead.y"),
        (&[KeyCode::KeyA, KeyCode::KeyD], Vec2::ZERO, Follow::plain(0.5).with_look_ahead(look_ahead), Vec2::ZERO, "left + right cancel"),
        (&[KeyCode::KeyD], Vec2::new(100.0, 0.0), Follow::plain(0.5), Vec2::new(100.0, 0.0), "look_ahead (0,0) is plain follow"),
        (&[KeyCode::KeyD], Vec2::ZERO, Follow::plain(0.5).with_dead_zone((160.0, 100.0)).with_look_ahead((220.0, 0.0)), Vec2::new(140.0, 0.0), "the dead zone absorbs its half-width of the lead"),
    ];
    for (keys, target_pos, follow, expected, why) in rows {
        let mut rig = Rig::with_target(target_pos, follow);
        rig.hold(keys, SETTLE_FRAMES);
        assert_near(rig.camera_position(), expected, why);
    }

    // The lead ramps in rather than snapping, and glides back out on release.
    let mut rig = Rig::with_target(Vec2::ZERO, Follow::plain(0.5).with_look_ahead((220.0, 0.0)));
    rig.hold(&[KeyCode::KeyD], 1);
    let x = rig.camera_position().x;
    assert!(x > 0.0 && x < 220.0, "one frame of holding right should ramp partway, got {x}");
    rig.hold(&[], SETTLE_FRAMES);
    rig.release(&[KeyCode::KeyD], SETTLE_FRAMES);
    assert_near(rig.camera_position(), Vec2::ZERO, "releasing should glide back to the plain follow position");
}

#[test]
fn test_negative_and_nan_look_ahead_degrade_to_plain_follow() {
    let mut rig = Rig::with_target(Vec2::new(100.0, 50.0), Follow::plain(0.5).with_look_ahead((-220.0, f32::NAN)));

    for index in 0..SETTLE_FRAMES {
        let events: &[InputEvent] = if index == 0 { &[InputEvent::KeyPressed(KeyCode::KeyD)] } else { &[] };
        rig.step(events, 1);
        assert!(rig.camera_position().is_finite(), "bad scene data must never produce a non-finite position");
    }

    assert_near(rig.camera_position(), Vec2::new(100.0, 50.0), "negative/NaN look-ahead should behave as plain follow");
}
