//! Headless command-API mode: the full
//! authoring loop — query → mutate → save — with NO window, GPU, or frame
//! loop. `--headless` in the editor binary routes here; CI drives it with
//! piped stdin/stdout, and the future web transport reuses the
//! same `answer_api_lines` dispatch over a WebSocket.
//!
//! Deliberately NOT a headless engine runner: `GameContext`/`AssetManager`/
//! winit are window-bound by design, and the pure half of the editor
//! (`EditorGame::answer_api_lines`) never needed them. Play mode does not
//! exist here — the session stays `Editing` forever (there is no play verb
//! in the protocol; writes-refused-while-Playing is unreachable).

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use engine_core::assets::sprite_sheet::SidecarCache;

use engine_core::contexts::GameContext;
use engine_core::{Game, SceneLoadError, TextureResolver};
use renderer::TextureHandle;

use super::EditorGame;

/// The headless stand-in for `AssetManager` (which requires a live wgpu
/// device): a path-recording [`TextureResolver`].
///
/// Every texture reference resolves to a deduped, stable handle and the ref
/// string is recorded VERBATIM (`#white` = handle 0, like the real asset
/// manager's built-in), so a scene saved from a headless session writes
/// back exactly the references it loaded — byte-stable round trips without
/// touching the filesystem or GPU. Deliberately permissive: a missing
/// texture file must not fail headless authoring (nothing renders here).
///
/// Limits (documented in `docs/EDITOR_COMMAND_API.md`): texture FILES are
/// not validated. `.sheet.ron` sidecars ARE consulted when an asset base
/// path is given (pure file I/O), so a headless save bakes
/// the CURRENT sidecar snapshot, same as the windowed editor.
pub struct HeadlessAssets {
    /// ref string → handle (dedup).
    by_ref: HashMap<String, u32>,
    /// handle → ref string (the serializer's inverse).
    by_handle: HashMap<u32, String>,
    next_handle: u32,
    /// The project's asset root — sidecar reads resolve against it.
    /// `None` = no sidecar support (in-memory tests).
    asset_base: Option<PathBuf>,
    sidecar_cache: SidecarCache,
}

impl Default for HeadlessAssets {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadlessAssets {
    pub fn new() -> Self {
        Self::with_asset_base(None)
    }

    /// A resolver whose sidecar reads resolve against `asset_base`
    /// (the project's `assets/` directory).
    pub fn with_asset_base(asset_base: Option<PathBuf>) -> Self {
        let mut assets = Self {
            by_ref: HashMap::new(),
            by_handle: HashMap::new(),
            next_handle: 1,
            asset_base,
            sidecar_cache: SidecarCache::default(),
        };
        assets.by_ref.insert("#white".to_string(), 0);
        assets.by_handle.insert(0, "#white".to_string());
        assets
    }

    /// The recorded reference string for a handle (the save-path inverse).
    pub fn texture_path(&self, handle: u32) -> Option<&str> {
        self.by_handle.get(&handle).map(|s| s.as_str())
    }
}

impl TextureResolver for HeadlessAssets {
    fn resolve_texture(&mut self, texture_ref: &str) -> Result<TextureHandle, SceneLoadError> {
        if let Some(&handle) = self.by_ref.get(texture_ref) {
            return Ok(TextureHandle { id: handle });
        }
        let handle = self.next_handle;
        self.next_handle += 1;
        self.by_ref.insert(texture_ref.to_string(), handle);
        self.by_handle.insert(handle, texture_ref.to_string());
        Ok(TextureHandle { id: handle })
    }

    fn sheet_for(&mut self, texture_ref: &str) -> Option<engine_core::SheetData> {
        let base = self.asset_base.clone()?;
        self.sidecar_cache
            .get(&base, texture_ref)
            .map(|prepared| prepared.sheet.clone())
    }

    fn clear_sidecar_cache(&mut self) {
        self.sidecar_cache.clear();
    }
}

/// The headless session's inner game: nothing (Play mode does not exist
/// without a frame loop).
struct NullGame;
impl Game for NullGame {
    fn update(&mut self, _ctx: &mut GameContext) {}
}

/// Run a blocking headless command-API session: optionally open a scene
/// through the REAL editor load path (dry-run guard, physics settings,
/// history reset — the first-scene seam), then answer one request line at a time
/// until EOF. Each response is one line of JSON, flushed immediately so a
/// driving agent never blocks on buffering.
///
/// A scene that fails to load is a hard error — an agent must never
/// silently author against an empty world it believes is the scene.
pub fn run_headless_editor_api(
    asset_base: Option<PathBuf>,
    initial_scene: Option<PathBuf>,
    input: impl BufRead,
    mut output: impl Write,
) -> Result<(), String> {
    // Engine components must be registered before any scene load or
    // dynamic-component write (idempotent; the windowed path does this in
    // run_game).
    engine_core::component_registration::register_engine_components();

    let mut editor_game = EditorGame::new(NullGame);
    if let Some(ref base) = asset_base {
        editor_game.asset_base = base.clone();
    }
    let mut world = ecs::World::new();
    let mut assets = HeadlessAssets::with_asset_base(asset_base);

    if initial_scene.is_none() {
        // An agent must know it is authoring against an empty world.
        log::info!("headless: no scene to open — starting with an empty scene");
    }
    if let Some(path) = initial_scene {
        editor_game
            .load_scene(&mut world, &mut assets, &path)
            .map_err(|e| format!("failed to open scene {}: {e}", path.display()))?;
        log::info!("headless: opened {}", path.display());
    }

    for line in input.lines() {
        let line = line.map_err(|e| format!("stdin read failed: {e}"))?;
        let texture_path = |handle: u32| assets.texture_path(handle).map(str::to_string);
        let responses = editor_game.answer_api_lines(&[line], &mut world, &texture_path);
        for response in responses {
            writeln!(output, "{response}").map_err(|e| format!("stdout write failed: {e}"))?;
        }
        output
            .flush()
            .map_err(|e| format!("stdout flush failed: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
