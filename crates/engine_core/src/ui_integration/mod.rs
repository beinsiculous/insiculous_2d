//! UI rendering integration - bridges UI draw commands to renderer sprites.
//!
//! This module provides the integration layer between the UI system and the renderer,
//! converting UI draw commands (rectangles, text, circles, lines) into sprites that
//! can be rendered by the sprite batching system.

use common::Camera;
use glam::Vec2;
use renderer::{
    sprite::{SpriteBatcher, Sprite},
    texture::TextureHandle,
};
use ui::{DrawCommand, Rect};
use std::collections::HashMap;
use crate::contexts::GlyphCacheKey;

/// Where UI sprites land in world space so they render at fixed SCREEN
/// pixels through `camera`. UI must not move when the game camera moves
/// (camera-follow gameplay, the editor's panel-derived camera), so the
/// conversion is camera-relative: position offsets and sizes are divided by
/// zoom and anchored at the camera position.
#[derive(Debug, Clone, Copy)]
struct UiCameraSpace {
    camera_position: Vec2,
    viewport_size: Vec2,
    inv_zoom: f32,
}

impl UiCameraSpace {
    fn new(camera: &Camera) -> Self {
        Self {
            camera_position: camera.position,
            viewport_size: camera.viewport_size,
            // Camera zoom is clamped elsewhere but guard anyway
            inv_zoom: if camera.zoom.abs() > f32::EPSILON { 1.0 / camera.zoom } else { 1.0 },
        }
    }

    /// World position for a screen-space rect center.
    /// Screen: (0,0) = top-left. Y flips into the Y-up world.
    fn rect_center(&self, bounds: &Rect) -> Vec2 {
        self.point(
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        )
    }

    /// World position for a screen-space point.
    fn point(&self, x: f32, y: f32) -> Vec2 {
        self.camera_position
            + Vec2::new(
                (x - self.viewport_size.x / 2.0) * self.inv_zoom,
                (self.viewport_size.y / 2.0 - y) * self.inv_zoom,
            )
    }

    /// World size for a screen-pixel size.
    fn size(&self, size: Vec2) -> Vec2 {
        size * self.inv_zoom
    }

    /// World length for a screen-pixel length (corner radii, stroke widths —
    /// SDF params live in the sprite's local units, so they scale with size).
    fn len(&self, len: f32) -> f32 {
        len * self.inv_zoom
    }
}

/// Renders UI draw commands as sprites in the sprite batcher.
/// 
/// This function converts UI draw commands into renderer sprites, handling:
/// - Rectangle rendering with proper coordinate transformation
/// - Rectangle borders as 4 separate rectangles
/// - Text rendering using cached glyph textures
/// - Text placeholders for debugging
/// - Circle rendering (as squares until circle shader is available)
/// - Line rendering as rotated rectangles
/// 
/// Coordinate transformation: UI uses screen coordinates (0,0 = top-left) while
/// the renderer uses world coordinates (0,0 = center of screen).
pub fn render_ui_commands(
    sprites: &mut SpriteBatcher,
    commands: &[DrawCommand],
    camera: &Camera,
    glyph_textures: &HashMap<GlyphCacheKey, TextureHandle>,
) {
    let cam = UiCameraSpace::new(camera);
    let white_texture = TextureHandle { id: 0 };
    let mut clip_stack: Vec<Rect> = Vec::new();

    for cmd in commands {
        // Software clip filtering: skip draw commands fully outside the active clip rect
        if !clip_stack.is_empty() {
            let cmd_bounds = match cmd {
                DrawCommand::Rect { bounds, .. }
                | DrawCommand::RectBorder { bounds, .. }
                | DrawCommand::Image { bounds, .. } => Some(*bounds),
                DrawCommand::Text { data, .. } => {
                    Some(Rect::new(data.position.x, data.position.y, data.width, data.height))
                }
                DrawCommand::TextPlaceholder { position, font_size, text, .. } => {
                    Some(Rect::new(position.x, position.y, text.len() as f32 * font_size * 0.6, *font_size))
                }
                DrawCommand::Circle { center, radius, .. } => {
                    Some(Rect::new(center.x - radius, center.y - radius, radius * 2.0, radius * 2.0))
                }
                DrawCommand::Line { start, end, width, .. } => {
                    // Grow the bbox by the stroke width: an axis-aligned line
                    // has a zero-width or zero-height bbox, and a degenerate
                    // bbox never overlaps the clip rect — which silently
                    // culled every horizontal/vertical line (crosshair,
                    // collider boxes, selection outlines).
                    Some(Rect::new(
                        start.x.min(end.x) - width * 0.5,
                        start.y.min(end.y) - width * 0.5,
                        (end.x - start.x).abs() + width,
                        (end.y - start.y).abs() + width,
                    ))
                }
                _ => None, // PushClipRect / PopClipRect handled below
            };

            if let Some(bounds) = cmd_bounds {
                let active_clip = clip_stack.last().unwrap();
                let overlap = intersect_rects(&bounds, active_clip);
                if overlap.width == 0.0 || overlap.height == 0.0 {
                    continue;
                }
            }
        }

        match cmd {
            DrawCommand::Rect { bounds, color, corner_radius, depth } => {
                // Convert screen coordinates (0,0 = top-left) to world coordinates (0,0 = center)
                let center = cam.rect_center(bounds);

                let mut sprite = Sprite::new(white_texture)
                    .with_position(center)
                    .with_scale(cam.size(Vec2::new(bounds.width, bounds.height)))
                    .with_color(glam::Vec4::new(color.r, color.g, color.b, color.a))
                    .with_depth(*depth);
                if *corner_radius > 0.0 {
                    sprite = sprite.with_corner_radius(cam.len(*corner_radius));
                }

                sprites.add_sprite(&sprite);
            }
            DrawCommand::RectBorder { bounds, color, width, corner_radius, depth } => {
                // A single SDF-bordered sprite (grown by the stroke width so
                // the border straddles the bounds like the old 4-rect version)
                let grown = Rect::new(
                    bounds.x - width / 2.0,
                    bounds.y - width / 2.0,
                    bounds.width + width,
                    bounds.height + width,
                );
                let center = cam.rect_center(&grown);

                let sprite = Sprite::new(white_texture)
                    .with_position(center)
                    .with_scale(cam.size(Vec2::new(grown.width, grown.height)))
                    .with_color(glam::Vec4::new(color.r, color.g, color.b, color.a))
                    .with_depth(*depth)
                    .with_corner_radius(cam.len(corner_radius.max(0.0)))
                    .with_border(cam.len(width.max(1.0)));

                sprites.add_sprite(&sprite);
            }
            DrawCommand::Text { data, depth } => {
                // Render text with rasterized glyph data
                if data.glyphs.is_empty() {
                    // No glyphs - render as placeholder rectangle
                    let text_bounds = Rect::new(data.position.x, data.position.y, data.width, data.height);
                    let center = cam.rect_center(&text_bounds);

                    let sprite = Sprite::new(white_texture)
                        .with_position(center)
                        .with_scale(cam.size(Vec2::new(data.width.max(data.font_size * 4.0), data.height.max(data.font_size))))
                        .with_color(glam::Vec4::new(data.color.r, data.color.g, data.color.b, data.color.a * 0.3))
                        .with_depth(*depth);

                    sprites.add_sprite(&sprite);
                } else {
                    // Render each glyph using cached glyph textures
                    for glyph in &data.glyphs {
                        // Skip glyphs with no bitmap (spaces, etc.)
                        if glyph.width == 0 || glyph.height == 0 {
                            continue;
                        }

                        // Calculate glyph position in world coordinates
                        // - data.position is at the BASELINE of the text
                        // - glyph.x is horizontal offset from text start
                        // - glyph.y is vertical offset from baseline to glyph top (negative = above baseline)
                        //
                        // We construct a rect for the glyph bounds and convert its center to world coords
                        let glyph_bounds = Rect::new(
                            data.position.x + glyph.x,
                            data.position.y + glyph.y,
                            glyph.width as f32,
                            glyph.height as f32,
                        );
                        let glyph_center = cam.rect_center(&glyph_bounds);

                        // Look up glyph texture in cache (color-agnostic)
                        let glyph_key = GlyphCacheKey::new(
                            glyph.character,
                            glyph.width,
                            glyph.height,
                            data.font_id,
                        );

                        let texture = match glyph_textures.get(&glyph_key) {
                            Some(&tex) => tex,
                            None => {
                                log::warn!(
                                    "Missing glyph texture for '{}' ({}x{}). Using white fallback. \
                                     This may indicate the glyph wasn't pre-cached.",
                                    glyph.character, glyph.width, glyph.height
                                );
                                white_texture
                            }
                        };

                        // Render glyph with text color - texture is grayscale alpha mask
                        let render_width = glyph.width as f32;
                        let render_height = glyph.height as f32;

                        let sprite = Sprite::new(texture)
                            .with_position(glyph_center)
                            .with_scale(cam.size(Vec2::new(render_width, render_height)))
                            .with_color(glam::Vec4::new(data.color.r, data.color.g, data.color.b, data.color.a))
                            .with_depth(*depth);

                        sprites.add_sprite(&sprite);
                    }

                    // Debug output removed - font rendering is working
                }
            }
            DrawCommand::TextPlaceholder { text, position, color, font_size, depth } => {
                // Placeholder: render a small rectangle where text would be
                let estimated_width = text.len() as f32 * *font_size * 0.6;
                let placeholder_bounds = Rect::new(position.x, position.y, estimated_width, *font_size);
                let center = cam.rect_center(&placeholder_bounds);

                let sprite = Sprite::new(white_texture)
                    .with_position(center)
                    .with_scale(cam.size(Vec2::new(estimated_width, *font_size)))
                    .with_color(glam::Vec4::new(color.r, color.g, color.b, color.a * 0.3))
                    .with_depth(*depth);

                sprites.add_sprite(&sprite);
            }
            DrawCommand::Circle { center, radius, color, depth } => {
                // Real circle via the sprite pipeline's SDF mask
                let world_center = cam.point(center.x, center.y);

                let sprite = Sprite::new(white_texture)
                    .with_position(world_center)
                    .with_scale(cam.size(Vec2::new(*radius * 2.0, *radius * 2.0)))
                    .with_color(glam::Vec4::new(color.r, color.g, color.b, color.a))
                    .with_depth(*depth)
                    .as_circle();

                sprites.add_sprite(&sprite);
            }
            DrawCommand::Line { start, end, color, width, depth } => {
                // Render line as a thin rotated rectangle
                let dx = end.x - start.x;
                let dy = end.y - start.y;
                let length = (dx * dx + dy * dy).sqrt();
                let angle = dy.atan2(dx);

                let midpoint = cam.point(
                    (start.x + end.x) / 2.0,
                    (start.y + end.y) / 2.0,
                );

                let sprite = Sprite::new(white_texture)
                    .with_position(midpoint)
                    .with_rotation(-angle) // Negate for coordinate system
                    .with_scale(cam.size(Vec2::new(length, *width)))
                    .with_color(glam::Vec4::new(color.r, color.g, color.b, color.a))
                    .with_depth(*depth);

                sprites.add_sprite(&sprite);
            }
            DrawCommand::Image { bounds, texture_id, tint, corner_radius, depth } => {
                // Same path as Rect, but sampling a real texture (the glyph
                // pipeline established this pattern)
                let center = cam.rect_center(bounds);

                let mut sprite = Sprite::new(TextureHandle { id: *texture_id })
                    .with_position(center)
                    .with_scale(cam.size(Vec2::new(bounds.width, bounds.height)))
                    .with_color(glam::Vec4::new(tint.r, tint.g, tint.b, tint.a))
                    .with_depth(*depth);
                if *corner_radius > 0.0 {
                    sprite = sprite.with_corner_radius(cam.len(*corner_radius));
                }

                sprites.add_sprite(&sprite);
            }
            DrawCommand::PushClipRect { bounds } => {
                let effective = if let Some(parent) = clip_stack.last() {
                    intersect_rects(bounds, parent)
                } else {
                    *bounds
                };
                clip_stack.push(effective);
                sprites.set_clip(Some(active_clip_scissor(&clip_stack)));
            }
            DrawCommand::PopClipRect => {
                clip_stack.pop();
                sprites.set_clip(if clip_stack.is_empty() {
                    None
                } else {
                    Some(active_clip_scissor(&clip_stack))
                });
            }
        }
    }
    // Leave the batcher's clip cursor clean even if push/pop were unbalanced
    // this frame (defensive — clear() also resets it).
    sprites.set_clip(None);
}

/// The innermost active clip as a GPU scissor rect. UI coordinates are
/// already physical surface pixels (`window_size` is winit's PhysicalSize),
/// so no DPI conversion applies — the old `logical_to_physical_rect` helper
/// double-scaled on HiDPI and was deleted with issue #41.
fn active_clip_scissor(clip_stack: &[Rect]) -> [u32; 4] {
    // Safe: callers only invoke this with a non-empty stack.
    let r = clip_stack.last().copied().unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
    renderer::scissor::quantize_rect(r.x, r.y, r.width, r.height)
}

/// Intersect two rects, returning the overlapping region.
///
/// Used for nested clip rects - the effective clip is the intersection of all active clips.
pub fn intersect_rects(a: &Rect, b: &Rect) -> Rect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);

    Rect::new(
        x,
        y,
        (right - x).max(0.0),
        (bottom - y).max(0.0),
    )
}

#[cfg(test)]
mod tests;
