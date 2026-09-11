//! Theme guards: the WCAG surface ladder and popup border are the spec —
//! tune hexes only with them green — and every pair of roles a user must
//! tell apart at a glance stays apart.

use super::*;

/// Adjacent surfaces on the elevation ladder are distinguishable (≥1.35:1)
/// and elevation gets lighter.
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

/// A popup reads as an object above the panel (≥1.35:1 against the panel
/// body) with a border of at least 3:1 against its own surface (§5.3).
#[test]
fn test_popup_reads_against_panel() {
    let theme = EditorTheme::default();
    let surface = theme.surface_4.contrast_ratio(theme.surface_1);
    assert!(surface >= 1.35, "popup surface vs panel body: {surface:.3}");
    let border = theme.popup_border.contrast_ratio(theme.surface_4);
    assert!(border >= 3.0, "popup border vs popup surface: {border:.3}");
}

/// Viewport selection outlines are DERIVED from theme tokens, not
/// hardcoded by the panel: secondary dims the primary but keeps its alpha,
/// hovered multiplies the primary's alpha, and none of them collides with
/// the collider overlay's selected colour so both overlays read at once.
#[test]
fn test_selection_outline_derivation_contract() {
    let theme = EditorTheme::default();
    let colors = theme.selection_outline_colors();
    assert_ne!(colors.primary, colors.secondary);
    assert_ne!(colors.primary, colors.hovered);
    assert_ne!(colors.secondary, colors.hovered);
    assert_eq!(colors.secondary.a, colors.primary.a, "secondary keeps the primary's alpha");
    assert!((colors.hovered.a - colors.primary.a * 0.4).abs() < 1e-6, "hovered is the primary at 0.4 alpha");
    assert_ne!(theme.selection_outline, theme.collider_selected);
}

/// Hierarchy row fills are derived the same way: the primary row reads
/// apart from the other selected rows (secondary = primary at half alpha),
/// its accent bar is the viewport selection colour so the two panels agree
/// on "selected", and a secondary row is not a hovered row.
#[test]
fn test_selection_row_fill_derivation_contract() {
    let theme = EditorTheme::default();
    let fills = theme.selection_row_fills();
    assert_ne!(fills.primary, fills.secondary);
    assert_ne!(fills.accent, fills.primary);
    assert_ne!(fills.accent, fills.secondary);
    assert!((fills.secondary.a - fills.primary.a * 0.5).abs() < 1e-6, "secondary is the primary at half alpha");
    assert_eq!(fills.accent, theme.selection_outline);
    assert_ne!(fills.secondary, theme.hover_fill, "a secondary row is not a hovered row");
}

/// Every pair of roles an editor user tells apart at a glance: the two
/// accents, the three play-state viewport borders, hover states against
/// their base, the three collider overlay states, and a disabled button
/// against a held one (§5.8).
#[test]
fn test_roles_that_must_read_apart_do() {
    let theme = EditorTheme::default();
    let ui_theme = theme.ui_theme();
    let pairs = [
        ("accent_blue/accent_cyan", theme.accent_blue, theme.accent_cyan),
        ("border_editing/border_playing", theme.border_editing, theme.border_playing),
        ("border_playing/border_paused", theme.border_playing, theme.border_paused),
        ("border_editing/border_paused", theme.border_editing, theme.border_paused),
        ("gizmo_x/hover", theme.gizmo_x, theme.gizmo_x_hover),
        ("gizmo_y/hover", theme.gizmo_y, theme.gizmo_y_hover),
        ("gizmo_scale_handle/hover", theme.gizmo_scale_handle, theme.gizmo_scale_handle_hover),
        ("selection_fill/hover_fill", theme.selection_fill, theme.hover_fill),
        ("collider_outline/sensor", theme.collider_outline, theme.collider_sensor),
        ("collider_outline/selected", theme.collider_outline, theme.collider_selected),
        ("collider_sensor/selected", theme.collider_sensor, theme.collider_selected),
        ("button/hovered", ui_theme.button.background, ui_theme.button.background_hovered),
        ("button disabled/pressed", ui_theme.button.background_disabled, ui_theme.button.background_pressed),
    ];
    for (name, a, b) in pairs {
        assert_ne!(a, b, "{name} must be visually distinct");
    }
}

/// Helper to composite color `c` over `base` with `base + (c - base) * c.a`.
fn composite_over(c: Color, base: Color) -> Color {
    Color::new(
        base.r + (c.r - base.r) * c.a,
        base.g + (c.g - base.g) * c.a,
        base.b + (c.b - base.b) * c.a,
        1.0,
    )
}

/// (a) The grid's primary line and the collider outline, each composited over surface_0,
/// contrast less against surface_0 than selection_outline does — the accent wins.
#[test]
fn test_grid_and_collider_lines_contrast_less_than_selection_outline() {
    let theme = EditorTheme::default();
    let base = theme.surface_0;

    let grid_comp = composite_over(theme.grid_primary, base);
    let collider_comp = composite_over(theme.collider_outline, base);
    let selection_comp = composite_over(theme.selection_outline, base);

    let grid_contrast = grid_comp.contrast_ratio(base);
    let collider_contrast = collider_comp.contrast_ratio(base);
    let selection_contrast = selection_comp.contrast_ratio(base);

    assert!(
        grid_contrast < selection_contrast,
        "grid primary contrast {grid_contrast:.2} must be less than selection {selection_contrast:.2}"
    );
    assert!(
        collider_contrast < selection_contrast,
        "collider contrast {collider_contrast:.2} must be less than selection {selection_contrast:.2}"
    );
}

/// (b) game_frame reads apart from collider_outline, collider_selected and selection_outline.
#[test]
fn test_game_frame_reads_apart_from_colliders_and_selection() {
    let theme = EditorTheme::default();
    assert_ne!(theme.game_frame, theme.collider_outline);
    assert_ne!(theme.game_frame, theme.collider_selected);
    assert_ne!(theme.game_frame, theme.selection_outline);
}

/// (c) grid_primary and grid_secondary differ and each axis is brighter than the primary line.
#[test]
fn test_grid_primary_and_secondary_differ_and_axes_are_brighter() {
    let theme = EditorTheme::default();
    assert_ne!(theme.grid_primary, theme.grid_secondary);
    assert!(
        theme.grid_axis_x.luminance() > theme.grid_primary.luminance(),
        "X axis luminance must exceed grid primary"
    );
    assert!(
        theme.grid_axis_y.luminance() > theme.grid_primary.luminance(),
        "Y axis luminance must exceed grid primary"
    );
}
