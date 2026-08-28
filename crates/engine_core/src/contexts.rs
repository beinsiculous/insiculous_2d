//! Core contexts for the Game API.
//!
//! This module provides the main context structures used by the Game trait
//! to give access to engine systems during the game loop.

use glam::Vec2;
use ecs::World;
use input::{InputHandler, InputSettings};
use audio::AudioManager;
use ui::UIContext;
use renderer::{line_pipeline::LineVertex, sprite::SpriteBatcher, Camera, texture::TextureHandle};
use std::collections::HashMap;
use crate::assets::AssetManager;
use crate::chaos_mode::ChaosMode;
use crate::achievements::AchievementManager;
use crate::particles::ParticleManager;

/// Key for caching glyph textures.
///
/// Note: Color is NOT included in the cache key because glyph textures are
/// grayscale alpha masks. The color is applied at render time by multiplying
/// the sprite color with the texture, allowing the same glyph texture to be
/// reused for any color.
///
/// The font id IS included: different fonts rasterize the same character at
/// the same bitmap size to different shapes, so a key without it would serve
/// stale glyphs after a locale font switch.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlyphCacheKey {
    /// Character being rendered
    character: char,
    /// Width of the glyph bitmap
    width: u32,
    /// Height of the glyph bitmap
    height: u32,
    /// Id of the font the glyph was rasterized from (`FontHandle.id`)
    font_id: u32,
}

impl GlyphCacheKey {
    pub(crate) fn new(character: char, width: u32, height: u32, font_id: u32) -> Self {
        Self {
            character,
            width,
            height,
            font_id,
        }
    }
}

/// Context passed to game methods, providing access to engine systems.
pub struct GameContext<'a> {
    /// Input handler for keyboard, mouse, and gamepad
    pub input: &'a InputHandler,
    /// Player-aware input bindings: the universal per-player mapping layer.
    /// Query with `ctx.players.is_active(PlayerId::P1, GameAction::Action1,
    /// ctx.input)` or `ctx.players.move_x(PlayerId::P2, ctx.input)`.
    /// Mutable so games can re-point pads at runtime (`assign_pad`); loaded
    /// from `GameConfig::input_settings_path` when set.
    pub players: &'a mut InputSettings,
    /// The ECS world for entity/component management
    pub world: &'a mut World,
    /// Asset manager for loading textures and other resources
    pub assets: &'a mut AssetManager,
    /// Audio manager for sound playback
    pub audio: &'a mut AudioManager,
    /// UI context for immediate-mode UI
    pub ui: &'a mut UIContext,
    /// Delta time since last frame in seconds
    pub delta_time: f32,
    /// Current window size
    pub window_size: Vec2,
    /// Project-wide gameplay intensity theme. Seeded from `GameConfig` and
    /// **read-write**: assign to it when the player picks a mode at runtime
    /// and the engine persists the change, so `ctx.chaos_mode` is always the
    /// current selection on later frames (no stale startup value).
    pub chaos_mode: ChaosMode,
    /// Engine time multiplier, **read-write** like `chaos_mode` (the engine
    /// persists writes across frames). Currently scales engine-side particle
    /// stepping only — set it to `0.0` while paused so bursts freeze with the
    /// game (`PauseMenu::time_scale()` provides the right value each frame).
    /// It does NOT scale `ctx.delta_time`; games gate their own logic.
    pub time_scale: f32,
    /// Set to `true` to quit the game (title-screen Exit items, the pause
    /// menu's Exit Game). The engine performs the same clean shutdown as
    /// closing the window: `Game::on_exit`, input-settings save, scene
    /// teardown, then the event loop exits at the end of the frame.
    pub exit_requested: bool,
    /// Write `Some(title)` to retitle the OS window; the engine applies it
    /// after `update()` (same writeback pattern as `exit_requested`).
    /// `None` (the default every frame) leaves the title unchanged — only
    /// write on change, since retitling is a window-system round-trip.
    pub window_title: Option<String>,
    /// Achievement / trophy manager. Register achievements in `init()`, then
    /// call `ctx.achievements.unlock("id")` from gameplay code.
    pub achievements: &'a mut AchievementManager,
    /// High-score lists (top-N per game-defined mode string). Call
    /// `ctx.scores.submit("single", score)` at game over; persisted when
    /// `GameConfig::score_save_path` is set.
    pub scores: &'a mut crate::scores::Scores,
    /// Particle system. Spawn bursts directly with
    /// `ctx.particles.spawn_burst(pos, &config)`, or attach a
    /// [`ParticleEmitter`](crate::particles::ParticleEmitter) component to
    /// any entity with a `Transform2D` for continuous emission.
    pub particles: &'a mut ParticleManager,
    /// Line-list vertex buffer for the line render pipeline. Pairs of
    /// vertices form line segments. Cleared each frame before `update()`.
    /// Typical use: step a [`GridMesh`](crate::grid::GridMesh) and append
    /// its `build_line_vertices()` output here, or push debug-draw segments.
    pub lines: &'a mut Vec<LineVertex>,
    /// Localization tables. `ctx.strings.tr("menu.play")` translates a key
    /// in the current locale (fallback: `en` → the key itself); mutable so
    /// games can switch locales at runtime (`set_locale`/`cycle_locale` —
    /// the engine applies any per-locale font after `update()`).
    pub strings: &'a mut crate::localization::Strings,
}

/// Render context passed to the render method.
pub struct RenderContext<'a> {
    /// The ECS world (read-only during render)
    pub world: &'a World,
    /// Sprite batcher for adding sprites to render
    pub sprites: &'a mut SpriteBatcher,
    /// The 2D camera
    pub camera: &'a mut Camera,
    /// Current window size
    pub window_size: Vec2,
    /// UI draw commands to render
    pub ui_commands: &'a [ui::DrawCommand],
    /// Cached glyph textures for text rendering
    pub glyph_textures: &'a HashMap<GlyphCacheKey, TextureHandle>,
    /// Writeback: bound the game-world passes (sprites, lines, bloom) to
    /// this rect in **physical surface pixels** (the same space as
    /// `window_size`). `None` (the default) renders full-window — shipped
    /// games never touch this. The editor writes its scene-panel bounds so
    /// the game stops painting over editor chrome (issue #41); a zero-size
    /// rect draws no game world at all (hidden scene panel). The UI pass is
    /// unaffected.
    pub viewport_scissor: &'a mut Option<common::Rect>,
}