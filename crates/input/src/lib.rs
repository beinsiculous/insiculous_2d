//! Input abstraction for the insiculous_2d game engine.
//!
//! This crate provides abstractions for keyboard, mouse, and gamepad input,
//! plus a generic action-mapping layer ([`InputMapping`]) that games use with
//! their own action types.

mod button_tracker;
mod gamepad;
mod input_handler;
mod input_mapping;
mod keyboard;
mod mouse;
mod pad_layout;
mod player;

pub mod prelude;

// Re-export for convenience
pub use button_tracker::*;
pub use gamepad::*;
pub use input_handler::*;
pub use input_mapping::*;
pub use keyboard::*;
pub use mouse::*;
pub use pad_layout::*;
pub use player::*;
