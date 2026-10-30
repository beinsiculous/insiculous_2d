use std::collections::HashMap;

use ecs::{GlobalTransform2D, Sprite, SpriteTiling, World};
use engine_core::prelude::{Game, GameContext, RenderContext, Transform2D};
use engine_core::test_support::{roundtrip, save_to_ron, StubResolver};
use engine_core::{SceneLoader, RENDER_UNIT};
use glam::{Vec2, Vec4};
use renderer::{SpriteBatcher, TextureHandle};

struct RenderProbe;
impl Game for RenderProbe {
    fn update(&mut self, _context: &mut GameContext) {}
}

fn render(world: &World) -> SpriteBatcher {
    let mut sprites = SpriteBatcher::new();
    let mut camera = common::Camera::new(Vec2::ZERO, Vec2::new(800.0, 600.0));
    let glyph_textures = HashMap::new();
    let mut viewport_scissor = None;
    RenderProbe.render(&mut RenderContext {
        world, sprites: &mut sprites, camera: &mut camera,
        window_size: Vec2::new(800.0, 600.0), ui_commands: &[],
        glyph_textures: &glyph_textures, viewport_scissor: &mut viewport_scissor,
    });
    sprites
}

fn tiling() -> SpriteTiling {
    SpriteTiling { source_size: Vec2::new(96.0, 192.0), borders: Vec4::new(48.0, 80.0, 16.0, 80.0) }
}

#[test]
fn animated_tiling_preserves_render_properties_and_rotates_patches_around_the_sprite_anchor() {
    let mut world = World::new();
    let entity = world.create_entity();
    let position = Vec2::new(300.0, 200.0);
    let rotation = std::f32::consts::FRAC_PI_2;
    let destination = Vec2::new(128.0, 256.0);
    let color = Vec4::new(0.2, 0.4, 0.6, 0.8);
    world.add_component(&entity, Transform2D::new(Vec2::ZERO)).ok();
    world.add_component(&entity, GlobalTransform2D {
        position, rotation, scale: destination / RENDER_UNIT,
    }).ok();
    world.add_component(&entity, Sprite::new(7).with_offset(Vec2::new(10.0, 20.0))
        .with_color(color).with_depth(-0.5).with_emissive(0.4)).ok();
    world.add_component(&entity, tiling()).ok();
    let mut animation = ecs::SpriteAnimation::new(common::SheetGrid::new(4, 2))
        .with_clip("score", ecs::AnimationClip::new(vec![5, 6], 10.0));
    assert!(animation.play("score"));
    world.add_component(&entity, animation).ok();
    ecs::System::update(&mut ecs::SpriteAnimationSystem, &mut world, 0.1);

    let batcher = render(&world);
    let batch = batcher.batch_for(TextureHandle { id: 7 }).expect("tiled sprite batch");
    assert_eq!(batch.instances.len(), 20);
    let first = &batch.instances[0];
    assert!((Vec2::from(first.position) - Vec2::new(192.0, 170.0)).length() < 0.001);
    assert_eq!(first.scale, [48.0, 80.0]);
    for patch in &batch.instances {
        assert_eq!(patch.rotation, rotation);
        assert_eq!(patch.color, color.to_array());
        assert_eq!(patch.depth, -0.5);
        assert_eq!(patch.emissive, 0.4);
        assert!(patch.tex_region[0] >= 0.5 && patch.tex_region[1] >= 0.5);
        assert!(patch.tex_region[0] + patch.tex_region[2] <= 0.75 + 0.0001);
        assert!(patch.tex_region[1] + patch.tex_region[3] <= 1.0 + 0.0001);
    }
    world.get_mut::<Sprite>(entity).expect("sprite").visible = false;
    assert!(render(&world).batches().is_empty());
}

#[test]
fn adding_tiling_changes_only_the_selected_sprites_draw_geometry() {
    let mut world = World::new();
    let tiled = world.spawn().with(Transform2D::from_parts(Vec2::ZERO, 0.0, Vec2::new(1.6, 3.2)))
        .with(Sprite::new(1)).with(tiling()).id();
    world.spawn().with(Transform2D::from_parts(Vec2::new(80.0, 0.0), 0.0, Vec2::new(1.6, 3.2)))
        .with(Sprite::new(2));
    let batcher = render(&world);
    assert_eq!(batcher.batch_for(TextureHandle { id: 1 }).expect("tiled").instances.len(), 20);
    let plain = batcher.batch_for(TextureHandle { id: 2 }).expect("plain");
    assert_eq!(plain.instances.len(), 1);
    assert_eq!(plain.instances[0].scale, [128.0, 256.0]);
    world.remove_component::<SpriteTiling>(&tiled).ok();
    assert_eq!(render(&world).batch_for(TextureHandle { id: 1 }).expect("restored plain").instances.len(), 1);
}

#[test]
fn scene_roundtrip_keeps_source_borders_destination_size_and_rendered_patches() {
    let mut world = World::new();
    world.spawn().with(ecs::Name::new("Modular Grill"))
        .with(Transform2D::from_parts(Vec2::new(15.0, 25.0), 0.0, Vec2::new(1.6, 3.2)))
        .with(Sprite::new(0)).with(tiling());
    let serialized = save_to_ron(&world);
    assert!(serialized.contains("SpriteTiling("));
    assert!(!serialized.contains("component_type: \"SpriteTiling\""));
    let (loaded, instance) = roundtrip(&world);
    let entity = instance.named_entities["Modular Grill"];
    assert_eq!(loaded.get::<SpriteTiling>(entity), Some(&tiling()));
    assert_eq!(loaded.get::<Transform2D>(entity).expect("destination").scale, Vec2::new(1.6, 3.2));
    assert_eq!(render(&loaded).batch_for(TextureHandle { id: 0 }).expect("roundtripped").instances.len(), 20);
}

#[test]
fn scenes_reject_tiling_without_a_repeatable_source_pixel() {
    let scene = SceneLoader::parse(r#"SceneData(name: "invalid", entities: [EntityData(components: [SpriteTiling(source_size: (16.0,16.0), borders: (8.0,0.0,8.0,0.0))])])"#).expect("valid RON");
    let result = SceneLoader::instantiate(&scene, &mut World::new(), &mut StubResolver::default());
    assert!(result.is_err(), "a zero-width repeating interior cannot load");
}

#[test]
fn prefab_overrides_change_one_tiling_field_and_preserve_the_other() {
    for (overrides, expected) in [
        ("SpriteTiling(borders: (16.0,80.0,48.0,80.0))", SpriteTiling {
            borders: Vec4::new(16.0,80.0,48.0,80.0), ..tiling()
        }),
        ("SpriteTiling(source_size: (128.0,256.0))", SpriteTiling {
            source_size: Vec2::new(128.0,256.0), ..tiling()
        }),
    ] {
        let text = format!(r#"SceneData(name: "Tiled prefab",
            prefabs: {{ "Grill": PrefabData(components: [
                SpriteTiling(source_size: (96.0,192.0), borders: (48.0,80.0,16.0,80.0))
            ]) }},
            entities: [EntityData(name: Some("Grill"), prefab: Some("Grill"), overrides: [{overrides}])])"#);
        let scene = SceneLoader::parse(&text).expect("partial override parses");
        let mut world = World::new();
        let instance = SceneLoader::instantiate(&scene, &mut world, &mut StubResolver::default())
            .expect("the merged tiling is valid");
        assert_eq!(world.get::<SpriteTiling>(instance.named_entities["Grill"]), Some(&expected));
    }
}

#[test]
fn transform_sprite_hierarchy_and_uv_flips_mirror_the_complete_tiled_assembly() {
    for mode in 0..6 {
        let mut world = World::new();
        let entity = world.spawn()
            .with(Transform2D::from_parts(Vec2::ZERO, 0.0, Vec2::new(1.6,3.2)))
            .with(Sprite::new(7)).with(tiling()).id();
        let reference = render(&world).batch_for(TextureHandle { id: 7 }).expect("reference").instances.clone();
        let mut expected_mirror = Vec2::new(-1.0,1.0);
        match mode {
            0 => world.get_mut::<Transform2D>(entity).expect("transform").scale.x *= -1.0,
            1 => world.get_mut::<Sprite>(entity).expect("sprite").scale.x = -1.0,
            2 => { world.add_component(&entity, GlobalTransform2D {
                position: Vec2::ZERO, rotation: 0.0, scale: Vec2::new(-1.6,3.2),
            }).expect("hierarchy"); },
            3 => world.get_mut::<Sprite>(entity).expect("sprite").tex_region = [1.0,0.0,-1.0,1.0],
            4 => {
                world.get_mut::<Sprite>(entity).expect("sprite").scale = Vec2::splat(-1.0);
                expected_mirror = Vec2::splat(-1.0);
            },
            _ => {
                let sprite = world.get_mut::<Sprite>(entity).expect("sprite");
                sprite.scale.x = -1.0;
                sprite.tex_region = [1.0,0.0,-1.0,1.0];
                expected_mirror = Vec2::ONE;
            },
        }
        let mirrored = render(&world);
        let actual = &mirrored.batch_for(TextureHandle { id: 7 }).expect("flipped sprite remains visible").instances;
        assert_eq!(actual.len(), reference.len());
        for (patch, original) in actual.iter().zip(&reference) {
            assert_eq!(Vec2::from(patch.position), Vec2::from(original.position) * expected_mirror);
            assert_eq!(Vec2::from(patch.scale), Vec2::from(original.scale) * expected_mirror);
            assert_eq!(patch.tex_region, original.tex_region);
        }
    }
}

#[test]
fn pixel_snapping_moves_every_patch_together_after_rotation_and_depth_sorting() {
    let camera = common::Camera::new(Vec2::new(0.15,0.35), Vec2::new(800.0,600.0));
    for rotation in [0.0, std::f32::consts::FRAC_PI_4] {
        for destination in [Vec2::new(128.0,256.0), Vec2::new(109.0,201.0)] {
            let mut world = World::new();
            world.spawn()
                .with(Transform2D::from_parts(Vec2::new(10.25,20.75), rotation, destination / RENDER_UNIT))
                .with(Sprite::new(7)).with(tiling());
            let mut batcher = render(&world);
            batcher.sort_all_batches();
            let before = batcher.batch_for(TextureHandle { id: 7 }).expect("patches").instances.clone();
            batcher.snap_origins(&camera);
            let after = &batcher.batch_for(TextureHandle { id: 7 }).expect("snapped patches").instances;
            let translation = Vec2::from(after[0].position) - Vec2::from(before[0].position);
            for (patch, original) in after.iter().zip(&before) {
                let correction = Vec2::from(patch.position) - Vec2::from(original.position);
                assert!((correction - translation).length() < 0.0001,
                    "adjacent patches must share one correction, including partial tiles");
            }
        }
    }
}

#[test]
fn saving_normalizes_invalid_runtime_tiling_without_mutating_the_world() {
    let mut world = World::new();
    let invalid = SpriteTiling { source_size: Vec2::splat(f32::INFINITY), borders: Vec4::splat(100.0) };
    let entity = world.spawn().with(ecs::Name::new("Runtime Grill")).with(invalid).id();
    let (loaded, instance) = roundtrip(&world);
    let restored = loaded.get::<SpriteTiling>(instance.named_entities["Runtime Grill"]).expect("saved tiling");
    assert!(restored.is_valid());
    assert_eq!(world.get::<SpriteTiling>(entity), Some(&invalid));
}

#[test]
fn multiple_tiled_sprites_share_a_bounded_frame_budget() {
    let tiling = SpriteTiling { source_size: Vec2::ONE, borders: Vec4::ZERO };
    let sprite = renderer::Sprite::new(TextureHandle::WHITE).with_scale(Vec2::splat(256.0));
    let mut batcher = SpriteBatcher::new();
    for _ in 0..3 {
        engine_core::prelude::append_tiled_sprite(&sprite, &tiling, &mut batcher);
    }
    assert_eq!(batcher.sprite_count(), SpriteTiling::MAX_PATCHES);
}
