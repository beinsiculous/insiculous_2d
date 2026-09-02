//! The editor's own UI faces, shipped with the crate (audit §5.6).
//!
//! The chrome font must never depend on the opened project — the old
//! search started at the GAME's `assets/fonts/font.ttf`, so Pong's serif
//! skinned the entire editor. `include_bytes!` also works unchanged on
//! wasm32 (no VFS boot-ordering involved), which the web editor (#48)
//! inherits for free.
//!
//! DejaVu fonts are free (Bitstream Vera + public-domain extensions —
//! see `assets/fonts/LICENSE`); DejaVu Sans was already this editor's
//! de-facto fallback face. Packaging note: any distributed editor binary
//! embeds these fonts, so ship `assets/fonts/LICENSE` alongside it (the
//! Bitstream Vera terms require the notice).

use ui::FontHandle;

/// Editor chrome face (regular).
pub const EDITOR_FONT_REGULAR: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
/// Bold face for headings and panel titles.
pub const EDITOR_FONT_BOLD: &[u8] = include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf");
/// Monospace face for numeric inspector fields (`EditableFieldStyle::numeric_font`,
/// #54): digits line up and a scrub never jitters the caret.
pub const EDITOR_FONT_MONO: &[u8] = include_bytes!("../assets/fonts/DejaVuSansMono.ttf");

/// Handles to the loaded editor faces, populated at editor init.
/// `None` = that face failed to load (the editor falls back to whatever
/// the default font is rather than crashing).
#[derive(Debug, Clone, Copy, Default)]
pub struct EditorFonts {
    pub regular: Option<FontHandle>,
    pub bold: Option<FontHandle>,
    pub mono: Option<FontHandle>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_font_bytes_are_loadable() {
        // The three shipped faces must parse and yield distinct handles —
        // a corrupted vendored file fails here, not at editor startup.
        let mut fonts = ui::FontManager::new();
        let regular = fonts.load_font(EDITOR_FONT_REGULAR).expect("regular loads");
        let bold = fonts.load_font(EDITOR_FONT_BOLD).expect("bold loads");
        let mono = fonts.load_font(EDITOR_FONT_MONO).expect("mono loads");
        assert_ne!(regular, bold);
        assert_ne!(bold, mono);
        assert_ne!(regular, mono);
    }

    #[test]
    fn test_numeric_field_draws_and_measures_in_the_mono_face() {
        // #54: a float field told to use the mono face rasterizes its glyphs
        // from that font, and every width it measures comes from the same
        // face — the caret and click placement follow monospace advances.
        let mut ui = ui::UIContext::new();
        let regular = ui.load_font(EDITOR_FONT_REGULAR).expect("regular loads");
        let mono = ui.load_font(EDITOR_FONT_MONO).expect("mono loads");
        ui.set_default_font(regular);

        let size = 13.0;
        let narrow_mono = ui.measure_text_with_font("iii", size, Some(mono)).x;
        let wide_mono = ui.measure_text_with_font("WWW", size, Some(mono)).x;
        let narrow_regular = ui.measure_text_with_font("iii", size, Some(regular)).x;
        let wide_regular = ui.measure_text_with_font("WWW", size, Some(regular)).x;
        assert!((narrow_mono - wide_mono).abs() < 0.01, "mono advances are equal");
        assert!(wide_regular > narrow_regular + 1.0, "the proportional face is not");

        ui.begin_frame(&input::InputHandler::new(), glam::Vec2::new(800.0, 600.0));
        let bounds = ui::Rect::new(10.0, 10.0, 80.0, 20.0);
        ui.float_input(
            "mono_field",
            1.0,
            ui::FloatFieldOpts::range(0.0, 10.0).with_font(Some(mono)),
            bounds,
        );
        let font_ids: Vec<u32> = ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                ui::DrawCommand::Text { data, .. } => Some(data.font_id),
                _ => None,
            })
            .collect();
        ui.end_frame();
        assert_eq!(font_ids, vec![mono.id], "the value is drawn from the mono face");
    }

    /// Font ids of every text draw command in the current frame.
    fn text_font_ids(ui: &ui::UIContext) -> Vec<u32> {
        ui.draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                ui::DrawCommand::Text { data, .. } => Some(data.font_id),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_focused_mono_field_measures_its_selection_in_the_mono_face() {
        // #54 review F3: drawing in mono is not enough — the selection band
        // (and so the caret, which shares `prefix_widths`) must be measured
        // with the same advances, or the caret jitters against the glyphs.
        let mut ui = ui::UIContext::new();
        let regular = ui.load_font(EDITOR_FONT_REGULAR).expect("regular loads");
        let mono = ui.load_font(EDITOR_FONT_MONO).expect("mono loads");
        ui.set_default_font(regular);
        let size = ui.theme().text_input.font_size;
        let text = "111.00";
        let mono_width = ui.measure_text_with_font(text, size, Some(mono)).x;
        let regular_width = ui.measure_text_with_font(text, size, Some(regular)).x;
        assert!((mono_width - regular_width).abs() > 0.5, "the faces must measure apart");

        // Focus with everything selected, then render one frame.
        ui.focus_text_input("mono_edit", text);
        ui.begin_frame(&input::InputHandler::new(), glam::Vec2::new(800.0, 600.0));
        let bounds = ui::Rect::new(10.0, 10.0, 120.0, 20.0);
        ui.float_input(
            "mono_edit",
            111.0,
            ui::FloatFieldOpts::range(0.0, 1000.0).with_font(Some(mono)),
            bounds,
        );
        let band_widths: Vec<f32> = ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                ui::DrawCommand::Rect { bounds: rect, .. }
                    if (rect.width - mono_width).abs() < 0.01 || (rect.width - regular_width).abs() < 0.01 =>
                {
                    Some(rect.width)
                }
                _ => None,
            })
            .collect();
        ui.end_frame();
        assert_eq!(band_widths, vec![mono_width], "the selection band spans the mono measurement");
    }

    #[test]
    fn test_vec2_and_color_rows_propagate_the_numeric_font() {
        let mut ui = ui::UIContext::new();
        let regular = ui.load_font(EDITOR_FONT_REGULAR).expect("regular loads");
        let mono = ui.load_font(EDITOR_FONT_MONO).expect("mono loads");
        ui.set_default_font(regular);
        let style = crate::EditableFieldStyle::default().with_numeric_font(Some(mono));

        ui.begin_frame(&input::InputHandler::new(), glam::Vec2::new(800.0, 600.0));
        let mut inspector = crate::EditableInspector::new(&mut ui, 10.0, 10.0).with_style(style);
        inspector.vec2("Position", glam::Vec2::ZERO, -100.0..=100.0);
        inspector.color("Tint", glam::Vec4::ONE);
        let font_ids = text_font_ids(&ui);
        ui.end_frame();

        // Labels and axis/channel badges stay in the default face; every
        // numeric input (2 axes + 4 channels) is drawn from the mono face.
        assert_eq!(font_ids.iter().filter(|id| **id == mono.id).count(), 6, "{font_ids:?}");
        assert!(font_ids.contains(&regular.id), "labels keep the default font");
    }

    #[test]
    fn test_stale_numeric_font_falls_back_to_the_default_face() {
        // #54 review F2: a handle that no longer resolves must not downgrade
        // the field to placeholders while a usable default font is loaded.
        let mut ui = ui::UIContext::new();
        let regular = ui.load_font(EDITOR_FONT_REGULAR).expect("regular loads");
        ui.set_default_font(regular);

        ui.begin_frame(&input::InputHandler::new(), glam::Vec2::new(800.0, 600.0));
        ui.float_input(
            "stale",
            1.0,
            ui::FloatFieldOpts::range(0.0, 10.0).with_font(Some(ui::FontHandle { id: 999 })),
            ui::Rect::new(10.0, 10.0, 80.0, 20.0),
        );
        let font_ids = text_font_ids(&ui);
        ui.end_frame();
        assert_eq!(font_ids, vec![regular.id]);
    }
}
