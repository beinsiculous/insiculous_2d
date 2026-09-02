//! 2D sprite rendering: sprite data, CPU batching, and the GPU pipeline.
//!
//! - [`Sprite`] (this module) — what a game asks to draw
//! - [`SpriteBatch`] / [`SpriteBatcher`] — group sprites by texture before upload
//! - [`SpritePipeline`] — instanced GPU rendering into the HDR target

use glam::{Vec2, Vec4};

use crate::sprite_data::SpriteInstance;
use crate::texture::TextureHandle;

mod batch;
#[cfg(test)]
mod fixtures;
mod instance_cache;
mod pipeline;

pub use batch::{SpriteBatch, SpriteBatcher};
pub use instance_cache::InstanceCache;
pub use pipeline::SpritePipeline;

/// A single sprite to be rendered
#[derive(Debug, Clone)]
pub struct Sprite {
    /// Position in world space
    pub position: Vec2,
    /// Rotation in radians
    pub rotation: f32,
    /// Scale
    pub scale: Vec2,
    /// Texture region (x, y, width, height) in texture coordinates [0, 1]
    pub tex_region: [f32; 4],
    /// Color tint
    pub color: Vec4,
    /// Layer depth for sorting (higher values render on top)
    pub depth: f32,
    /// Emissive intensity — 0.0 disables glow, values above 0.0 produce HDR output that bloom picks up
    pub emissive: f32,
    /// SDF shape params `[kind, corner_radius, border_width, reserved]`.
    /// kind 0 = plain quad, 1 = rounded rect, 2 = circle (see [`SpriteShape`]).
    pub shape: [f32; 4],
    /// Texture handle
    pub texture_handle: TextureHandle,
}

/// SDF shape kinds a sprite can render as (fragment-shader masked).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteShape {
    /// Plain textured quad (the default)
    Quad,
    /// Rectangle with rounded corners (radius in local pixels)
    RoundedRect,
    /// Circle/ellipse inscribed in the sprite bounds
    Circle,
}

impl SpriteShape {
    /// Shape kind as the f32 the instance data carries.
    pub fn to_f32(self) -> f32 {
        match self {
            SpriteShape::Quad => 0.0,
            SpriteShape::RoundedRect => 1.0,
            SpriteShape::Circle => 2.0,
        }
    }
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            tex_region: [0.0, 0.0, 1.0, 1.0], // Full texture
            color: Vec4::ONE, // White
            depth: 0.0,
            emissive: 0.0,
            shape: [0.0; 4],
            texture_handle: TextureHandle::default(),
        }
    }
}

impl Sprite {
    /// Create a new sprite
    pub fn new(texture_handle: TextureHandle) -> Self {
        Self {
            texture_handle,
            ..Default::default()
        }
    }

    /// Set sprite position
    pub fn with_position(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    /// Set sprite rotation
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set sprite scale
    pub fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    /// Set texture region (UV coordinates)
    pub fn with_tex_region(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.tex_region = [x, y, width, height];
        self
    }

    /// Set color tint
    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }

    /// Set depth
    pub fn with_depth(mut self, depth: f32) -> Self {
        self.depth = depth;
        self
    }

    /// Set emissive intensity (0.0 disables glow, larger values bloom more strongly)
    pub fn with_emissive(mut self, emissive: f32) -> Self {
        self.emissive = emissive;
        self
    }

    /// Render as a rounded rect with the given corner radius (local pixels).
    /// Radius 0 keeps sharp corners but still applies border/AA masking.
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.shape[0] = SpriteShape::RoundedRect.to_f32();
        self.shape[1] = radius.max(0.0);
        self
    }

    /// Render as a circle inscribed in the sprite bounds.
    pub fn as_circle(mut self) -> Self {
        self.shape[0] = SpriteShape::Circle.to_f32();
        self
    }

    /// Render only a border ring of the given width (local pixels) instead
    /// of a filled shape. Applies to rounded rects and circles; a plain quad
    /// is promoted to a rounded rect with radius 0 (a square outline).
    pub fn with_border(mut self, width: f32) -> Self {
        if self.shape[0] == SpriteShape::Quad.to_f32() {
            self.shape[0] = SpriteShape::RoundedRect.to_f32();
        }
        self.shape[2] = width.max(0.0);
        self
    }

    /// Convert to sprite instance for batching
    pub fn to_instance(&self) -> SpriteInstance {
        SpriteInstance::with_emissive(
            self.position,
            self.rotation,
            self.scale,
            self.tex_region,
            self.color,
            self.depth,
            self.emissive,
        )
        .with_shape(self.shape)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shape kind 0 with no emissive is what the shader treats as "no SDF
    /// mask, no glow" — the legacy textured quad every scene sprite starts as.
    #[test]
    fn test_default_sprite_renders_as_a_plain_unlit_textured_quad() {
        let instance = Sprite::default().to_instance();

        assert_eq!(instance.shape, [0.0; 4], "zeroed shape == legacy textured quad");
        assert_eq!(instance.shape[0], SpriteShape::Quad.to_f32());
        assert_eq!(instance.emissive, 0.0, "no glow unless asked for");
        assert_eq!(instance.tex_region, [0.0, 0.0, 1.0, 1.0], "the whole texture");
        assert_eq!(instance.color, [1.0; 4], "untinted");
    }

    /// `SpriteAnimationSystem` writes `tex_region` every frame and the SDF
    /// builders write `shape`; `to_instance` is the only place either
    /// reaches the GPU.
    #[test]
    fn test_every_sprite_field_lands_on_the_gpu_instance() {
        let sprite = Sprite::new(TextureHandle::WHITE)
            .with_position(Vec2::new(100.0, 200.0))
            .with_rotation(0.5)
            .with_scale(Vec2::new(2.0, 3.0))
            .with_tex_region(0.25, 0.5, 0.25, 0.5)
            .with_color(Vec4::new(1.0, 0.5, 0.25, 1.0))
            .with_depth(5.0)
            .with_emissive(1.5)
            .with_corner_radius(8.0)
            .with_border(2.0);

        let instance = sprite.to_instance();

        assert_eq!(instance.position, [100.0, 200.0]);
        assert_eq!(instance.rotation, 0.5);
        assert_eq!(instance.scale, [2.0, 3.0]);
        assert_eq!(instance.tex_region, [0.25, 0.5, 0.25, 0.5]);
        assert_eq!(instance.color, [1.0, 0.5, 0.25, 1.0]);
        assert_eq!(instance.depth, 5.0);
        assert_eq!(instance.emissive, 1.5);
        assert_eq!(instance.shape, [SpriteShape::RoundedRect.to_f32(), 8.0, 2.0, 0.0]);
    }
}
