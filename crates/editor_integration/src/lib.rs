//! Editor integration layer for the insiculous_2d game engine.
//!
//! This crate provides `run_game_with_editor()` — a single function call that
//! wraps any `Game` implementation with the full editor UI (menu bar, toolbar,
//! dock panels, hierarchy, inspector, gizmo, tool shortcuts).
//!
//! # Example
//! ```no_run
//! use engine_core::prelude::*;
//! use editor_integration::run_game_with_editor;
//!
//! struct MyGame;
//! impl Game for MyGame {
//!     fn update(&mut self, _ctx: &mut GameContext) { /* game logic */ }
//! }
//!
//! fn main() {
//!     run_game_with_editor(MyGame, GameConfig::new("My Game")).unwrap();
//! }
//! ```

mod constants;
mod editor_game;
mod entity_ops;
mod panel_renderer;

pub use constants::find_first_scene;
pub use editor_game::headless::{run_headless_editor_api, HeadlessAssets};
pub use editor_game::{run_game_with_editor, run_game_with_editor_opts, EditorRunOptions};
