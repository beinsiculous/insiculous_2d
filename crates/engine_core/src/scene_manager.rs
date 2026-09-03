//! Scene management for the engine.
//!
//! This module provides the `SceneManager` which manages the scene stack and lifecycle.
//! It handles pushing and popping scenes, and provides access to the active scene.

use crate::Scene;

/// Manages the scene stack and provides access to active scenes.
///
/// The SceneManager maintains a stack of scenes where the top scene is considered
/// the active scene. This allows for scene transitions, pause menus, and layered
/// scene management similar to other game engines.
pub struct SceneManager {
    /// Stack of scenes (0+ scenes)
    scenes: Vec<Scene>,
}

impl SceneManager {
    /// Create a new scene manager with no scenes.
    pub fn new() -> Self {
        Self {
            scenes: Vec::new(),
        }
    }

    /// Create a new scene manager with an initial scene.
    pub fn with_scene(scene: Scene) -> Self {
        Self {
            scenes: vec![scene],
        }
    }

    /// Push a scene onto the stack.
    pub fn push(&mut self, scene: Scene) {
        self.scenes.push(scene);
    }

    /// Pop a scene from the stack.
    ///
    /// Returns the removed scene if the stack was not empty.
    pub fn pop(&mut self) -> Option<Scene> {
        self.scenes.pop()
    }

    /// Get a reference to the active scene (last scene in the stack).
    pub fn active(&self) -> Option<&Scene> {
        self.scenes.last()
    }

    /// Get a mutable reference to the active scene (last scene in the stack).
    pub fn active_mut(&mut self) -> Option<&mut Scene> {
        self.scenes.last_mut()
    }

    /// Get a reference to all scenes in the stack.
    pub fn scenes(&self) -> &[Scene] {
        &self.scenes
    }

    /// Get a mutable reference to all scenes in the stack.
    pub fn scenes_mut(&mut self) -> &mut [Scene] {
        &mut self.scenes
    }

    /// Check if the scene manager has any scenes.
    pub fn is_empty(&self) -> bool {
        self.scenes.is_empty()
    }

    /// Get the number of scenes in the stack.
    pub fn len(&self) -> usize {
        self.scenes.len()
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

