//! Scene graph management for the engine.
//!
//! This module provides functionality for managing the game scene graph, similar to Godot's scene system.

use std::path::Path;

use ecs::World;
use crate::assets::AssetManager;
use crate::lifecycle::{LifecycleManager, LifecycleState};
use crate::scene_data::SceneLoadError;
use crate::scene_loader::{SceneInstance, SceneLoader};

/// A simple scene graph container
pub struct Scene {
    /// Lifecycle manager for scene state
    lifecycle: LifecycleManager,
    /// Name of the scene
    name: String,
    /// The ECS world
    pub world: World,
}

impl Scene {
    /// Create a new scene with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            lifecycle: LifecycleManager::new(),
            name: name.into(),
            world: World::default(),
        }
    }

    /// Initialize the scene with proper lifecycle management
    pub fn initialize(&mut self) -> Result<(), String> {
        self.lifecycle.begin_initialization()?;
        
        // Initialize the world
        self.world.initialize().map_err(|e| format!("Failed to initialize world: {}", e))?;
        
        self.lifecycle.complete_initialization()?;
        log::info!("Scene '{}' initialized successfully", self.name);
        Ok(())
    }

    /// Start the scene (transition to running state)
    pub fn start(&mut self) -> Result<(), String> {
        self.lifecycle.start()?;
        self.world.start().map_err(|e| format!("Failed to start world: {}", e))?;
        log::info!("Scene '{}' started", self.name);
        Ok(())
    }

    /// Stop the scene (transition from running to initialized)
    pub fn stop(&mut self) -> Result<(), String> {
        self.lifecycle.stop()?;
        self.world.stop().map_err(|e| format!("Failed to stop world: {}", e))?;
        log::info!("Scene '{}' stopped", self.name);
        Ok(())
    }

    /// Shutdown the scene
    pub fn shutdown(&mut self) -> Result<(), String> {
        self.lifecycle.begin_shutdown()?;
        self.world.shutdown().map_err(|e| format!("Failed to shutdown world: {}", e))?;
        self.lifecycle.complete_shutdown()?;
        log::info!("Scene '{}' shut down successfully", self.name);
        Ok(())
    }

    /// Check if the scene is operational (initialized or running)
    pub fn is_operational(&self) -> bool {
        self.lifecycle.is_operational()
    }

    /// Check if the scene is initialized (can be either Initialized or Running)
    pub fn is_initialized(&self) -> bool {
        matches!(self.lifecycle.current_state(), LifecycleState::Initialized | LifecycleState::Running)
    }

    /// Check if the scene is running
    pub fn is_running(&self) -> bool {
        matches!(self.lifecycle.current_state(), LifecycleState::Running)
    }

    /// Get the current lifecycle state
    pub fn lifecycle_state(&self) -> LifecycleState {
        self.lifecycle.current_state()
    }

    /// Get the name of the scene
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Load entities from a RON scene file into this scene
    ///
    /// This loads the scene file, instantiates all entities, and returns
    /// a SceneInstance with named entity lookups.
    ///
    /// # Example
    /// ```
    /// # use engine_core::prelude::*;
    /// # fn load(scene: &mut Scene, assets: &mut AssetManager) -> Result<(), SceneLoadError> {
    /// let instance = scene.load_from_file("assets/scenes/level1.scene.ron", assets)?;
    /// let player = instance.get_entity("player").unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file(
        &mut self,
        path: impl AsRef<Path>,
        assets: &mut AssetManager,
    ) -> Result<SceneInstance, SceneLoadError> {
        let instance = SceneLoader::load_and_instantiate(path, &mut self.world, assets)?;
        log::info!(
            "Loaded scene '{}' with {} entities",
            instance.name,
            instance.entity_count
        );
        Ok(instance)
    }
}
