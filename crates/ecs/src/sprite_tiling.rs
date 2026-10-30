//! Native-size patches for sprites with fixed borders and repeating interiors.

use glam::{Vec2, Vec4};
use serde::{Deserialize, Serialize};

use crate::{ComponentMeta, DeriveComponentMeta};

/// Repeat a sprite's interior instead of stretching its pixels when its draw size changes.
/// Coordinates describe the current animation frame, not the whole texture atlas.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, DeriveComponentMeta)]
pub struct SpriteTiling {
    /// The current source frame's width and height in pixels.
    pub source_size: Vec2,
    /// Fixed border widths in source pixels, ordered left, top, right, bottom.
    pub borders: Vec4,
}

impl Default for SpriteTiling {
    fn default() -> Self {
        Self { source_size: Vec2::splat(16.0), borders: Vec4::ZERO }
    }
}

/// One unscaled source rectangle and its placement relative to the sprite centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteTile {
    /// Patch centre in world pixels, before the sprite's rotation; Y points up.
    pub offset: Vec2,
    /// Patch dimensions in pixels, equal to the sampled source dimensions.
    pub size: Vec2,
    /// Normalized texture coordinates within the active animation frame.
    pub tex_region: [f32; 4],
}

/// A corrupt size or subpixel repeat must not monopolize a render frame.
const MAX_PATCHES: usize = 65_536;
/// Beyond this size, f32 cannot distinguish adjacent whole source pixels.
const MAX_SOURCE_PIXELS: f32 = 16_777_216.0;

impl SpriteTiling {
    /// Maximum patches emitted for one sprite; 16-pixel tiles cover a 4K backdrop.
    pub const MAX_PATCHES: usize = MAX_PATCHES;

    /// Whether the source leaves at least one pixel to repeat on each axis.
    pub fn is_valid(&self) -> bool {
        self.source_size.is_finite()
            && self.borders.is_finite()
            && self.source_size.cmpge(Vec2::ONE).all()
            && self.source_size.cmple(Vec2::splat(MAX_SOURCE_PIXELS)).all()
            && self.borders.cmpge(Vec4::ZERO).all()
            && self.borders.x + self.borders.z <= self.source_size.x - 1.0
            && self.borders.y + self.borders.w <= self.source_size.y - 1.0
    }

    /// Clamp editable source dimensions and margins to the rendering contract.
    pub fn normalize(&mut self) {
        for axis in 0..2 {
            self.source_size[axis] = if self.source_size[axis].is_finite() {
                self.source_size[axis].clamp(1.0, MAX_SOURCE_PIXELS)
            } else {
                Self::default().source_size[axis]
            };
        }
        for side in 0..4 {
            self.borders[side] = if self.borders[side].is_finite() { self.borders[side].max(0.0) } else { 0.0 };
        }
        self.borders.x = self.borders.x.min(self.source_size.x - 1.0);
        self.borders.y = self.borders.y.min(self.source_size.y - 1.0);
        self.borders.z = self.borders.z.min(self.source_size.x - self.borders.x - 1.0);
        self.borders.w = self.borders.w.min(self.source_size.y - self.borders.y - 1.0);
    }

    /// Number of patches for a valid bounded layout, or zero when it cannot render.
    pub fn patch_count(&self, destination_size: Vec2, region: [f32; 4]) -> usize {
        self.layout(destination_size, region)
            .map_or(0, |(horizontal, vertical)| horizontal.count() * vertical.count())
    }

    /// Tile a destination rectangle using the sprite's active atlas region.
    /// Undersized destinations crop the inner sides of their borders. Invalid input or
    /// more than 65536 patches emits nothing; pixels are never stretched as a fallback.
    pub fn tiles(&self, destination_size: Vec2, region: [f32; 4]) -> impl Iterator<Item = SpriteTile> {
        let layout = self.layout(destination_size, region);
        layout.into_iter().flat_map(move |(horizontal, vertical)| {
            (0..vertical.count()).flat_map(move |row| {
                let vertical_segment = vertical.segment(row);
                (0..horizontal.count()).map(move |column| {
                    let horizontal_segment = horizontal.segment(column);
                    let size = Vec2::new(horizontal_segment.size, vertical_segment.size);
                    SpriteTile {
                        offset: Vec2::new(
                            horizontal_segment.destination + size.x / 2.0 - destination_size.x / 2.0,
                            destination_size.y / 2.0 - vertical_segment.destination - size.y / 2.0,
                        ),
                        size,
                        tex_region: [
                            region[0] + horizontal_segment.source / horizontal.source_size * region[2],
                            region[1] + vertical_segment.source / vertical.source_size * region[3],
                            size.x / horizontal.source_size * region[2],
                            size.y / vertical.source_size * region[3],
                        ],
                    }
                })
            })
        })
    }

    fn layout(&self, destination: Vec2, region: [f32; 4]) -> Option<(AxisLayout, AxisLayout)> {
        if !self.is_valid() || !destination.is_finite() || !destination.cmpgt(Vec2::ZERO).all()
            || !region.into_iter().all(f32::is_finite) || region[2] <= 0.0 || region[3] <= 0.0
        {
            return None;
        }
        let horizontal = AxisLayout::new(self.source_size.x, destination.x, self.borders.x, self.borders.z)?;
        let vertical = AxisLayout::new(self.source_size.y, destination.y, self.borders.y, self.borders.w)?;
        (horizontal.count().checked_mul(vertical.count())? <= MAX_PATCHES).then_some((horizontal, vertical))
    }
}

#[derive(Clone, Copy)]
struct AxisLayout {
    source_size: f32,
    destination_size: f32,
    source_leading: f32,
    leading: f32,
    trailing: f32,
    middle_size: f32,
    repeats: usize,
}

struct AxisSegment {
    source: f32,
    destination: f32,
    size: f32,
}

impl AxisLayout {
    fn new(source_size: f32, destination_size: f32, leading: f32, trailing: f32) -> Option<Self> {
        let border_size = leading + trailing;
        let available_fraction = if border_size > destination_size { destination_size / border_size } else { 1.0 };
        let destination_leading = leading * available_fraction;
        let destination_trailing = trailing * available_fraction;
        let middle_size = source_size - border_size;
        let destination_middle = (destination_size - destination_leading - destination_trailing).max(0.0);
        let repeats = (destination_middle / middle_size).ceil() as usize;
        (repeats <= MAX_PATCHES).then_some(Self {
            source_size, destination_size, source_leading: leading,
            leading: destination_leading, trailing: destination_trailing, middle_size, repeats,
        })
    }

    fn count(self) -> usize {
        usize::from(self.leading > 0.0) + self.repeats + usize::from(self.trailing > 0.0)
    }

    fn segment(self, index: usize) -> AxisSegment {
        if self.leading > 0.0 && index == 0 {
            return AxisSegment { source: 0.0, destination: 0.0, size: self.leading };
        }
        let repeat_index = index - usize::from(self.leading > 0.0);
        if repeat_index < self.repeats {
            let destination = self.leading + repeat_index as f32 * self.middle_size;
            return AxisSegment {
                source: self.source_leading,
                destination,
                size: self.middle_size.min(self.destination_size - self.trailing - destination),
            };
        }
        AxisSegment {
            source: self.source_size - self.trailing,
            destination: self.destination_size - self.trailing,
            size: self.trailing,
        }
    }
}

#[cfg(test)]
mod tests;
