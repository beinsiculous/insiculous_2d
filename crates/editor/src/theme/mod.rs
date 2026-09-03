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
    // ── Surface elevation ladder (audit §5.2) ──────────────────────
    // surface_0 (lowest: viewport well) .. surface_4 (floating popups).
    // Adjacent steps hold ≥1.35:1 WCAG contrast — the guard test in
    // theme/tests.rs is the spec; tune values only with it green.
    /// Elevation 0: the viewport well behind everything.
    pub surface_0: Color,
    /// Elevation 1: panel bodies.
    pub surface_1: Color,
    /// Elevation 2: panel headers, status bar.
    pub surface_2: Color,
    /// Elevation 3: input fields, wells.
    pub surface_3: Color,
    /// Elevation 4: floating surfaces (dropdowns, popups, future modals).
    pub surface_4: Color,
    /// Border for floating surfaces — ≥3:1 against surface_4 so popups
    /// read as bounded objects (audit §5.3).
    pub popup_border: Color,

    pub bg_primary: Color,
    /// Viewport / canvas area (`#000000`)
    pub bg_viewport: Color,
    /// Input fields, dropdowns (`#2d2d2d`)
    pub bg_input: Color,
    /// Panel header background — LIGHTER than bg_primary (surface_2 over
    /// surface_1; the old doc claimed darker and the old value was neither)
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

    // ── Grid ────────────────────────────────────────────────────
    /// Primary grid line color
    pub grid_primary: Color,
    /// Secondary (subdivision) grid line color
    pub grid_secondary: Color,
    /// Grid X-axis color (red)
    pub grid_axis_x: Color,
    /// Grid Y-axis color (green)
    pub grid_axis_y: Color,

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
        // The elevation ladder (audit §5.2). WCAG contrast is dominated by
        // the +0.05 flare term near black, so honest ≥1.35:1 steps need
        // bigger jumps than classic editor themes use — that is the point:
        // adjacent surfaces must actually be distinguishable.
        let surface_0 = Color::from_hex(0x0a0a0a);
        let surface_1 = Color::from_hex(0x2a2a2a);
        let surface_2 = Color::from_hex(0x404040);
        let surface_3 = Color::from_hex(0x545454);
        let surface_4 = Color::from_hex(0x686868);

        Self {
            // Typography
            fonts: crate::typography::FontSizes::default(),

            // Surface elevation ladder
            surface_0,
            surface_1,
            surface_2,
            surface_3,
            surface_4,
            popup_border: Color::from_hex(0xc6c6c6),

            // Backgrounds (aliases into the ladder — legacy names kept to
            // avoid churning 30+ call sites this sprint)
            bg_primary: surface_1,
            bg_viewport: surface_0,
            bg_input: surface_3,
            bg_header: surface_2,

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

            // Gizmos — X red / Y green, the universal DCC convention (the
            // grid origin axes and the gizmo's own defaults agree; these
            // were swapped for a while, caught by the sprint-4 visual pass)
            gizmo_x: Color::new(1.0, 0.0, 0.0, 1.0),
            gizmo_y: Color::new(0.0, 1.0, 0.0, 1.0),
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

            // Grid
            grid_primary: Color::new(0.3, 0.3, 0.3, 0.5),
            grid_secondary: Color::new(0.25, 0.25, 0.25, 0.3),
            grid_axis_x: Color::new(0.8, 0.2, 0.2, 0.8),
            grid_axis_y: Color::new(0.2, 0.8, 0.2, 0.8),

            // Status bar
            status_bar_bg: surface_2,

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
        // Distinct from pressed (audit §5.8): a disabled button must not
        // look like a held one — flatten toward the panel body instead.
        theme.button.background_disabled = self.bg_primary;
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
        theme.text_input.border_invalid = self.error_red;
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

    /// Fills for selected hierarchy rows (#51). Derivation contract: the
    /// primary row keeps `selection_fill`, every other selected row is the
    /// same fill at half its alpha, and the primary's accent bar is the
    /// viewport's `selection_outline` — one selection color in both places.
    pub fn selection_row_fills(&self) -> crate::SelectionRowFills {
        let primary = self.selection_fill;
        crate::SelectionRowFills {
            primary,
            secondary: primary.with_alpha(primary.a * 0.5),
            accent: self.selection_outline,
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
mod tests;
