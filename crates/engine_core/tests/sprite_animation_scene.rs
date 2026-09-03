//! Scene chain and render-path coverage for named-clip `SpriteAnimation`.
//!
//! Everything here is headless: the sidecar and PNG-dimension probe live
//! behind the `TextureResolver` seam, so `test_support::StubResolver` (no
//! sidecars) and the `SidecarResolver` below stand in for `AssetManager`
//! without a filesystem or a GPU.

use std::collections::HashMap;

use common::SheetGrid;
use ecs::sprite_components::{AnimationClip, Name, Sprite, SpriteAnimation, Transform2D};
use ecs::World;
use engine_core::prelude::{GameContext, RenderContext};
use engine_core::scene_data::{ComponentData, SceneLoadError};
use engine_core::scene_loader::SceneLoader;
use engine_core::scene_serializer::world_to_scene_data;
use engine_core::test_support::{load_ron, roundtrip, roundtrip_with, test_texture_path, StubResolver};
use engine_core::{SheetData, TextureResolver};
use glam::Vec2;

/// Resolver that serves one canned sidecar, standing in for a `.sheet.ron`
/// the artist has since edited.
struct SidecarResolver {
    path: String,
    data: SheetData,
    reads: usize,
}

impl SidecarResolver {
    fn new(path: &str, data: SheetData) -> Self {
        Self { path: path.to_string(), data, reads: 0 }
    }
}

impl TextureResolver for SidecarResolver {
    fn resolve_texture(&mut self, _texture_ref: &str) -> Result<renderer::TextureHandle, SceneLoadError> {
        Ok(renderer::TextureHandle::WHITE)
    }

    fn sheet_for(&mut self, texture_ref: &str) -> Option<SheetData> {
        (texture_ref == self.path).then(|| {
            self.reads += 1;
            self.data.clone()
        })
    }
}

const SHEET: &str = "sprites/deion_16.png";

/// A playing animation over a 4x2 sheet with one looping clip.
fn walking_animation() -> SpriteAnimation {
    let mut animation = SpriteAnimation::new(SheetGrid::new(4, 2))
        .with_clip("walk", AnimationClip::new(vec![0, 1, 2], 12.0))
        .with_clip("idle", AnimationClip::new(vec![4], 4.0).with_looping(false));
    animation.sheet = Some(SHEET.to_string());
    assert!(animation.play("walk"));
    animation
}

/// A world holding one entity named "hero" with the walking animation.
fn hero_world() -> World {
    let mut world = World::new();
    let hero = world.create_entity();
    world.add_component(&hero, Name::new("hero")).ok();
    world.add_component(&hero, walking_animation()).ok();
    world
}

#[test]
fn old_format_sprite_animation_loads_as_inert_default() {
    // The pre-named-clip schema: every new field is serde-defaulted, so the
    // old fields are ignored rather than erroring. The loader warns about
    // the do-nothing component; this locks the no-error, no-animation
    // outcome — `playing: true` must not resurrect.
    let (world, instance) = load_ron(
        r#"(
            name: "Legacy",
            entities: [(
                name: Some("prop"),
                components: [SpriteAnimation(
                    fps: 12.0,
                    frames: [(0.0, 0.0, 0.25, 1.0), (0.25, 0.0, 0.25, 1.0)],
                    playing: true,
                    loop_animation: true,
                )],
            )],
        )"#,
    );

    let animation = world.get::<SpriteAnimation>(instance.entities[0]).expect("component still attaches");
    assert_eq!(animation.sheet, None);
    assert_eq!(animation.clips.len(), 0);
    assert!(!animation.playing);
    assert_eq!(animation.current_uv(), None, "inert: never touches the sprite");
}

#[test]
fn sprite_animation_round_trips_through_scene_ron() {
    // Three entities cover the whole contract: a playing hero whose saved
    // tex_region is a mid-animation snapshot, a static prop that keeps its
    // authored cell and visibility (E5 — before it, a saved prop reloaded
    // showing the whole sheet), and a paused sleeper that must not come
    // back playing.
    let mut world = hero_world();
    let hero = world.entities()[0];
    world.add_component(&hero, Sprite::new(0).with_tex_region(0.5, 0.0, 0.25, 0.5)).ok();
    let prop = world.create_entity();
    world.add_component(&prop, Name::new("prop")).ok();
    world.add_component(&prop, Sprite::new(0).with_tex_region(0.25, 0.5, 0.25, 0.5).with_visible(false)).ok();
    let sleeper = world.create_entity();
    world.add_component(&sleeper, Name::new("sleeper")).ok();
    let mut paused = walking_animation();
    paused.update(0.1);
    paused.pause();
    world.add_component(&sleeper, paused).ok();

    // The wire carries sheet, grid, clips and — only for a playing
    // animation — autoplay.
    let scene = world_to_scene_data(&world, "Anim", None, &test_texture_path);
    let autoplay_of = |name: &str| {
        let entity = scene.entities.iter().find(|e| e.name.as_deref() == Some(name)).expect("named");
        entity
            .components
            .iter()
            .find_map(|c| match c {
                ComponentData::SpriteAnimation { sheet, grid, clips, autoplay } => {
                    assert_eq!(sheet.as_deref(), Some(SHEET));
                    assert_eq!((grid.cols, grid.rows), (4, 2));
                    assert_eq!(clips.len(), 2);
                    assert_eq!((clips[0].0.as_str(), clips[0].1.frames.clone(), clips[0].1.fps, clips[0].1.looping), ("walk", vec![0, 1, 2], 12.0, true));
                    assert!(!clips[1].1.looping);
                    Some(autoplay.clone())
                }
                _ => None,
            })
            .expect("SpriteAnimation row")
    };
    assert_eq!(autoplay_of("hero").as_deref(), Some("walk"));
    assert_eq!(autoplay_of("sleeper"), None, "a paused animation writes no autoplay");

    let (mut loaded, instance) = roundtrip(&world);
    let hero = instance.named_entities["hero"];
    let prop = instance.named_entities["prop"];
    let sleeper = instance.named_entities["sleeper"];

    let animation = loaded.get::<SpriteAnimation>(hero).expect("animation survives");
    assert_eq!(animation.sheet.as_deref(), Some(SHEET));
    assert_eq!((animation.grid.cols, animation.grid.rows), (4, 2));
    assert_eq!(
        animation.clips,
        vec![
            ("walk".to_string(), AnimationClip::new(vec![0, 1, 2], 12.0)),
            ("idle".to_string(), AnimationClip::new(vec![4], 4.0).with_looping(false)),
        ]
    );
    assert_eq!(animation.current_clip.as_deref(), Some("walk"), "autoplay restored the clip");
    assert!(animation.playing);
    assert_eq!(animation.current_frame, 0, "from the top");
    let prop_sprite = loaded.get::<Sprite>(prop).expect("prop sprite survives");
    assert_eq!(prop_sprite.tex_region, [0.25, 0.5, 0.25, 0.5]);
    assert!(!prop_sprite.visible);
    let sleeping = loaded.get::<SpriteAnimation>(sleeper).expect("paused animation survives");
    assert!(!sleeping.playing);
    assert_eq!(sleeping.current_clip, None);
    assert_eq!(sleeping.clips.len(), 2, "only the playback state is dropped");

    // On load the autoplaying clip is the SSOT: one system step overwrites
    // the saved snapshot (cell 2) with the clip start (cell 0), so the
    // editor never shows a stale mid-animation cell. The prop is untouched.
    ecs::System::update(&mut ecs::SpriteAnimationSystem, &mut loaded, 0.0);
    assert_eq!(loaded.get::<Sprite>(hero).expect("hero sprite").tex_region, [0.0, 0.0, 0.25, 0.5]);
    assert_eq!(loaded.get::<Sprite>(prop).expect("prop sprite").tex_region, [0.25, 0.5, 0.25, 0.5]);
}

#[test]
fn sidecar_grid_and_clips_win_over_baked_scene_values() {
    // The artist re-cut the sheet to 8x4 and gave "walk" four frames.
    let world = hero_world();
    let mut resolver = SidecarResolver::new(
        SHEET,
        SheetData {
            grid: SheetGrid::new(8, 4),
            clips: vec![("walk".to_string(), AnimationClip::new(vec![0, 1, 2, 3], 16.0))],
        },
    );

    let (loaded, instance) = roundtrip_with(&world, &mut resolver);

    let animation = loaded.get::<SpriteAnimation>(instance.entities[0]).expect("animation survives");
    assert_eq!((animation.grid.cols, animation.grid.rows), (8, 4));
    assert_eq!(animation.clips, vec![("walk".to_string(), AnimationClip::new(vec![0, 1, 2, 3], 16.0))]);
    assert_eq!(animation.current_clip.as_deref(), Some("walk"), "autoplay resolves against the sidecar's clips");
}

#[test]
fn missing_sidecar_falls_back_to_the_baked_values() {
    // The resolver knows a different sheet — this one has no sidecar.
    let world = hero_world();
    let mut resolver = SidecarResolver::new("sprites/other.png", SheetData { grid: SheetGrid::new(1, 1), clips: Vec::new() });

    let (loaded, instance) = roundtrip_with(&world, &mut resolver);

    let animation = loaded.get::<SpriteAnimation>(instance.entities[0]).expect("animation survives");
    assert_eq!(resolver.reads, 0);
    assert_eq!((animation.grid.cols, animation.grid.rows), (4, 2));
    assert_eq!(animation.clips.len(), 2);
    assert_eq!(animation.current_clip.as_deref(), Some("walk"));
}

#[test]
fn autoplay_naming_a_clip_the_sidecar_dropped_leaves_it_stopped() {
    // The sidecar renamed "walk" to "stroll" — the scene's autoplay is stale:
    // warned and left stopped rather than guessing a clip.
    let world = hero_world();
    let mut resolver = SidecarResolver::new(
        SHEET,
        SheetData { grid: SheetGrid::new(4, 2), clips: vec![("stroll".to_string(), AnimationClip::new(vec![0, 1], 12.0))] },
    );

    let (loaded, instance) = roundtrip_with(&world, &mut resolver);

    let animation = loaded.get::<SpriteAnimation>(instance.entities[0]).expect("animation survives");
    assert!(!animation.playing);
    assert_eq!(animation.current_clip, None);
    assert!(animation.has_clip("stroll"));
}

#[test]
fn scene_load_clears_the_sidecar_cache_once_per_load() {
    // That is what makes an edited sidecar take effect on reload without a
    // file watcher.
    let scene = SceneLoader::parse(r#"SceneData(name: "Empty", entities: [])"#).expect("parse");
    let mut resolver = StubResolver::default();
    let mut world = World::new();

    SceneLoader::instantiate(&scene, &mut world, &mut resolver).expect("instantiate");
    SceneLoader::instantiate(&scene, &mut world, &mut resolver).expect("instantiate");

    assert_eq!(resolver.cache_clears, 2);
}

#[test]
fn clip_wire_format_is_stable() {
    // Golden form: the shape artists and hand-written scenes rely on, and
    // the same shape a `.sheet.ron` clip list uses.
    let (world, instance) = load_ron(
        r#"SceneData(
            name: "Golden",
            entities: [EntityData(
                name: Some("hero"),
                components: [SpriteAnimation(
                    sheet: Some("sprites/deion_16.png"),
                    grid: (cols: 4, rows: 2),
                    clips: [("walk", (frames: [0, 1, 2, 3], fps: 8.0, looping: true))],
                    autoplay: Some("walk"),
                )],
            )],
        )"#,
    );

    let animation = world.get::<SpriteAnimation>(instance.entities[0]).expect("animation");
    assert_eq!((animation.grid.cols, animation.grid.rows), (4, 2));
    assert_eq!(animation.clips, vec![("walk".to_string(), AnimationClip::new(vec![0, 1, 2, 3], 8.0))]);
    assert_eq!(animation.current_clip.as_deref(), Some("walk"));

    // And the same fields come back out under the same names — no derived UVs.
    let written = world_to_scene_data(&world, "Golden", None, &test_texture_path);
    let text = ron::ser::to_string(&written).expect("serialize");
    assert!(text.contains("frames:"), "clip frames keep their wire name: {text}");
    assert!(text.contains("looping:"), "clip looping keeps its wire name: {text}");
    assert!(text.contains("cols:"), "grid writes cols/rows: {text}");
    assert!(!text.contains("cell_uv"), "derived cell UV never reaches the wire: {text}");
}

#[test]
fn omitted_clip_looping_defaults_to_true_in_scene_ron() {
    // The scene-side twin of the sidecar's `looping` default: one DTO, two
    // wire surfaces, both deliberately pinned.
    let (world, instance) = load_ron(
        r#"SceneData(name: "Defaults", entities: [EntityData(components: [SpriteAnimation(clips: [("walk", (frames: [0, 1], fps: 8.0))])])])"#,
    );

    let animation = world.get::<SpriteAnimation>(instance.entities[0]).expect("animation");
    assert!(animation.clips[0].1.looping);
    assert_eq!(animation.grid.cell_count(), 1, "an omitted grid is the 1x1 fallback");
    assert!(!animation.playing, "nothing autoplays unless asked");
}

// === Render path ===

/// Minimal game that keeps the default `render`, which is the code under test.
struct RenderProbe;
impl engine_core::Game for RenderProbe {
    fn update(&mut self, _ctx: &mut GameContext) {}
}

/// Run the engine's default render over `world` and return the built sprite
/// instances' texture regions, sorted for order-free comparison.
fn rendered_regions(world: &World) -> Vec<[f32; 4]> {
    use engine_core::Game;

    let mut sprites = renderer::SpriteBatcher::new();
    let mut camera = common::Camera::new(Vec2::ZERO, Vec2::new(800.0, 600.0));
    let glyph_textures = HashMap::new();
    let mut viewport_scissor = None;
    let mut ctx = RenderContext {
        world,
        sprites: &mut sprites,
        camera: &mut camera,
        window_size: Vec2::new(800.0, 600.0),
        ui_commands: &[],
        glyph_textures: &glyph_textures,
        viewport_scissor: &mut viewport_scissor,
    };
    RenderProbe.render(&mut ctx);

    let mut regions: Vec<[f32; 4]> = sprites
        .batches()
        .values()
        .flat_map(|batch| batch.instances.iter().map(|instance| instance.tex_region))
        .collect();
    regions.sort_by(|a, b| a.partial_cmp(b).expect("finite regions"));
    regions
}

#[test]
fn animated_sprite_region_reaches_the_renderer_and_plain_sprites_stay_full() {
    let mut world = World::new();
    let animated = world.create_entity();
    world.add_component(&animated, Transform2D::new(Vec2::ZERO)).ok();
    world.add_component(&animated, Sprite::new(0)).ok();
    let mut animation = SpriteAnimation::new(SheetGrid::new(4, 2)).with_clip("walk", AnimationClip::new(vec![5], 10.0));
    assert!(animation.play("walk"));
    world.add_component(&animated, animation).ok();
    let plain = world.create_entity();
    world.add_component(&plain, Transform2D::new(Vec2::new(50.0, 0.0))).ok();
    world.add_component(&plain, Sprite::new(0)).ok();

    // The system is what writes the cell region onto the sprite; the render
    // path forwards it, so a pre-existing plain sprite stays pixel-identical.
    ecs::System::update(&mut ecs::SpriteAnimationSystem, &mut world, 0.0);
    let regions = rendered_regions(&world);

    assert_eq!(regions, vec![[0.0, 0.0, 1.0, 1.0], [0.25, 0.5, 0.25, 0.5]]);
}
