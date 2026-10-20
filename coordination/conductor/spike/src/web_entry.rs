//! Browser entry point: fetch every asset into the engine's VFS, then run the same game
//! the native binary runs. Served locally only (`scripts/build_wasm.sh` output); the site
//! has no page for it.

use engine_core::prelude::run_game;
use engine_core::web::{init_web_logging, preload_assets, set_boot_status};
use wasm_bindgen::prelude::wasm_bindgen;

/// Where the local build serves its assets from; also the VFS key base. Must agree with
/// `scripts/build_wasm.sh coordination/conductor/spike conductor-spike --version v1`.
const ASSET_BASE: &str = "/games/conductor-spike/v1/assets";

#[wasm_bindgen(start)]
pub fn start() {
    init_web_logging();
    wasm_bindgen_futures::spawn_local(async {
        let started = common::clock::Instant::now();
        if let Err(error) = preload_assets(ASSET_BASE).await {
            log::error!("asset preload failed: {error}");
            set_boot_status(&format!("Failed to load assets: {error}"));
            return;
        }
        log::info!("asset preload took {:.1} ms", started.elapsed().as_secs_f64() * 1000.0);
        if let Err(error) = run_game(crate::SpikeGame::default(), crate::game_config(ASSET_BASE)) {
            log::error!("failed to start the spike: {error}");
            set_boot_status(&format!("Failed to start: {error}"));
        }
    });
}
