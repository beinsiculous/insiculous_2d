//! The `System` trait: the one method every engine system implements.
//!
//! There is no scheduler. A system is stepped by whoever owns it — the frame
//! tail for the engine's own systems, `Game::update` for a game's.

use std::any::Any;

use crate::world::World;

/// A trait for systems in the ECS
pub trait System: Any + Send + Sync {
    /// Advance the system by `delta_time` seconds.
    fn update(&mut self, world: &mut World, delta_time: f32);
}
