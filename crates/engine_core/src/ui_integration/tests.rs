use super::*;
use renderer::texture::TextureHandle;
use ui::Color;

fn test_camera() -> Camera {
    Camera::new(Vec2::ZERO, Vec2::new(800.0, 600.0))
}

/// All white-texture instances regardless of clip batch (clipped UI
/// splits same-texture sprites into per-clip batches since issue #41).
fn white_instances(batcher: &SpriteBatcher) -> Vec<renderer::sprite_data::SpriteInstance> {
    let mut keys: Vec<_> = batcher
        .batches()
        .keys()
        .filter(|(h, _)| h.id == 0)
        .copied()
        .collect();
    keys.sort_by_key(|(_, clip)| *clip);
    keys.iter()
        .flat_map(|k| batcher.batches()[k].instances.iter().copied())
        .collect()
}

#[test]
fn test_ui_stays_at_screen_position_under_moved_zoomed_camera() {
    // THE camera-follow/editor invariant: UI sprites must land at the
    // same SCREEN pixels no matter where the camera is or how far it
    // zooms. (Regression: the editor's panel-derived camera used to
    // shift the entire editor UI off screen.)
    let screen_bounds = Rect::new(10.0, 10.0, 100.0, 40.0);
    let screen_center = Vec2::new(60.0, 30.0);

    for camera in [
        Camera::new(Vec2::ZERO, Vec2::new(800.0, 600.0)),
        Camera::new(Vec2::new(320.0, -150.0), Vec2::new(800.0, 600.0)),
        Camera::new(Vec2::new(-75.5, 12.25), Vec2::new(800.0, 600.0)).with_zoom(2.0),
    ] {
        let mut batcher = SpriteBatcher::new();
        let cmd = DrawCommand::Rect {
            bounds: screen_bounds,
            color: Color::WHITE,
            corner_radius: 0.0,
            depth: 1.0,
        };
        render_ui_commands(&mut batcher, &[cmd], &camera, &HashMap::new());

        let instance = &white_instances(&batcher)[0];
        let world_pos = Vec2::new(instance.position[0], instance.position[1]);
        let back_on_screen = camera.world_to_screen(world_pos);
        assert!(
            (back_on_screen - screen_center).length() < 0.01,
            "camera {:?} zoom {}: expected screen {screen_center}, got {back_on_screen}",
            camera.position,
            camera.zoom
        );
        // On-screen size = world scale * zoom = the original pixel size
        assert!((instance.scale[0] * camera.zoom - 100.0).abs() < 0.01);
        assert!((instance.scale[1] * camera.zoom - 40.0).abs() < 0.01);
    }
}

#[test]
fn test_rounded_rect_emits_shape_params() {
    let mut batcher = SpriteBatcher::new();
    let cmd = DrawCommand::Rect {
        bounds: Rect::new(10.0, 10.0, 100.0, 40.0),
        color: Color::WHITE,
        corner_radius: 6.0,
        depth: 1.0,
    };
    render_ui_commands(&mut batcher, &[cmd], &test_camera(), &HashMap::new());

    let instances = white_instances(&batcher);
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].shape[0], 1.0, "kind = rounded rect");
    assert_eq!(instances[0].shape[1], 6.0, "corner radius carried through");
    assert_eq!(instances[0].shape[2], 0.0, "filled, not bordered");
}

#[test]
fn test_square_rect_stays_plain_quad() {
    let mut batcher = SpriteBatcher::new();
    let cmd = DrawCommand::Rect {
        bounds: Rect::new(0.0, 0.0, 50.0, 50.0),
        color: Color::WHITE,
        corner_radius: 0.0,
        depth: 0.0,
    };
    render_ui_commands(&mut batcher, &[cmd], &test_camera(), &HashMap::new());
    assert_eq!(white_instances(&batcher)[0].shape, [0.0; 4], "radius 0 keeps the legacy quad path");
}

#[test]
fn test_rect_border_is_one_bordered_sprite() {
    let mut batcher = SpriteBatcher::new();
    let cmd = DrawCommand::RectBorder {
        bounds: Rect::new(10.0, 10.0, 100.0, 40.0),
        color: Color::WHITE,
        width: 2.0,
        corner_radius: 4.0,
        depth: 1.0,
    };
    render_ui_commands(&mut batcher, &[cmd], &test_camera(), &HashMap::new());

    let instances = white_instances(&batcher);
    assert_eq!(instances.len(), 1, "border must be ONE SDF sprite, not 4 thin rects");
    assert_eq!(instances[0].shape[0], 1.0);
    assert_eq!(instances[0].shape[2], 2.0, "border width carried through");
    // Grown by width so the stroke straddles the bounds
    assert_eq!(instances[0].scale, [102.0, 42.0]);
}

#[test]
fn test_circle_emits_circle_kind_at_diameter() {
    let mut batcher = SpriteBatcher::new();
    let cmd = DrawCommand::Circle {
        center: Vec2::new(400.0, 300.0),
        radius: 8.0,
        color: Color::WHITE,
        depth: 0.5,
    };
    render_ui_commands(&mut batcher, &[cmd], &test_camera(), &HashMap::new());

    let instances = white_instances(&batcher);
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].shape[0], 2.0, "kind = circle");
    assert_eq!(instances[0].scale, [16.0, 16.0], "sprite spans the diameter");
    // Screen center of an 800x600 window = world origin
    assert_eq!(instances[0].position, [0.0, 0.0]);
}

#[test]
fn test_axis_aligned_lines_survive_clip_rect() {
    // Regression: a horizontal line has a zero-height bbox (vertical:
    // zero-width), and a degenerate bbox never overlaps the clip rect —
    // so every axis-aligned line inside a clipped panel was silently
    // culled (origin crosshair, collider boxes, selection outlines).
    let mut batcher = SpriteBatcher::new();
    let clip = Rect::new(100.0, 100.0, 400.0, 300.0);
    let cmds = [
        DrawCommand::PushClipRect { bounds: clip },
        DrawCommand::Line {
            start: Vec2::new(150.0, 200.0),
            end: Vec2::new(350.0, 200.0), // horizontal, inside the clip
            color: Color::WHITE,
            width: 1.0,
            depth: 0.5,
        },
        DrawCommand::Line {
            start: Vec2::new(200.0, 150.0),
            end: Vec2::new(200.0, 350.0), // vertical, inside the clip
            color: Color::WHITE,
            width: 1.0,
            depth: 0.5,
        },
        DrawCommand::PopClipRect,
    ];
    render_ui_commands(&mut batcher, &cmds, &test_camera(), &HashMap::new());
    assert_eq!(
        white_instances(&batcher).len(),
        2,
        "axis-aligned lines inside the clip must render"
    );
}

#[test]
fn test_line_outside_clip_rect_is_culled() {
    let mut batcher = SpriteBatcher::new();
    let cmds = [
        DrawCommand::PushClipRect { bounds: Rect::new(100.0, 100.0, 400.0, 300.0) },
        DrawCommand::Line {
            start: Vec2::new(600.0, 500.0),
            end: Vec2::new(700.0, 500.0), // fully outside the clip
            color: Color::WHITE,
            width: 1.0,
            depth: 0.5,
        },
        DrawCommand::PopClipRect,
    ];
    render_ui_commands(&mut batcher, &cmds, &test_camera(), &HashMap::new());
    assert_eq!(
        batcher.sprite_count(),
        0,
        "a line fully outside the clip stays culled"
    );
}

#[test]
fn test_clipped_commands_land_in_a_clip_tagged_batch() {
    // Issue #41: commands drawn between Push/PopClipRect carry the clip
    // on their batch so the GPU scissors them; commands outside stay
    // unclipped.
    let mut batcher = SpriteBatcher::new();
    let clip = Rect::new(100.0, 100.0, 400.0, 300.0);
    let cmds = vec![
        DrawCommand::Rect {
            bounds: Rect::new(0.0, 0.0, 50.0, 50.0),
            color: Color::WHITE,
            corner_radius: 0.0,
            depth: 0.5,
        },
        DrawCommand::PushClipRect { bounds: clip },
        DrawCommand::Rect {
            bounds: Rect::new(120.0, 120.0, 50.0, 50.0),
            color: Color::WHITE,
            corner_radius: 0.0,
            depth: 0.5,
        },
        DrawCommand::PopClipRect,
    ];
    render_ui_commands(&mut batcher, &cmds, &test_camera(), &HashMap::new());

    let white = TextureHandle { id: 0 };
    assert_eq!(batcher.batches().len(), 2, "unclipped + clipped batches");
    assert_eq!(batcher.batch_for(white).unwrap().len(), 1);
    let clipped = batcher
        .batches()
        .get(&(white, Some([100, 100, 400, 300])))
        .expect("clipped batch keyed by the quantized clip rect");
    assert_eq!(clipped.len(), 1);
    assert_eq!(clipped.clip, Some([100, 100, 400, 300]));
}

#[test]
fn test_nested_clips_intersect_on_the_batch() {
    let mut batcher = SpriteBatcher::new();
    let cmds = vec![
        DrawCommand::PushClipRect { bounds: Rect::new(0.0, 0.0, 200.0, 200.0) },
        DrawCommand::PushClipRect { bounds: Rect::new(100.0, 100.0, 200.0, 200.0) },
        DrawCommand::Rect {
            bounds: Rect::new(110.0, 110.0, 50.0, 50.0),
            color: Color::WHITE,
            corner_radius: 0.0,
            depth: 0.5,
        },
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
        DrawCommand::Rect {
            bounds: Rect::new(10.0, 10.0, 20.0, 20.0),
            color: Color::WHITE,
            corner_radius: 0.0,
            depth: 0.5,
        },
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
