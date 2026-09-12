//! The game-only runtime behind the preview window.
//!
//! A child module of `project_host` because the preview drives the host's
//! `pub(crate)` frame step and play-state reset directly: it has no editor
//! chrome to route them through, and the visitor is looking at the game, not
//! at an editor with the panels hidden.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use ecs::World;
use engine_core::prelude::*;

use super::ProjectHost;

/// What the page asks of a running preview. Shared with the page's exports,
/// which set the flags between frames.
#[derive(Default)]
pub struct PreviewControls {
    paused: AtomicBool,
    restart_requested: AtomicBool,
    /// Raised once a frame has actually stepped. `Ok` from the page's load
    /// export only means scheduled, so this is what "running" is read from.
    started: AtomicBool,
    /// Why the scene did not load, when it did not.
    scene_error: Mutex<Option<String>>,
}

impl PreviewControls {
    /// Flip the pause and report the state the preview is now in.
    pub fn toggle_paused(&self) -> bool {
        let paused = !self.paused.load(Ordering::Relaxed);
        self.paused.store(paused, Ordering::Relaxed);
        paused
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    /// Ask for the scene to be reloaded on the next frame.
    pub fn request_restart(&self) {
        self.restart_requested.store(true, Ordering::Relaxed);
    }

    /// Whether at least one frame has stepped.
    pub fn has_started(&self) -> bool {
        self.started.load(Ordering::Relaxed)
    }

    /// The scene-load failure, if the preview has one.
    pub fn scene_error(&self) -> Option<String> {
        self.scene_error.lock().ok().and_then(|error| error.clone())
    }

    fn record_scene_error(&self, error: Option<String>) {
        if let Ok(mut slot) = self.scene_error.lock() {
            *slot = error;
        }
    }
}

/// What the controls did to a frame before it stepped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewTick {
    Restarted,
    Frozen,
    Running,
}

/// A `Game` that plays one scene and nothing else: no chrome, no editing, no
/// persistence.
pub struct PreviewHost {
    host: ProjectHost,
    scene_path: PathBuf,
    controls: Arc<PreviewControls>,
}

impl PreviewHost {
    pub fn new(project_path: PathBuf, scene_path: PathBuf, controls: Arc<PreviewControls>) -> Self {
        Self {
            host: ProjectHost::new(project_path),
            scene_path,
            controls,
        }
    }

    /// Instantiate the preview's scene into `world`, replacing whatever is
    /// there, and publish its physics block as the resource the host's lazy
    /// physics reads.
    pub(crate) fn load_scene(
        &mut self,
        world: &mut World,
        assets: &mut impl engine_core::TextureResolver,
    ) -> Result<(), String> {
        let data = engine_core::scene_loader::SceneLoader::load_from_file(&self.scene_path)
            .map_err(|error| error.to_string())?;
        world.clear();
        let instance = engine_core::scene_loader::SceneLoader::instantiate(&data, world, assets)
            .map_err(|error| error.to_string())?;

        match instance.physics.clone() {
            Some(settings) => world.insert_resource(settings),
            None => {
                world.remove_resource::<engine_core::scene_data::PhysicsSettings>();
            }
        }
        // The bodies of the previous run belong to the previous run: without
        // this the rebuilt scene inherits the old rapier world.
        self.host.reset_play_state();
        Ok(())
    }

    /// Apply the page's requests before the frame steps.
    pub(crate) fn tick_controls(
        &mut self,
        world: &mut World,
        assets: &mut impl engine_core::TextureResolver,
    ) -> PreviewTick {
        if self.controls.restart_requested.swap(false, Ordering::Relaxed) {
            match self.load_scene(world, assets) {
                Ok(()) => self.controls.record_scene_error(None),
                Err(error) => {
                    log::error!("preview restart failed: {error}");
                    self.controls.record_scene_error(Some(error));
                }
            }
            self.controls.paused.store(false, Ordering::Relaxed);
            request_backdrop_reset(world);
            return PreviewTick::Restarted;
        }
        if self.controls.is_paused() {
            return PreviewTick::Frozen;
        }
        PreviewTick::Running
    }
}

impl Game for PreviewHost {
    fn init(&mut self, ctx: &mut GameContext) {
        self.host.init(ctx);
        // The editor's own face, because a preview project need not ship a
        // font and scene-authored labels would otherwise draw as boxes.
        match ctx.ui.load_font(editor::fonts::EDITOR_FONT_REGULAR) {
            Ok(font) => ctx.ui.set_default_font(font),
            Err(error) => log::warn!("preview font not loaded: {error}"),
        }
        if let Err(error) = self.load_scene(ctx.world, ctx.assets) {
            log::error!("preview scene {} failed: {error}", self.scene_path.display());
            // The page asks for readiness rather than watching frames, so a
            // scene that never loads must be visible to that question.
            self.controls.record_scene_error(Some(error));
        }
    }

    fn update(&mut self, ctx: &mut GameContext) {
        if self.tick_controls(ctx.world, ctx.assets) == PreviewTick::Frozen {
            // The step is skipped outright; the multiplier is what holds the
            // engine's own particle and animation stepping still with it.
            ctx.time_scale = 0.0;
            return;
        }
        ctx.time_scale = 1.0;
        self.controls.started.store(true, Ordering::Relaxed);
        self.host.update_frame(
            ctx.world,
            ctx.input,
            ctx.players,
            ctx.scripts,
            ctx.assets.base_path(),
            ctx.delta_time,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecs::Transform2D;
    use engine_core::test_support::StubResolver;
    use std::path::Path;
    use input::{InputHandler, InputSettings};

    const PATROL_SCENE: &str = r#"SceneData(
    name: "preview",
    entities: [
        EntityData(
            name: Some("walker"),
            components: [
                Transform2D(position: (0.0, 0.0)),
                Behavior(Patrol(point_a: (0.0, 0.0), point_b: (100.0, 0.0), speed: 50.0, wait_time: 0.0)),
            ],
        ),
    ],
)"#;

    /// A preview over a temp-dir scene, plus the world and resolver to drive it.
    fn preview_over_patrol_scene(
        directory: &Path,
    ) -> std::io::Result<(PreviewHost, Arc<PreviewControls>)> {
        let scene_path = directory.join("preview.scene.ron");
        std::fs::write(&scene_path, PATROL_SCENE)?;
        let controls = Arc::new(PreviewControls::default());
        Ok((
            PreviewHost::new(directory.to_path_buf(), scene_path, Arc::clone(&controls)),
            controls,
        ))
    }

    fn step(preview: &mut PreviewHost, world: &mut World, frames: u32) {
        let input = InputHandler::new();
        let players = InputSettings::default_two_player();
        let mut scripts = engine_core::scripting::ScriptRunner::new();
        for _ in 0..frames {
            preview
                .host
                .update_frame(world, &input, &players, &mut scripts, "", 0.016);
        }
    }

    fn walker_x(world: &World) -> f32 {
        let entity = world
            .entities()
            .into_iter()
            .find(|entity| world.get::<Transform2D>(*entity).is_some())
            .expect("the scene has an entity with a transform");
        world.get::<Transform2D>(entity).expect("transform").position.x
    }

    #[test]
    fn test_restart_returns_the_world_to_the_loaded_scene_and_clears_the_pause(
    ) -> std::io::Result<()> {
        let directory = tempfile::tempdir()?;
        let (mut preview, controls) = preview_over_patrol_scene(directory.path())?;
        let mut world = World::new();
        let mut assets = StubResolver::default();
        preview.load_scene(&mut world, &mut assets).expect("scene loads");

        step(&mut preview, &mut world, 20);
        assert!(walker_x(&world) > 0.0, "the patrol advanced before the restart");

        controls.toggle_paused();
        controls.request_restart();
        assert_eq!(preview.tick_controls(&mut world, &mut assets), PreviewTick::Restarted);

        assert_eq!(walker_x(&world), 0.0, "restart returns the authored position");
        assert!(!controls.is_paused(), "restart clears the pause");
        assert!(
            world.has_resource::<engine_core::grid::GridBackdropReset>(),
            "the backdrop is asked to rebuild with the scene"
        );
        Ok(())
    }

    #[test]
    fn test_pause_freezes_the_step_until_toggled_back() -> std::io::Result<()> {
        let directory = tempfile::tempdir()?;
        let (mut preview, controls) = preview_over_patrol_scene(directory.path())?;
        let mut world = World::new();
        let mut assets = StubResolver::default();
        preview.load_scene(&mut world, &mut assets).expect("scene loads");

        step(&mut preview, &mut world, 10);
        let frozen_at = walker_x(&world);
        assert!(frozen_at > 0.0);

        assert!(controls.toggle_paused(), "the first toggle pauses");
        assert_eq!(preview.tick_controls(&mut world, &mut assets), PreviewTick::Frozen);
        assert_eq!(walker_x(&world), frozen_at, "a frozen frame never steps the world");

        assert!(!controls.toggle_paused(), "the second toggle resumes");
        assert_eq!(preview.tick_controls(&mut world, &mut assets), PreviewTick::Running);
        step(&mut preview, &mut world, 10);
        assert!(walker_x(&world) > frozen_at, "the patrol advances again after the resume");
        Ok(())
    }
}
