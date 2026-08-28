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
        // Prepare glyph textures for text rendering
        if let Some(asset_manager) = &mut self.asset_manager {
            self.glyph_textures.prepare(ui_commands, asset_manager);
        }

        // Phase 1: Game sprites — render into their own batcher so they never
        // share a batch with UI elements (which would cause UI panel backgrounds
        // to paint over game sprites due to painter's algorithm). The batchers
        // are persistent fields: clear() retains capacity, so a steady-state
        // frame allocates nothing here (GPP-15).
        self.game_batcher.clear();
        // A main-camera entity (Camera { is_main_camera } + Transform2D)
        // drives the render camera; games can still override ctx.camera below.
        self.render_manager.sync_main_camera(&self.scene.world);
        {
            let empty_commands: &[DrawCommand] = &[];
            let mut ctx = RenderContext {
                world: &self.scene.world,
                sprites: &mut self.game_batcher,
                camera: self.render_manager.camera_mut(),
                window_size,
                ui_commands: empty_commands,
                glyph_textures: self.glyph_textures.textures(),
            };
            self.game.render(&mut ctx);
        }

        // Append particle sprites into the game batcher. Particles render
        // after gameplay sprites so they appear on top of static objects
        // but below UI.
        append_particle_sprites(&mut self.game_batcher, &self.particles);

        // Phase 2: UI sprites — separate batcher. Conversion is
        // camera-relative so UI stays at fixed screen pixels even when the
        // game (or editor) moves/zooms the camera during Phase 1.
        self.ui_batcher.clear();
        render_ui_commands(
            &mut self.ui_batcher,
            ui_commands,
            self.render_manager.camera(),
            self.glyph_textures.textures(),
        );

        // Sort within each batch, then order the batch refs (game first, then
        // UI on top; by min depth then texture handle for determinism). Refs
        // only — batches are never cloned. A persistent batcher can hold
        // now-empty batches for textures with no sprites this frame; skip them.
        self.game_batcher.sort_all_batches();
        self.ui_batcher.sort_all_batches();
        let mut batch_refs: Vec<&SpriteBatch> =
            self.game_batcher.batches().values().filter(|b| !b.instances.is_empty()).collect();
        Self::sort_batch_refs(&mut batch_refs);
        // UI batches stay separate: they draw in their own post-tonemap
        // pass so authored UI colors display exactly (issue #26).
        let mut ui_batch_refs: Vec<&SpriteBatch> =
            self.ui_batcher.batches().values().filter(|b| !b.instances.is_empty()).collect();
        Self::sort_batch_refs(&mut ui_batch_refs);

        // Get textures from asset manager (need to reborrow after RenderContext)
        if let Some(asset_manager) = &self.asset_manager {
            let textures = asset_manager.textures();
            if let Err(e) = self.render_manager.render(&batch_refs, &ui_batch_refs, textures) {
                log::error!("Render error: {}", e);
            }
        }
    }

    /// Sort sprite batch refs by depth (min, then max, then texture handle for determinism).
    fn sort_batch_refs(batches: &mut [&SpriteBatch]) {
        batches.sort_by(|a, b| {
            let a_min = a.instances.iter().map(|i| i.depth).min_by(|x, y| x.total_cmp(y)).unwrap_or(0.0);
            let b_min = b.instances.iter().map(|i| i.depth).min_by(|x, y| x.total_cmp(y)).unwrap_or(0.0);
            a_min.total_cmp(&b_min)
                .then_with(|| {
                    let a_max = a.instances.iter().map(|i| i.depth).max_by(|x, y| x.total_cmp(y)).unwrap_or(0.0);
                    let b_max = b.instances.iter().map(|i| i.depth).max_by(|x, y| x.total_cmp(y)).unwrap_or(0.0);
                    a_max.total_cmp(&b_max)
                })
                .then_with(|| a.texture_handle.id.cmp(&b.texture_handle.id))
        });
    }
}
