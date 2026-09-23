//! GameRunner's post-update frame tail: everything the engine does after
//! the game's `update()` returns — particle stepping, line forwarding,
//! scene-defined UI elements, achievement toasts, and locale-font capture.
//!
//! Child module of `game` (like `render`) so it can reach the runner's
//! private fields without widening visibility.

use ecs::{System as _, World};
use glam::Vec2;

use super::{Game, GameRunner};

/// The world systems the frame tail runs, in order, on the time-scaled delta.
///
/// Animation first (a clip that stopped this frame is already stopped when
/// the machine looks at it), then lifetimes — so an expired entity never
/// animates or transitions again — then the clip machine, which applies a
/// finished clip's `Next`/`Despawn` in the frame it finished and re-asserts
/// the current state's clip. Every game and the playground get all three;
/// a game that owns its own `LifetimeSystem` would halve every lifetime, so
/// it must not.
pub(crate) fn step_world_systems(world: &mut World, delta_time: f32) {
    ecs::SpriteAnimationSystem.update(world, delta_time);
    ecs::LifetimeSystem.update(world, delta_time);
    ecs::ClipStateMachineSystem.update(world, delta_time);
}

impl<G: Game> GameRunner<G> {
    /// Engine-side work that runs right after `game.update()` each frame.
    pub(super) fn post_update(&mut self, delta_time: f32, window_size: Vec2) {
        self.step_simulations(delta_time);
        self.draw_scene_ui(window_size, delta_time);
        self.apply_frame_requests();
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

        step_world_systems(&mut self.scene.world, scaled_delta);

        // Each grid's vertices go to the layer it asked for: over-sprites
        // grids in front of the game's own lines so its wireframes stay on
        // top, behind-sprites grids in the buffer the renderer draws first.
        self.grid_backdrops.update(
            &mut self.scene.world,
            scaled_delta,
            &mut self.lines,
            &mut self.behind_lines,
        );

        self.render_manager.set_lines(&self.lines, &self.behind_lines);
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
    fn apply_frame_requests(&mut self) {
        // At most one window-system round-trip per frame, only when requested.
        if let Some(title) = self.requests.window_title.take() {
            self.window_manager.set_title(&title);
        }

        // The pointer shape is the UI's ask rather than the game's, so it
        // comes straight off the context instead of `FrameRequests`. The
        // manager drops a repeat, which is what keeps an editor whose
        // pointer never moves off a button from asking every frame.
        self.window_manager.set_cursor(self.ui.requested_cursor());

        // The base font a locale switch restores to was captured right after
        // `init()` (`initialize_if_needed`), before any locale font applied.
        self.apply_locale_font();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecs::sprite_components::{AnimationClip, SheetGrid, SpriteAnimation};
    use ecs::{ClipState, ClipStateMachine, Lifetime, OnFinished};

    /// A three-cell sheet: `close` is a one-shot, `hum` loops.
    fn animation() -> SpriteAnimation {
        SpriteAnimation::new(SheetGrid::new(3, 1))
            .with_clip("close", AnimationClip::new(vec![0, 1, 2], 10.0).with_looping(false))
            .with_clip("hum", AnimationClip::new(vec![0], 10.0))
    }

    #[test]
    fn the_tail_expires_lifetimes_and_moves_a_finished_clip_on_in_the_same_step() {
        let mut world = World::new();
        let door = world.create_entity();
        world.add_component(&door, animation()).ok();
        world
            .add_component(
                &door,
                ClipStateMachine::new(
                    "closing",
                    vec![
                        (
                            "closing".to_string(),
                            ClipState::new("close", OnFinished::Next("open".to_string())),
                        ),
                        ("open".to_string(), ClipState::staying("hum")),
                    ],
                ),
            )
            .ok();
        let bullet = world.create_entity();
        world.add_component(&bullet, Lifetime::new(0.2)).ok();

        // Frame 1: the machine selects its state's clip; the lifetime holds.
        step_world_systems(&mut world, 0.1);
        assert_eq!(world.get::<ClipStateMachine>(door).expect("machine").state(), "closing");
        assert!(world.entities().contains(&bullet), "0.1s of a 0.2s lifetime is not expiry");

        // Frame 2: the lifetime crosses zero with no game code involved.
        step_world_systems(&mut world, 0.1);
        assert!(!world.entities().contains(&bullet), "the tail expires a lifetime on its own");

        // Frames 3 and 4: the play-once clip stops and, in that same step,
        // the machine is in the next state playing its clip.
        step_world_systems(&mut world, 0.1);
        step_world_systems(&mut world, 0.1);
        assert_eq!(world.get::<ClipStateMachine>(door).expect("machine").state(), "open");
        let animation = world.get::<SpriteAnimation>(door).expect("animation");
        assert_eq!(animation.current_clip.as_deref(), Some("hum"));
        assert!(animation.playing, "the next state's clip is already playing");
    }
}
