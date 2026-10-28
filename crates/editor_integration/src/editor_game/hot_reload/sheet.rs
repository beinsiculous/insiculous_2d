//! `ReloadSheet`: a `.sheet.ron` edited on disk reaches every animation cut from its PNG,
//! the grid, the clips and the texture together, as one transaction; and Stop puts what a
//! play session reloaded back on the world it restores.

use std::collections::BTreeSet;

use ecs::sprite_components::{AnimationClip, Sprite, SpriteAnimation};
use ecs::World;
use editor::console::Severity;
use engine_core::assets::AssetManager;
use engine_core::Game;

use super::super::EditorGame;

/// A sheet as reloaded: what went into the world, kept by a play session for Stop.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::editor_game) struct ReloadedSheet {
    pub grid: common::SheetGrid,
    pub clips: Vec<(String, AnimationClip)>,
    /// The texture handles the sheet's sprites held before, each one this PNG's: the only
    /// handles a reload replaces, so a sprite showing another texture keeps it.
    pub old_handles: BTreeSet<u32>,
    pub new_handle: u32,
}

impl ReloadedSheet {
    /// This reload on top of an earlier one in the same session: every handle either
    /// replaced is replaced by the newest.
    fn after(mut self, earlier: &ReloadedSheet) -> Self {
        self.old_handles.extend(&earlier.old_handles);
        self.old_handles.insert(earlier.new_handle);
        self.old_handles.remove(&self.new_handle);
        self
    }
}

/// Put `sheet` into every `SpriteAnimation` naming `png`: its grid and clips, and its texture
/// on the entity's `Sprite` where that sprite holds one of the sheet's old handles. A playing
/// clip's frame past its new length clamps to the last, and a clip the sheet no longer has is
/// left selected, its sprite showing the frame it last drew; each comes back as a sentence.
/// No edit and nothing dirty: the sidecar is the sheet's source of truth on every load.
pub(in crate::editor_game) fn apply_sheet(world: &mut World, png: &str, sheet: &ReloadedSheet) -> Vec<String> {
    let mut clamps = Vec::new();
    for entity in world.entities() {
        let Some(animation) = world.get_mut::<SpriteAnimation>(entity) else { continue };
        if animation.sheet.as_deref() != Some(png) {
            continue;
        }
        animation.grid = sheet.grid;
        animation.clips = sheet.clips.clone();
        if let Some(clamp) = clamp_current_frame(animation) {
            clamps.push(clamp);
        }
        if let Some(sprite) = world.get_mut::<Sprite>(entity) {
            if sheet.old_handles.contains(&sprite.texture_handle) {
                sprite.texture_handle = sheet.new_handle;
            }
        }
    }
    clamps
}

fn clamp_current_frame(animation: &mut SpriteAnimation) -> Option<String> {
    let clip_name = animation.current_clip.clone()?;
    let Some(frame_count) = animation
        .clips
        .iter()
        .find(|(name, _)| *name == clip_name)
        .map(|(_, clip)| clip.frame_indices.len())
    else {
        return Some(format!("clip '{clip_name}' is no longer in the sheet; the sprite keeps its last frame"));
    };
    let last_frame = frame_count.checked_sub(1)?;
    if animation.current_frame <= last_frame {
        return None;
    }
    let was = animation.current_frame;
    animation.current_frame = last_frame;
    Some(format!("clip '{clip_name}' now has {frame_count} frame(s); its frame {was} clamped to {last_frame}"))
}

impl<G: Game> EditorGame<G> {
    /// Reload the sheet cut from `png`: its sidecar parsed and its texture loaded before
    /// anything live changes, so a failure keeps the old sheet and says why. In a play
    /// session the result is kept for Stop.
    pub(in crate::editor_game) fn reload_sheet(&mut self, world: &mut World, assets: &mut AssetManager, png: &str) {
        let loaded = match assets.load_sprite_sheet(png) {
            Ok(loaded) => loaded,
            Err(error) => {
                let message = format!("{png}: the sheet did not reload, the old one is kept — {error}");
                self.report(Severity::Error, message, None);
                return;
            }
        };
        assets.clear_sidecar_cache();
        let reloaded = ReloadedSheet {
            old_handles: sheet_handles(world, assets, png, loaded.texture.id),
            grid: loaded.grid,
            clips: loaded.clips,
            new_handle: loaded.texture.id,
        };
        for clamp in apply_sheet(world, png, &reloaded) {
            log::warn!("{png}: {clamp}");
            self.report(Severity::Info, format!("{png}: {clamp}"), None);
        }
        self.report(Severity::Info, format!("{png}: sheet reloaded"), None);
        if self.editor.in_play_session() {
            let sessions = &mut self.hot_reload.session_sheets;
            let reloaded = match sessions.get(png) {
                Some(earlier) => reloaded.after(earlier),
                None => reloaded,
            };
            sessions.insert(png.to_string(), reloaded);
        }
    }

    /// After Stop's restore: the sheets the session reloaded, applied to the restored world by
    /// the same rules, so the snapshot never brings an old cut back. World-only.
    pub(in crate::editor_game) fn reapply_session_sheets(&mut self, world: &mut World) {
        for (png, sheet) in std::mem::take(&mut self.hot_reload.session_sheets) {
            for clamp in apply_sheet(world, &png, &sheet) {
                log::warn!("{png}: {clamp}");
            }
        }
    }
}

/// The texture handles of `png` that the sprites animated from it hold now, other than `new_handle`.
fn sheet_handles(world: &World, assets: &AssetManager, png: &str, new_handle: u32) -> BTreeSet<u32> {
    world
        .entity_ids()
        .filter(|entity| {
            world
                .get::<SpriteAnimation>(*entity)
                .is_some_and(|animation| animation.sheet.as_deref() == Some(png))
        })
        .filter_map(|entity| world.get::<Sprite>(entity).map(|sprite| sprite.texture_handle))
        .filter(|handle| *handle != new_handle && assets.texture_path(*handle) == Some(png))
        .collect()
}
