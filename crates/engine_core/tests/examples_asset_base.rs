//! The examples' asset base is `examples/assets`, as a game's is its own `assets/`: every file
//! the hello world scene names, and the Pirate locale's font, resolve against it. Resolved one
//! folder higher, the scene's paths carried an `assets/` prefix that the playground — whose base
//! for a project is its `assets/` — read as `assets/assets/…`, and nothing loaded there.

use std::path::Path;

use ecs::{SoundEmitter, SoundRef, World};
use engine_core::localization::Strings;
use engine_core::prelude::GameConfig;
use engine_core::scene_data::SceneLoadError;
use engine_core::scene_loader::SceneLoader;
use engine_core::TextureResolver;
use renderer::TextureHandle;

const EXAMPLES_ASSET_BASE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/assets");

/// Answers every texture with white and remembers each file a scene asked for.
#[derive(Default)]
struct RecordingResolver {
    texture_files: Vec<String>,
}

impl TextureResolver for RecordingResolver {
    fn resolve_texture(&mut self, texture_ref: &str) -> Result<TextureHandle, SceneLoadError> {
        if !texture_ref.starts_with('#') {
            self.texture_files.push(texture_ref.to_string());
        }
        Ok(TextureHandle::WHITE)
    }
}

#[test]
fn test_the_hello_world_scene_and_the_pirate_font_resolve_against_the_examples_asset_base() {
    let config = GameConfig::new("Hello World")
        .with_asset_base_path(EXAMPLES_ASSET_BASE)
        .with_locales_dir("locales");
    let base = Path::new(config.asset_base());
    let scene_path = base.join("scenes/hello_world.scene.ron");
    let source = std::fs::read_to_string(&scene_path).expect("the hello world scene");
    let parsed = SceneLoader::parse(&source).expect("the hello world scene parses");
    let mut world = World::new();
    let mut resolver = RecordingResolver::default();
    SceneLoader::instantiate(&parsed, &mut world, &mut resolver).expect("the hello world scene instantiates");

    let mut named_files = resolver.texture_files;
    named_files.extend(world.entities().into_iter().filter_map(|entity| {
        match &world.get::<SoundEmitter>(entity)?.sound {
            SoundRef::Path(path) => Some(path.clone()),
            SoundRef::Binding(_) => None,
        }
    }));
    named_files.sort();
    named_files.dedup();
    assert_eq!(named_files.len(), 3, "the scene names a texture and two sounds: {named_files:?}");
    for file in &named_files {
        assert!(
            config.resolve_asset_path(file).is_file(),
            "{file} names no file under {}",
            base.display()
        );
    }

    let mut strings = Strings::load_dir(&base.join(&config.locales_dir));
    strings.set_locale("pirate");
    let font = strings.current_font().expect("the Pirate locale names a font");
    assert!(
        config.resolve_asset_path(font).is_file(),
        "the Pirate font {font} names no file under {}",
        base.display()
    );
}
