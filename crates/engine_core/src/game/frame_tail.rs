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
        // Step the particle system after the game's update — emitter
        // accumulators see the latest transforms, and pool stepping
        // happens once per frame. Scaled by time_scale so a paused game
        // (time_scale 0.0) freezes its particles with the rest of the world.
        crate::particles::ParticleSystem::update(
            &mut self.scene.world,
            &mut self.particles,
            delta_time * self.time_scale,
        );

        // Advance named-clip sprite animations and stamp the resulting cell
        // region onto each Sprite. Same time-scaled delta as the particles,
        // so pausing (time_scale 0.0) freezes animation with the world.
        ecs::SpriteAnimationSystem.update(&mut self.scene.world, delta_time * self.time_scale);

        // Scene-authored spring grids: same time-scaled delta (the editor's
        // freeze holds them still, still drawn), vertices spliced in FRONT
        // of the game's so its wireframes stay on top (#46).
        self.grid_backdrops
            .update(&mut self.scene.world, delta_time * self.time_scale, &mut self.lines);

        // Forward the line vertices the game pushed during update to the
        // renderer. Empty buffer == no lines drawn this frame.
        self.render_manager.set_lines(&self.lines);

        // Draw scene-defined UI elements (labels/panels/buttons) over the
        // game's own UI; presses buffer until the next frame's event flush.
        // An editor-style host clips these tail draws to its game view via
        // ctx.game_ui_clip (plain games leave it None — unclipped).
        if let Some(clip) = self.pending_game_ui_clip {
            self.ui
                .push_clip_rect(ui::Rect::new(clip.x, clip.y, clip.width, clip.height));
        }
        let ui_presses = crate::ui_element_system::draw_ui_elements(
            &self.scene.world,
            &mut self.ui,
            window_size,
            &self.strings,
        );
        self.pending_ui_events.extend(ui_presses);

        // Draw achievement toasts on top of whatever the game drew.
        self.achievements
            .draw_toasts(&mut self.ui, window_size);
        self.achievements.tick(delta_time);
        if self.pending_game_ui_clip.take().is_some() {
            self.ui.pop_clip_rect();
        }

        // Apply a window title requested via ctx.window_title this frame
        // (editor dirty indicator, save-as renames). At most one
        // window-system round-trip per frame, only when requested.
        if let Some(title) = self.pending_window_title.take() {
            self.window_manager.set_title(&title);
        }

        // The font the game set up in init() is the one locale switches
        // restore to — capture it once, before any locale font applies.
        if first_frame {
            self.base_font = self.ui.default_font();
        }
        // Apply a pending locale font change (from init/update set_locale).
        self.apply_locale_font();
    }
}
