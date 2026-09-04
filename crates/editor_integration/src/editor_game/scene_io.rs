//! Scene save/load/new operations for the editor.

use std::path::{Path, PathBuf};

use ecs::World;
use engine_core::scene_data::SceneLoadError;
use engine_core::Game;

use crate::constants::DEFAULT_SCENE_PATH;

use super::EditorGame;

/// Errors that can occur during scene save or load operations.
#[derive(Debug)]
pub enum SceneIoError {
    MidSimulation,
    CreateDirectory(std::io::Error),
    Write(String),
    Load(SceneLoadError),
}

impl std::fmt::Display for SceneIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneIoError::MidSimulation => write!(f, "scene is mid-simulation — stop Play first"),
            SceneIoError::CreateDirectory(err) => write!(f, "Failed to create directory: {err}"),
            SceneIoError::Write(err) => write!(f, "{err}"),
            SceneIoError::Load(err) => write!(f, "Failed to load scene: {err}"),
        }
    }
}

impl std::error::Error for SceneIoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SceneIoError::CreateDirectory(err) => Some(err),
            SceneIoError::Load(err) => Some(err),
            _ => None,
        }
    }
}

/// The texture reference a scene file records for `handle`: the resolver's
/// recorded string, `#white` for the built-in handle 0, else a `#texture_N`
/// placeholder that fails loud on the next load. Shared by the GUI save and
/// the API's hosted save — one rule, one place.
pub(super) fn texture_ref_for_save(handle: u32, recorded: Option<impl Into<String>>) -> String {
    match recorded {
        Some(reference) => reference.into(),
        None if handle == 0 => "#white".to_string(),
        None => format!("#texture_{handle}"),
    }
}

impl<G: Game> EditorGame<G> {
    /// Save the current scene to the existing scene path (or default if none set).
    pub(super) fn save_scene(
        &mut self,
        world: &mut World,
        assets: &engine_core::assets::AssetManager,
    ) -> Result<(), SceneIoError> {
        let path = self.editor.scene_path()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SCENE_PATH));
        self.save_scene_as(world, assets, path)
    }

    /// Save the current scene to a specific path.
    pub(super) fn save_scene_as(
        &mut self,
        world: &mut World,
        assets: &engine_core::assets::AssetManager,
        path: PathBuf,
    ) -> Result<(), SceneIoError> {
        let texture_path_fn =
            |handle: u32| -> String { texture_ref_for_save(handle, assets.texture_path(handle)) };
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
        world: &mut World,
        texture_path_fn: &dyn Fn(u32) -> String,
        path: PathBuf,
    ) -> Result<(), SceneIoError> {
        if self.editor.in_play_session() {
            return Err(SceneIoError::MidSimulation);
        }

        // Scripts persist Entity params by NAME: give referenced unnamed
        // targets one now (auto-name) instead of silently
        // dropping the binding on save. Executed THROUGH CommandHistory so
        // the naming is undoable and dirty-tracked like every other editor
        // mutation, and a failed write leaves an undoable entry rather than
        // a silent one.
        let planned = engine_core::script_data::plan_script_target_names(world);
        if !planned.is_empty() {
            use editor::commands::{EditorCommand, RenameEntityCommand};
            let commands: Vec<Box<dyn EditorCommand>> = planned
                .iter()
                .map(|(entity, name)| {
                    Box::new(RenameEntityCommand::new(world, *entity, ecs::Name::new(name.clone())))
                        as Box<dyn EditorCommand>
                })
                .collect();
            self.command_history
                .execute_as_one("Name script targets", commands, world);
            let names: Vec<String> = planned.iter().map(|(_, n)| n.clone()).collect();
            self.editor.status_bar.show_message(format!(
                "Named {} script target(s): {} (undoable)",
                names.len(),
                names.join(", ")
            ));
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
                std::fs::create_dir_all(parent).map_err(SceneIoError::CreateDirectory)?;
            }
        }

        engine_core::scene_serializer::save_scene_to_file(&scene_data, &path)
            .map_err(SceneIoError::Write)?;

        self.editor.set_scene_path(Some(path.clone()));
        // The history is the dirty source of truth; the mirror is set too so
        // the same frame's title/status already read clean.
        self.command_history.mark_saved();
        self.sync_dirty_mirror();
        self.editor.status_bar.show_message("Scene saved");
        log::info!("Scene saved to: {:?}", path);
        Ok(())
    }

    /// Load a scene from disk, replacing the current world.
    ///
    /// The current world is only touched once the file is known-good: the
    /// scene is parsed first, then instantiated into a scratch world as a
    /// dry run (a corrupt file, unknown prefab, or missing
    /// texture must not cost the user's unsaved scene). The price is
    /// instantiating twice; `AssetManager` deduplicates texture loads by
    /// (path, filter), so the dry run warms the cache and the real
    /// instantiate uploads nothing twice.
    pub(super) fn load_scene(
        &mut self,
        world: &mut World,
        assets: &mut impl engine_core::TextureResolver,
        path: &Path,
    ) -> Result<(), SceneIoError> {
        if self.scene_replace_refusal().is_some() {
            return Err(SceneIoError::MidSimulation);
        }

        if self.command_history.is_dirty() {
            log::warn!("Current scene has unsaved changes. Save first to avoid losing work.");
        }

        // Parse and dry-run BEFORE touching the world.
        let data = engine_core::scene_loader::SceneLoader::load_from_file(path)
            .map_err(SceneIoError::Load)?;
        let mut scratch = World::new();
        engine_core::scene_loader::SceneLoader::instantiate(&data, &mut scratch, assets)
            .map_err(SceneIoError::Load)?;

        // Known-good: replace the live world.
        world.clear();
        let scene_instance = engine_core::scene_loader::SceneLoader::instantiate(&data, world, assets)
            .map_err(SceneIoError::Load)?;

        // Store physics settings from loaded scene, and publish them as a
        // world resource so the host game (EditorApp's lazy physics preview)
        // can build its PhysicsSystem without reaching into EditorGame.
        // Resources deliberately survive the play snapshot restore — so if
        // physics settings ever become EDITABLE, route edits through
        // `self.physics_settings` (reset on load/new), NOT this resource: a
        // runtime write to it would outlive Stop and get saved.
        self.physics_settings = scene_instance.physics.clone();
        match scene_instance.physics.clone() {
            Some(settings) => world.insert_resource(settings),
            None => {
                world.remove_resource::<engine_core::scene_data::PhysicsSettings>();
            }
        }

        log::info!("Scene loaded from: {:?} ({} entities)", path, scene_instance.entity_count);

        self.editor.set_scene_path(Some(path.to_path_buf()));
        self.reset_session();
        if let Some(first) = scene_instance.load_warnings.first() {
            // Non-fatal load diagnostics reach the user, not just the log.
            self.editor.status_bar.show_error(format!(
                "Loaded with {} warning(s): {}",
                scene_instance.load_warnings.len(),
                first
            ));
        } else {
            self.editor.status_bar.show_message("Scene loaded");
        }

        Ok(())
    }

    fn reset_session(&mut self) {
        self.command_history = editor::CommandHistory::new();
        self.sync_dirty_mirror();
        // A stale API batch would hold commands referencing the replaced
        // world — drop it with the history.
        self.api.batch = None;
        self.editor.selection.clear();
        self.gizmo_drag = None;
        self.editor.gizmo.cancel();
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

    /// Where "Open Scene…"/"Save As…" default to: a `scene.ron` next to the
    /// currently open scene, else the legacy cwd-relative default (the old
    /// hardcoded default wrote into the wrong directory after opening a project
    /// elsewhere).
    pub(super) fn default_scene_path(&self) -> PathBuf {
        self.editor
            .scene_path()
            .and_then(|p| p.parent().map(|d| d.join("scene.ron")))
            .unwrap_or_else(|| PathBuf::from(crate::constants::DEFAULT_SCENE_PATH))
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

        if self.command_history.is_dirty() {
            log::warn!("Current scene has unsaved changes. Save first to avoid losing work.");
        }

        world.clear();

        self.editor.set_scene_path(None);
        self.reset_session();
        self.entity_counter = 0;
        self.physics_settings = None;
        world.remove_resource::<engine_core::scene_data::PhysicsSettings>();
        log::info!("New scene created");
    }
}
