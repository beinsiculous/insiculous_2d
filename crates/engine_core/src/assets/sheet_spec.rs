//! A game's contract with one measured sprite sheet.
//!
//! A re-skinned game names each sheet once, as a [`SheetSpec`] constant: where the
//! synced copy lives, how big one art cell is, and where the reference frame's opaque
//! pixels sit inside that cell. Everything that must agree with the art — the sprite's
//! scale, the offset that lands the artwork on the entity, the collider or hit extent —
//! is derived from those three measured facts, so none of them can drift from the PNG.

use glam::Vec2;

use crate::RENDER_UNIT;

/// One sheet's contract: the synced copy's path, the size of one art cell, and the
/// opaque bounds of the reference frame the collider is measured from — all in art
/// pixels. The cell drives the sprite's `Transform2D.scale` (one art pixel per window
/// pixel, `px / RENDER_UNIT`); the bounds drive the collider and the sprite offset that
/// lands the artwork on it, so neither can fake a footprint the art does not have.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetSpec {
    /// Path under the asset base, as written in `assets/sprites/sync.list`.
    pub path: &'static str,
    /// One cell of the sheet, in art pixels.
    pub cell: Vec2,
    /// The reference frame's opaque box within the cell, in art pixels, Y down:
    /// `(top-left, bottom-right)`. The bottom-right corner is one past the last opaque
    /// pixel, so `bounds.1 - bounds.0` is the opaque size — a box whose last opaque
    /// column is 42 ends at 43.
    pub bounds: (Vec2, Vec2),
}

impl SheetSpec {
    /// The transform scale that draws the sheet at 1x, which is what keeps nearest
    /// filtering crisp.
    pub fn scale(&self) -> Vec2 {
        Vec2::new(self.cell.x / RENDER_UNIT, self.cell.y / RENDER_UNIT)
    }

    /// Whether the bounds are an ordered box inside the cell. A transposed tuple or a
    /// coordinate measured past the cell still yields a plausible offset and a wrong
    /// footprint, so a game asserts this over every spec it declares.
    pub fn is_within_cell(&self) -> bool {
        let (top_left, bottom_right) = self.bounds;
        top_left.cmpge(Vec2::ZERO).all()
            && top_left.cmple(bottom_right).all()
            && bottom_right.cmple(self.cell).all()
    }

    /// Where the cell's centre must be drawn relative to the entity for the
    /// reference frame's opaque centre to land on the entity. World units, Y up: the
    /// cell's Y grows downward, so a body that sits low in its cell draws the cell
    /// above the entity. Zero for every subject whose reference frame is centred.
    pub fn sprite_offset(&self) -> Vec2 {
        let box_centre = (self.bounds.0 + self.bounds.1) * 0.5;
        Vec2::new(self.cell.x * 0.5 - box_centre.x, box_centre.y - self.cell.y * 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CENTRED: SheetSpec = SheetSpec {
        path: "sprites/centred.png",
        cell: Vec2::new(64.0, 96.0),
        bounds: (Vec2::new(21.0, 9.0), Vec2::new(43.0, 87.0)),
    };

    const LOW_BODY: SheetSpec = SheetSpec {
        path: "sprites/low_body.png",
        cell: Vec2::new(48.0, 64.0),
        bounds: (Vec2::new(9.0, 27.0), Vec2::new(40.0, 58.0)),
    };

    #[test]
    fn test_is_within_cell_accepts_an_ordered_box_inside_the_cell() {
        assert!(CENTRED.is_within_cell());
        assert!(LOW_BODY.is_within_cell());
        let whole_cell = SheetSpec { bounds: (Vec2::ZERO, CENTRED.cell), ..CENTRED };
        assert!(whole_cell.is_within_cell());
    }

    #[test]
    fn test_is_within_cell_rejects_a_transposed_or_overflowing_box() {
        let transposed = SheetSpec { bounds: (CENTRED.bounds.1, CENTRED.bounds.0), ..CENTRED };
        assert!(!transposed.is_within_cell());
        let past_the_cell = SheetSpec {
            bounds: (Vec2::new(21.0, 9.0), Vec2::new(43.0, 97.0)),
            ..CENTRED
        };
        assert!(!past_the_cell.is_within_cell());
        let negative = SheetSpec { bounds: (Vec2::new(-1.0, 9.0), Vec2::new(43.0, 87.0)), ..CENTRED };
        assert!(!negative.is_within_cell());
    }

    #[test]
    fn test_scale_draws_one_art_pixel_per_window_pixel() {
        assert_eq!(CENTRED.scale() * RENDER_UNIT, CENTRED.cell);
        assert_eq!(LOW_BODY.scale() * RENDER_UNIT, LOW_BODY.cell);
    }

    #[test]
    fn test_sprite_offset_is_zero_for_a_centred_reference_frame() {
        assert_eq!(CENTRED.sprite_offset(), Vec2::ZERO);
    }

    #[test]
    fn test_sprite_offset_raises_the_cell_over_a_body_that_sits_low() {
        // The body's centre is half a pixel right of the cell's and ten and a half
        // below it, so the cell is drawn half a pixel left and ten and a half up.
        assert_eq!(LOW_BODY.sprite_offset(), Vec2::new(-0.5, 10.5));
    }
}
