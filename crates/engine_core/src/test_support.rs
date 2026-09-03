//! Shared fixtures for engine_core's tests.
//!
//! Two facts every scene test used to restate: a scene round-trip is
//! `world → world_to_scene_data → RON → parse → instantiate` through a
//! GPU-free [`TextureResolver`], and an input frame is "end the previous
//! frame, queue this frame's events, process them". Both live here once.
//!
//! Compiled for the crate's own unit tests and, behind the `test-support`
//! feature, for its integration tests (`tests/`) and downstream crates'
//! test builds — never for a shipped game.

use ecs::World;
use input::{InputEvent, InputHandler};
use renderer::TextureHandle;

use crate::scene_data::SceneLoadError;
use crate::scene_loader::{SceneInstance, SceneLoader};
use crate::scene_serializer::{serialize_to_ron, world_to_scene_data};
use crate::texture_ref::TextureResolver;

/// The texture-path function a save uses in tests: handle 0 is the built-in
/// white texture, every other handle becomes `#texture_<id>`.
pub fn test_texture_path(handle: u32) -> String {
    if handle == 0 {
        "#white".to_string()
    } else {
        format!("#texture_{handle}")
    }
}

/// GPU-free resolver: every reference resolves to the white texture and no
/// sheet has a sidecar, so a load falls back to the values baked into the
/// scene. Counts cache clears so a test can prove the loader asks for one
/// per load.
#[derive(Debug, Default)]
pub struct StubResolver {
    /// How many times a scene load asked for the sidecar cache to be dropped.
    pub cache_clears: usize,
}

impl TextureResolver for StubResolver {
    fn resolve_texture(&mut self, _texture_ref: &str) -> Result<TextureHandle, SceneLoadError> {
        Ok(TextureHandle::WHITE)
    }

    fn clear_sidecar_cache(&mut self) {
        self.cache_clears += 1;
    }
}

/// Save `world` to RON text the way the editor does.
pub fn save_to_ron(world: &World) -> String {
    let scene = world_to_scene_data(world, "RoundTrip", None, &test_texture_path);
    serialize_to_ron(&scene).expect("scene serializes")
}

/// Save `world` to RON, parse it back and instantiate it into a fresh world
/// through `resolver`.
pub fn roundtrip_with(world: &World, resolver: &mut impl TextureResolver) -> (World, SceneInstance) {
    let ron_text = save_to_ron(world);
    let parsed = SceneLoader::parse(&ron_text).expect("saved scene parses");
    let mut loaded = World::new();
    let instance =
        SceneLoader::instantiate(&parsed, &mut loaded, resolver).expect("saved scene instantiates");
    (loaded, instance)
}

/// [`roundtrip_with`] through a [`StubResolver`].
pub fn roundtrip(world: &World) -> (World, SceneInstance) {
    roundtrip_with(world, &mut StubResolver::default())
}

/// Parse RON scene text and instantiate it into a fresh world through a
/// [`StubResolver`].
pub fn load_ron(ron_text: &str) -> (World, SceneInstance) {
    let parsed = SceneLoader::parse(ron_text).expect("scene text parses");
    let mut world = World::new();
    let instance = SceneLoader::instantiate(&parsed, &mut world, &mut StubResolver::default())
        .expect("scene text instantiates");
    (world, instance)
}

/// One input frame: last frame's just-pressed / just-released edges are
/// cleared, then `events` are queued and processed. Held keys and buttons
/// stay held across frames — a hold is one `KeyPressed` until its
/// `KeyReleased`.
pub fn frame(input: &mut InputHandler, events: &[InputEvent]) {
    input.end_frame();
    for event in events {
        input.queue_event(event.clone());
    }
    input.process_queued_events();
}
