//! A title screen's art: the game's title composition at 1×, at the top of the
//! window, on its own backdrop, with the menu panel starting just under the art's
//! content — over the art's blank lower margin, which is the backdrop's colour.
//!
//! One function lays the title screen out for both of its halves, the drawing and
//! the mouse hit-test, so a click lands on the row it appears to. A window too small
//! for art and menu together, art that will not load, or art with nothing drawn on
//! it keeps the plain centred menu with no art and no fill.
//!
//! ```no_run
//! use engine_core::prelude::*;
//!
//! const MENU: TitleMenu = TitleMenu { panel_width: 380.0, rows: 4, fit_rows: 4, lines_below_panel: 0.0 };
//!
//! # fn draw(ctx: &mut GameContext, art: Option<&TitleArt>, style: &MenuStyle) {
//! let layout = title_layout("MY GAME", ctx.window_size, art, &MENU);
//! draw_title_art(ctx.ui, ctx.window_size, &layout, art);
//! let _first_row = layout.panel.begin(ctx.ui, style);
//! # }
//! ```

use glam::Vec2;
use renderer::TextureFilter;
use ui::{Color, Rect, UIContext};

use crate::assets::AssetManager;
use crate::menu_panel::MenuPanel;

/// Space between the art's last content row and the menu panel's top.
const PANEL_GAP_UNDER_ART: f32 = 18.0;
/// Room kept between the lowest thing on the title screen and the window's bottom.
const BOTTOM_MARGIN: f32 = 16.0;

/// A loaded title composition: its texture, its size, its backdrop — the top-left
/// pixel's colour, which the window is filled with around it — and the last row
/// holding anything but that backdrop.
#[derive(Debug, Clone, Copy)]
pub struct TitleArt {
    texture_id: u32,
    size: Vec2,
    backdrop: Color,
    content_bottom: f32,
}

impl TitleArt {
    /// Load the title art at `path` (resolved against the asset base), or `None`
    /// — logged — when it will not read or load, or when nothing is drawn on it;
    /// the title screen then keeps its plain centred menu.
    pub fn load(assets: &mut AssetManager, path: &str) -> Option<Self> {
        let backdrop = assets.image_backdrop(path)?;
        let Some(content_bottom) = backdrop.content_bottom else {
            log::warn!("The title art {path} is all backdrop, so the title keeps its plain menu");
            return None;
        };
        match assets.load_texture_filtered(path, TextureFilter::Nearest) {
            Ok(texture) => Some(Self {
                texture_id: texture.id,
                size: Vec2::new(backdrop.size.x as f32, backdrop.size.y as f32),
                backdrop: backdrop.corner,
                content_bottom: content_bottom as f32,
            }),
            Err(error) => {
                log::warn!("The title art {path} did not load, so the title keeps its plain menu: {error}");
                None
            }
        }
    }

    /// The texture the art draws with.
    pub fn texture_id(&self) -> u32 {
        self.texture_id
    }

    /// The last pixel row holding anything but the backdrop; the menu panel
    /// starts a little under it.
    pub fn content_bottom(&self) -> f32 {
        self.content_bottom
    }
}

/// A game's title menu, as the layout needs it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TitleMenu {
    pub panel_width: f32,
    /// The rows this build shows — always the game's own title list's length.
    pub rows: usize,
    /// The rows the fit is decided on: the native build's count on every target,
    /// so a window shows the art or falls back the same way on the web, whose
    /// title list is shorter, as natively.
    pub fit_rows: usize,
    /// Height of the lines the game draws under the panel (controls, a tagline).
    pub lines_below_panel: f32,
}

/// Where the title screen's parts go: the menu panel, and the art's box when the
/// window has room for it (`None` in the fallback).
pub struct TitleLayout {
    pub panel: MenuPanel,
    pub art: Option<Rect>,
}

/// The title screen's layout in this window.
pub fn title_layout(title: &str, window_size: Vec2, art: Option<&TitleArt>, menu: &TitleMenu) -> TitleLayout {
    let centred = || TitleLayout {
        panel: MenuPanel::new(title, window_size / 2.0, menu.panel_width, menu.rows),
        art: None,
    };
    let Some(art) = art else { return centred() };
    let panel_top = art.content_bottom + PANEL_GAP_UNDER_ART;
    // The larger of the two, so a panel showing more rows than the fit names cannot run off the window.
    let fit_rows = menu.fit_rows.max(menu.rows);
    let lowest = panel_top + panel_height(menu.panel_width, fit_rows) + menu.lines_below_panel + BOTTOM_MARGIN;
    if window_size.x < art.size.x || window_size.y < lowest {
        return centred();
    }
    let panel_center = Vec2::new(window_size.x / 2.0, panel_top + panel_height(menu.panel_width, menu.rows) / 2.0);
    TitleLayout {
        panel: MenuPanel::new(title, panel_center, menu.panel_width, menu.rows),
        art: Some(Rect::new(((window_size.x - art.size.x) / 2.0).floor(), 0.0, art.size.x, art.size.y)),
    }
}

/// A title panel's height for `rows` rows.
fn panel_height(panel_width: f32, rows: usize) -> f32 {
    MenuPanel::new("", Vec2::ZERO, panel_width, rows).panel_rect().height
}

/// Fill the window with the art's backdrop and draw the art, when the layout has
/// room for it — and nothing at all in the fallback. Drawn before the menu panel,
/// which then sits over the art's blank lower margin.
pub fn draw_title_art(ui: &mut UIContext, window_size: Vec2, layout: &TitleLayout, art: Option<&TitleArt>) {
    let (Some(bounds), Some(art)) = (layout.art, art) else { return };
    ui.rect(Rect::new(0.0, 0.0, window_size.x, window_size.y), art.backdrop);
    ui.image(bounds, art.texture_id, Color::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Vec2 = Vec2::new(800.0, 600.0);

    fn art(content_bottom: f32) -> TitleArt {
        TitleArt { texture_id: 1, size: Vec2::new(480.0, 288.0), backdrop: Color::BLACK, content_bottom }
    }

    fn menu(rows: usize, fit_rows: usize, lines_below_panel: f32) -> TitleMenu {
        TitleMenu { panel_width: 380.0, rows, fit_rows, lines_below_panel }
    }

    #[test]
    fn the_art_sits_centred_at_the_top_and_the_panel_starts_under_its_content() {
        let layout = title_layout("", WINDOW, Some(&art(234.0)), &menu(4, 4, 0.0));
        assert_eq!(layout.art, Some(Rect::new(160.0, 0.0, 480.0, 288.0)));
        let panel = layout.panel.panel_rect();
        assert_eq!(panel.y, 252.0, "the content's last row, 234, plus the gap");
        assert_eq!(panel.y + panel.height, 490.0, "four rows are 238 high");
    }

    #[test]
    fn a_window_too_small_or_no_art_keeps_the_plain_centred_menu() {
        let small = Vec2::new(640.0, 480.0);
        for art in [Some(art(234.0)), None] {
            let layout = title_layout("", small, art.as_ref(), &menu(4, 4, 0.0));
            assert!(layout.art.is_none(), "no art at 640x480, art given: {}", art.is_some());
            let panel = layout.panel.panel_rect();
            assert_eq!(panel.y + panel.height / 2.0, small.y / 2.0, "centred");
        }
        assert!(title_layout("", WINDOW, None, &menu(4, 4, 0.0)).art.is_none(), "art that did not load");
        let narrow = Vec2::new(479.0, 600.0);
        assert!(title_layout("", narrow, Some(&art(234.0)), &menu(4, 4, 0.0)).art.is_none(), "narrower than the art");
    }

    #[test]
    fn the_fit_is_decided_on_the_fit_rows_and_the_lines_under_the_panel() {
        // 234 + 18 + a four-row panel (238) + 64 of lines + 16 is 570; three rows would fit in less.
        let just_enough = Vec2::new(800.0, 570.0);
        assert!(title_layout("", just_enough, Some(&art(234.0)), &menu(4, 4, 64.0)).art.is_some());
        assert!(title_layout("", just_enough - Vec2::Y, Some(&art(234.0)), &menu(4, 4, 64.0)).art.is_none());
        let web = title_layout("", just_enough - Vec2::Y, Some(&art(234.0)), &menu(3, 4, 64.0));
        assert!(web.art.is_none(), "three rows shown, but the fit is the native four's");
        let shown = title_layout("", just_enough, Some(&art(234.0)), &menu(3, 4, 64.0)).panel.panel_rect();
        assert_eq!((shown.y, shown.height), (252.0, 208.0), "the panel drawn is the three rows'");
    }

    #[test]
    fn more_rows_than_the_fit_names_still_have_to_fit() {
        let four_rows_fit = Vec2::new(800.0, 570.0);
        let layout = title_layout("", four_rows_fit, Some(&art(234.0)), &menu(5, 4, 64.0));
        assert!(layout.art.is_none(), "a fifth row would end under the window's bottom");
    }

    #[test]
    fn the_art_lands_on_whole_pixels_in_an_odd_width_window() {
        let layout = title_layout("", Vec2::new(801.0, 600.0), Some(&art(234.0)), &menu(4, 4, 0.0));
        assert_eq!(layout.art.map(|bounds| bounds.x), Some(160.0));
    }
}
