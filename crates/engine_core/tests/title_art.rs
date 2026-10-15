//! A title screen's art through `test_support::GameHarness`: the content's last row
//! measured from the file, and art with nothing drawn on it refused, so the title
//! keeps its plain centred menu. The art is a small PNG the test writes itself.

use engine_core::prelude::{
    draw_title_art, title_layout, Game, GameConfig, GameContext, TitleArt, TitleMenu,
};
use engine_core::test_support::GameHarness;
use ui::DrawCommand;

const BACKDROP: [u8; 4] = [20, 16, 31, 255];
const MENU: TitleMenu = TitleMenu { panel_width: 380.0, rows: 4, fit_rows: 4, lines_below_panel: 0.0 };

/// Loads the title art in `init` and draws it, as a game's title screen does.
#[derive(Default)]
struct TitleScreen {
    art: Option<TitleArt>,
}

impl Game for TitleScreen {
    fn init(&mut self, ctx: &mut GameContext) {
        self.art = TitleArt::load(ctx.assets, "sprites/title.png");
    }

    fn update(&mut self, ctx: &mut GameContext) {
        let layout = title_layout("TITLE", ctx.window_size, self.art.as_ref(), &MENU);
        draw_title_art(ctx.ui, ctx.window_size, &layout, self.art.as_ref());
    }
}

/// A 480 × 288 backdrop with `content_rows` painted another colour, in a temp
/// asset directory.
fn title_art_with(content_rows: &[u32]) -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(directory.path().join("sprites")).expect("sprites directory");
    let mut art = image::RgbaImage::from_pixel(480, 288, image::Rgba(BACKDROP));
    for &row in content_rows {
        art.put_pixel(240, row, image::Rgba([250, 220, 120, 255]));
    }
    art.save(directory.path().join("sprites").join("title.png")).expect("write the title art");
    directory
}

fn harness(directory: &tempfile::TempDir) -> GameHarness<TitleScreen> {
    let config = GameConfig::new("Title")
        .with_size(800, 600)
        .with_asset_base_path(directory.path().to_string_lossy().to_string());
    let mut harness = GameHarness::new(TitleScreen::default(), config);
    harness.step(1.0 / 60.0, &[]);
    harness
}

fn draws_an_image(harness: &GameHarness<TitleScreen>) -> bool {
    harness.ui_commands().iter().any(|command| matches!(command, DrawCommand::Image { .. }))
}

#[test]
fn the_content_bottom_is_the_last_row_holding_anything_but_the_corner() {
    for (rows, bottom) in [(&[16_u32, 120, 234][..], Some(234)), (&[287][..], Some(287)), (&[][..], None)] {
        let directory = title_art_with(rows);
        let mut harness = harness(&directory);
        let backdrop = harness
            .context(|_, ctx| ctx.assets.image_backdrop("sprites/title.png"))
            .expect("the art decodes");
        assert_eq!(backdrop.content_bottom, bottom, "content on rows {rows:?}");
    }
}

#[test]
fn title_art_loads_with_its_measured_bottom_and_draws_above_the_panel() {
    let directory = title_art_with(&[30, 234]);
    let harness = harness(&directory);
    let art = harness.game().art.expect("the art loads");
    assert_eq!(art.content_bottom(), 234.0);
    assert!(draws_an_image(&harness), "the title draws its art");
    let layout = title_layout("TITLE", glam::Vec2::new(800.0, 600.0), Some(&art), &MENU);
    assert_eq!(layout.panel.panel_rect().y, 252.0, "the panel 18 px under the last content row");
}

#[test]
fn art_with_nothing_drawn_on_it_keeps_the_plain_centred_menu() {
    let directory = title_art_with(&[]);
    let harness = harness(&directory);
    assert!(harness.game().art.is_none(), "an all-backdrop image is not title art");
    assert!(!draws_an_image(&harness), "no art and no fill");
    assert!(harness.ui_commands().is_empty(), "nothing drawn at all: {:?}", harness.ui_commands());
}
