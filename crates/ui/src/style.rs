//! UI styling system with colors and visual properties.

// Re-export Color from common crate
pub use common::Color;

/// Named palette constants shared by the style defaults so each color is
/// defined exactly once per theme.
mod palette {
    pub mod dark {
        pub const SURFACE: u32 = 0x3A3A3A;
        pub const SURFACE_HOVERED: u32 = 0x4A4A4A;
        pub const SURFACE_PRESSED: u32 = 0x2A2A2A;
        pub const BORDER: u32 = 0x5A5A5A;
        pub const BORDER_SUBTLE: u32 = 0x4A4A4A;
        pub const ACCENT: u32 = 0x4A90D9;
    }
}

/// Style configuration for buttons.
#[derive(Debug, Clone)]
pub struct ButtonStyle {
    /// Background color in normal state
    pub background: Color,
    /// Background color when hovered
    pub background_hovered: Color,
    /// Background color when pressed
    pub background_pressed: Color,
    /// Background color when disabled
    pub background_disabled: Color,
    /// Border color
    pub border: Color,
    /// Border width in pixels
    pub border_width: f32,
    /// Corner radius in pixels
    pub corner_radius: f32,
    /// Text color
    pub text_color: Color,
    /// Text color when disabled
    pub text_color_disabled: Color,
    /// Padding inside the button
    pub padding: f32,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        use palette::dark;
        Self {
            background: Color::from_hex(dark::SURFACE),
            background_hovered: Color::from_hex(dark::SURFACE_HOVERED),
            background_pressed: Color::from_hex(dark::SURFACE_PRESSED),
            background_disabled: Color::from_hex(dark::SURFACE_PRESSED),
            border: Color::from_hex(dark::BORDER),
            border_width: 1.0,
            corner_radius: 4.0,
            text_color: Color::WHITE,
            text_color_disabled: Color::GRAY,
            padding: 8.0,
        }
    }
}

/// Style configuration for panels/containers.
#[derive(Debug, Clone)]
pub struct PanelStyle {
    /// Background color
    pub background: Color,
    /// Border color
    pub border: Color,
    /// Border width in pixels
    pub border_width: f32,
    /// Corner radius in pixels
    pub corner_radius: f32,
    /// Padding inside the panel
    pub padding: f32,
}

impl Default for PanelStyle {
    fn default() -> Self {
        use palette::dark;
        Self {
            background: Color::from_hex(dark::SURFACE_PRESSED).with_alpha(0.9),
            border: Color::from_hex(dark::BORDER_SUBTLE),
            border_width: 1.0,
            corner_radius: 4.0,
            padding: 8.0,
        }
    }
}

/// Style configuration for sliders.
#[derive(Debug, Clone)]
pub struct SliderStyle {
    /// Track background color
    pub track_background: Color,
    /// Track fill color (portion before thumb)
    pub track_fill: Color,
    /// Track height in pixels
    pub track_height: f32,
    /// Thumb (handle) color
    pub thumb_color: Color,
    /// Thumb color when hovered
    pub thumb_hovered: Color,
    /// Thumb color when pressed
    pub thumb_pressed: Color,
    /// Thumb radius in pixels
    pub thumb_radius: f32,
}

impl Default for SliderStyle {
    fn default() -> Self {
        use palette::dark;
        Self {
            track_background: Color::from_hex(dark::SURFACE),
            track_fill: Color::from_hex(dark::ACCENT),
            track_height: 6.0,
            thumb_color: Color::WHITE,
            thumb_hovered: Color::LIGHT_GRAY,
            thumb_pressed: Color::from_hex(dark::ACCENT),
            thumb_radius: 8.0,
        }
    }
}

/// Style configuration for text input fields (e.g. `float_input`).
#[derive(Debug, Clone, Copy)]
pub struct TextInputStyle {
    /// Background color in normal state
    pub background: Color,
    /// Background color when hovered or focused
    pub background_focused: Color,
    /// Border color in normal state
    pub border: Color,
    /// Border color when hovered or focused
    pub border_focused: Color,
    /// Border color while the focused edit buffer fails to parse (numeric
    /// inputs) — the "this text is not a number" affordance
    pub border_invalid: Color,
    /// Border width in pixels
    pub border_width: f32,
    /// Corner radius in pixels
    pub corner_radius: f32,
    /// Text color
    pub text_color: Color,
    /// Font size in pixels
    pub font_size: f32,
    /// Horizontal padding between the border and the text
    pub padding: f32,
    /// Selected-text highlight color (drawn behind the text)
    pub selection_color: Color,
    /// Caret (text cursor) color
    pub cursor_color: Color,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        use palette::dark;
        Self {
            background: Color::new(0.15, 0.15, 0.18, 1.0),
            background_focused: Color::new(0.2, 0.2, 0.25, 1.0),
            border: Color::new(0.3, 0.3, 0.35, 1.0),
            border_focused: Color::new(0.4, 0.6, 1.0, 1.0),
            border_invalid: Color::new(1.0, 0.27, 0.27, 1.0),
            border_width: 1.0,
            corner_radius: 2.0,
            text_color: Color::WHITE,
            font_size: 13.0,
            padding: 4.0,
            selection_color: Color::from_hex(dark::ACCENT).with_alpha(0.35),
            cursor_color: Color::WHITE,
        }
    }
}

/// Style configuration for text labels.
#[derive(Debug, Clone)]
pub struct TextStyle {
    /// Text color
    pub color: Color,
    /// Font size in pixels
    pub font_size: f32,
    /// Line height multiplier
    pub line_height: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            font_size: 16.0,
            line_height: 1.2,
        }
    }
}

/// Global UI theme containing all widget styles.
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct Theme {
    /// Button style
    pub button: ButtonStyle,
    /// Panel style
    pub panel: PanelStyle,
    /// Slider style
    pub slider: SliderStyle,
    /// Text style
    pub text: TextStyle,
    /// Text input style
    pub text_input: TextInputStyle,
}


impl Theme {
    /// Create a dark theme (default).
    pub fn dark() -> Self {
        Self::default()
    }
}
