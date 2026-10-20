use conductor_spike::{game_config, SpikeGame};

fn main() {
    let root = engine_core::game_root!();
    let asset_base = root.join("assets");
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.iter().any(|argument| argument == "--measure") {
        if let Err(error) = conductor_spike::measure::run(&asset_base, &arguments) {
            eprintln!("measurement failed: {error}");
            std::process::exit(1);
        }
        return;
    }
    let config = game_config(&asset_base.to_string_lossy())
        .with_achievement_save_path(root.join("saves/achievements.json").to_string_lossy());
    if let Err(error) = engine_core::prelude::run_game(SpikeGame::default(), config) {
        eprintln!("the spike failed to run: {error}");
        std::process::exit(1);
    }
}
