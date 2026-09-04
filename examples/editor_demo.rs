//! Editor demo - wraps the Hello World platformer in the full editor UI.
//!
//! Run with: cargo run --example editor_demo --features editor
//!
//! This loads the same scene and game logic as hello_world.rs, but wrapped
//! inside the editor. Use Play/Pause/Stop (Ctrl+P / Ctrl+Shift+P) to run
//! the game simulation, inspect entities in the hierarchy & inspector,
//! and watch the world restore on Stop.
//!
//! Controls (while Playing):
//!   WASD to move player, SPACE to jump, R to reset
//!
//! Editor shortcuts:
//!   Ctrl+P       Play / Pause toggle
//!   Ctrl+Shift+P Stop (restore scene)
//!   F5           Play / Resume
//!   Q/W/E/R      Select / Move / Rotate / Scale tool
//!   G            Toggle grid

#[path = "hello_world/platformer.rs"]
mod platformer;

use editor_integration::run_game_with_editor;
use engine_core::prelude::*;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = GameConfig::new("Insiculous 2D - Editor Demo")
        .with_size(1280, 720)
        .with_clear_color(0.1, 0.1, 0.15, 1.0)
        .with_asset_base_path(platformer::EXAMPLES_DIR)
        .with_locales_dir("assets/locales");

    if let Err(error) = run_game_with_editor(platformer::PlatformerGame::new(), config) {
        log::error!("Editor error: {}", error);
    }
}
