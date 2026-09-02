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

    #[test]
    fn test_selection_row_fill_derivation_contract() {
        let theme = EditorTheme::default();
        let fills = theme.selection_row_fills();
        // The primary row must read differently from the other selected rows,
        // and its accent bar from both fills.
        assert_ne!(fills.primary, fills.secondary);
        assert_ne!(fills.accent, fills.primary);
        assert_ne!(fills.accent, fills.secondary);
        // Secondary is the primary at half alpha; the accent is the viewport
        // selection color, so hierarchy and viewport agree on "selected".
        assert!((fills.secondary.a - fills.primary.a * 0.5).abs() < 1e-6);
        assert_eq!(fills.accent, theme.selection_outline);
        assert_ne!(fills.secondary, theme.hover_fill, "a secondary row is not a hovered row");
    }

// ==================== Elevation ladder (audit §5.2, §5.3) ====================

#[test]
fn test_adjacent_surfaces_are_distinguishable() {
    let theme = EditorTheme::default();
    let ladder = [
        ("surface_0/1", theme.surface_0, theme.surface_1),
        ("surface_1/2", theme.surface_1, theme.surface_2),
        ("surface_2/3", theme.surface_2, theme.surface_3),
        ("surface_3/4", theme.surface_3, theme.surface_4),
    ];
    for (name, lower, higher) in ladder {
        let ratio = lower.contrast_ratio(higher);
        assert!(ratio >= 1.35, "{name} contrast {ratio:.3} < 1.35 — surfaces have no edges");
        assert!(higher.luminance() > lower.luminance(), "{name}: elevation must get lighter");
    }
}

#[test]
fn test_popup_reads_against_panel() {
    let theme = EditorTheme::default();
    let surface = theme.surface_4.contrast_ratio(theme.surface_1);
    assert!(surface >= 1.35, "popup surface vs panel body: {surface:.3}");
    let border = theme.popup_border.contrast_ratio(theme.surface_4);
    assert!(border >= 3.0, "popup border vs popup surface: {border:.3}");
}

#[test]
fn test_header_is_lighter_than_panel_body() {
    // The old theme documented bg_header as darker than bg_primary and
    // shipped it 0.6/255 LIGHTER — neither readable nor honest (§5.2).
    let theme = EditorTheme::default();
    assert!(theme.bg_header.luminance() > theme.bg_primary.luminance());
    assert!(theme.bg_header.contrast_ratio(theme.bg_primary) >= 1.35);
}

#[test]
fn test_disabled_button_differs_from_pressed() {
    // §5.8: a disabled button must not render identically to a held one.
    let theme = EditorTheme::default().ui_theme();
    assert_ne!(theme.button.background_disabled, theme.button.background_pressed);
}
