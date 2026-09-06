//! Headless contract verification tests for the scripting subsystem.

mod contracts;
mod lifecycle;
mod rhai;

use ecs::World;
use input::{InputHandler, InputSettings};

pub(crate) fn test_inputs() -> (InputHandler, InputSettings) {
    (InputHandler::new(), InputSettings::default())
}

pub(crate) fn test_world() -> World {
    World::new()
}
