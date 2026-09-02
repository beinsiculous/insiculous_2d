//! Unified color type for the engine.

use glam::Vec4;
use serde::{Deserialize, Serialize};

/// RGBA color representation.
///
/// All components are in the range 0.0 to 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    /// Red component (0.0 - 1.0)
    pub r: f32,
    /// Green component (0.0 - 1.0)
    pub g: f32,
    /// Blue component (0.0 - 1.0)
    pub b: f32,
    /// Alpha component (0.0 - 1.0)
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl Color {
    // Common color constants
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const RED: Color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Color = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Color = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const YELLOW: Color = Color { r: 1.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const CYAN: Color = Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const MAGENTA: Color = Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const GRAY: Color = Color { r: 0.5, g: 0.5, b: 0.5, a: 1.0 };
    pub const DARK_GRAY: Color = Color { r: 0.25, g: 0.25, b: 0.25, a: 1.0 };
    pub const LIGHT_GRAY: Color = Color { r: 0.75, g: 0.75, b: 0.75, a: 1.0 };

    /// Create a new color from RGBA components (0.0 - 1.0).
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create a color from RGB components with full opacity.
    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create a color from 8-bit RGB values (0-255).
    #[inline]
    pub fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    /// Create a color from 8-bit RGBA values (0-255).
    #[inline]
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Create a color from a hex value (0xRRGGBB).
    #[inline]
    pub fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as u8;
        let g = ((hex >> 8) & 0xFF) as u8;
        let b = (hex & 0xFF) as u8;
        Self::from_rgb8(r, g, b)
    }

    /// Create a color from a hex value with alpha (0xRRGGBBAA).
    #[inline]
    pub fn from_hex_rgba(hex: u32) -> Self {
        let r = ((hex >> 24) & 0xFF) as u8;
        let g = ((hex >> 16) & 0xFF) as u8;
        let b = ((hex >> 8) & 0xFF) as u8;
        let a = (hex & 0xFF) as u8;
        Self::from_rgba8(r, g, b, a)
    }

    /// Create a color with modified alpha.
    #[inline]
    pub fn with_alpha(self, alpha: f32) -> Self {
        Self { a: alpha, ..self }
    }

    /// Linearly interpolate between two colors.
    #[inline]
    pub fn lerp(self, other: Color, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    /// Darken the color by a factor (0.0 = black, 1.0 = unchanged).
    #[inline]
    pub fn darken(self, factor: f32) -> Self {
        Self {
            r: self.r * factor,
            g: self.g * factor,
            b: self.b * factor,
            a: self.a,
        }
    }

    /// Lighten the color by a factor (0.0 = unchanged, 1.0 = white).
    #[inline]
    pub fn lighten(self, factor: f32) -> Self {
        Self {
            r: self.r + (1.0 - self.r) * factor,
            g: self.g + (1.0 - self.g) * factor,
            b: self.b + (1.0 - self.b) * factor,
            a: self.a,
        }
    }

    /// Convert to Vec4 for GPU/rendering use.
    #[inline]
    pub fn to_vec4(self) -> Vec4 {
        Vec4::new(self.r, self.g, self.b, self.a)
    }

    /// Convert to array for GPU buffers.
    #[inline]
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Convert to 8-bit RGBA array.
    #[inline]
    pub fn to_rgba8(self) -> [u8; 4] {
        [
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        ]
    }

    /// WCAG relative luminance in `[0, 1]`: linearizes the sRGB-authored
    /// channels (piecewise transfer, not `pow(2.2)`) and weights them.
    /// Alpha is ignored.
    pub fn luminance(self) -> f32 {
        fn lin(c: f32) -> f32 {
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        }
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    /// WCAG contrast ratio between two colors, symmetric, in `[1, 21]`.
    /// The theme's surface-ladder guard tests are built on this.
    pub fn contrast_ratio(self, other: Color) -> f32 {
        let (a, b) = (self.luminance(), other.luminance());
        let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
        (hi + 0.05) / (lo + 0.05)
    }
}

// Conversions to/from glam types
impl From<Color> for Vec4 {
    #[inline]
    fn from(color: Color) -> Self {
        color.to_vec4()
    }
}

impl From<Vec4> for Color {
    #[inline]
    fn from(v: Vec4) -> Self {
        Self::new(v.x, v.y, v.z, v.w)
    }
}

impl From<Color> for [f32; 4] {
    #[inline]
    fn from(color: Color) -> Self {
        color.to_array()
    }
}

impl From<[f32; 4]> for Color {
    #[inline]
    fn from(arr: [f32; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_hex_unpacks_rrggbb_into_the_same_opaque_channels_as_rgb8() {
        // The editor theme and every 0xRRGGBB literal in the games decode
        // through from_hex; a dropped channel (alpha especially) would
        // silently tint or fade every one of them.
        let cases = [
            ("from_hex(0xFF8000)", Color::from_hex(0xFF8000), [1.0, 128.0 / 255.0, 0.0, 1.0]),
            ("from_rgb8(255, 128, 0)", Color::from_rgb8(255, 128, 0), [1.0, 128.0 / 255.0, 0.0, 1.0]),
            ("from_hex(0x000000)", Color::from_hex(0x000000), [0.0, 0.0, 0.0, 1.0]),
            ("from_hex(0xFFFFFF)", Color::from_hex(0xFFFFFF), [1.0, 1.0, 1.0, 1.0]),
            ("from_hex(0x0000FF)", Color::from_hex(0x0000FF), [0.0, 0.0, 1.0, 1.0]),
        ];

        for (label, color, expected) in cases {
            let actual = color.to_array();
            let channels_match = actual
                .iter()
                .zip(expected.iter())
                .all(|(a, e)| (a - e).abs() < 0.001);
            assert!(channels_match, "{label}: expected {expected:?}, got {actual:?}");
        }
    }

    #[test]
    fn test_contrast_ratio_is_symmetric_and_spans_one_to_twenty_one() {
        let dark = Color::from_hex(0x1e1e1e);
        let darker = Color::from_hex(0x333333);

        let white_on_black = Color::WHITE.contrast_ratio(Color::BLACK);
        let dark_on_darker = dark.contrast_ratio(darker);
        let darker_on_dark = darker.contrast_ratio(dark);
        let self_contrast = dark.contrast_ratio(dark);

        assert!(
            (white_on_black - 21.0).abs() < 0.01,
            "white on black is WCAG's maximum 21:1, got {white_on_black}"
        );
        assert_eq!(dark_on_darker, darker_on_dark, "contrast must not depend on argument order");
        assert_eq!(self_contrast, 1.0, "a color against itself is WCAG's minimum 1:1");
    }

    #[test]
    fn test_luminance_linearizes_srgb_mid_gray_to_the_wcag_reference() {
        // sRGB #808080 has WCAG relative luminance ≈ 0.2159; a pow(2.2)
        // shortcut or a missing linearization lands nowhere near it.
        let mid_gray = Color::from_hex(0x808080);

        let luminance = mid_gray.luminance();

        assert!(
            (luminance - 0.2159).abs() < 0.002,
            "expected ≈ 0.2159 for #808080, got {luminance}"
        );
    }

    #[test]
    fn test_all_four_channels_survive_vec4_and_array_conversions() {
        // scene_serializer writes sprite colors through `color.into()`; a
        // dropped or reordered channel would corrupt every saved scene.
        let color = Color::new(0.1, 0.2, 0.3, 0.4);

        let as_vec4: Vec4 = color.into();
        let as_array: [f32; 4] = color.into();
        let from_vec4: Color = as_vec4.into();
        let from_array: Color = as_array.into();

        assert_eq!(as_vec4, Vec4::new(0.1, 0.2, 0.3, 0.4));
        assert_eq!(as_array, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(from_vec4, color, "Color -> Vec4 -> Color must be lossless");
        assert_eq!(from_array, color, "Color -> [f32; 4] -> Color must be lossless");
    }
}
