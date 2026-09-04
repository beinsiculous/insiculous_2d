//! Asset management system for the Insiculous 2D engine.
//!
//! Provides a unified interface for loading and managing game assets including
//! textures, audio (future), and other resources.
//!
//! # Example
//!
//! ```
//! use engine_core::prelude::*;
//!
//! struct MyGame {
//!     player_texture: TextureHandle,
//! }
//!
//! impl Game for MyGame {
//!     fn init(&mut self, ctx: &mut GameContext) {
//!         // Load a texture from file
//!         self.player_texture = ctx.assets.load_texture("player.png").unwrap();
//!     }
//!
//!     fn update(&mut self, _ctx: &mut GameContext) {
//!         // Use the texture handle for sprite rendering
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use renderer::{
    TextureManager, TextureHandle, TextureResource, TextureLoadConfig, TextureError,
    TextureFilter,
};

use crate::game_config::GameConfig;

// Re-export wgpu types from renderer
use renderer::wgpu::{Device, Queue};

pub mod sprite_sheet;

/// Asset loading errors
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("Texture error: {0}")]
    Texture(#[from] TextureError),

    #[error("Asset not found: {0}")]
    NotFound(String),

    #[error("Asset manager not initialized")]
    NotInitialized,

    #[error("Invalid texture data: {0}")]
    InvalidData(String),
}

/// Validate raw RGBA dimensions/length before any GPU work.
/// Kept as a free function so the error path is headless-testable
/// (constructing an `AssetManager` requires a live wgpu device).
fn validate_rgba(width: u32, height: u32, len: usize) -> Result<(), AssetError> {
    if width == 0 || height == 0 {
        return Err(AssetError::InvalidData(format!(
            "texture dimensions must be non-zero (got {width}x{height})"
        )));
    }
    let expected = width as usize * height as usize * 4;
    if len != expected {
        return Err(AssetError::InvalidData(format!(
            "expected {expected} bytes for {width}x{height} RGBA, got {len}"
        )));
    }
    Ok(())
}

/// Configuration for the asset manager
#[derive(Debug, Clone)]
pub struct AssetConfig {
    /// Base path for asset loading (relative to the executable)
    pub base_path: String,
    /// Whether to log asset loading operations
    pub log_loading: bool,
    /// Filter applied to textures loaded from files or bytes when the caller
    /// doesn't ask for a specific one.
    pub default_filter: TextureFilter,
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            base_path: "assets".to_string(),
            log_loading: true,
            default_filter: TextureFilter::Linear,
        }
    }
}

impl From<&GameConfig> for AssetConfig {
    /// Derive the asset-manager configuration the engine runs with: the
    /// game's asset base path (falling back to the default `assets`
    /// directory) and its default texture filter.
    fn from(config: &GameConfig) -> Self {
        let mut asset_config = Self {
            default_filter: config.texture_filter,
            ..Self::default()
        };
        if let Some(base_path) = &config.asset_base_path {
            asset_config.base_path = base_path.clone();
        }
        asset_config
    }
}

/// Unified asset manager for loading and managing game resources.
///
/// The AssetManager provides a convenient interface for loading textures
/// and other game assets. It handles caching and resource lifecycle management.
pub struct AssetManager {
    texture_manager: TextureManager,
    config: AssetConfig,
    /// Maps texture handle IDs back to their original path strings for serialization.
    handle_to_path: HashMap<u32, String>,
    /// Deduplicates file loads: a texture file is decoded and uploaded once
    /// per (path, filter); repeat loads (scene reloads, dry-run validation)
    /// reuse the existing GPU texture instead of allocating another copy.
    loaded_by_path: HashMap<(String, TextureFilter), TextureHandle>,
    /// `.sheet.ron` reads, cached per texture path for the duration of a
    /// scene load (see [`sprite_sheet::SidecarCache`]).
    sidecar_cache: sprite_sheet::SidecarCache,
}

impl AssetManager {
    /// Create a new asset manager with the given WGPU device and queue
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        let mut handle_to_path = HashMap::new();
        // Handle 0 is always the white texture
        handle_to_path.insert(0, "#white".to_string());
        Self {
            texture_manager: TextureManager::new(device, queue),
            config: AssetConfig::default(),
            handle_to_path,
            loaded_by_path: HashMap::new(),
            sidecar_cache: sprite_sheet::SidecarCache::default(),
        }
    }

    /// Create a new asset manager with custom configuration
    pub fn with_config(device: Arc<Device>, queue: Arc<Queue>, config: AssetConfig) -> Self {
        let mut handle_to_path = HashMap::new();
        // Handle 0 is always the white texture
        handle_to_path.insert(0, "#white".to_string());
        Self {
            texture_manager: TextureManager::new(device, queue),
            config,
            handle_to_path,
            loaded_by_path: HashMap::new(),
            sidecar_cache: sprite_sheet::SidecarCache::default(),
        }
    }

    /// Load a texture from a file path
    ///
    /// The path can be absolute or relative. If relative, it will be resolved
    /// against the asset manager's base path.
    ///
    /// # Example
    /// ```
    /// # use engine_core::prelude::*;
    /// # fn load(assets: &mut AssetManager) -> Result<(), AssetError> {
    /// let handle = assets.load_texture("player.png")?;
    /// // Or with a longer relative path:
    /// let handle = assets.load_texture("sprites/enemies/boss.png")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Sampling uses the configured default filter
    /// (`GameConfig::with_texture_filter`, Linear unless changed). Use
    /// [`load_texture_filtered`](Self::load_texture_filtered) to override it
    /// for a single texture.
    pub fn load_texture<P: AsRef<Path>>(&mut self, path: P) -> Result<TextureHandle, AssetError> {
        self.load_texture_filtered(path, self.config.default_filter)
    }

    /// Load a texture from a file path with an explicit sampling filter,
    /// ignoring the configured default.
    ///
    /// Use this for the odd texture that disagrees with the project default —
    /// a pixel-art sprite in a linear-filtered game, or a smooth gradient in a
    /// nearest-filtered one.
    ///
    /// # Example
    /// ```
    /// # use engine_core::prelude::*;
    /// # fn load(assets: &mut AssetManager) -> Result<(), AssetError> {
    /// let handle = assets.load_texture_filtered("tiles.png", TextureFilter::Nearest)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_texture_filtered<P: AsRef<Path>>(
        &mut self,
        path: P,
        filter: TextureFilter,
    ) -> Result<TextureHandle, AssetError> {
        let path = path.as_ref();
        let original_path_string = path.to_string_lossy().to_string();

        // Same file + same filter = same GPU texture. Without this, every
        // scene reload re-decoded and re-uploaded its whole texture set.
        let cache_key = (original_path_string.clone(), filter);
        if let Some(&handle) = self.loaded_by_path.get(&cache_key) {
            return Ok(handle);
        }

        // Resolve path against base path if relative
        let full_path = if path.is_relative() {
            Path::new(&self.config.base_path).join(path)
        } else {
            path.to_path_buf()
        };

        if self.config.log_loading {
            log::info!("Loading texture: {:?}", full_path);
        }

        let config = TextureLoadConfig {
            sampler_config: filter.into(),
            ..TextureLoadConfig::default()
        };
        let handle = self.texture_manager.load_texture(&full_path, config)?;
        self.handle_to_path.insert(handle.id, original_path_string);
        self.loaded_by_path.insert(cache_key, handle);

        Ok(handle)
    }

    /// Load a texture from raw bytes (file contents)
    ///
    /// Useful for loading textures from embedded assets or network resources.
    /// Sampling uses the configured default filter
    /// (`GameConfig::with_texture_filter`, Linear unless changed).
    pub fn load_texture_from_bytes(&mut self, bytes: &[u8]) -> Result<TextureHandle, AssetError> {
        let config = TextureLoadConfig {
            sampler_config: self.config.default_filter.into(),
            ..TextureLoadConfig::default()
        };
        let handle = self.texture_manager.load_texture_from_bytes(bytes, config)?;
        Ok(handle)
    }

    /// Create a solid color texture
    ///
    /// Useful for placeholder textures or colored rectangles.
    ///
    /// Always Linear-filtered regardless of the configured default — a flat
    /// color samples identically either way.
    ///
    /// The handle's recorded path is the canonical `#solid:RRGGBB` (with an
    /// alpha byte when translucent), so the color survives scene save/load.
    /// The dimensions are not recorded — a reload rebuilds it as 1×1, which
    /// samples identically on a stretched quad.
    pub fn create_solid_color(
        &mut self,
        width: u32,
        height: u32,
        color: [u8; 4],
    ) -> Result<TextureHandle, AssetError> {
        let handle = self.texture_manager.create_solid_color(width, height, color)?;
        self.handle_to_path.insert(handle.id, crate::texture_ref::solid_color_path(color));
        Ok(handle)
    }

    /// Create a checkerboard pattern texture
    ///
    /// Useful for debugging or placeholder textures. Always Linear-filtered
    /// regardless of the configured default.
    pub fn create_checkerboard(
        &mut self,
        width: u32,
        height: u32,
        color1: [u8; 4],
        color2: [u8; 4],
        check_size: u32,
    ) -> Result<TextureHandle, AssetError> {
        let handle = self.texture_manager.create_checkerboard(width, height, color1, color2, check_size)?;
        Ok(handle)
    }

    /// Create a glyph texture from grayscale bitmap data.
    ///
    /// Converts a grayscale bitmap to an RGBA texture where all channels (RGBA)
    /// contain the grayscale value. This creates an alpha mask that can be
    /// multiplied by any text color at render time, enabling cache reuse for
    /// the same glyph rendered in different colors.
    ///
    /// Always Linear-filtered regardless of the configured default: nearest
    /// sampling would alias the antialiased glyph edges fontdue produces.
    pub fn create_glyph_texture(
        &mut self,
        width: u32,
        height: u32,
        grayscale: &[u8],
    ) -> Result<TextureHandle, AssetError> {
        if width == 0 || height == 0 {
            // Empty glyphs (spaces) draw the white texture at zero size.
            return Ok(TextureHandle::WHITE);
        }

        // Convert grayscale to RGBA where RGBA all = grayscale value
        // This allows the shader to multiply by text color while preserving shape
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for &gray in grayscale {
            rgba.push(gray); // R = grayscale
            rgba.push(gray); // G = grayscale
            rgba.push(gray); // B = grayscale
            rgba.push(gray); // A = grayscale
        }

        let handle = self.texture_manager.load_texture_from_rgba(
            width,
            height,
            &rgba,
            TextureLoadConfig::default(),
        )?;
        Ok(handle)
    }

    /// Create a texture from raw RGBA8 pixel data (row-major, `width * height * 4` bytes).
    ///
    /// Intended for pixel-data textures built in code — tileset strips,
    /// palettes, generated art — so it samples with **nearest** filtering:
    /// linear filtering would bleed adjacent cells across tile borders.
    ///
    /// Dimensions and data length are validated before any GPU work; bad
    /// input returns [`AssetError::InvalidData`], never panics.
    ///
    /// The handle's recorded path is the sentinel `"#rgba"` — it is
    /// **non-unique** and cannot be resolved back to pixel data, so textures
    /// created here do not survive scene save/load or any path-based lookup
    /// (unlike `create_solid_color`, whose `#solid:RRGGBB` path rebuilds).
    pub fn create_texture_from_rgba(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<TextureHandle, AssetError> {
        validate_rgba(width, height, rgba.len())?;
        let config = TextureLoadConfig {
            format: None,
            sampler_config: TextureFilter::Nearest.into(),
        };
        let handle = self
            .texture_manager
            .load_texture_from_rgba(width, height, rgba, config)?;
        self.handle_to_path.insert(handle.id, "#rgba".to_string());
        Ok(handle)
    }

    /// Get a texture resource by handle
    pub fn get_texture(&self, handle: TextureHandle) -> Option<&TextureResource> {
        self.texture_manager.get_texture(handle)
    }

    /// Check if a texture exists
    pub fn has_texture(&self, handle: TextureHandle) -> bool {
        self.texture_manager.has_texture(handle)
    }

    /// Unload a texture, freeing GPU resources
    pub fn unload_texture(&mut self, handle: TextureHandle) -> bool {
        self.loaded_by_path.retain(|_, &mut h| h != handle);
        self.texture_manager.remove_texture(handle).is_some()
    }

    /// Get the number of loaded textures
    pub fn texture_count(&self) -> usize {
        self.texture_manager.texture_count()
    }

    /// Get all texture handles
    pub fn texture_handles(&self) -> Vec<TextureHandle> {
        self.texture_manager.texture_handles()
    }

    /// Get all textures as a HashMap for rendering
    ///
    /// This is used internally to pass textures to the sprite renderer.
    pub fn textures(&self) -> &HashMap<TextureHandle, TextureResource> {
        self.texture_manager.textures()
    }

    /// Set the base path for asset loading.
    ///
    /// Also drops the path-deduplication cache: relative paths recorded
    /// under the old base must not resolve to the old base's textures.
    pub fn set_base_path(&mut self, path: impl Into<String>) {
        self.config.base_path = path.into();
        self.loaded_by_path.clear();
    }

    /// Get the current base path
    pub fn base_path(&self) -> &str {
        &self.config.base_path
    }

    /// Look up the original path string for a texture handle.
    ///
    /// Returns `None` if the handle was not loaded through this manager.
    /// Handle 0 always maps to `"#white"`.
    pub fn texture_path(&self, handle: u32) -> Option<&str> {
        self.handle_to_path.get(&handle).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_config_from_game_config_maps_filter_and_base_path() {
        for (game_config, filter, base_path) in [
            (GameConfig::new("Test"), TextureFilter::Linear, "assets"),
            (
                GameConfig::new("Test")
                    .with_texture_filter(TextureFilter::Nearest)
                    .with_asset_base_path("/games/pong/assets"),
                TextureFilter::Nearest,
                "/games/pong/assets",
            ),
        ] {
            let config = AssetConfig::from(&game_config);
            assert_eq!(config.default_filter, filter);
            assert_eq!(config.base_path, base_path);
        }
    }

    #[test]
    fn test_rgba_validation_accepts_exact_length_and_names_the_expected_size() {
        assert!(validate_rgba(2, 2, 16).is_ok());
        assert!(validate_rgba(64, 16, 64 * 16 * 4).is_ok());

        for (width, height, len) in [(0, 16, 0), (16, 0, 0), (2, 2, 15), (2, 2, 17)] {
            assert!(
                matches!(validate_rgba(width, height, len), Err(AssetError::InvalidData(_))),
                "{width}x{height} with {len} bytes must be InvalidData"
            );
        }
        // 2x2 RGBA needs 16 bytes — the message says so.
        let err = validate_rgba(2, 2, 15).expect_err("length mismatch");
        assert!(format!("{err}").contains("expected 16 bytes"), "{err}");
    }
}

/// Resolve a game's root directory for asset/save anchoring: the
/// executable's directory when it has an `assets/` folder next to it
/// (shipped layout), otherwise the given manifest directory so `cargo run`
/// works from any working directory.
///
/// Games call this through the [`game_root!`](crate::game_root) macro so the
/// *game crate's* `CARGO_MANIFEST_DIR` is baked in, not the engine's.
pub fn game_root_from(manifest_dir: &str) -> std::path::PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if dir.join("assets").is_dir() {
                return dir.to_path_buf();
            }
        }
    }
    std::path::PathBuf::from(manifest_dir)
}

#[cfg(test)]
mod game_root_tests {
    #[test]
    fn test_game_root_falls_back_to_manifest_dir() {
        // The test binary has no assets/ folder next to it, so the manifest
        // fallback must win.
        let root = super::game_root_from("/some/game/crate");
        assert_eq!(root, std::path::PathBuf::from("/some/game/crate"));
    }
}
