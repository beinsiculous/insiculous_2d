//! Centralized editor theme with design system color tokens.
//!
//! All editor colors and visual constants are defined here so that a single
//! change propagates across the entire editor. Derived from the target
//! mockup (`crates/editor/IdealEditor.png`).
//!
//! # Usage
//! ```
//! use editor::EditorTheme;
//!
//! let theme = EditorTheme::default();
//!
//! // Reference color tokens directly when drawing panel chrome...
//! let header_color = theme.accent_cyan;
//! let panel_bg = theme.bg_primary;
//!
//! // ...and use the converter methods for subsystem style bundles.
//! let inspector_style = theme.inspector_style();
//! let grid_colors = theme.grid_colors();
//! # let _ = (header_color, panel_bg, inspector_style, grid_colors);
//! ```

use glam::Vec4;
use ui::Color;

/// Centralized design-system theme for the entire editor.
///
/// Every color used in panels, toolbars, inspector, hierarchy, status bar,
/// gizmos, and grid should reference a field on this struct — never a
/// hardcoded literal.
#[derive(Debug, Clone)]
pub struct EditorTheme {
    // ── Backgrounds ─────────────────────────────────────────────
    /// Main panel backgrounds (`#1e1e1e`)
    pub bg_primary: Color,
    /// Viewport / canvas area (`#000000`)
    pub bg_viewport: Color,
    /// Input fields, dropdowns (`#2d2d2d`)
    pub bg_input: Color,
    /// Panel header background (darker than bg_primary)
    pub bg_header: Color,

    // ── Accents ─────────────────────────────────────────────────
    /// Selection highlights, active buttons, "+ Add Component" (`#0078d4`)
    pub accent_blue: Color,
    /// Panel headers, interactive highlights, gizmo labels (`#00d9ff`)
    pub accent_cyan: Color,

    // ── Borders ─────────────────────────────────────────────────
    /// Panel borders — bright blue (`#007acc`)
    pub border_panel: Color,
    /// Grid lines, separators (`#333333`)
    pub border_subtle: Color,

    // ── Text ────────────────────────────────────────────────────
    /// Primary text (`#ffffff`)
    pub text_primary: Color,
    /// Secondary text, labels (`#cccccc`)
    pub text_secondary: Color,
    /// Disabled text, placeholders (`#888888`)
    pub text_muted: Color,

    // ── Gizmos ──────────────────────────────────────────────────
    /// X-axis gizmo color (green, horizontal) (`#00ff00`)
    pub gizmo_x: Color,
    /// Y-axis gizmo color (red, vertical) (`#ff0000`)
    pub gizmo_y: Color,
    /// Center/free-move gizmo handle
    pub gizmo_center: Color,
    /// X-axis handle while hovered/dragged
    pub gizmo_x_hover: Color,
    /// Y-axis handle while hovered/dragged
    pub gizmo_y_hover: Color,
    /// Center handle while hovered/dragged
    pub gizmo_center_hover: Color,
    /// Rotation ring
    pub gizmo_ring: Color,
    /// Current-rotation indicator line
    pub gizmo_rotation_indicator: Color,
    /// Scale gizmo box outline
    pub gizmo_scale_outline: Color,
    /// Scale gizmo corner handles
    pub gizmo_scale_handle: Color,
    /// Scale gizmo corner handles while hovered/dragged
    pub gizmo_scale_handle_hover: Color,

    // ── Selection / rows ────────────────────────────────────────
    /// Selected row background (hierarchy, lists)
    pub selection_fill: Color,
    /// Hovered row background (hierarchy, lists)
    pub hover_fill: Color,
    /// Active tool button background (toolbar)
    pub toolbar_active: Color,

    // ── Menu ────────────────────────────────────────────────────
    /// Background highlight behind an open menu title
    pub menu_open_highlight: Color,
    /// Separator lines inside menu dropdowns
    pub menu_separator: Color,
    /// Keyboard shortcut hint text (menus, toolbar)
    pub shortcut_hint: Color,

    // ── Inspector field labels ──────────────────────────────────
    /// "X" axis label in Vec2 fields
    pub axis_x_label: Color,
    /// "Y" axis label in Vec2 fields
    pub axis_y_label: Color,
    /// "R", "G", "B", "A" channel labels in color fields
    pub channel_labels: [Color; 4],

    // ── Play state ──────────────────────────────────────────────
    /// Play button / playing border tint (`#00cc44`)
    pub play_green: Color,
    /// Pause border tint (`#ffcc00`)
    pub pause_yellow: Color,
    /// Stop button (`#cc3333`)
    pub stop_red: Color,

    // ── Semantic ────────────────────────────────────────────────
    /// Error logs, validation (`#ff4444`)
    pub error_red: Color,
    /// Warning logs (`#ffcc00`)
    pub warn_yellow: Color,

    // ── Play control button backgrounds ─────────────────────────
    /// Dark green tint behind play/resume button
    pub play_button_bg: Color,
    /// Dark red tint behind stop button
    pub stop_button_bg: Color,

    // ── Separator ───────────────────────────────────────────────
    /// Thin separator lines between toolbar sections
    pub separator: Color,

    // ── Grid (Vec4 for renderer sprite pipeline) ────────────────
    /// Primary grid line color
    pub grid_primary: Vec4,
    /// Secondary (subdivision) grid line color
    pub grid_secondary: Vec4,
    /// Grid X-axis color (red)
    pub grid_axis_x: Vec4,
    /// Grid Y-axis color (green)
    pub grid_axis_y: Vec4,

    // ── Status bar ──────────────────────────────────────────────
    /// Status bar background (slightly darker than panels)
    pub status_bar_bg: Color,

    // ── Inspector ───────────────────────────────────────────────
    /// Inspector label color (field names)
    pub inspector_label: Color,
    /// Inspector value color (field values)
    pub inspector_value: Color,
    /// Inspector section header color
    pub inspector_header: Color,

    // ── Play-state viewport borders ─────────────────────────────
    /// Viewport border tint while editing
    pub border_editing: Color,
    /// Viewport border tint while playing
    pub border_playing: Color,
    /// Viewport border tint while paused
    pub border_paused: Color,

    // ── Collider overlay ────────────────────────────────────────
    /// Solid (non-sensor) collider outlines in the scene view
    pub collider_outline: Color,
    /// Sensor (trigger-only) collider outlines
    pub collider_sensor: Color,
    /// Collider outline on selected entities
    pub collider_selected: Color,

    // ── Selection outline ───────────────────────────────────────
    /// Viewport outline of selected entities (primary; secondary and hover
    /// derive from it in `selection_outline_colors()`)
    pub selection_outline: Color,

    // ── Typography ──────────────────────────────────────────────
    /// Font-size tokens — all editor text sizes come from here
    pub fonts: crate::typography::FontSizes,
}

impl Default for EditorTheme {
    fn default() -> Self {
        Self {
            // Typography
            fonts: crate::typography::FontSizes::default(),

            // Backgrounds
            bg_primary: Color::from_hex(0x1e1e1e),
            bg_viewport: Color::BLACK,
            bg_input: Color::from_hex(0x2d2d2d),
            bg_header: Color::new(0.12, 0.12, 0.12, 1.0),

            // Accents
            accent_blue: Color::from_hex(0x0078d4),
            accent_cyan: Color::from_hex(0x00d9ff),

            // Borders
            border_panel: Color::from_hex(0x007acc),
            border_subtle: Color::from_hex(0x333333),

            // Text
            text_primary: Color::WHITE,
            text_secondary: Color::from_hex(0xcccccc),
            text_muted: Color::from_hex(0x888888),

            // Gizmos
            gizmo_x: Color::new(0.0, 1.0, 0.0, 1.0),
            gizmo_y: Color::new(1.0, 0.0, 0.0, 1.0),
            gizmo_center: Color::new(0.9, 0.9, 0.2, 1.0),
            gizmo_x_hover: Color::new(1.0, 0.4, 0.4, 1.0),
            gizmo_y_hover: Color::new(0.4, 1.0, 0.4, 1.0),
            gizmo_center_hover: Color::new(1.0, 1.0, 0.4, 1.0),
            gizmo_ring: Color::new(0.3, 0.3, 0.9, 1.0),
            gizmo_rotation_indicator: Color::new(0.9, 0.9, 0.9, 1.0),
            gizmo_scale_outline: Color::new(0.6, 0.6, 0.6, 1.0),
            gizmo_scale_handle: Color::new(0.7, 0.7, 0.7, 1.0),
            gizmo_scale_handle_hover: Color::new(0.9, 0.9, 0.4, 1.0),

            // Selection / rows
            selection_fill: Color::new(0.3, 0.5, 0.8, 0.5),
            hover_fill: Color::new(0.5, 0.5, 0.5, 0.2),
            toolbar_active: Color::new(0.3, 0.5, 0.8, 1.0),

            // Menu
            menu_open_highlight: Color::new(0.2, 0.2, 0.2, 1.0),
            menu_separator: Color::new(0.3, 0.3, 0.3, 1.0),
            shortcut_hint: Color::new(0.5, 0.5, 0.5, 1.0),

            // Inspector field labels
            axis_x_label: Color::new(0.8, 0.4, 0.4, 1.0),
            axis_y_label: Color::new(0.4, 0.8, 0.4, 1.0),
            channel_labels: [
                Color::new(0.9, 0.4, 0.4, 1.0), // R
                Color::new(0.4, 0.9, 0.4, 1.0), // G
                Color::new(0.4, 0.4, 0.9, 1.0), // B
                Color::new(0.7, 0.7, 0.7, 1.0), // A
            ],

            // Play state
            play_green: Color::from_hex(0x00cc44),
            pause_yellow: Color::from_hex(0xffcc00),
            stop_red: Color::from_hex(0xcc3333),

            // Semantic
            error_red: Color::from_hex(0xff4444),
            warn_yellow: Color::from_hex(0xffcc00),

            // Play control button backgrounds
            play_button_bg: Color::new(0.15, 0.35, 0.15, 1.0),
            stop_button_bg: Color::new(0.4, 0.15, 0.15, 1.0),

            // Separator
            separator: Color::new(0.4, 0.4, 0.4, 0.6),

            // Grid (Vec4 for sprite renderer)
            grid_primary: Vec4::new(0.3, 0.3, 0.3, 0.5),
            grid_secondary: Vec4::new(0.25, 0.25, 0.25, 0.3),
            grid_axis_x: Vec4::new(0.8, 0.2, 0.2, 0.8),
            grid_axis_y: Vec4::new(0.2, 0.8, 0.2, 0.8),

            // Status bar
            status_bar_bg: Color::new(0.10, 0.10, 0.10, 1.0),

            // Inspector
            inspector_label: Color::from_hex(0xcccccc),
            inspector_value: Color::WHITE,
            inspector_header: Color::from_hex(0x00d9ff),

            // Play-state viewport borders
            border_editing: Color::new(0.0, 0.48, 0.83, 0.5),
            border_playing: Color::new(0.0, 0.8, 0.27, 0.8),
            border_paused: Color::new(1.0, 0.8, 0.0, 0.8),

            // Collider overlay
            collider_outline: Color::new(0.2, 1.0, 0.4, 0.9),
            collider_sensor: Color::new(0.2, 0.85, 1.0, 0.9),
            collider_selected: Color::new(1.0, 0.85, 0.2, 1.0),

            // Selection outline — orange: distinct from the collider overlay's
            // yellow/green and the grid's cyan
            selection_outline: Color::new(1.0, 0.55, 0.15, 1.0),
        }
    }
}

impl EditorTheme {
    /// Create the default dark editor theme (matches the design mockup).
    pub fn dark() -> Self {
        Self::default()
    }

    /// Convert a theme `Color` to a `Vec4` for the renderer sprite pipeline.
    pub fn color_to_vec4(color: Color) -> Vec4 {
        Vec4::new(color.r, color.g, color.b, color.a)
    }

    /// Create `GridColors` from this theme.
    pub fn grid_colors(&self) -> crate::GridColors {
        crate::GridColors {
            primary: self.grid_primary,
            secondary: self.grid_secondary,
            axis_x: self.grid_axis_x,
            axis_y: self.grid_axis_y,
        }
    }

    /// Create `InspectorStyle` from this theme.
    pub fn inspector_style(&self) -> crate::InspectorStyle {
        crate::InspectorStyle {
            label_color: self.inspector_label,
            value_color: self.inspector_value,
            header_color: self.inspector_header,
            ..Default::default()
        }
    }

    /// Create `EditableFieldStyle` from this theme.
    pub fn editable_field_style(&self) -> crate::EditableFieldStyle {
        crate::EditableFieldStyle {
            label_color: self.inspector_label,
            value_color: self.inspector_value,
            header_color: self.inspector_header,
            axis_x_label: self.axis_x_label,
            axis_y_label: self.axis_y_label,
            channel_labels: self.channel_labels,
            slot_bg: self.bg_input,
            drop_highlight: self.accent_blue,
            label_font: self.fonts.body,
            header_font: self.fonts.heading,
            axis_font: self.fonts.small,
            channel_font: self.fonts.small,
            ..Default::default()
        }
    }

    /// Derive a `ui::Theme` from this editor theme so generic widgets
    /// (buttons, sliders, inputs) match the editor's palette instead of the
    /// ui crate's stock blue. Injected once at editor startup via
    /// `UIContext::set_theme`.
    pub fn ui_theme(&self) -> ui::Theme {
        let mut theme = ui::Theme::dark();

        theme.button.background = self.bg_input;
        theme.button.background_hovered = self.bg_input.lighten(0.12);
        theme.button.background_pressed = self.bg_input.darken(0.25);
        theme.button.background_disabled = self.bg_input.darken(0.25);
        theme.button.border = self.border_subtle;
        theme.button.text_color = self.text_primary;
        theme.button.text_color_disabled = self.text_muted;

        theme.panel.background = self.bg_primary;
        theme.panel.border = self.border_subtle;

        theme.slider.track_background = self.bg_input;
        theme.slider.track_fill = self.accent_blue;
        theme.slider.thumb_pressed = self.accent_cyan;

        theme.text.color = self.text_primary;
        theme.text.font_size = self.fonts.body;

        theme.text_input.background = self.bg_input;
        theme.text_input.background_focused = self.bg_input.lighten(0.08);
        theme.text_input.border = self.border_subtle;
        theme.text_input.border_focused = self.accent_blue;
        theme.text_input.text_color = self.text_primary;
        theme.text_input.font_size = self.fonts.body;
        theme.text_input.selection_color = self.accent_blue.with_alpha(0.35);
        theme.text_input.cursor_color = self.text_primary;

        theme
    }

    /// Create a `GizmoPalette` from this theme.
    pub fn gizmo_palette(&self) -> crate::gizmo::GizmoPalette {
        crate::gizmo::GizmoPalette {
            x: self.gizmo_x,
            y: self.gizmo_y,
            center: self.gizmo_center,
            x_hover: self.gizmo_x_hover,
            y_hover: self.gizmo_y_hover,
            center_hover: self.gizmo_center_hover,
            ring: self.gizmo_ring,
            rotation_indicator: self.gizmo_rotation_indicator,
            scale_outline: self.gizmo_scale_outline,
            scale_handle: self.gizmo_scale_handle,
            scale_handle_hover: self.gizmo_scale_handle_hover,
        }
    }

    /// Create `ColliderOverlayColors` from this theme.
    pub fn collider_overlay_colors(&self) -> crate::ColliderOverlayColors {
        crate::ColliderOverlayColors {
            solid: self.collider_outline,
            sensor: self.collider_sensor,
            selected: self.collider_selected,
        }
    }

    /// Colors for the viewport selection/hover outlines. Derivation contract:
    /// secondary is the primary dimmed (alpha preserved), hovered is the
    /// primary at 40% of its own alpha (multiplied, so translucent themes
    /// stay proportionally translucent).
    pub fn selection_outline_colors(&self) -> crate::SelectionOutlineColors {
        let primary = self.selection_outline;
        crate::SelectionOutlineColors {
            primary,
            secondary: primary.darken(0.7),
            hovered: primary.with_alpha(primary.a * 0.4),
        }
    }

    /// Get the viewport border color for a given play state.
    pub fn play_state_border(&self, state: crate::EditorPlayState) -> Color {
        match state {
            crate::EditorPlayState::Editing => self.border_editing,
            crate::EditorPlayState::Playing => self.border_playing,
            crate::EditorPlayState::Paused => self.border_paused,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme_colors_are_opaque() {
        let theme = EditorTheme::default();
        // Background colors should be fully opaque
        assert_eq!(theme.bg_primary.a, 1.0);
        assert_eq!(theme.bg_viewport.a, 1.0);
        assert_eq!(theme.bg_input.a, 1.0);
        // Text colors should be fully opaque
        assert_eq!(theme.text_primary.a, 1.0);
        assert_eq!(theme.text_secondary.a, 1.0);
        assert_eq!(theme.text_muted.a, 1.0);
    }

    #[test]
    fn test_accent_colors_are_distinct() {
        let theme = EditorTheme::default();
        // Blue (#0078d4) vs Cyan (#00d9ff) differ in green and blue channels
        assert_ne!(theme.accent_blue.g, theme.accent_cyan.g);
        assert_ne!(theme.accent_blue.b, theme.accent_cyan.b);
    }

    #[test]
    fn test_play_state_borders_are_distinct() {
        let theme = EditorTheme::default();
        let editing = theme.border_editing;
        let playing = theme.border_playing;
        let paused = theme.border_paused;
        // Each state has a unique dominant channel
        assert!(editing.b > editing.r && editing.b > editing.g); // blue-ish
        assert!(playing.g > playing.r && playing.g > playing.b); // green-ish
        assert!(paused.r > paused.b); // warm/yellow-ish
    }

    #[test]
    fn test_ui_theme_hover_and_type_are_usable() {
        let theme = EditorTheme::default();
        let ui_theme = theme.ui_theme();
        assert!(ui_theme.text_input.font_size >= crate::typography::MIN_READABLE_FONT);
        assert_ne!(
            ui_theme.button.background_hovered, ui_theme.button.background,
            "hover state must be visually distinct"
        );
    }

    #[test]
    fn test_hover_colors_differ_from_base() {
        let theme = EditorTheme::default();
        assert_ne!(theme.gizmo_x_hover, theme.gizmo_x);
        assert_ne!(theme.gizmo_y_hover, theme.gizmo_y);
        assert_ne!(theme.gizmo_scale_handle_hover, theme.gizmo_scale_handle);
        assert_ne!(theme.selection_fill, theme.hover_fill);
    }

    #[test]
    fn test_collider_overlay_colors_are_distinct() {
        let theme = EditorTheme::default();
        // Each state must be visually distinguishable
        assert_ne!(theme.collider_outline, theme.collider_sensor);
        assert_ne!(theme.collider_outline, theme.collider_selected);
        assert_ne!(theme.collider_sensor, theme.collider_selected);
    }

    #[test]
    fn test_selection_outline_derivation_contract() {
        let theme = EditorTheme::default();
        let c = theme.selection_outline_colors();
        // Each role must be visually distinguishable
        assert_ne!(c.primary, c.secondary);
        assert_ne!(c.primary, c.hovered);
        assert_ne!(c.secondary, c.hovered);
        // Secondary dims the primary but preserves its alpha; hovered
        // multiplies the primary's own alpha (translucent themes stay
        // proportionally translucent) — see selection_outline_colors().
        assert_eq!(c.secondary.a, c.primary.a);
        assert!((c.hovered.a - c.primary.a * 0.4).abs() < 1e-6);
        // Distinct from the collider overlay's selected color so both
        // overlays can be read at once
        assert_ne!(theme.selection_outline, theme.collider_selected);
    }
}
