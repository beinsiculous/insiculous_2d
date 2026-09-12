//! Hex text for RGBA colours — the colour editor's text field and the
//! read-only colour row read and write the same spelling.

use glam::Vec4;

/// `#rrggbb` for an opaque colour, `#rrggbbaa` when it carries alpha.
/// Channels outside `0..=1` clamp: the editor's channel fields are hard
/// clamped, and a scene file may still carry anything.
pub fn to_hex(color: Vec4) -> String {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    let (red, green, blue, alpha) = (
        channel(color.x),
        channel(color.y),
        channel(color.z),
        channel(color.w),
    );
    if alpha == 255 {
        format!("#{red:02x}{green:02x}{blue:02x}")
    } else {
        format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}")
    }
}

/// Parse `#rrggbb` or `#rrggbbaa` (the leading `#` optional, case
/// insensitive). A six-digit value keeps the alpha it is given, so typing
/// a colour into a translucent swatch does not silently make it opaque.
pub fn from_hex(text: &str, current_alpha: f32) -> Option<Vec4> {
    let digits = text.trim().trim_start_matches('#');
    if digits.len() != 6 && digits.len() != 8 {
        return None;
    }
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |index: usize| {
        u8::from_str_radix(&digits[index * 2..index * 2 + 2], 16)
            .ok()
            .map(|byte| byte as f32 / 255.0)
    };
    let alpha = if digits.len() == 8 {
        channel(3)?
    } else {
        current_alpha
    };
    Some(Vec4::new(channel(0)?, channel(1)?, channel(2)?, alpha))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_round_trips_every_channel_and_keeps_alpha_a_six_digit_value_omits() {
        let opaque = Vec4::new(1.0, 0.5, 0.0, 1.0);
        assert_eq!(to_hex(opaque), "#ff8000");
        let translucent = Vec4::new(0.0, 0.0, 1.0, 0.5);
        assert_eq!(to_hex(translucent), "#0000ff80");
        for text in ["#ff8000", "FF8000", " #Ff8000 "] {
            let parsed = from_hex(text, 1.0).expect("valid hex");
            assert!((parsed.x - 1.0).abs() < 1e-6);
            assert!((parsed.y - 128.0 / 255.0).abs() < 1e-6);
            assert_eq!(parsed.z, 0.0);
            assert_eq!(parsed.w, 1.0, "six digits keep the colour's own alpha");
        }
        assert_eq!(from_hex("#0000ff80", 1.0).expect("eight digits").w, 128.0 / 255.0);
        assert_eq!(from_hex("", 1.0), None);
        assert_eq!(from_hex("#12345", 1.0), None, "five digits is not a colour");
        assert_eq!(from_hex("#gggggg", 1.0), None);
    }

    #[test]
    fn test_out_of_range_channels_clamp_instead_of_wrapping() {
        // A scene file can carry an HDR-ish value; the swatch must still
        // spell a colour rather than wrap to a dark one.
        assert_eq!(to_hex(Vec4::new(2.0, -1.0, 0.0, 1.0)), "#ff0000");
    }
}
