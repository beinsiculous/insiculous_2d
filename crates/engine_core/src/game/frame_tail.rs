//! GameRunner's post-update frame tail: everything the engine does after
//! the game's `update()` returns — particle stepping, line forwarding,
//! scene-defined UI elements, achievement toasts, and locale-font capture.
//!
//! Child module of `game` (like `render`) so it can reach the runner's
//! private fields without widening visibility.

use ecs::System as _;
use glam::Vec2;

use super::{Game, GameRunner};

impl<G: Game> GameRunner<G> {
    /// Engine-side work that runs right after `game.update()` each frame.
    pub(super) fn post_update(&mut self, delta_time: f32, window_size: Vec2, first_frame: bool) {
        self.step_simulations(delta_time);
        self.draw_scene_ui(window_size, delta_time);
        self.apply_frame_requests(first_frame);
    }

    /// Step simulation systems scaled by `time_scale`.
    ///
    /// Emitter accumulators see latest transforms, and pool stepping happens once per frame.
    /// Scaled by `time_scale` so a paused game (time_scale 0.0) freezes its particles,
    /// sprite animations, and spring grids with the rest of the world.
    fn step_simulations(&mut self, delta_time: f32) {
        let scaled_delta = delta_time * self.time_scale;

        crate::particles::ParticleSystem::update(
            &mut self.scene.world,
            &mut self.particles,
            scaled_delta,
        );

        ecs::SpriteAnimationSystem.update(&mut self.scene.world, scaled_delta);

        // Spring grid vertices spliced in front of the game's lines so its wireframes stay on top.
        self.grid_backdrops
            .update(&mut self.scene.world, scaled_delta, &mut self.lines);

        self.render_manager.set_lines(&self.lines);
    }

    /// Draw scene-authored UI elements, toasts, and tick achievements.
    ///
    /// An editor-style host clips these tail draws to its game view via `ctx.clip_engine_ui`
    /// (plain games leave it None — unclipped). UI element presses buffer until the next
    /// frame's event flush.
    fn draw_scene_ui(&mut self, window_size: Vec2, delta_time: f32) {
        if let Some(clip) = self.requests.engine_ui_clip {
            self.ui
                .push_clip_rect(ui::Rect::new(clip.x, clip.y, clip.width, clip.height));
        }
        let ui_presses = crate::ui_element_system::draw_ui_elements(
            &self.scene.world,
            &mut self.ui,
            window_size,
            &self.localization.strings,
        );
        self.pending_ui_events.extend(ui_presses);

        self.achievements.draw_toasts(&mut self.ui, window_size);
        self.achievements.tick(delta_time);
        if self.requests.engine_ui_clip.take().is_some() {
            self.ui.pop_clip_rect();
        }
    }

    /// Apply per-frame requests made during init or update.
    fn apply_frame_requests(&mut self, first_frame: bool) {
        // At most one window-system round-trip per frame, only when requested.
        if let Some(title) = self.requests.window_title.take() {
            self.window_manager.set_title(&title);
        }

        // The font the game set up in init() is the one locale switches
        // restore to — capture it once, before any locale font applies.
        if first_frame {
            self.localization.base_font = self.ui.default_font();
        }
        self.apply_locale_font();
    }
}
