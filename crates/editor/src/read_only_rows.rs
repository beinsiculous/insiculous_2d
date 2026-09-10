//! The read-only form of every editable inspector row.
//!
//! While a play session runs the inspector draws the same rows it draws
//! while editing, with the control replaced by the value it would hold.
//! Each row keeps the height its editable form has, so the panel's content
//! height — and therefore its scroll offset — is identical in both states.
//! No widget id is created here: a read-only row cannot take focus, so
//! nothing typed while the game runs can reach the live world.
//!
//! The parity holds for a fixed component value, not across one that the
//! simulation changes: an editor function may draw a different set of rows
//! for a different value — `edit_audio_source` draws its spatial block only
//! while `spatial` is true — so a script that flips such a field mid-Play
//! changes the row count, and the panel's scroll offset clamps to the new
//! content height. No shipped game does that, and the behaviour is
//! accepted; a row set that must survive it has to draw every row
//! unconditionally.

use glam::{Vec2, Vec4};
use ui::UIContext;

use crate::field_style::EditableFieldStyle;
use crate::row_layout::RowLayout;

/// Draw one read-only row: the field's label and its value, both in the
/// style's muted colour.
pub(crate) fn value_row(
    ui: &mut UIContext,
    label: &str,
    value: &str,
    layout: RowLayout,
    style: &EditableFieldStyle,
) {
    let mut muted = style.clone();
    muted.label_color = style.muted_color;
    muted.value_color = style.muted_color;
    crate::text_field::display_string(ui, label, value, layout, &muted);
}

/// A float as its editable field shows it (`ui::UIContext::float_input`
/// renders two decimals plus the unit suffix), so a row does not change
/// its reading when the game starts.
pub(crate) fn float_text(value: f32, suffix: &str) -> String {
    format!("{value:.2}{suffix}")
}

/// A Vec2 as its axis badges read it.
pub(crate) fn vec2_text(value: Vec2) -> String {
    format!("X {:.2}  Y {:.2}", value.x, value.y)
}

/// A boolean as the scene file spells it, not as the checkbox draws it.
pub(crate) fn bool_text(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

/// A colour as the colour editor's hex field spells it.
pub(crate) fn color_text(value: Vec4) -> String {
    crate::color_hex::to_hex(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_only_values_read_the_same_as_the_controls_they_replace() {
        // A row that changed its reading when the game started would look
        // like an edit the play session had made.
        assert_eq!(float_text(1.5, ""), "1.50", "float_input draws two decimals");
        assert_eq!(float_text(-90.0, "°"), "-90.00°", "the suffix comes along");
        assert_eq!(vec2_text(Vec2::new(1.0, -2.25)), "X 1.00  Y -2.25");
        assert_eq!(bool_text(true), "true");
        assert_eq!(bool_text(false), "false");
        assert_eq!(color_text(Vec4::new(1.0, 0.0, 0.0, 1.0)), "#ff0000");
    }
}
