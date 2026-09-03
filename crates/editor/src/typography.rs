//! Editor typography tokens.
//!
//! Every piece of editor chrome takes its font size from these tokens instead
//! of hard-coded literals — the guard test below is what keeps 10px labels
//! from creeping back in.

/// The smallest font size any editor text is allowed to use.
pub const MIN_READABLE_FONT: f32 = 12.0;

/// Font-size tokens for the editor UI.
#[derive(Debug, Clone)]
pub struct FontSizes {
    /// Hints, status bar, axis/channel labels, grid label
    pub small: f32,
    /// Field labels, panel titles, menu items, input text
    pub body: f32,
    /// Component headers, section titles
    pub heading: f32,
}

impl Default for FontSizes {
    fn default() -> Self {
        Self {
            small: 12.0,
            body: 14.0,
            heading: 16.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No editor chrome may be set below the readable floor.
    #[test]
    fn test_every_font_token_is_readable() {
        let fonts = FontSizes::default();
        for (name, size) in [("small", fonts.small), ("body", fonts.body), ("heading", fonts.heading)] {
            assert!(size >= MIN_READABLE_FONT, "{name}: {size} is below MIN_READABLE_FONT");
        }
    }
}
