//! Scene chain and render-path coverage for named-clip `SpriteAnimation`.
//!
//! Everything here is headless: the sidecar and PNG-dimension probe live
//! behind the `TextureResolver` seam, so the stubs below stand in for
//! `AssetManager` without a filesystem or a GPU.

use std::collections::HashMap;

use common::SheetGrid;
use ecs::sprite_components::{AnimationClip, Sprite, SpriteAnimation, Transform2D};
use ecs::World;
use engine_core::scene_data::SceneLoadError;
use engine_core::scene_loader::SceneLoader;
use engine_core::scene_serializer::world_to_scene_data;
use engine_core::prelude::{GameContext, RenderContext};
use engine_core::{SheetData, TextureResolver};
use glam::Vec2;

/// Resolver with no sidecars at all — every animation falls back to the
/// values baked into the scene.
#[derive(Default)]
struct BareResolver {
    cache_clears: usize,
}

impl TextureResolver for BareResolver {
    fn resolve_texture(&mut self, _texture_ref: &str) -> Result<renderer::TextureHandle, SceneLoadError> {
        Ok(renderer::TextureHandle { id: 0 })
    }

    fn clear_sidecar_cache(&mut self) {
        self.cache_clears += 1;
    }
}

/// Resolver that serves one canned sidecar, standing in for a `.sheet.ron`
/// the artist has since edited.
struct SidecarResolver {
    path: String,
    data: SheetData,
    reads: usize,
}

impl SidecarResolver {
    fn new(path: &str, data: SheetData) -> Self {
        Self {
            path: path.to_string(),
            data,
            reads: 0,
        }
    }
}

impl TextureResolver for SidecarResolver {
    fn resolve_texture(&mut self, _texture_ref: &str) -> Result<renderer::TextureHandle, SceneLoadError> {
        Ok(renderer::TextureHandle { id: 0 })
    }

    fn sheet_for(&mut self, texture_ref: &str) -> Option<SheetData> {
        (texture_ref == self.path).then(|| {
            self.reads += 1;
            self.data.clone()
        })
    }
}

/// A playing animation over a 4x2 sheet with one looping clip.
fn walking_animation() -> SpriteAnimation {
    let mut animation = SpriteAnimation::new(SheetGrid::new(4, 2))
        .with_clip("walk", AnimationClip::new(vec![0, 1, 2], 12.0))
        .with_clip("idle", AnimationClip::new(vec![4], 4.0).with_looping(false));
    animation.sheet = Some("sprites/deion_16.png".to_string());
    animation.play("walk");
    animation
}

/// Old-format scene data (the pre-named-clip schema) parses into the inert
/// default — every new field is serde-defaulted, so serde ignores the old
/// fields rather than erroring. The loader warns about the do-nothing
/// component (adjudicated in the E3/E4 code review); this locks the
/// no-error, no-animation outcome.
#[test]
fn test_old_format_sprite_animation_loads_as_inert_default() {
    let ron_string = r#"(
        name: "Legacy",
        entities: [(
            name: Some("prop"),
            components: [
                SpriteAnimation(
                    fps: 12.0,
                    frames: [(0.0, 0.0, 0.25, 1.0), (0.25, 0.0, 0.25, 1.0)],
                    playing: true,
                    loop_animation: true,
                ),
            ],
        )],
    )"#;

    let parsed = SceneLoader::parse(ron_string).expect("old-format scene still parses");
    let mut world = World::new();
    let mut resolver = BareResolver::default();
    let instance =
        SceneLoader::instantiate(&parsed, &mut world, &mut resolver).expect("instantiate");

    let animation = world
        .get::<SpriteAnimation>(instance.entities[0])
        .expect("component still attaches");
    assert!(animation.sheet.is_none());
    assert!(animation.clips.is_empty());
    assert!(!animation.playing, "old `playing: true` must not resurrect");
    assert_eq!(animation.current_uv(), None, "inert: never touches the sprite");
}

/// Serialize a one-entity world to RON, parse it back, and instantiate it
/// through `resolver`.
fn roundtrip(world: &World, resolver: &mut impl TextureResolver) -> (World, ecs::EntityId) {
    let scene = world_to_scene_data(world, "AnimRoundTrip", None, &|_| "#white".to_string());
    let ron_string = ron::ser::to_string_pretty(&scene, ron::ser::PrettyConfig::default())
        .expect("serialize scene");
    let parsed = SceneLoader::parse(&ron_string).expect("parse scene");
    let mut loaded = World::new();
    let instance = SceneLoader::instantiate(&parsed, &mut loaded, resolver).expect("instantiate");
    let entity = instance.entities[0];
    (loaded, entity)
}

#[test]
fn test_serializer_writes_the_sheet_grid_clips_and_autoplay() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, walking_animation()).ok();

    let scene = world_to_scene_data(&world, "AnimTest", None, &|_| "#white".to_string());

    match &scene.entities[0].components[0] {
        engine_core::ComponentData::SpriteAnimation {
            sheet,
            grid,
            clips,
            autoplay,
        } => {
            assert_eq!(sheet.as_deref(), Some("sprites/deion_16.png"));
            assert_eq!((grid.cols, grid.rows), (4, 2));
            assert_eq!(clips.len(), 2);
            assert_eq!(clips[0].0, "walk");
            assert_eq!(clips[0].1.frames, vec![0, 1, 2]);
            assert_eq!(clips[0].1.fps, 12.0);
            assert!(clips[0].1.looping);
            assert!(!clips[1].1.looping);
            assert_eq!(autoplay.as_deref(), Some("walk"));
        }
        other => panic!("Expected SpriteAnimation, got {other:?}"),
    }
}

#[test]
fn test_serializer_omits_autoplay_for_a_paused_animation() {
    let mut world = World::new();
    let entity = world.create_entity();
    let mut animation = walking_animation();
    animation.pause();
    world.add_component(&entity, animation).ok();

    let scene = world_to_scene_data(&world, "AnimTest", None, &|_| "#white".to_string());

    match &scene.entities[0].components[0] {
        // A paused animation must not come back playing: the clip set is
        // still written, but nothing tells the loader to start it.
        engine_core::ComponentData::SpriteAnimation { autoplay, clips, .. } => {
            assert_eq!(*autoplay, None);
            assert_eq!(clips.len(), 2);
        }
        other => panic!("Expected SpriteAnimation, got {other:?}"),
    }
}

#[test]
fn test_sprite_animation_round_trips_through_scene_ron() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, walking_animation()).ok();

    let (loaded, e) = roundtrip(&world, &mut BareResolver::default());
    let animation = loaded.get::<SpriteAnimation>(e).expect("animation survives");

    assert_eq!(animation.sheet.as_deref(), Some("sprites/deion_16.png"));
    assert_eq!((animation.grid.cols, animation.grid.rows), (4, 2));
    assert_eq!(
        animation.clips,
        vec![
            ("walk".to_string(), AnimationClip::new(vec![0, 1, 2], 12.0)),
            ("idle".to_string(), AnimationClip::new(vec![4], 4.0).with_looping(false)),
        ]
    );
    // autoplay restored the clip, from the top.
    assert_eq!(animation.current_clip.as_deref(), Some("walk"));
    assert!(animation.playing);
    assert_eq!(animation.current_frame, 0);
}

#[test]
fn test_static_sprite_region_and_visibility_round_trip_through_scene_ron() {
    // A static sheet prop (no animation) keeps its authored cell and
    // visibility across save/load — the E5 fix; before it, a saved prop
    // reloaded showing the whole sheet.
    let mut world = World::new();
    let entity = world.create_entity();
    let sprite = Sprite::new(0)
        .with_tex_region(0.25, 0.5, 0.25, 0.5)
        .with_visible(false);
    world.add_component(&entity, sprite).ok();

    let (loaded, e) = roundtrip(&world, &mut BareResolver::default());
    let sprite = loaded.get::<Sprite>(e).expect("sprite survives");

    assert_eq!(sprite.tex_region, [0.25, 0.5, 0.25, 0.5]);
    assert!(!sprite.visible);
}

#[test]
fn test_autoplaying_clip_overwrites_the_saved_region_snapshot_on_load() {
    // A scene saved mid-animation carries a frame snapshot in the sprite's
    // tex_region. On load the autoplaying clip is the SSOT: the animation
    // system re-asserts its current frame (the clip start), overwriting the
    // snapshot — the editor never shows a stale mid-animation cell.
    let mut world = World::new();
    let entity = world.create_entity();
    let sprite = Sprite::new(0).with_tex_region(0.5, 0.0, 0.25, 0.5); // cell 2 snapshot
    world.add_component(&entity, sprite).ok();
    world.add_component(&entity, walking_animation()).ok();

    let (mut loaded, e) = roundtrip(&world, &mut BareResolver::default());
    ecs::System::update(&mut ecs::SpriteAnimationSystem, &mut loaded, 0.0);

    let sprite = loaded.get::<Sprite>(e).expect("sprite survives");
    // "walk" restarts at frame index 0 → cell 0 of the 4x2 grid.
    assert_eq!(sprite.tex_region, [0.0, 0.0, 0.25, 0.5]);
}

#[test]
fn test_paused_animation_loads_stopped() {
    let mut world = World::new();
    let entity = world.create_entity();
    let mut animation = walking_animation();
    animation.update(0.1);
    animation.pause();
    world.add_component(&entity, animation).ok();

    let (loaded, e) = roundtrip(&world, &mut BareResolver::default());
    let animation = loaded.get::<SpriteAnimation>(e).expect("animation survives");

    assert!(!animation.playing);
    assert_eq!(animation.current_clip, None);
    // Clips still round-trip — only the playback state is dropped.
    assert_eq!(animation.clips.len(), 2);
}

#[test]
fn test_sidecar_grid_and_clips_win_over_baked_scene_values() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, walking_animation()).ok();

    // The artist re-cut the sheet to 8x4 and gave "walk" four frames.
    let mut resolver = SidecarResolver::new(
        "sprites/deion_16.png",
        SheetData {
            grid: SheetGrid::new(8, 4),
            clips: vec![("walk".to_string(), AnimationClip::new(vec![0, 1, 2, 3], 16.0))],
        },
    );

    let (loaded, e) = roundtrip(&world, &mut resolver);
    let animation = loaded.get::<SpriteAnimation>(e).expect("animation survives");

    assert_eq!((animation.grid.cols, animation.grid.rows), (8, 4));
    assert_eq!(animation.clips.len(), 1);
    assert_eq!(animation.clips[0].1.frame_indices, vec![0, 1, 2, 3]);
    assert_eq!(animation.clips[0].1.fps, 16.0);
    // Autoplay still resolves against the sidecar's clip set.
    assert_eq!(animation.current_clip.as_deref(), Some("walk"));
}

#[test]
fn test_missing_sidecar_falls_back_to_the_baked_values() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, walking_animation()).ok();

    // Resolver knows a different sheet — this one has no sidecar.
    let mut resolver = SidecarResolver::new(
        "sprites/other.png",
        SheetData {
            grid: SheetGrid::new(1, 1),
            clips: Vec::new(),
        },
    );

    let (loaded, e) = roundtrip(&world, &mut resolver);
    let animation = loaded.get::<SpriteAnimation>(e).expect("animation survives");

    assert_eq!(resolver.reads, 0);
    assert_eq!((animation.grid.cols, animation.grid.rows), (4, 2));
    assert_eq!(animation.clips.len(), 2);
    assert_eq!(animation.current_clip.as_deref(), Some("walk"));
}

#[test]
fn test_autoplay_naming_a_clip_the_sidecar_dropped_leaves_it_stopped() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, walking_animation()).ok();

    // The sidecar renamed "walk" to "stroll" — the scene's autoplay is stale.
    let mut resolver = SidecarResolver::new(
        "sprites/deion_16.png",
        SheetData {
            grid: SheetGrid::new(4, 2),
            clips: vec![("stroll".to_string(), AnimationClip::new(vec![0, 1], 12.0))],
        },
    );

    let (loaded, e) = roundtrip(&world, &mut resolver);
    let animation = loaded.get::<SpriteAnimation>(e).expect("animation survives");

    // Warned and left stopped rather than guessing a clip.
    assert!(!animation.playing);
    assert_eq!(animation.current_clip, None);
    assert!(animation.has_clip("stroll"));
}

#[test]
fn test_scene_load_clears_the_sidecar_cache_first() {
    let scene = SceneLoader::parse("SceneData(name: \"Empty\", entities: [])").expect("parse");
    let mut resolver = BareResolver::default();
    let mut world = World::new();

    SceneLoader::instantiate(&scene, &mut world, &mut resolver).expect("instantiate");
    SceneLoader::instantiate(&scene, &mut world, &mut resolver).expect("instantiate");

    // Once per load — that is what makes an edited sidecar take effect on
    // reload without a file watcher.
    assert_eq!(resolver.cache_clears, 2);
}

#[test]
fn test_clip_wire_format_is_stable() {
    // Golden form: this is the shape artists and hand-written scenes rely on,
    // and the same shape a `.sheet.ron` clip list uses.
    let ron_text = r#"SceneData(
    name: "Golden",
    entities: [
        EntityData(
            name: Some("hero"),
            components: [
                SpriteAnimation(
                    sheet: Some("sprites/deion_16.png"),
                    grid: (cols: 4, rows: 2),
                    clips: [("walk", (frames: [0, 1, 2, 3], fps: 8.0, looping: true))],
                    autoplay: Some("walk"),
                ),
            ],
        ),
    ],
)"#;

    let scene = SceneLoader::parse(ron_text).expect("golden RON parses");
    let mut world = World::new();
    let instance = SceneLoader::instantiate(&scene, &mut world, &mut BareResolver::default())
        .expect("instantiate");
    let animation = world
        .get::<SpriteAnimation>(instance.entities[0])
        .expect("animation");

    assert_eq!((animation.grid.cols, animation.grid.rows), (4, 2));
    assert_eq!(animation.clips[0].0, "walk");
    assert_eq!(animation.clips[0].1.frame_indices, vec![0, 1, 2, 3]);
    assert_eq!(animation.clips[0].1.fps, 8.0);
    assert!(animation.clips[0].1.looping);
    assert_eq!(animation.current_clip.as_deref(), Some("walk"));

    // And the same fields come back out under the same names.
    let written = world_to_scene_data(&world, "Golden", None, &|_| "#white".to_string());
    let text = ron::ser::to_string(&written).expect("serialize");
    assert!(text.contains("frames:"), "clip frames keep their wire name: {text}");
    assert!(text.contains("looping:"), "clip looping keeps its wire name: {text}");
    assert!(text.contains("cols:"), "grid writes cols/rows: {text}");
    assert!(!text.contains("cell_uv"), "derived cell UV never reaches the wire: {text}");
}

#[test]
fn test_omitted_clip_looping_defaults_to_true_in_scene_ron() {
    let ron_text = r#"SceneData(
    name: "Defaults",
    entities: [
        EntityData(
            components: [
                SpriteAnimation(
                    clips: [("walk", (frames: [0, 1], fps: 8.0))],
                ),
            ],
        ),
    ],
)"#;

    let scene = SceneLoader::parse(ron_text).expect("parses without looping");
    let mut world = World::new();
    let instance = SceneLoader::instantiate(&scene, &mut world, &mut BareResolver::default())
        .expect("instantiate");
    let animation = world
        .get::<SpriteAnimation>(instance.entities[0])
        .expect("animation");

    assert!(animation.clips[0].1.looping);
    // An omitted grid is the 1x1 fallback, and nothing autoplays.
    assert_eq!(animation.grid.cell_count(), 1);
    assert!(!animation.playing);
}

// === Render path ===

/// Minimal game that keeps the default `render`, which is the code under test.
struct RenderProbe;
impl engine_core::Game for RenderProbe {
    fn update(&mut self, _ctx: &mut GameContext) {}
}

/// Run the engine's default render over `world` and return the built sprite
/// instances.
fn render_instances(world: &World) -> Vec<renderer::SpriteInstance> {
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

    sprites
        .batches()
        .values()
        .flat_map(|batch| batch.instances.clone())
        .collect()
}

#[test]
fn test_default_region_sprite_renders_the_full_texture() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(Vec2::ZERO)).ok();
    world.add_component(&entity, Sprite::new(0)).ok();

    let instances = render_instances(&world);

    assert_eq!(instances.len(), 1);
    // Every pre-existing sprite defaults to the full texture, so forwarding
    // the region leaves their output pixel-identical.
    assert_eq!(instances[0].tex_region, [0.0, 0.0, 1.0, 1.0]);
}

#[test]
fn test_animated_sprite_region_reaches_the_renderer() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(Vec2::ZERO)).ok();
    world.add_component(&entity, Sprite::new(0)).ok();
    let mut animation = SpriteAnimation::new(SheetGrid::new(4, 2))
        .with_clip("walk", AnimationClip::new(vec![5], 10.0));
    animation.play("walk");
    world.add_component(&entity, animation).ok();

    // The system is what writes the cell region onto the sprite.
    ecs::System::update(&mut ecs::SpriteAnimationSystem, &mut world, 0.0);
    let instances = render_instances(&world);

    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].tex_region, [0.25, 0.5, 0.25, 0.5]);
}
