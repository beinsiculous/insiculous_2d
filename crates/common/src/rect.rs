//! Rectangle type for 2D bounds and UI layout.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Axis-aligned rectangle defined by position and size.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Rect {
    /// Top-left position
    pub x: f32,
    pub y: f32,
    /// Dimensions
    pub width: f32,
    pub height: f32,
}

impl Rect {
    /// Create a new rectangle from position and size.
    #[inline]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Create a rectangle from position and size vectors.
    #[inline]
    pub fn from_pos_size(pos: Vec2, size: Vec2) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            width: size.x,
            height: size.y,
        }
    }

    /// Create a rectangle from min and max corners.
    #[inline]
    pub fn from_min_max(min: Vec2, max: Vec2) -> Self {
        Self {
            x: min.x,
            y: min.y,
            width: max.x - min.x,
            height: max.y - min.y,
        }
    }

    /// Create a rectangle centered at a position.
    #[inline]
    pub fn centered(center: Vec2, size: Vec2) -> Self {
        Self {
            x: center.x - size.x * 0.5,
            y: center.y - size.y * 0.5,
            width: size.x,
            height: size.y,
        }
    }

    /// Get the position (top-left corner).
    #[inline]
    pub fn position(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Get the size.
    #[inline]
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.width, self.height)
    }

    /// Get the center point.
    #[inline]
    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    /// Get the minimum corner (top-left).
    #[inline]
    pub fn min(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Get the maximum corner (bottom-right).
    #[inline]
    pub fn max(&self) -> Vec2 {
        Vec2::new(self.x + self.width, self.y + self.height)
    }

    /// Get left edge X coordinate.
    #[inline]
    pub fn left(&self) -> f32 {
        self.x
    }

    /// Get right edge X coordinate.
    #[inline]
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Get top edge Y coordinate.
    #[inline]
    pub fn top(&self) -> f32 {
        self.y
    }

    /// Get bottom edge Y coordinate.
    #[inline]
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Check if a point is inside the rectangle.
    #[inline]
    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    /// Check if this rectangle intersects another.
    #[inline]
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Get the intersection of two rectangles, if any.
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        if x1 < x2 && y1 < y2 {
            Some(Rect::new(x1, y1, x2 - x1, y2 - y1))
        } else {
            None
        }
    }

    /// Get the bounding box containing both rectangles.
    #[inline]
    pub fn union(&self, other: &Rect) -> Rect {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);
        Rect::new(x1, y1, x2 - x1, y2 - y1)
    }

    /// Expand the rectangle by the given amount on all sides.
    #[inline]
    pub fn expand(&self, amount: f32) -> Rect {
        Rect::new(
            self.x - amount,
            self.y - amount,
            self.width + amount * 2.0,
            self.height + amount * 2.0,
        )
    }

    /// Shrink the rectangle by the given amount on all sides.
    #[inline]
    pub fn shrink(&self, amount: f32) -> Rect {
        self.expand(-amount)
    }

    /// Translate the rectangle by the given offset.
    #[inline]
    pub fn translate(&self, offset: Vec2) -> Rect {
        Rect::new(self.x + offset.x, self.y + offset.y, self.width, self.height)
    }

    /// Alias for translate (compatibility with UI rect).
    #[inline]
    pub fn offset(&self, delta: Vec2) -> Rect {
        self.translate(delta)
    }

    /// Get the area of the rectangle.
    #[inline]
    pub fn area(&self) -> f32 {
        self.width * self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_is_inclusive_on_every_edge_and_rejects_points_just_outside() {
        // Every widget hit-test in the ui crate goes through contains, so an
        // exclusive edge would drop clicks landing exactly on a border.
        let rect = Rect::new(-10.0, -20.0, 30.0, 40.0);
        let cases = [
            ("interior", Vec2::new(0.0, 0.0), true),
            ("min corner", Vec2::new(-10.0, -20.0), true),
            ("max corner", Vec2::new(20.0, 20.0), true),
            ("left edge", Vec2::new(-10.0, 5.0), true),
            ("right edge", Vec2::new(20.0, 5.0), true),
            ("top edge", Vec2::new(5.0, -20.0), true),
            ("bottom edge", Vec2::new(5.0, 20.0), true),
            ("just left", Vec2::new(-10.001, 5.0), false),
            ("just right", Vec2::new(20.001, 5.0), false),
            ("just above", Vec2::new(5.0, -20.001), false),
            ("just below", Vec2::new(5.0, 20.001), false),
            ("far negative", Vec2::new(-100.0, -100.0), false),
        ];

        for (label, point, expected) in cases {
            assert_eq!(rect.contains(point), expected, "{label}: {point:?} in {rect:?}");
        }
    }

    #[test]
    fn test_center_and_expand_derive_exact_geometry_from_the_origin_corner() {
        let rect = Rect::new(10.0, 20.0, 80.0, 40.0);

        let center = rect.center();
        let grown = rect.expand(5.0);
        let shrunk = rect.expand(-5.0);

        assert_eq!(center, Vec2::new(50.0, 40.0));
        assert_eq!(grown, Rect::new(5.0, 15.0, 90.0, 50.0), "expand grows every side by the amount");
        assert_eq!(shrunk, Rect::new(15.0, 25.0, 70.0, 30.0), "a negative amount insets every side");
        assert_eq!(grown.center(), center, "expanding must not move the center");
    }
}
