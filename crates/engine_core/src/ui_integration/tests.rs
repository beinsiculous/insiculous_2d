use super::*;
use renderer::texture::TextureHandle;
use ui::Color;

fn test_camera() -> Camera {
    Camera::new(Vec2::ZERO, Vec2::new(800.0, 600.0))
}

/// All white-texture instances regardless of clip batch (clipped UI
/// splits same-texture sprites into per-clip batches since issue #41).
fn white_instances(batcher: &SpriteBatcher) -> Vec<renderer::sprite_data::SpriteInstance> {
    let mut keys: Vec<_> = batcher.batches().keys().filter(|(h, _)| h.id == 0).copied().collect();
    keys.sort_by_key(|(_, clip)| *clip);
    keys.iter().flat_map(|k| batcher.batches()[k].instances.iter().copied()).collect()
}

fn render_one(cmd: DrawCommand, camera: &Camera) -> Vec<renderer::sprite_data::SpriteInstance> {
    let mut batcher = SpriteBatcher::new();
    render_ui_commands(&mut batcher, &[cmd], camera, &HashMap::new());
    white_instances(&batcher)
}

fn rect(bounds: Rect, corner_radius: f32) -> DrawCommand {
    DrawCommand::Rect { bounds, color: Color::WHITE, corner_radius, depth: 1.0 }
}

fn line(start: Vec2, end: Vec2) -> DrawCommand {
    DrawCommand::Line { start, end, color: Color::WHITE, width: 1.0, depth: 0.5 }
}

#[test]
fn test_ui_stays_at_screen_position_with_its_authored_color_under_moved_zoomed_camera() {
    // THE camera-follow/editor invariant: UI sprites must land at the
    // same SCREEN pixels no matter where the camera is or how far it
    // zooms (regression: the editor's panel-derived camera used to shift
    // the entire editor UI off screen) — and the authored color reaches
    // the instance untouched, which is what the post-tonemap UI pass
    // (issue #26) displays byte for byte.
    let screen_bounds = Rect::new(10.0, 10.0, 100.0, 40.0);
    let screen_center = Vec2::new(60.0, 30.0);
    let authored = Color::new(0.25, 0.5, 0.75, 0.5);

    for camera in [
        Camera::new(Vec2::ZERO, Vec2::new(800.0, 600.0)),
        Camera::new(Vec2::new(320.0, -150.0), Vec2::new(800.0, 600.0)),
        Camera::new(Vec2::new(-75.5, 12.25), Vec2::new(800.0, 600.0)).with_zoom(2.0),
    ] {
        let cmd = DrawCommand::Rect { bounds: screen_bounds, color: authored, corner_radius: 0.0, depth: 1.0 };
        let instances = render_one(cmd, &camera);

        let instance = &instances[0];
        let world_pos = Vec2::new(instance.position[0], instance.position[1]);
        let back_on_screen = camera.world_to_screen(world_pos);
        assert!(
            (back_on_screen - screen_center).length() < 0.01,
            "camera {:?} zoom {}: expected screen {screen_center}, got {back_on_screen}",
            camera.position,
            camera.zoom
        );
        // On-screen size = world scale * zoom = the original pixel size.
        assert!((instance.scale[0] * camera.zoom - 100.0).abs() < 0.01);
        assert!((instance.scale[1] * camera.zoom - 40.0).abs() < 0.01);
        assert_eq!(instance.color, [0.25, 0.5, 0.75, 0.5], "the authored color survives unchanged");
    }
}

#[test]
fn test_sdf_shapes_carry_their_kind_and_params_on_one_sprite_each() {
    let camera = test_camera();

    // A rounded rect: kind 1, its radius, filled (no border).
    let rounded = render_one(rect(Rect::new(10.0, 10.0, 100.0, 40.0), 6.0), &camera);
    assert_eq!(rounded.len(), 1);
    assert_eq!(rounded[0].shape, [1.0, 6.0, 0.0, 0.0], "kind = rounded rect, radius 6, filled");

    // Radius 0 keeps the legacy plain-quad path.
    let square = render_one(rect(Rect::new(0.0, 0.0, 50.0, 50.0), 0.0), &camera);
    assert_eq!(square[0].shape, [0.0; 4]);

    // A border is ONE bordered sprite, grown by its width so the stroke
    // straddles the bounds — not four thin rects.
    let border = render_one(
        DrawCommand::RectBorder {
            bounds: Rect::new(10.0, 10.0, 100.0, 40.0),
            color: Color::WHITE,
            width: 2.0,
            corner_radius: 4.0,
            depth: 1.0,
        },
        &camera,
    );
    assert_eq!(border.len(), 1, "border must be ONE SDF sprite");
    assert_eq!((border[0].shape[0], border[0].shape[2]), (1.0, 2.0), "kind rounded rect, border width 2");
    assert_eq!(border[0].scale, [102.0, 42.0]);

    // A circle: kind 2, spanning its diameter, at the screen center = world origin.
    let circle = render_one(
        DrawCommand::Circle { center: Vec2::new(400.0, 300.0), radius: 8.0, color: Color::WHITE, depth: 0.5 },
        &camera,
    );
    assert_eq!(circle.len(), 1);
    assert_eq!(circle[0].shape[0], 2.0, "kind = circle");
    assert_eq!(circle[0].scale, [16.0, 16.0], "sprite spans the diameter");
    assert_eq!(circle[0].position, [0.0, 0.0]);
}

#[test]
fn test_clipped_commands_land_in_a_clip_tagged_batch_and_axis_aligned_lines_survive_the_clip() {
    // Issue #41: commands drawn between Push/PopClipRect carry the clip on
    // their batch so the GPU scissors them; commands outside stay
    // unclipped. Regression folded in: a horizontal line has a zero-height
    // bbox (vertical: zero-width), and a degenerate bbox never overlaps
    // the clip rect — so every axis-aligned line inside a clipped panel
    // was silently culled (origin crosshair, collider boxes, selection
    // outlines). A line fully outside the clip stays culled on the CPU.
    let mut batcher = SpriteBatcher::new();
    let clip = Rect::new(100.0, 100.0, 400.0, 300.0);
    let cmds = vec![
        rect(Rect::new(0.0, 0.0, 50.0, 50.0), 0.0),
        DrawCommand::PushClipRect { bounds: clip },
        rect(Rect::new(120.0, 120.0, 50.0, 50.0), 0.0),
        line(Vec2::new(150.0, 200.0), Vec2::new(350.0, 200.0)), // horizontal, inside the clip
        line(Vec2::new(200.0, 150.0), Vec2::new(200.0, 350.0)), // vertical, inside the clip
        line(Vec2::new(600.0, 500.0), Vec2::new(700.0, 500.0)), // fully outside the clip
        DrawCommand::PopClipRect,
    ];

    render_ui_commands(&mut batcher, &cmds, &test_camera(), &HashMap::new());

    let white = TextureHandle { id: 0 };
    assert_eq!(batcher.batches().len(), 2, "unclipped + clipped batches");
    assert_eq!(batcher.batch_for(white).expect("unclipped batch").len(), 1);
    let clipped = batcher
        .batches()
        .get(&(white, Some([100, 100, 400, 300])))
        .expect("clipped batch keyed by the quantized clip rect");
    assert_eq!(clipped.clip, Some([100, 100, 400, 300]));
    assert_eq!(clipped.len(), 3, "the rect and both axis-aligned lines render; the outside line is culled");
}

#[test]
fn test_nested_clips_intersect_on_the_batch() {
    let mut batcher = SpriteBatcher::new();
    let cmds = vec![
        DrawCommand::PushClipRect { bounds: Rect::new(0.0, 0.0, 200.0, 200.0) },
        DrawCommand::PushClipRect { bounds: Rect::new(100.0, 100.0, 200.0, 200.0) },
        rect(Rect::new(110.0, 110.0, 50.0, 50.0), 0.0),
        DrawCommand::PopClipRect,
        DrawCommand::PopClipRect,
    ];

    render_ui_commands(&mut batcher, &cmds, &test_camera(), &HashMap::new());

    let white = TextureHandle { id: 0 };
    let clipped = batcher
        .batches()
        .get(&(white, Some([100, 100, 100, 100])))
        .expect("nested clip = intersection of both rects");
    assert_eq!(clipped.len(), 1);
}

#[test]
fn test_pop_restores_parent_clip_for_later_commands() {
    let mut batcher = SpriteBatcher::new();
    let outer = Rect::new(0.0, 0.0, 300.0, 300.0);
    let cmds = vec![
        DrawCommand::PushClipRect { bounds: outer },
        DrawCommand::PushClipRect { bounds: Rect::new(50.0, 50.0, 100.0, 100.0) },
        DrawCommand::PopClipRect,
        rect(Rect::new(10.0, 10.0, 20.0, 20.0), 0.0),
        DrawCommand::PopClipRect,
    ];

    render_ui_commands(&mut batcher, &cmds, &test_camera(), &HashMap::new());

    let white = TextureHandle { id: 0 };
    let outer_batch = batcher
        .batches()
        .get(&(white, Some([0, 0, 300, 300])))
        .expect("after the inner pop, the outer clip is active again");
    assert_eq!(outer_batch.len(), 1);
}
