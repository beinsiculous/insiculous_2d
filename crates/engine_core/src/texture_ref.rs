//! Texture reference resolution for scene files.
//!
//! Texture references in scene RON can be:
//! - `#white` — the built-in white texture (handle 0) for color tinting
//! - `#solid:RRGGBB` (or `RRGGBBAA`) — a generated solid-color texture
//! - Any other string — loaded as a file path

use common::SheetGrid;
use ecs::sprite_components::AnimationClip;
use renderer::TextureHandle;

use crate::assets::AssetManager;
use crate::scene_data::SceneLoadError;

/// What a `.sheet.ron` sidecar contributes to a `SpriteAnimation`: how the
/// sheet is cut up, and the clips authored over it.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetData {
    pub grid: SheetGrid,
    pub clips: Vec<(String, AnimationClip)>,
}

/// Resolves scene texture references (`#white`, `#solid:RRGGBB`, file paths)
/// to texture handles, and reads the `.sheet.ron` sidecars that sit beside
/// sheet PNGs.
///
/// [`AssetManager`] is the production implementation; tests substitute a
/// stub so scene and prefab instantiation stay headless-testable (no GPU).
/// Everything that touches the filesystem or the GPU lives behind this trait,
/// including the PNG dimension probe a sidecar needs to derive its grid.
pub trait TextureResolver {
    /// Resolve a texture reference string to a handle.
    fn resolve_texture(&mut self, texture_ref: &str) -> Result<TextureHandle, SceneLoadError>;

    /// Read the sidecar belonging to a sheet PNG, if there is a usable one.
    ///
    /// `texture_ref` is the PNG path; the sidecar is its same-stem
    /// `.sheet.ron` neighbour. `None` covers every "carry on without it"
    /// case — no sidecar, unreadable PNG, malformed RON — so a scene load
    /// falls back to its baked values rather than dying on a half-saved
    /// file. Implementations warn; they do not fail.
    fn sheet_for(&mut self, _texture_ref: &str) -> Option<SheetData> {
        None
    }

    /// Drop any cached sidecar reads, so the next resolve sees the files as
    /// they are on disk. Called at the start of every scene load.
    fn clear_sidecar_cache(&mut self) {}
}

impl TextureResolver for AssetManager {
    fn resolve_texture(&mut self, texture_ref: &str) -> Result<TextureHandle, SceneLoadError> {
        resolve_texture(texture_ref, self)
    }

    fn sheet_for(&mut self, texture_ref: &str) -> Option<SheetData> {
        self.sidecar_sheet(texture_ref)
    }

    fn clear_sidecar_cache(&mut self) {
        // Spelled out rather than `self.clear_sidecar_cache()`, which would
        // read as recursion even though the inherent method wins.
        AssetManager::clear_sidecar_cache(self)
    }
}

/// Resolve a texture reference to a TextureHandle.
pub(crate) fn resolve_texture(
    texture_ref: &str,
    assets: &mut AssetManager,
) -> Result<TextureHandle, SceneLoadError> {
    if texture_ref == "#white" {
        // White texture is always handle 0
        return Ok(TextureHandle { id: 0 });
    }

    if let Some(hex) = texture_ref.strip_prefix("#solid:") {
        // Parse hex color and create solid color texture
        let color = parse_hex_color(hex)?;
        assets
            .create_solid_color(1, 1, color)
            .map_err(|e| SceneLoadError::TextureLoadError(e.to_string()))
    } else if is_unresolvable_sentinel(texture_ref) {
        // Placeholder paths recorded for runtime-generated textures
        // ("#rgba", bare "#solid" from pre-E5 saves) carry no pixel data to
        // rebuild from. Degrade to white instead of failing the scene load.
        log::warn!(
            "texture ref '{texture_ref}' is a generated-texture placeholder and cannot be \
             reloaded; substituting the white texture"
        );
        Ok(TextureHandle { id: 0 })
    } else {
        // Load as file path. A sheet declares its own sampling in its
        // `.sheet.ron` sidecar, so scene-referenced pixel-art sheets get
        // Nearest without every game repeating itself; anything without a
        // usable sidecar takes the project default.
        match assets.sidecar_filter(texture_ref) {
            Some(filter) => assets.load_texture_filtered(texture_ref, filter),
            None => assets.load_texture(texture_ref),
        }
        .map_err(|e| SceneLoadError::TextureLoadError(e.to_string()))
    }
}

/// A `#`-prefixed reference that names a runtime-generated texture rather
/// than a reconstructible scheme (`#white`, `#solid:RRGGBB`). Such refs can
/// be *written* by scene save (via `AssetManager::texture_path`) but carry
/// no data to rebuild the texture from on load.
pub(crate) fn is_unresolvable_sentinel(texture_ref: &str) -> bool {
    texture_ref.starts_with('#')
        && texture_ref != "#white"
        && !texture_ref.starts_with("#solid:")
}

/// The canonical `#solid:RRGGBB` reference for a solid-color texture —
/// what `AssetManager::create_solid_color` records so the color survives
/// scene save/load. Opaque colors omit the alpha byte; anything else is
/// written `#solid:RRGGBBAA`. Inverse of [`parse_hex_color`].
pub(crate) fn solid_color_path(color: [u8; 4]) -> String {
    let [r, g, b, a] = color;
    if a == 255 {
        format!("#solid:{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#solid:{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

/// Parse a hex color string (RRGGBB or RRGGBBAA) to [u8; 4]
pub(crate) fn parse_hex_color(hex: &str) -> Result<[u8; 4], SceneLoadError> {
    let hex = hex.trim_start_matches('#');

    if hex.len() == 6 {
        Ok([parse_hex_byte(hex, 0)?, parse_hex_byte(hex, 2)?, parse_hex_byte(hex, 4)?, 255])
    } else if hex.len() == 8 {
        Ok([parse_hex_byte(hex, 0)?, parse_hex_byte(hex, 2)?, parse_hex_byte(hex, 4)?, parse_hex_byte(hex, 6)?])
    } else {
        Err(SceneLoadError::InvalidTextureRef(format!(
            "Hex color must be 6 or 8 characters: {}",
            hex
        )))
    }
}

/// Parse a 2-character hex byte from a string at the given offset.
fn parse_hex_byte(hex: &str, start: usize) -> Result<u8, SceneLoadError> {
    u8::from_str_radix(&hex[start..start + 2], 16)
        .map_err(|_| SceneLoadError::InvalidTextureRef(format!("Invalid hex color: {}", hex)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solid_color_path_round_trips_through_parse() {
        // The writer omits the alpha byte only for opaque colors ...
        assert_eq!(solid_color_path([255, 0, 255, 255]), "#solid:FF00FF");
        assert_eq!(solid_color_path([0, 128, 7, 255]), "#solid:008007");
        assert_eq!(solid_color_path([255, 0, 0, 128]), "#solid:FF000080");
        assert_eq!(solid_color_path([0, 0, 0, 0]), "#solid:00000000");

        // ... and the parser reads both forms back to the same bytes.
        for color in [[255, 0, 255, 255], [1, 2, 3, 255], [10, 20, 30, 40], [255, 0, 0, 128]] {
            let path = solid_color_path(color);
            let hex = path.strip_prefix("#solid:").expect("canonical prefix");
            assert_eq!(parse_hex_color(hex).expect("parses"), color, "path {path}");
        }

        // Hand-written hex: 6 and 8 digits parse, anything else is a typed error.
        assert_eq!(parse_hex_color("00FF00").expect("6 digits"), [0, 255, 0, 255]);
        assert_eq!(parse_hex_color("FF000080").expect("8 digits"), [255, 0, 0, 128]);
        for bad in ["FFF", "FF00FF00FF", "GGGGGG"] {
            assert!(
                matches!(parse_hex_color(bad), Err(SceneLoadError::InvalidTextureRef(_))),
                "{bad}"
            );
        }
    }

    #[test]
    fn test_generated_texture_sentinels_are_flagged() {
        // Runtime-generated placeholders: degrade to white, never file-load.
        assert!(is_unresolvable_sentinel("#rgba"));
        assert!(is_unresolvable_sentinel("#solid"));
        // Reconstructible schemes and real paths are not sentinels — the
        // recorded `#solid:` path must take the rebuild branch on load.
        assert!(!is_unresolvable_sentinel("#white"));
        assert!(!is_unresolvable_sentinel("#solid:FF00FF"));
        assert!(!is_unresolvable_sentinel(&solid_color_path([9, 9, 9, 9])));
        assert!(!is_unresolvable_sentinel("sprites/player.png"));
    }
}
