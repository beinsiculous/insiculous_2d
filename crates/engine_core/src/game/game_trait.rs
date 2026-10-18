//! The `Game` trait: the contract a game implements and the engine drives.

use winit::keyboard::KeyCode;

use crate::achievements::AchievementManager;
use crate::contexts::{GameContext, RenderContext};
use crate::localization::Strings;
use crate::ui_integration::render_ui_commands;
use ecs::sprite_components::{Sprite as EcsSprite, Transform2D};
use renderer::texture::TextureHandle;

use super::render;

/// The main game trait. Implement this to create your game.
///
/// Only `update` is required - all other methods have default implementations.
pub trait Game: Sized + 'static {
    /// Register the game's achievements. Called once by `run_game` before the window opens, so the
    /// registry exists ahead of `init` and can be exported with no GPU (`--achievements-manifest`).
    /// `strings` carries the configured locale; a game whose names are localized re-registers from
    /// its own code on a locale switch, as before.
    fn register_achievements(&self, _achievements: &mut AchievementManager, _strings: &Strings) {}

    /// Called once when the game starts, after the window and renderer are ready
    /// and before the first `update` or a key handler reaches the game (a resize
    /// may arrive earlier: `on_resize` must not assume `init` has run). Use this
    /// to set up your initial game state, create entities, load assets, etc. Set
    /// the default font here: the font in place when `init` returns is the one a
    /// locale switch restores to. `ctx.delta_time` is always zero here — no frame
    /// has elapsed.
    fn init(&mut self, _ctx: &mut GameContext) {}

    /// Called every frame. Update your game logic here.
    /// This is the only required method.
    fn update(&mut self, ctx: &mut GameContext);

    /// Called every frame to render sprites. Add sprites to `ctx.sprites`.
    /// Default implementation extracts sprites from ECS entities with Transform2D and Sprite components,
    /// then renders UI draw commands on top.
    fn render(&mut self, ctx: &mut RenderContext) {
        // Tilemaps first, so equal-depth entity sprites draw over tiles
        crate::tilemap_render::append_tilemap_sprites(ctx.world, ctx.sprites);

        // Default: extract sprites from ECS
        for entity_id in ctx.world.entities() {
            let sprite = ctx.world.get::<EcsSprite>(entity_id);

            if let Some(ecs_sprite) = sprite {
                if !ecs_sprite.visible { continue; }
                // Use GlobalTransform2D if available (for hierarchical entities),
                // otherwise fall back to local Transform2D
                let (position, rotation, scale) = if let Some(global) = ctx.world.get::<ecs::hierarchy::GlobalTransform2D>(entity_id) {
                    (global.position, global.rotation, global.scale)
                } else if let Some(transform) = ctx.world.get::<Transform2D>(entity_id) {
                    (transform.position, transform.rotation, transform.scale)
                } else {
                    continue; // No transform, skip this entity
                };

                // Use the texture handle from the ECS sprite component
                let texture = TextureHandle { id: ecs_sprite.texture_handle };
                // The region carries sheet-cell UVs for animated and
                // sub-image sprites; the default [0, 0, 1, 1] is the full
                // texture, so unanimated content renders exactly as before.
                let region = ecs_sprite.tex_region;
                let renderer_sprite = renderer::Sprite::new(texture)
                    .with_position(render::sprite_draw_position(position, rotation, ecs_sprite.offset))
                    .with_rotation(rotation)
                    .with_scale(scale * ecs_sprite.scale * crate::RENDER_UNIT)
                    .with_tex_region(region[0], region[1], region[2], region[3])
                    .with_color(ecs_sprite.color)
                    .with_depth(ecs_sprite.depth)
                    .with_emissive(ecs_sprite.emissive);

                ctx.sprites.add_sprite(&renderer_sprite);
            }
        }

        // Render UI draw commands on top
        render_ui_commands(ctx.sprites, ctx.ui_commands, &*ctx.camera, ctx.glyph_textures);
    }

    /// Called by the editor when play mode is stopped and the world has been
    /// restored to its pre-play state. Override this to reset any non-ECS state
    /// (e.g., physics world) that was modified during play.
    fn on_play_stopped(&mut self, _ctx: &mut GameContext) {}

    /// Called when a key is pressed. Override for custom key handling.
    fn on_key_pressed(&mut self, _key: KeyCode, _ctx: &mut GameContext) {}

    /// Called when a key is released. Override for custom key handling.
    fn on_key_released(&mut self, _key: KeyCode, _ctx: &mut GameContext) {}

    /// Called when the window is resized.
    fn on_resize(&mut self, _width: u32, _height: u32) {}

    /// Called when the game is about to exit. Clean up resources here.
    fn on_exit(&mut self) {}

    /// Register custom script behaviors with the script runner's registry.
    #[cfg(feature = "physics")]
    fn register_scripts(&mut self, _registry: &mut crate::scripting::ScriptRegistry) {}
}
