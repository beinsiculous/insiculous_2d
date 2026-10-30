use super::*;

fn grill() -> SpriteTiling {
    SpriteTiling { source_size: Vec2::new(96.0, 192.0), borders: Vec4::new(48.0, 80.0, 16.0, 80.0) }
}

fn assert_filled_once(tiles: &[SpriteTile], destination: Vec2) {
    for row in 0..destination.y as usize {
        for column in 0..destination.x as usize {
            let point = Vec2::new(column as f32 + 0.5, row as f32 + 0.5) - destination / 2.0;
            let covers = tiles.iter().filter(|tile| {
                let minimum = tile.offset - tile.size / 2.0;
                let maximum = tile.offset + tile.size / 2.0;
                point.cmpge(minimum).all() && point.cmplt(maximum).all()
            }).count();
            assert_eq!(covers, 1, "pixel {point} must be covered once");
        }
    }
}

#[test]
fn resized_rectangles_repeat_native_pixels_and_keep_the_outer_corners() {
    let source = grill();
    let destination = Vec2::new(128.0, 256.0);
    let tiles: Vec<_> = source.tiles(destination, [0.0, 0.0, 1.0, 1.0]).collect();
    assert_eq!(tiles.len(), 20);
    assert_eq!(tiles[0].size, Vec2::new(48.0, 80.0));
    assert_eq!(tiles[0].offset, Vec2::new(-40.0, 88.0));
    assert_eq!(tiles.last().unwrap().size, Vec2::new(16.0, 80.0));
    assert_eq!(tiles.last().unwrap().offset, Vec2::new(56.0, -88.0));
    for tile in &tiles {
        let sampled_pixels = Vec2::new(tile.tex_region[2], tile.tex_region[3]) * source.source_size;
        assert!((sampled_pixels - tile.size).abs().max_element() < 0.0001);
    }
    assert_filled_once(&tiles, destination);
}

#[test]
fn a_partial_repeat_is_cropped_and_does_not_sample_the_next_animation_cell() {
    let source = grill();
    let destination = Vec2::new(112.0, 208.0);
    let frame = [0.5, 0.25, 0.25, 0.5];
    let tiles: Vec<_> = source.tiles(destination, frame).collect();
    assert!(tiles.iter().any(|tile| tile.size == Vec2::splat(16.0)));
    for tile in &tiles {
        assert!(tile.tex_region[0] >= frame[0] && tile.tex_region[1] >= frame[1]);
        assert!(tile.tex_region[0] + tile.tex_region[2] <= frame[0] + frame[2] + 0.0001);
        assert!(tile.tex_region[1] + tile.tex_region[3] <= frame[1] + frame[3] + 0.0001);
        let sampled = Vec2::new(tile.tex_region[2] / frame[2], tile.tex_region[3] / frame[3]) * source.source_size;
        assert!((sampled - tile.size).abs().max_element() < 0.0001);
    }
    assert_filled_once(&tiles, destination);
}

#[test]
fn undersized_rectangles_crop_border_interiors_without_crossing_the_destination() {
    for destination in [Vec2::new(32.0, 64.0), Vec2::new(1.0, 1.0), Vec2::new(8.0, 192.0)] {
        let tiles: Vec<_> = grill().tiles(destination, [0.0, 0.0, 1.0, 1.0]).collect();
        for tile in &tiles {
            assert!((tile.offset.abs() + tile.size / 2.0).cmple(destination / 2.0).all());
            let sampled = Vec2::new(tile.tex_region[2], tile.tex_region[3]) * grill().source_size;
            assert!((sampled - tile.size).abs().max_element() < 0.0001);
        }
        assert_filled_once(&tiles, destination);
    }
}

#[test]
fn invalid_and_pathological_inputs_produce_no_geometry() {
    for destination in [Vec2::ZERO, Vec2::splat(-1.0), Vec2::splat(f32::NAN), Vec2::splat(f32::INFINITY), Vec2::splat(1.0e10)] {
        assert_eq!(grill().tiles(destination, [0.0, 0.0, 1.0, 1.0]).count(), 0);
    }
    for source in [
        SpriteTiling { source_size: Vec2::ZERO, ..grill() },
        SpriteTiling { borders: Vec4::splat(-1.0), ..grill() },
        SpriteTiling { borders: Vec4::splat(1000.0), ..grill() },
        SpriteTiling { source_size: Vec2::splat(f32::INFINITY), ..grill() },
    ] {
        assert_eq!(source.tiles(Vec2::splat(100.0), [0.0, 0.0, 1.0, 1.0]).count(), 0);
    }
    assert_eq!(grill().tiles(Vec2::splat(100.0), [0.0, f32::NAN, 1.0, 1.0]).count(), 0);
}

#[test]
fn normalization_keeps_one_interior_pixel_available() {
    let mut tiling = SpriteTiling { source_size: Vec2::new(0.0, 16.0), borders: Vec4::new(-1.0, 100.0, 3.0, 100.0) };
    tiling.normalize();
    assert!(tiling.is_valid());
    assert_eq!(tiling.source_size, Vec2::new(1.0, 16.0));
    assert_eq!(tiling.borders, Vec4::new(0.0, 15.0, 0.0, 0.0));
}

#[test]
fn normalization_repairs_overflow_and_preserves_an_exact_source_pixel() {
    for size in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, 33_554_432.0] {
        let mut tiling = SpriteTiling { source_size: Vec2::splat(size), borders: Vec4::splat(f32::MAX) };
        tiling.normalize();
        assert!(tiling.is_valid(), "normalized source {size} must remain loadable: {tiling:?}");
    }
}

#[test]
fn ordinary_sixteen_pixel_tiles_cover_a_four_k_backdrop() {
    let backdrop = Vec2::new(3840.0,2160.0) * 1.2;
    assert_eq!(SpriteTiling::default().tiles(backdrop, [0.0,0.0,1.0,1.0]).count(), 46_656);
}
