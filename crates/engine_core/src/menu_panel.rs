//! Shared menu window chrome: an opaque, bordered panel with a title band,
//! accent separator, corner ticks, cursor-highlighted rows, and a hint
//! footer — so menu screens read as real UI instead of floating text.
//!
//! The engine owns the chrome; games own the content and colors (build a
//! [`MenuStyle`] from the game's [`ChaosTheme`]). Structural flair (borders,
//! separator bars, highlight bars, corner ticks) is drawn with rects so it
//! can never fall victim to missing font glyphs; the `▶` selection cursor is
//! verified present in the games' shared `font.ttf`.
//!
//! Typical menu screen:
//! ```no_run
//! use engine_core::prelude::*;
//! use engine_core::menu_panel::{MenuPanel, MenuStyle};
//!
//! # fn draw(ctx: &mut GameContext, selection: u8) {
//! let style = MenuStyle::from_theme(&ChaosTheme::for_mode(ctx.chaos_mode));
//! let panel = MenuPanel::new("MY GAME", ctx.window_size / 2.0, 340.0, 3);
//! let mut y = panel.begin(ctx.ui, &style);
//! for (i, item) in ["Play", "Options", "Quit"].iter().enumerate() {
//!     y = panel.item(ctx.ui, y, item, i as u8 == selection, &style);
//! }
//! panel.hint(ctx.ui, "W/S navigate - SPACE confirm", &style);
//! # }
//! ```

use glam::{Vec2, Vec4};
use input::InputHandler;
use ui::{Color, Rect, UIContext};
use winit::event::MouseButton;

use crate::chaos_theme::ChaosTheme;

/// Vertical space per selectable row.
const ROW_HEIGHT: f32 = 30.0;
/// Space reserved for the title label + separator at the top of the panel.
const TITLE_BAND: f32 = 52.0;
/// Space reserved for the hint footer at the bottom of the panel.
const HINT_BAND: f32 = 34.0;
/// Inner padding on every side.
const PADDING: f32 = 16.0;
/// Border thickness of the window frame.
const BORDER: f32 = 2.0;
/// Length of the corner accent ticks.
const TICK_LEN: f32 = 14.0;

fn color(v: Vec4) -> Color {
    Color::new(v.x, v.y, v.z, v.w)
}

/// Colors for one menu window, typically derived from the game's chaos
/// theme so menus keep each game's identity.
#[derive(Debug, Clone, Copy)]
pub struct MenuStyle {
    /// Opaque window fill — darker than the game background so the panel
    /// reads as a distinct surface.
    pub background: Vec4,
    /// Window frame color.
    pub border: Vec4,
    /// Title, separator, corner ticks, and selected-item highlight.
    pub accent: Vec4,
    /// Body text.
    pub text: Vec4,
    /// Unselected items and hint text.
    pub dim: Vec4,
}

impl MenuStyle {
    /// Derive a window style from a chaos theme: near-black fill tinted
    /// toward the theme's background, structure-colored frame, accent
    /// highlights.
    pub fn from_theme(theme: &ChaosTheme) -> Self {
        let bg = theme.bg_color;
        Self {
            background: Vec4::new(
                bg.x * 0.35 + 0.02,
                bg.y * 0.35 + 0.02,
                bg.z * 0.35 + 0.05,
                0.97,
            ),
            border: theme.structure_color,
            accent: theme.accent_color,
            text: Vec4::new(0.9, 0.92, 0.95, 1.0),
            dim: Vec4::new(0.55, 0.57, 0.63, 1.0),
        }
    }
}

/// One frame's mouse interaction with a menu window's selectable rows,
/// from [`MenuPanel::mouse_select`]. Convention (all games): **hover moves
/// the shared selection cursor, a left-click on a row confirms it** — treat
/// `clicked` exactly like [`MenuInput`](crate::MenuInput)'s `confirm` on
/// that row. Clicks outside the rows do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuMouse {
    /// Row under the cursor — reported only on frames where the mouse
    /// actually moved, so a resting cursor never fights keyboard/pad
    /// navigation for the selection.
    pub hovered: Option<u8>,
    /// Row the left button was just pressed on this frame (reported whether
    /// or not the mouse moved).
    pub clicked: Option<u8>,
}

/// One menu window: fixed title/center/width/row-count, drawn as layered
/// rects + labels. Construct per frame (it's just layout parameters).
pub struct MenuPanel {
    title: String,
    center: Vec2,
    width: f32,
    rows: usize,
}

impl MenuPanel {
    pub fn new(title: &str, center: Vec2, width: f32, rows: usize) -> Self {
        Self { title: title.to_string(), center, width, rows }
    }

    /// The window's bounds: title band + rows + hint band + padding,
    /// centered on `center`.
    pub fn panel_rect(&self) -> Rect {
        let height = TITLE_BAND + self.rows as f32 * ROW_HEIGHT + HINT_BAND + 2.0 * PADDING;
        Rect::new(
            self.center.x - self.width / 2.0,
            self.center.y - height / 2.0,
            self.width,
            height,
        )
    }

    /// The clickable bounds of selectable row `index` — the same footprint
    /// the selection highlight bar covers, but the full row tall.
    pub fn row_rect(&self, index: usize) -> Rect {
        let rect = self.panel_rect();
        let inset = PADDING + 2.0;
        Rect::new(
            rect.x + inset,
            rect.y + PADDING + TITLE_BAND + index as f32 * ROW_HEIGHT,
            rect.width - 2.0 * inset,
            ROW_HEIGHT,
        )
    }

    /// Which selectable row (if any) contains `pos` (window pixels).
    /// The title band, hint band, and padding are not rows.
    pub fn row_at(&self, pos: Vec2) -> Option<u8> {
        (0..self.rows).find(|&i| self.row_rect(i).contains(pos)).map(|i| i as u8)
    }

    /// Read this frame's mouse interaction with the selectable rows (see
    /// [`MenuMouse`] for the hover-selects / click-confirms convention).
    /// Headless-testable: driven entirely by [`InputHandler`] state, like
    /// [`MenuInput`](crate::MenuInput).
    pub fn mouse_select(&self, input: &InputHandler) -> MenuMouse {
        let pos = input.mouse_position();
        let pos = Vec2::new(pos.x, pos.y);
        let row = self.row_at(pos);
        let moved = input.mouse_movement_delta() != (0.0, 0.0);
        MenuMouse {
            hovered: if moved { row } else { None },
            clicked: if input.is_mouse_button_just_pressed(MouseButton::Left) { row } else { None },
        }
    }

    /// Whether the left button was just pressed anywhere inside the window —
    /// title band, rows, hint band, padding included. The dismiss check for
    /// non-selectable info screens ("click to go back"), where row-band
    /// hit-testing would ignore clicks on headers and margins.
    pub fn clicked_inside(&self, input: &InputHandler) -> bool {
        let pos = input.mouse_position();
        input.is_mouse_button_just_pressed(MouseButton::Left)
            && self.panel_rect().contains(Vec2::new(pos.x, pos.y))
    }

    /// Draw the window frame (opaque fill, border, title, accent separator,
    /// corner ticks). Returns the y center of the first content row.
    pub fn begin(&self, ui: &mut UIContext, style: &MenuStyle) -> f32 {
        let rect = self.panel_rect();
        ui.panel_styled(rect, color(style.background), color(style.border), BORDER);

        // Title + accent separator underneath it
        ui.label_centered_styled(
            &self.title,
            Vec2::new(self.center.x, rect.y + PADDING + 14.0),
            color(style.accent),
            20.0,
        );
        let sep_y = rect.y + PADDING + TITLE_BAND - 14.0;
        let sep_inset = PADDING + 6.0;
        ui.panel_styled(
            Rect::new(rect.x + sep_inset, sep_y, rect.width - 2.0 * sep_inset, 2.0),
            color(style.accent),
            color(style.accent),
            0.0,
        );

        // Corner ticks: small accent L-marks just inside each corner
        for (cx, sx) in [(rect.x + 4.0, 1.0), (rect.x + rect.width - 4.0 - TICK_LEN, -1.0)] {
            let _ = sx;
            for cy in [rect.y + 4.0, rect.y + rect.height - 7.0] {
                ui.panel_styled(
                    Rect::new(cx, cy, TICK_LEN, 3.0),
                    color(style.accent),
                    color(style.accent),
                    0.0,
                );
            }
        }

        rect.y + PADDING + TITLE_BAND + ROW_HEIGHT / 2.0
    }

    /// One selectable row at y-center `y`: the selection gets a translucent
    /// accent highlight bar, a `▶` cursor, and accent text; unselected rows
    /// are dim. Returns the next row's y.
    pub fn item(&self, ui: &mut UIContext, y: f32, text: &str, selected: bool, style: &MenuStyle) -> f32 {
        let item_color = if selected { style.accent } else { style.dim };
        self.item_colored(ui, y, text, item_color, selected, style)
    }

    /// Like [`item`](Self::item) but with a caller-chosen text color (e.g.
    /// breakout's per-chaos-mode level entries). Selection still draws the
    /// highlight bar + cursor.
    pub fn item_colored(
        &self,
        ui: &mut UIContext,
        y: f32,
        text: &str,
        text_color: Vec4,
        selected: bool,
        style: &MenuStyle,
    ) -> f32 {
        let rect = self.panel_rect();
        if selected {
            let inset = PADDING + 2.0;
            let bar = Vec4::new(style.accent.x, style.accent.y, style.accent.z, 0.18);
            ui.panel_styled(
                Rect::new(rect.x + inset, y - ROW_HEIGHT / 2.0 + 4.0, rect.width - 2.0 * inset, ROW_HEIGHT - 8.0),
                color(bar),
                color(bar),
                0.0,
            );
            ui.label_centered_styled(
                &format!("▶ {text}"),
                Vec2::new(self.center.x, y),
                color(text_color),
                16.0,
            );
        } else {
            ui.label_centered_styled(text, Vec2::new(self.center.x, y), color(text_color), 16.0);
        }
        y + ROW_HEIGHT
    }

    /// A non-selectable centered text row (status lines, prompts on
    /// game-over panels). Returns the next row's y.
    pub fn line(&self, ui: &mut UIContext, y: f32, text: &str, style: &MenuStyle) -> f32 {
        ui.label_centered_styled(text, Vec2::new(self.center.x, y), color(style.text), 16.0);
        y + ROW_HEIGHT
    }

    /// Dim footer hint inside the bottom band of the window.
    pub fn hint(&self, ui: &mut UIContext, text: &str, style: &MenuStyle) {
        let rect = self.panel_rect();
        ui.label_centered_styled(
            text,
            Vec2::new(self.center.x, rect.y + rect.height - PADDING - 8.0),
            color(style.dim),
            12.0,
        );
    }

    /// Draw this window as an input-blocking overlay (pause menus): dims the
    /// whole screen, blocks clicks through it, then runs `content` with the
    /// first row's y (draw items/hints inside).
    pub fn draw_as_overlay(
        &self,
        ui: &mut UIContext,
        window_size: Vec2,
        style: &MenuStyle,
        content: impl FnOnce(&Self, &mut UIContext, f32),
    ) {
        let screen = Rect::new(0.0, 0.0, window_size.x, window_size.y);
        ui.begin_overlay(screen);
        // Dim the frozen world beneath
        let scrim = Color::new(0.0, 0.0, 0.0, 0.55);
        ui.panel_styled(screen, scrim, scrim, 0.0);
        let first_y = self.begin(ui, style);
        content(self, ui, first_y);
        ui.end_overlay();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::frame;
    use input::InputEvent;
    use winit::event::MouseButton;

    fn panel(rows: usize) -> MenuPanel {
        MenuPanel::new("TITLE", Vec2::new(400.0, 300.0), 320.0, rows)
    }

    fn center_of(rect: &Rect) -> Vec2 {
        Vec2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    /// An InputHandler whose mouse already has a position: MouseState
    /// suppresses the movement delta on the first-ever position update
    /// (anti-startup-warp), so hover tests must move from somewhere.
    fn input_with_mouse_at_origin() -> InputHandler {
        let mut input = InputHandler::new();
        frame(&mut input, &[InputEvent::MouseMoved(0.0, 0.0)]);
        input
    }

    #[test]
    fn row_at_round_trips_every_row_center_and_rejects_the_bands() {
        // The geometry SSOT: the window centers where it is asked to, grows
        // one ROW_HEIGHT per row, fits the standard 800x600 window at the
        // biggest roster (4 rows), and hit-tests exactly the rows it draws.
        let three = panel(3);
        let rect = three.panel_rect();
        assert_eq!(center_of(&rect), Vec2::new(400.0, 300.0));
        assert_eq!(rect.width, 320.0);
        let five = panel(5).panel_rect();
        assert_eq!(five.height - rect.height, 2.0 * ROW_HEIGHT);
        assert_eq!(center_of(&five), Vec2::new(400.0, 300.0), "still centered after growing");
        let biggest = panel(4).panel_rect();
        assert!(biggest.y > 0.0 && biggest.y + biggest.height < 600.0, "{biggest:?}");
        assert!(biggest.x > 0.0 && biggest.x + biggest.width < 800.0, "{biggest:?}");

        for row in 0..3 {
            assert_eq!(three.row_at(center_of(&three.row_rect(row))), Some(row as u8), "row {row}");
        }
        // Title band, hint band, and points outside the window are not rows.
        assert_eq!(three.row_at(Vec2::new(400.0, rect.y + PADDING + TITLE_BAND / 2.0)), None);
        assert_eq!(three.row_at(Vec2::new(400.0, rect.y + rect.height - PADDING - HINT_BAND / 2.0)), None);
        assert_eq!(three.row_at(Vec2::new(rect.x - 5.0, 300.0)), None);

        // The theme-derived style reads as a solid window, darker than the
        // game background it covers.
        let theme = ChaosTheme::for_mode(crate::chaos_mode::ChaosMode::Normal);
        let style = MenuStyle::from_theme(&theme);
        assert!(style.background.w > 0.9, "panel must read as a solid window");
        assert!(style.background.x < theme.bg_color.x + 0.06);
        assert!(style.background.y < theme.bg_color.y + 0.06);
        assert_eq!(style.accent, theme.accent_color);
        assert_eq!(style.border, theme.structure_color);
    }

    #[test]
    fn resting_cursor_does_not_hover_but_still_clicks() {
        let p = panel(3);
        let row2 = p.row_rect(2);
        let mut input = input_with_mouse_at_origin();

        // Frame 1: move onto row 2 — hover reports it.
        frame(&mut input, &[InputEvent::MouseMoved(row2.x + 4.0, row2.y + 4.0)]);
        assert_eq!(p.mouse_select(&input).hovered, Some(2));

        // Frame 2: no movement — hover stops fighting keyboard navigation,
        // but a click on the resting position still confirms the row.
        frame(&mut input, &[InputEvent::MouseButtonPressed(MouseButton::Left)]);
        let mouse = p.mouse_select(&input);
        assert_eq!(mouse.hovered, None);
        assert_eq!(mouse.clicked, Some(2));

        // Frame 3: move-and-click on row 1 in one frame reports both.
        let row1 = center_of(&p.row_rect(1));
        frame(&mut input, &[
            InputEvent::MouseButtonReleased(MouseButton::Left),
            InputEvent::MouseMoved(row1.x, row1.y),
            InputEvent::MouseButtonPressed(MouseButton::Left),
        ]);
        assert_eq!(p.mouse_select(&input), MenuMouse { hovered: Some(1), clicked: Some(1) });

        // Frame 4: a click in the title band is no row, but it IS inside the
        // window — what lets info screens dismiss on any click.
        let rect = p.panel_rect();
        let title_band = Vec2::new(400.0, rect.y + PADDING + TITLE_BAND / 2.0);
        frame(&mut input, &[
            InputEvent::MouseButtonReleased(MouseButton::Left),
            InputEvent::MouseMoved(title_band.x, title_band.y),
            InputEvent::MouseButtonPressed(MouseButton::Left),
        ]);
        assert_eq!(p.mouse_select(&input).clicked, None);
        assert!(p.clicked_inside(&input));

        // Frame 5: a click outside the window registers nowhere.
        frame(&mut input, &[
            InputEvent::MouseButtonReleased(MouseButton::Left),
            InputEvent::MouseMoved(rect.x - 20.0, rect.y - 20.0),
            InputEvent::MouseButtonPressed(MouseButton::Left),
        ]);
        assert_eq!(p.mouse_select(&input), MenuMouse::default());
        assert!(!p.clicked_inside(&input));
    }
}
