//! Scene save/load/new operations for the editor.

use std::path::{Path, PathBuf};

use ecs::World;
use engine_core::Game;

use crate::constants::DEFAULT_SCENE_PATH;

use super::EditorGame;

impl<G: Game> EditorGame<G> {
    /// Save the current scene to the existing scene path (or default if none set).
    pub(super) fn save_scene(
        &mut self,
        world: &World,
        assets: &engine_core::assets::AssetManager,
    ) -> Result<(), String> {
        let path = self.editor.scene_path()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SCENE_PATH));
        self.save_scene_as(world, assets, path)
    }

    /// Save the current scene to a specific path.
    pub(super) fn save_scene_as(
        &mut self,
        world: &World,
        assets: &engine_core::assets::AssetManager,
        path: PathBuf,
    ) -> Result<(), String> {
        let texture_path_fn = |handle: u32| -> String {
            assets.texture_path(handle)
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    if handle == 0 { "#white".to_string() } else { format!("#texture_{}", handle) }
                })
        };
        self.save_scene_with(world, &texture_path_fn, path)
    }

    /// Serialize the world and write the scene file — the MANDATORY save
    /// choke point: every save path (menu, shortcuts, future command API)
    /// must route through here; never call `world_to_scene_data` /
    /// `save_scene_to_file` directly from new code.
    ///
    /// Refuses while a play session is active (Playing or Paused): the world
    /// is mid-simulation, and saving it would overwrite the authored scene
    /// with runtime state.
    pub(super) fn save_scene_with(
        &mut self,
        world: &World,
        texture_path_fn: &dyn Fn(u32) -> String,
        path: PathBuf,
    ) -> Result<(), String> {
        if self.editor.in_play_session() {
            return Err("scene is mid-simulation — stop Play before saving".to_string());
        }

        let scene_name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let scene_data = engine_core::scene_serializer::world_to_scene_data(
            world, &scene_name, self.physics_settings.clone(), texture_path_fn,
        );

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            }
        }

        engine_core::scene_serializer::save_scene_to_file(&scene_data, &path)?;

        self.editor.set_scene_path(Some(path.clone()));
        // The history is the dirty source of truth; the mirror is set too so
        // the same frame's title/status already read clean.
        self.command_history.mark_saved();
        self.editor.set_dirty(false);
        self.editor.status_bar.show_message("Scene saved");
        log::info!("Scene saved to: {:?}", path);
        Ok(())
    }

    /// Load a scene from disk, replacing the current world.
    ///
    /// The current world is only touched once the file is known-good: the
    /// scene is parsed first, then instantiated into a scratch world as a
    /// dry run (issue #50 — a corrupt file, unknown prefab, or missing
    /// texture must not cost the user's unsaved scene). The price is
    /// instantiating twice; `AssetManager` deduplicates texture loads by
    /// (path, filter), so the dry run warms the cache and the real
    /// instantiate uploads nothing twice.
    pub(super) fn load_scene(
        &mut self,
        world: &mut World,
        assets: &mut impl engine_core::TextureResolver,
        path: &Path,
    ) -> Result<(), String> {
        if let Some(msg) = self.scene_replace_refusal() {
            return Err(msg.to_string());
        }

        if self.editor.is_dirty() {
            log::warn!("Current scene has unsaved changes. Save first to avoid losing work.");
        }

        // Parse and dry-run BEFORE touching the world.
        let data = engine_core::scene_loader::SceneLoader::load_from_file(path)
            .map_err(|e| format!("Failed to load scene: {}", e))?;
        let mut scratch = World::new();
        engine_core::scene_loader::SceneLoader::instantiate(&data, &mut scratch, assets)
            .map_err(|e| format!("Failed to load scene: {}", e))?;

        // Known-good: replace the live world.
        for entity in world.entities() {
            world.remove_entity(&entity).ok();
        }
        let scene_instance = engine_core::scene_loader::SceneLoader::instantiate(&data, world, assets)
            .map_err(|e| format!("Failed to load scene: {}", e))?;

        // Store physics settings from loaded scene
        self.physics_settings = scene_instance.physics.clone();

        log::info!("Scene loaded from: {:?} ({} entities)", path, scene_instance.entity_count);

        self.editor.set_scene_path(Some(path.to_path_buf()));
        self.editor.set_dirty(false);
        self.command_history = editor::CommandHistory::new();
        self.editor.selection.clear();
        self.gizmo_drag_start = None;
        self.editor.status_bar.show_message("Scene loaded");

        Ok(())
    }

    /// Guard shared by every scene-replacing operation (New Scene, Open
    /// Scene): `Some(message)` while a play session is active (Playing or
    /// Paused). Replacing the world under a pending play snapshot would make
    /// the next Stop resurrect the old scene's entities into the new one.
    pub(super) fn scene_replace_refusal(&self) -> Option<&'static str> {
        self.editor
            .in_play_session()
            .then_some("scene is mid-simulation — stop Play first")
    }

    /// Load a scene and surface any failure on the status bar.
    pub(super) fn load_scene_with_feedback(
        &mut self,
        world: &mut World,
        assets: &mut impl engine_core::TextureResolver,
        path: &Path,
    ) {
        if let Err(e) = self.load_scene(world, assets, path) {
            self.editor.status_bar.show_error(format!("Load failed: {}", e));
            log::error!("Failed to load scene: {}", e);
        }
    }

    /// Create a new empty scene, clearing the world.
    ///
    /// Refused during a play session (Playing or Paused): clearing the world
    /// under a pending play snapshot would make the next Stop resurrect the
    /// old scene's entities into the new one.
    pub(super) fn new_scene(&mut self, world: &mut World) {
        if let Some(msg) = self.scene_replace_refusal() {
            self.editor.status_bar.show_error(msg);
            log::warn!("{}", msg);
            return;
        }

        if self.editor.is_dirty() {
            log::warn!("Current scene has unsaved changes. Save first to avoid losing work.");
        }

        // Clear existing world
        for entity in world.entities() {
            world.remove_entity(&entity).ok();
        }

        self.editor.set_scene_path(None);
        self.editor.set_dirty(false);
        self.command_history = editor::CommandHistory::new();
        self.editor.selection.clear();
        self.entity_counter = 0;
        self.physics_settings = None;
        self.gizmo_drag_start = None;
        log::info!("New scene created");
    }
}
