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
/// Monospace face for numeric fields (consumed by the Sprint-3 inspector
/// work; the handle is loaded and stored now so it's one field away).
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
}
