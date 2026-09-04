//! Hello World - Demonstrates the simplified Game API with Physics, Audio, UI, and Scene Graph
//!
//! This example shows how easy it is to create a game with the Insiculous 2D engine.
//! All the window, event loop, and rendering boilerplate is handled internally.
//!
//! Controls: WASD to move player, SPACE (or pad A) to jump, R to reset,
//!           M to toggle music, +/- to adjust volume, H to toggle UI,
//!           L to cycle language (English/Pirate), ESC to exit
//!           Walk right past the gap — the camera follows! Collect the coins!
//!
//! Scene file: examples/assets/scenes/hello_world.scene.ron

#[path = "hello_world/platformer.rs"]
mod platformer;

use engine_core::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let game_config = GameConfig::new("Hello World - Insiculous 2D Physics Demo")
        .with_size(800, 600)
        .with_clear_color(0.1, 0.1, 0.15, 1.0)
        .with_asset_base_path(platformer::EXAMPLES_DIR)
        .with_locales_dir("assets/locales");

    let game = platformer::PlatformerGame::new();
    run_game(game, game_config)?;
    Ok(())
}
