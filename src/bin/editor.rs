//! Standalone editor binary for the insiculous_2d engine.
//!
//! Opens the editor UI pointed at a game project directory.
//!
//! Usage:
//!   cargo run --bin editor --features editor -- /path/to/project
//!   cargo run --bin editor --features editor              # defaults to "."

use std::path::PathBuf;

use engine_core::prelude::*;
use editor_integration::run_game_with_editor_api;

/// Standalone editor application — a minimal `Game` that provides physics
/// preview during play mode. All real editing is handled by `EditorGame`
/// wrapping this.
struct EditorApp {
    project_path: PathBuf,
    physics: Option<PhysicsSystem>,
    transform_hierarchy: TransformHierarchySystem,
}

impl EditorApp {
    fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            physics: None,
            transform_hierarchy: TransformHierarchySystem::new(),
        }
    }
}

impl Game for EditorApp {
    fn init(&mut self, ctx: &mut GameContext) {
        let assets_path = self.project_path.join("assets");
        ctx.assets.set_base_path(assets_path.to_string_lossy());

        // Auto-load first scene found in assets/scenes/
        let scenes_dir = assets_path.join("scenes");
        if scenes_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(scenes_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("ron") {
                        log::info!("Loading scene: {}", path.display());
                        match SceneLoader::load_and_instantiate(&path, ctx.world, ctx.assets) {
                            Ok(instance) => {
                                log::info!(
                                    "Loaded scene '{}' with {} entities",
                                    instance.name,
                                    instance.entity_count
                                );
                                // Set up physics from scene settings
                                let physics_config =
                                    if let Some(settings) = &instance.physics {
                                        PhysicsConfig::new(Vec2::new(
                                            settings.gravity.0,
                                            settings.gravity.1,
                                        ))
                                        .with_scale(settings.pixels_per_meter)
                                    } else {
                                        PhysicsConfig::platformer()
                                    };
                                self.physics =
                                    Some(PhysicsSystem::with_config(physics_config));
                            }
                            Err(e) => {
                                log::warn!("Failed to load scene {}: {}", path.display(), e);
                            }
                        }
                        break; // load only the first scene
                    }
                }
            }
        }

        // Initialize systems
        if let Some(physics) = &mut self.physics {
            physics.initialize(ctx.world).ok();
        }
        self.transform_hierarchy.initialize(ctx.world).ok();

        // Add Name + GlobalTransform2D for entities that lack them
        use ecs::{GlobalTransform2D, Name};
        for entity_id in ctx.world.entities() {
            if ctx.world.get::<Name>(entity_id).is_none() {
                ctx.world
                    .add_component(&entity_id, Name::new(format!("{}", entity_id)))
                    .ok();
            }
            if ctx.world.get::<Transform2D>(entity_id).is_some()
                && ctx.world.get::<GlobalTransform2D>(entity_id).is_none()
            {
                ctx.world
                    .add_component(&entity_id, GlobalTransform2D::default())
                    .ok();
            }
        }

        log::info!(
            "Editor opened project: {}  ({} entities)",
            self.project_path.display(),
            ctx.world.entity_count()
        );
    }

    fn update(&mut self, ctx: &mut GameContext) {
        // Physics preview during play mode
        if let Some(physics) = &mut self.physics {
            physics.update(ctx.world, ctx.delta_time);
        }
        self.transform_hierarchy.update(ctx.world, ctx.delta_time);
    }

    fn on_play_stopped(&mut self, _ctx: &mut GameContext) {
        // Clear rapier world so it re-syncs from restored ECS snapshot
        if let Some(physics) = &mut self.physics {
            physics.clear();
        }
    }
}

/// Spawn the stdin reader feeding the command API (audit §9 Stage A).
/// The thread only moves bytes; dispatch happens on the frame thread.
/// Ends on stdin EOF/error or when the editor side drops the receiver.
fn spawn_api_stdin_reader() -> std::sync::mpsc::Receiver<String> {
    use std::io::BufRead as _;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break;
            }
        }
        log::info!("command API: stdin closed, reader stopped");
    });
    rx
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Args: [project-path] [--api]. `--api` answers line-oriented queries
    // from stdin with JSON on stdout (docs/EDITOR_COMMAND_API.md).
    // Only the known flag is treated as one — any other argument is a
    // positional path, even one starting with `--`.
    let (flags, paths): (Vec<String>, Vec<String>) =
        std::env::args().skip(1).partition(|a| a == "--api");
    let api_rx = (!flags.is_empty()).then(spawn_api_stdin_reader);

    let project_path: PathBuf = paths
        .into_iter()
        .next()
        .unwrap_or_else(|| ".".into())
        .into();

    let project_name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Insiculous 2D Editor");

    let title = format!("Insiculous 2D Editor — {}", project_name);

    let config = GameConfig::new(&title)
        .with_size(1280, 720)
        .with_clear_color(0.1, 0.1, 0.15, 1.0);

    if let Err(e) = run_game_with_editor_api(EditorApp::new(project_path), config, api_rx) {
        log::error!("Editor error: {}", e);
        std::process::exit(1);
    }
}
