//! Core functionality for the insiculous_2d game engine.
//!
//! This crate provides the game loop, timing, and scene graph management.
//!
//! # Quick Start
//!
//! The easiest way to create a game is using the `Game` trait:
//!
//! ```no_run
//! use engine_core::prelude::*;
//!
//! struct MyGame;
//!
//! impl Game for MyGame {
//!     fn update(&mut self, _ctx: &mut GameContext) {
//!         // Game logic here
//!     }
//! }
//!
//! fn main() {
//!     run_game(MyGame, GameConfig::default()).unwrap();
//! }
//! ```

#[cfg(feature = "physics")]
pub mod behavior_runner;
#[cfg(target_arch = "wasm32")]
pub mod web;
mod game;
mod glyph_texture_cache;
mod scene;
pub mod lifecycle;
pub mod assets;
pub mod chaos_theme;
pub mod script_data;
pub mod scene_data;
pub mod component_registration;
pub mod scene_loader;
mod scene_loader_components;
pub mod scene_serializer;
mod texture_ref;
pub mod sheet_file;
pub(crate) mod texture_filter_serde;
mod tilemap_render;
pub mod render_manager;
pub mod window_manager;
pub mod game_loop_manager;
pub mod game_config;
pub mod gamepad_backend;
pub mod input_settings_io;
pub mod save_store;
pub mod scores;
pub mod contexts;
pub mod ui_integration;
pub mod chaos_mode;
pub mod achievements;
pub mod localization;
pub mod ui_element_system;
pub mod menu_input;
pub mod menu_panel;
pub mod pause;
pub mod spawn_helpers;
#[cfg(feature = "physics")]
pub mod pickups;
pub mod particles;
pub mod grid;
pub mod debug;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub mod prelude;

// Re-export the public API surface explicitly (no globs) so the crate's
// top-level names are visible at a glance.
#[cfg(feature = "physics")]
pub use behavior_runner::{BehaviorRunner, EntityCollected};
pub use game::{run_game, Game};
pub use scene::Scene;
pub use lifecycle::{Lifecycle, LifecycleManager, LifecycleState};
pub use assets::{sprite_sheet::SpriteSheet, AssetConfig, AssetError, AssetManager};
pub use sheet_file::{sidecar_path_for, SheetFile, SHEET_FILE_VERSION};
pub use scene_data::{
    BehaviorData, ColliderShapeData, ComponentData, EditorSettings, EntityData, PhysicsSettings,
    PrefabData, RigidBodyTypeData, SceneData, SceneLoadError,
};
pub use chaos_theme::ChaosTheme;
pub use scene_loader::{SceneInstance, SceneLoader};
pub use texture_ref::{SheetData, TextureResolver};

/// The game's root directory for asset/save anchoring (exe dir when shipped
/// with an `assets/` folder beside it, the game crate's directory under
/// `cargo run`). Expands `CARGO_MANIFEST_DIR` at the CALL SITE, so it must
/// be a macro — a plain engine function would bake in the engine's own path.
#[macro_export]
macro_rules! game_root {
    () => {
        $crate::assets::game_root_from(env!("CARGO_MANIFEST_DIR"))
    };
}
pub use scene_serializer::{save_scene_to_file, serialize_to_ron, world_to_scene_data};
pub use render_manager::{main_camera_pose, main_camera_position, RenderManager};
pub use window_manager::{WindowConfig, WindowManager};
pub use game_loop_manager::{GameLoopManager, MAX_DELTA_TIME};
pub use game_config::GameConfig;
pub use localization::{LocaleFile, Strings, FALLBACK_LOCALE};
pub use ui_element_system::{draw_ui_elements, UiButtonPressed, UiElementsHidden};
pub use menu_panel::{MenuMouse, MenuPanel, MenuStyle};
pub use pause::{PauseAction, PauseMenu, PauseMenuLabels};
pub use input_settings_io::{load_or_create as load_input_settings, save as save_input_settings, InputSettingsError};
pub use chaos_mode::ChaosMode;
pub use scores::{ScoreEntry, Scores, ScoresError, MAX_SCORES_PER_MODE};
pub use achievements::{
    Achievement, AchievementError, AchievementManager, ToastStyle, DEFAULT_TOAST_DURATION,
};
pub use menu_input::MenuInput;
pub use spawn_helpers::spawn_background;

/// Pixels per world unit: the renderer draws a sprite at
/// `Transform2D.scale × RENDER_UNIT` pixels. Physics colliders are sized in
/// absolute pixels and IGNORE `Transform2D.scale`, so games size both from
/// the same pixel constants divided/multiplied by this.
pub const RENDER_UNIT: f32 = 80.0;

/// Initialize the engine core
pub fn init() -> Result<(), EngineError> {
    log::info!("Engine core initialized");
    Ok(())
}

/// Errors that can occur in the engine core
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Failed to initialize engine: {0}")]
    InitializationError(String),

    #[error("Game loop error: {0}")]
    GameLoopError(String),
}
