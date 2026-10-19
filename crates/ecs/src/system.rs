//! The `System` trait: the contract every engine system implements.
//!
//! There is no scheduler. A system is stepped by whoever owns it — the frame
//! tail for the engine's own systems, a game's `update` for the game's — and
//! its lifecycle hooks are that owner's to call. The engine's frame tail calls
//! only `update`; nothing in the engine calls `initialize`, `start`, `stop` or
//! `shutdown` on its own systems.

use std::any::Any;

use crate::world::World;

/// A trait for systems in the ECS
pub trait System: Any + Send + Sync {
    /// Initialize the system; its owner calls this once before the first update
    fn initialize(&mut self, _world: &mut World) -> Result<(), String> {
        Ok(())
    }

    /// Start the system; its owner calls this when it begins stepping it
    fn start(&mut self, _world: &mut World) -> Result<(), String> {
        Ok(())
    }

    /// Update the system
    fn update(&mut self, world: &mut World, delta_time: f32);

    /// Stop the system; its owner calls this when it stops stepping it
    fn stop(&mut self, _world: &mut World) -> Result<(), String> {
        Ok(())
    }

    /// Shutdown the system; its owner calls this once, after the last update
    fn shutdown(&mut self, _world: &mut World) -> Result<(), String> {
        Ok(())
    }

    /// Get the name of the system
    fn name(&self) -> &str;
}
