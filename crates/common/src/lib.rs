//! Common types and utilities for the Insiculous 2D game engine.
//!
//! This crate provides shared types used across multiple engine crates,
//! eliminating duplication and ensuring consistency.

pub mod clock;
pub mod color;
pub mod hash;
pub mod vfs;
pub mod transform;
pub mod camera;
pub mod rect;
pub mod macros;
pub mod sheet_grid;

pub mod prelude {
    //! Prelude module for common types.
    //!
    //! Import with `use common::prelude::*;`

    pub use crate::color::Color;
    pub use crate::transform::Transform2D;
    pub use crate::camera::Camera;
    pub use crate::rect::Rect;
    pub use crate::sheet_grid::SheetGrid;
    pub use crate::clamp_volume;
}

// Re-export at crate root for convenience
pub use color::Color;
pub use hash::{hash_f32, hash_u32};
pub use transform::Transform2D;
pub use camera::Camera;
pub use rect::Rect;
pub use sheet_grid::SheetGrid;

/// Clamp an audio volume to the valid 0.0..=1.0 range.
pub fn clamp_volume(volume: f32) -> f32 {
    volume.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_volume_bounds() {
        assert_eq!(clamp_volume(-0.5), 0.0);
        assert_eq!(clamp_volume(0.0), 0.0);
        assert_eq!(clamp_volume(0.5), 0.5);
        assert_eq!(clamp_volume(1.0), 1.0);
        assert_eq!(clamp_volume(1.5), 1.0);
    }
}
