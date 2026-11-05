//! The points-to-physical boundary for drawing and input geometry.

use glam::Vec2;
use crate::{Color, Rect};
use super::UIContext;

impl UIContext {
    /// Physical pixels per point until the next call. Geometry retained across calls
    /// is physical, so changing scale mid-frame cannot move an existing hit region.
    /// Non-finite scales become 1.0; finite scales clamp to 0.5 through 4.0.
    pub fn set_pixels_per_point(&mut self, scale: f32) {
        self.pixels_per_point = if scale.is_finite() { scale.clamp(0.5, 4.0) } else { 1.0 };
    }

    /// The physical pixels in one point for the current caller.
    pub fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }

    pub(super) fn to_physical(&self, rect: Rect) -> Rect {
        Rect::new(
            self.to_physical_length(rect.x), self.to_physical_length(rect.y),
            self.to_physical_length(rect.width), self.to_physical_length(rect.height),
        )
    }

    pub(super) fn to_points(&self, position: Vec2) -> Vec2 {
        position / self.pixels_per_point
    }

    pub(super) fn to_physical_length(&self, points: f32) -> f32 {
        points * self.pixels_per_point
    }

    // ================== Drawing Primitives ==================

    /// Draw a colored rectangle.
    pub fn rect(&mut self, bounds: Rect, color: Color) {
        self.draw_list.rect(self.to_physical(bounds), color);
    }

    /// Draw a colored rectangle with rounded corners.
    pub fn rect_rounded(&mut self, bounds: Rect, color: Color, corner_radius: f32) {
        self.draw_list.rect_rounded(self.to_physical(bounds), color, self.to_physical_length(corner_radius));
    }

    /// Draw a rectangle outline with the given stroke width and corner radius.
    pub fn rect_border(&mut self, bounds: Rect, color: Color, width: f32, corner_radius: f32) {
        self.draw_list.rect_border_rounded(self.to_physical(bounds), color, self.to_physical_length(width), self.to_physical_length(corner_radius));
    }

    /// Draw a textured image by renderer texture id (thumbnails, previews).
    pub fn image(&mut self, bounds: Rect, texture_id: u32, tint: Color) {
        self.draw_list.image(self.to_physical(bounds), texture_id, tint);
    }

    /// Draw a circle.
    pub fn circle(&mut self, center: Vec2, radius: f32, color: Color) {
        self.draw_list.circle(center * self.pixels_per_point, self.to_physical_length(radius), color);
    }

    /// Draw a line.
    pub fn line(&mut self, start: Vec2, end: Vec2, color: Color, width: f32) {
        self.draw_list.line(start * self.pixels_per_point, end * self.pixels_per_point, color, self.to_physical_length(width));
    }

    /// Begin clipping all subsequent draws to the given bounds.
    ///
    /// Use this to prevent content from rendering outside panel boundaries.
    /// Must be paired with `pop_clip_rect()`. Clip regions can be nested.
    pub fn push_clip_rect(&mut self, bounds: Rect) {
        self.draw_list.push_clip_rect(self.to_physical(bounds));
    }

    /// End the current clip region, restoring the previous clip state.
    ///
    /// If no clip region is active, this resets to the full viewport.
    pub fn pop_clip_rect(&mut self) {
        self.draw_list.pop_clip_rect();
    }

}
