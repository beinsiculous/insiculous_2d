//! The K1 spike: one owned conductor, proving shared MIDI parts and custom instruments on
//! native and web before the engine commits to the design.
//!
//! The pieces are the design under test, small enough to read in one sitting: an
//! [`instrument_slot`] per instrument (one synthesizer, a lane per source, a ledger that
//! keeps the synthesizer's stealer off sounding notes), a [`sample_slot`] for a user WAV,
//! a [`score`] player for the background music, all mixed by the [`mixer`] inside one
//! rodio source ([`spike_source`]) that the game thread drives through a bounded channel.
//! `coordination/K1_SPIKE.md` in the engine holds the measurements.

pub mod bank;
pub mod frame_stats;
pub mod game;
pub mod instrument_slot;
pub mod memory;
pub mod mixer;
pub mod phrase;
pub mod rig;
pub mod sample_slot;
pub mod score;
pub mod spike_source;
pub mod storm;

#[cfg(not(target_arch = "wasm32"))]
pub mod measure;

#[cfg(test)]
mod tests;

#[cfg(target_arch = "wasm32")]
mod web_entry;

use engine_core::prelude::*;

pub use game::SpikeGame;

/// The shared `GameConfig` for both targets; `asset_base` is anchored (an absolute path
/// natively, the deploy URL base on the web).
pub fn game_config(asset_base: &str) -> GameConfig {
    GameConfig::new("Conductor Spike")
        .with_size(960, 540)
        .with_clear_color(0.05, 0.05, 0.08, 1.0)
        .with_fps(60)
        .with_asset_base_path(asset_base)
}
