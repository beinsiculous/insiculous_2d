//! The cards a game shows before it starts — a studio card, an engine card —
//! and the lifecycle that keeps the game shut while they show.
//!
//! The game names its cards in `GameConfig::startup_splashes`; the engine
//! carries none of its own. The runner consults [`StartupSplash`] once per
//! frame: while it [holds the game](StartupSplash::holds_game) the game is
//! neither initialized, updated nor drawn, and no key reaches it. A card fades
//! in, holds, fades out; a press skips to the next once the card has faded in;
//! after the last card one black frame hands off, and the game's `init` runs on
//! the frame after that — never on the frame of the press that ended the cards.
//! Presses in the game's first moments are then swallowed, so a player mashing
//! through the cards does not also press the title's first row.
//!
//! The cards draw on the UI path, which sorts whole batches by their lowest
//! depth and batches by texture: every white-textured rectangle must come
//! before the card's image, and nothing may be drawn over what fades. So a
//! frame is a black base, the card's backdrop as four margins around the image
//! (never under it), and the image — the margins and the image share one alpha
//! and never overlap, so no pixel is blended twice.

use common::{Color, Rect};
use glam::{UVec2, Vec2};
use ui::UIContext;

use renderer::TextureFilter;

use crate::assets::AssetManager;

/// A card's fade from black, and its fade back to it.
pub(crate) const CARD_FADE_SECONDS: f32 = 0.25;
/// How long a card holds at full strength between its fades.
pub(crate) const CARD_HOLD_SECONDS: f32 = 1.5;
/// A card's whole life when nobody skips it.
pub(crate) const CARD_SECONDS: f32 = CARD_FADE_SECONDS + CARD_HOLD_SECONDS + CARD_FADE_SECONDS;
/// How long after the cards end the game's new presses are swallowed.
pub(crate) const QUIET_SECONDS: f32 = 0.2;
/// The largest whole scale a card is drawn at.
const MAX_CARD_SCALE: u32 = 2;

/// Where the startup sequence stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Cards are named but not yet loaded: the first frame loads them.
    Pending,
    /// A card is showing.
    Splashing,
    /// The last card has ended; this frame is black and the next starts the game.
    Handoff,
    /// The game runs.
    Running,
}

/// One loaded card.
#[derive(Debug, Clone, Copy)]
struct Card {
    texture_id: u32,
    size: UVec2,
    /// The card's top-left pixel: the colour the window around it is filled with.
    backdrop: Color,
}

pub(crate) struct StartupSplash {
    paths: Vec<String>,
    cards: Vec<Card>,
    stage: Stage,
    current: usize,
    /// Seconds the current card has shown.
    elapsed: f32,
    /// Seconds of the quiet period still to run once the game has started.
    quiet_remaining: f32,
}

impl StartupSplash {
    /// The sequence for these card paths; with none, the game runs at once.
    pub(crate) fn new(paths: Vec<String>) -> Self {
        let stage = if paths.is_empty() { Stage::Running } else { Stage::Pending };
        Self { paths, cards: Vec::new(), stage, current: 0, elapsed: 0.0, quiet_remaining: 0.0 }
    }

    /// Whether the game must stay shut this frame: not initialized, updated,
    /// drawn, nor handed a key.
    pub(crate) fn holds_game(&self) -> bool {
        self.stage != Stage::Running
    }

    /// Whether a key must not reach the game's key handlers: while the cards hold
    /// it, and through the quiet period after them, the same moments in which
    /// `update` sees no new press.
    pub(crate) fn holds_keys(&self) -> bool {
        self.holds_game() || self.quiet_remaining > 0.0
    }

    /// Load the named cards, once. A card that will not read or load is logged
    /// and left out; with none left the game runs at once.
    pub(crate) fn load_cards(&mut self, assets: &mut AssetManager) {
        if self.stage != Stage::Pending {
            return;
        }
        for path in &self.paths {
            let Some(backdrop) = assets.image_backdrop(path) else { continue };
            match assets.load_texture_filtered(path, TextureFilter::Nearest) {
                Ok(texture) => self.cards.push(Card {
                    texture_id: texture.id,
                    size: backdrop.size,
                    backdrop: backdrop.corner,
                }),
                Err(error) => log::warn!("Startup card {path} does not load, skipped: {error}"),
            }
        }
        self.stage = if self.cards.is_empty() { Stage::Running } else { Stage::Splashing };
    }

    /// End the handoff frame: the game starts this frame, and its quiet period
    /// with it.
    pub(crate) fn leave_handoff(&mut self) {
        if self.stage == Stage::Handoff {
            self.stage = Stage::Running;
            self.quiet_remaining = QUIET_SECONDS;
        }
    }

    /// Run the current card's clock. `skip` is whether anything was pressed
    /// this frame; it ends the card only once the card has faded in, so a press
    /// made to focus the page, or queued while it loaded, cannot erase a card
    /// unseen. `paused` holds the clock — a page nobody can see.
    pub(crate) fn advance(&mut self, delta_time: f32, skip: bool, paused: bool) {
        if self.stage != Stage::Splashing || paused {
            return;
        }
        let faded_in = self.elapsed >= CARD_FADE_SECONDS;
        self.elapsed += delta_time;
        if (skip && faded_in) || self.elapsed >= CARD_SECONDS {
            self.current += 1;
            self.elapsed = 0.0;
            if self.current >= self.cards.len() {
                self.stage = Stage::Handoff;
            }
        }
    }

    /// Whether this frame falls in the quiet period, spending `delta_time` of it.
    pub(crate) fn take_quiet(&mut self, delta_time: f32) -> bool {
        if self.quiet_remaining <= 0.0 {
            return false;
        }
        self.quiet_remaining -= delta_time;
        true
    }

    /// Draw this frame: the black base, then, while a card shows, its backdrop
    /// around it and the card itself.
    pub(crate) fn draw(&self, ui: &mut UIContext, window_size: Vec2) {
        ui.rect(Rect::new(0.0, 0.0, window_size.x, window_size.y), Color::BLACK);
        if self.stage != Stage::Splashing {
            return;
        }
        let Some(card) = self.cards.get(self.current) else { return };
        let alpha = card_alpha(self.elapsed);
        let bounds = card_bounds(card.size, window_size);
        for margin in margins_around(bounds, window_size) {
            ui.rect(margin, card.backdrop.with_alpha(alpha));
        }
        ui.image(bounds, card.texture_id, Color::WHITE.with_alpha(alpha));
    }
}

/// A card's strength `elapsed` seconds into its life: up from black, held,
/// back down to black.
pub(crate) fn card_alpha(elapsed: f32) -> f32 {
    if elapsed < CARD_FADE_SECONDS {
        elapsed / CARD_FADE_SECONDS
    } else if elapsed < CARD_FADE_SECONDS + CARD_HOLD_SECONDS {
        1.0
    } else {
        ((CARD_SECONDS - elapsed) / CARD_FADE_SECONDS).clamp(0.0, 1.0)
    }
}

/// The whole scale a card of `size` is drawn at in this window: the largest up
/// to [`MAX_CARD_SCALE`] that fits, and never below 1 — a window smaller than
/// the card crops it rather than losing it.
pub(crate) fn card_scale(size: UVec2, window_size: Vec2) -> u32 {
    let fits = |window: f32, card: u32| (window / card.max(1) as f32).floor() as u32;
    MAX_CARD_SCALE.min(fits(window_size.x, size.x)).min(fits(window_size.y, size.y)).max(1)
}

/// Where a card of `size` is drawn: at its whole scale, centred on whole pixels.
pub(crate) fn card_bounds(size: UVec2, window_size: Vec2) -> Rect {
    let scale = card_scale(size, window_size) as f32;
    let drawn = Vec2::new(size.x as f32, size.y as f32) * scale;
    let origin = ((window_size - drawn) / 2.0).floor();
    Rect::new(origin.x, origin.y, drawn.x, drawn.y)
}

/// The parts of the window outside `bounds`, as up to four rectangles that
/// neither overlap each other nor `bounds`: full-height strips left and right,
/// and strips above and below between them.
pub(crate) fn margins_around(bounds: Rect, window_size: Vec2) -> Vec<Rect> {
    let left = bounds.x.max(0.0);
    let right = (bounds.x + bounds.width).min(window_size.x);
    let top = bounds.y.max(0.0);
    let bottom = (bounds.y + bounds.height).min(window_size.y);
    [
        Rect::new(0.0, 0.0, left, window_size.y),
        Rect::new(right, 0.0, window_size.x - right, window_size.y),
        Rect::new(left, 0.0, right - left, top),
        Rect::new(left, bottom, right - left, window_size.y - bottom),
    ]
    .into_iter()
    .filter(|margin| margin.width > 0.0 && margin.height > 0.0)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a_card_rises_from_black_holds_and_returns_to_it() {
        assert_eq!(card_alpha(0.0), 0.0);
        assert_eq!(card_alpha(CARD_FADE_SECONDS / 2.0), 0.5);
        assert_eq!(card_alpha(CARD_FADE_SECONDS), 1.0);
        assert_eq!(card_alpha(1.0), 1.0);
        assert_eq!(card_alpha(CARD_SECONDS - CARD_FADE_SECONDS / 2.0), 0.5);
        assert_eq!(card_alpha(CARD_SECONDS), 0.0);
        assert_eq!(CARD_SECONDS, 2.0);
    }

    #[test]
    fn test_a_card_draws_at_the_largest_whole_scale_up_to_two_and_never_below_one() {
        let splash = UVec2::new(320, 192);
        assert_eq!(card_scale(splash, Vec2::new(800.0, 600.0)), 2);
        assert_eq!(card_scale(splash, Vec2::new(720.0, 768.0)), 2);
        assert_eq!(card_scale(splash, Vec2::new(1920.0, 1080.0)), 2, "never past two");
        assert_eq!(card_scale(splash, Vec2::new(600.0, 400.0)), 1);
        assert_eq!(card_scale(splash, Vec2::new(300.0, 180.0)), 1, "a smaller window crops");
        assert_eq!(card_bounds(splash, Vec2::new(800.0, 600.0)), Rect::new(80.0, 108.0, 640.0, 384.0));
        assert_eq!(card_bounds(splash, Vec2::new(300.0, 180.0)), Rect::new(-10.0, -6.0, 320.0, 192.0));
    }

    #[test]
    fn test_the_margins_tile_the_window_around_the_card_without_touching_it() {
        for window in [Vec2::new(800.0, 600.0), Vec2::new(720.0, 768.0), Vec2::new(300.0, 180.0)] {
            let bounds = card_bounds(UVec2::new(320, 192), window);
            let margins = margins_around(bounds, window);
            let overlaps = |a: &Rect, b: &Rect| {
                a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
            };
            for (index, margin) in margins.iter().enumerate() {
                assert!(!overlaps(margin, &bounds), "{window:?}: {margin:?} is under the card");
                for other in &margins[index + 1..] {
                    assert!(!overlaps(margin, other), "{window:?}: two margins overlap");
                }
            }
            let visible_card = (bounds.width.min(window.x) * bounds.height.min(window.y)).max(0.0);
            let covered: f32 = margins.iter().map(|margin| margin.width * margin.height).sum::<f32>() + visible_card;
            assert_eq!(covered, window.x * window.y, "{window:?}: card and margins cover the window");
        }
    }

    #[test]
    fn test_a_skip_counts_only_once_the_card_has_faded_in() {
        let mut splash = StartupSplash::new(vec!["a.png".into(), "b.png".into()]);
        splash.cards = vec![Card { texture_id: 1, size: UVec2::ONE, backdrop: Color::BLACK }; 2];
        splash.stage = Stage::Splashing;
        splash.advance(0.1, true, false);
        assert_eq!((splash.current, splash.stage), (0, Stage::Splashing), "a press on the first frame");
        splash.advance(0.2, false, false);
        splash.advance(0.016, true, false);
        assert_eq!((splash.current, splash.elapsed), (1, 0.0), "a press after the fade-in skips");
        splash.advance(10.0, false, true);
        assert_eq!(splash.elapsed, 0.0, "a hidden page holds the clock");
        splash.advance(CARD_SECONDS, false, false);
        assert_eq!(splash.stage, Stage::Handoff, "the last card ends in the handoff");
        splash.leave_handoff();
        assert_eq!(splash.stage, Stage::Running);
        assert!(splash.take_quiet(0.1) && splash.take_quiet(0.15) && !splash.take_quiet(0.1));
    }
}
