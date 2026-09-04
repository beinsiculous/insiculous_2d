//! Frame rendering tail of the game loop, split out of `game.rs`.
//!
//! Owns sprite-batch assembly and submission: game sprites, particles,
//! UI sprites, batch ordering, and the final render call.

use glam::Vec2;

use renderer::{
    sprite::{SpriteBatch, SpriteBatcher},
    texture::TextureHandle,
};
use ui::DrawCommand;

use crate::contexts::RenderContext;
use crate::ui_integration::render_ui_commands;

use super::{Game, GameRunner};

/// Append the manager's alive particles to a [`SpriteBatcher`].
///
/// Called from the engine after `Game::render` so particles always render,
/// regardless of whether the game overrides the default render impl.
fn append_particle_sprites(
    batcher: &mut SpriteBatcher,
    particles: &crate::particles::ParticleManager,
) {
    for p in particles.iter_alive() {
        let color = crate::particles::ParticleManager::current_color(p);
        let scale = crate::particles::ParticleManager::current_scale(p);
        let sprite = renderer::Sprite::new(TextureHandle { id: p.texture })
            .with_position(p.position)
            .with_rotation(p.rotation)
            .with_scale(Vec2::splat(scale))
            .with_color(color)
            .with_emissive(p.emissive)
            // Just behind UI (positive depth) so particles glow on top of gameplay.
            .with_depth(0.5);
        batcher.add_sprite(&sprite);
    }
}

impl<G: Game> GameRunner<G> {
    /// Render complete frame with sprites and UI
    pub(super) fn render_frame(&mut self, window_size: Vec2, ui_commands: &[DrawCommand]) {
        if let Some(asset_manager) = &mut self.asset_manager {
            self.glyph_textures.prepare(ui_commands, asset_manager);
        }

        self.collect_game_sprites(window_size);
        self.collect_ui_sprites(ui_commands);
        self.submit_frame();
    }

    fn collect_game_sprites(&mut self, window_size: Vec2) {
        // Game sprites — render into their own batcher so they never
        // share a batch with UI elements (which would cause UI panel backgrounds
        // to paint over game sprites due to painter's algorithm). The batchers
        // are persistent fields: clear() retains capacity, so a steady-state
        // frame allocates nothing here.
        self.game_batcher.clear();
        // A main-camera entity (Camera { is_main_camera } + Transform2D)
        // drives the render camera; games can still override ctx.camera below.
        self.render_manager.sync_main_camera(&self.scene.world);
        let mut viewport_scissor: Option<common::Rect> = None;
        {
            let empty_commands: &[DrawCommand] = &[];
            let mut ctx = RenderContext {
                world: &self.scene.world,
                sprites: &mut self.game_batcher,
                camera: self.render_manager.camera_mut(),
                window_size,
                ui_commands: empty_commands,
                glyph_textures: self.glyph_textures.textures(),
                viewport_scissor: &mut viewport_scissor,
            };
            self.game.render(&mut ctx);
        }
        // Editor-style hosts bound the game-world passes to a sub-rect of
        // the window; plain games leave it None (full window). Forwarded
        // every frame — per-frame state, like set_lines.
        self.render_manager.set_viewport_scissor(viewport_scissor.map(|rect| {
            renderer::scissor::quantize_rect(rect.x, rect.y, rect.width, rect.height)
        }));

        // Append particle sprites into the game batcher. Particles render
        // after gameplay sprites so they appear on top of static objects
        // but below UI.
        append_particle_sprites(&mut self.game_batcher, &self.particles);
    }

    fn collect_ui_sprites(&mut self, ui_commands: &[DrawCommand]) {
        // UI sprites — separate batcher. Conversion is camera-relative so UI
        // stays at fixed screen pixels even when the game (or editor) moves/zooms
        // the camera.
        self.ui_batcher.clear();
        render_ui_commands(
            &mut self.ui_batcher,
            ui_commands,
            self.render_manager.camera(),
            self.glyph_textures.textures(),
        );
    }

    fn submit_frame(&mut self) {
        // Sort within each batch, then order the batch refs (game first, then
        // UI on top; by min depth then texture handle for determinism). Refs
        // only — batches are never cloned. A persistent batcher can hold
        // now-empty batches for textures with no sprites this frame; skip them.
        self.game_batcher.sort_all_batches();
        self.ui_batcher.sort_all_batches();
        let mut batch_refs: Vec<&SpriteBatch> =
            self.game_batcher.batches().values().filter(|batch| !batch.instances.is_empty()).collect();
        Self::sort_batch_refs(&mut batch_refs);
        // UI batches stay separate: they draw in their own post-tonemap
        // pass so authored UI colors display exactly.
        let mut ui_batch_refs: Vec<&SpriteBatch> =
            self.ui_batcher.batches().values().filter(|batch| !batch.instances.is_empty()).collect();
        Self::sort_batch_refs(&mut ui_batch_refs);

        if let Some(asset_manager) = &self.asset_manager {
            let textures = asset_manager.textures();
            if let Err(error) = self.render_manager.render(&batch_refs, &ui_batch_refs, textures) {
                if matches!(error, renderer::RendererError::DeviceLost) {
                    // Fail-stop: the frame driver halts the loop instead of
                    // submitting to a dead queue every rAF (which is what
                    // crashed Firefox's in-process WebGPU).
                    self.render_fatal = true;
                    log::error!("Fatal: graphics device lost — stopping the frame loop");
                } else {
                    log::error!("Render error: {error}");
                }
            }
        }
    }

    /// Sort sprite batch refs by depth (min, then max, then texture handle for determinism).
    fn sort_batch_refs(batches: &mut [&SpriteBatch]) {
        let mut keyed: Vec<_> = batches
            .iter()
            .map(|&batch| {
                let min = batch
                    .instances
                    .iter()
                    .map(|i| i.depth)
                    .min_by(|x, y| x.total_cmp(y))
                    .unwrap_or(0.0);
                let max = batch
                    .instances
                    .iter()
                    .map(|i| i.depth)
                    .max_by(|x, y| x.total_cmp(y))
                    .unwrap_or(0.0);
                ((min, max, batch.texture_handle.id, batch.clip), batch)
            })
            .collect();
        keyed.sort_by(|(a, _), (b, _)| {
            a.0.total_cmp(&b.0)
                .then_with(|| a.1.total_cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
                .then_with(|| a.3.cmp(&b.3))
        });
        for (slot, (_, batch)) in batches.iter_mut().zip(keyed) {
            *slot = batch;
        }
    }
}
