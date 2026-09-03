//! The `.sheet.ron` sidecar schema — the single source of truth for how a
//! sprite sheet is cut up and what clips it carries.
//!
//! A sidecar sits next to its PNG under the same stem, so
//! `sprites/deion_16.png` is described by `sprites/deion_16.sheet.ron`:
//!
//! ```ron
//! SheetFile(
//!     version: 1,
//!     cell: (16, 16),          // PIXEL cell size — what the artist knows
//!     filter: Nearest,         // omitted means Nearest: sheets are pixel art
//!     clips: [
//!         ("idle", (frames: [0, 1, 2, 3], fps: 6.0, looping: true)),
//!         ("walk", (frames: [8, 9, 10, 11], fps: 10.0)),  // looping defaults true
//!     ],
//! )
//! ```
//!
//! Clip names are the stable API: game code plays `"walk"`, never cell index
//! 8, so re-cutting a sheet is an art change rather than a code change. The
//! grid itself is derived at load from the PNG's dimensions divided by `cell`,
//! which is why normalized UVs never appear in the file.
//!
//! Authored data fails loud: this module rejects an unknown version, a zero
//! cell dimension, an empty frame list, a non-finite or non-positive `fps`,
//! and (once the PNG size is known) any frame index past the last cell. The
//! guards on `SpriteAnimation::update` are the second net, for components
//! built programmatically rather than authored.

use std::path::{Path, PathBuf};

use common::SheetGrid;
use ecs::sprite_components::AnimationClip;
use renderer::TextureFilter;
use serde::{Deserialize, Serialize};

use crate::assets::AssetError;
use crate::scene_data::ClipData;

/// Schema version this engine reads. Bump only for a breaking change, and
/// only alongside a migration story — an unknown version is a hard error.
pub const SHEET_FILE_VERSION: u32 = 1;

/// A sidecar resolved against its PNG: how the sheet is cut, the clips over
/// it, and how it is sampled.
pub type SheetParts = (SheetGrid, Vec<(String, AnimationClip)>, TextureFilter);

/// Sheets are pixel art unless the file says otherwise.
///
/// Deliberately not the plain serde default: `TextureFilter`'s own default is
/// `Linear`, which would silently blur every sheet whose author omitted the
/// field.
pub fn default_sheet_filter() -> TextureFilter {
    TextureFilter::Nearest
}

/// A parsed `.sheet.ron` sidecar, before it is resolved against its PNG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SheetFile {
    /// Schema version; must be [`SHEET_FILE_VERSION`].
    pub version: u32,
    /// Cell size in pixels, `(width, height)`.
    pub cell: (u32, u32),
    /// How the sheet's texture is sampled. Omitted means
    /// [`TextureFilter::Nearest`].
    #[serde(default = "default_sheet_filter", with = "crate::texture_filter_serde")]
    pub filter: TextureFilter,
    /// Named clips over the sheet's cells, in declaration order.
    #[serde(default)]
    pub clips: Vec<(String, ClipData)>,
}

/// The sidecar path belonging to a sheet PNG: same stem, `.sheet.ron`
/// extension.
///
/// `sprites/tiles.png` → `sprites/tiles.sheet.ron`. The PNG's extension is
/// replaced, not appended, so nothing ever probes `tiles.png.sheet.ron`.
pub fn sidecar_path_for(png_path: impl AsRef<Path>) -> PathBuf {
    png_path.as_ref().with_extension("sheet.ron")
}

/// Parse a sidecar's text, rejecting anything an author can get wrong before
/// the PNG is even opened.
///
/// `label` names the file in error messages — pass the sidecar path.
pub fn parse_sheet_file(label: &str, ron_str: &str) -> Result<SheetFile, AssetError> {
    let file: SheetFile = ron::from_str(ron_str)
        .map_err(|e| AssetError::InvalidData(format!("{label}: malformed sheet file: {e}")))?;
    file.validate(label)?;
    Ok(file)
}

impl SheetFile {
    /// Everything checkable without the PNG: version, cell size, and each
    /// clip's frame list and rate.
    fn validate(&self, label: &str) -> Result<(), AssetError> {
        if self.version != SHEET_FILE_VERSION {
            return Err(AssetError::InvalidData(format!(
                "{label}: unsupported sheet version {} (this engine reads version {})",
                self.version, SHEET_FILE_VERSION
            )));
        }
        if self.cell.0 == 0 || self.cell.1 == 0 {
            return Err(AssetError::InvalidData(format!(
                "{label}: cell size must be non-zero, got {}x{}",
                self.cell.0, self.cell.1
            )));
        }
        for (name, clip) in &self.clips {
            if clip.frames.is_empty() {
                return Err(AssetError::InvalidData(format!(
                    "{label}: clip '{name}' has no frames"
                )));
            }
            if !clip.fps.is_finite() || clip.fps <= 0.0 {
                return Err(AssetError::InvalidData(format!(
                    "{label}: clip '{name}' has fps {}, which must be finite and greater than zero",
                    clip.fps
                )));
            }
        }
        Ok(())
    }

    /// Resolve the sidecar against its PNG's pixel dimensions, yielding the
    /// grid, the clips, and the sampling filter.
    ///
    /// The grid is the PNG divided by the cell size; a partial trailing cell
    /// is excluded rather than stretched over. Any frame index past the last
    /// whole cell is an error naming the clip — a renumbered sheet must not
    /// quietly sample the wrong art.
    pub fn into_parts(
        self,
        label: &str,
        png_width: u32,
        png_height: u32,
    ) -> Result<SheetParts, AssetError> {
        let grid = SheetGrid::from_cell_size(png_width, png_height, self.cell.0, self.cell.1);
        let cell_count = grid.cell_count();

        let mut clips = Vec::with_capacity(self.clips.len());
        for (name, clip) in self.clips {
            if let Some(bad) = clip.frames.iter().find(|index| **index >= cell_count) {
                return Err(AssetError::InvalidData(format!(
                    "{label}: clip '{name}' uses cell {bad}, but a {png_width}x{png_height} sheet \
                     of {}x{} cells only has {cell_count} ({}x{})",
                    self.cell.0, self.cell.1, grid.cols, grid.rows
                )));
            }
            clips.push((name, AnimationClip::from(clip)));
        }

        Ok((grid, clips, self.filter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOLDEN: &str = r#"SheetFile(
    version: 1,
    cell: (16, 16),
    filter: Nearest,
    clips: [
        ("idle", (frames: [0, 1, 2, 3], fps: 6.0, looping: true)),
        ("walk", (frames: [8, 9, 10, 11], fps: 10.0, looping: false)),
    ],
)"#;

    fn parse(text: &str) -> Result<SheetFile, AssetError> {
        parse_sheet_file("deion.sheet.ron", text)
    }

    #[test]
    fn golden_sheet_file_round_trips_and_omitted_fields_take_their_defaults() -> Result<(), AssetError> {
        let sheet = parse(GOLDEN)?;

        assert_eq!(sheet.version, 1);
        assert_eq!(sheet.cell, (16, 16));
        assert_eq!(sheet.filter, TextureFilter::Nearest);
        assert_eq!(sheet.clips.len(), 2);
        assert_eq!(sheet.clips[0].0, "idle");
        assert_eq!(sheet.clips[0].1.frames, vec![0, 1, 2, 3]);
        assert_eq!(sheet.clips[1].1.fps, 10.0);
        assert!(!sheet.clips[1].1.looping);
        // Written back out, it reads as the same file.
        let text = ron::ser::to_string(&sheet).expect("serialize");
        assert_eq!(parse(&text)?, sheet);

        // The whole point of the pipeline: pixel art must not blur because
        // someone left `filter` out, and a clip loops unless told otherwise.
        let minimal = parse(r#"SheetFile(version: 1, cell: (16, 16), clips: [("idle", (frames: [0], fps: 6.0))])"#)?;
        assert_eq!(minimal.filter, TextureFilter::Nearest);
        assert!(minimal.clips[0].1.looping);
        // The filter accepts the lowercase alias the config file uses.
        assert_eq!(parse(r#"SheetFile(version: 1, cell: (8, 8), filter: linear)"#)?.filter, TextureFilter::Linear);
        Ok(())
    }

    #[test]
    fn authored_errors_fail_loud_naming_the_file_and_the_clip() {
        // RON parses `inf` and `NaN`, so the validator has to catch them.
        let mut cases = vec![
            (r#"SheetFile(version: 99, cell: (16, 16))"#.to_string(), vec!["99"]),
            (r#"SheetFile(version: 1, cell: (0, 16))"#.to_string(), vec!["cell size"]),
            (
                r#"SheetFile(version: 1, cell: (16, 16), clips: [("idle", (frames: [], fps: 6.0))])"#.to_string(),
                vec!["'idle'", "no frames"],
            ),
        ];
        for fps in ["0.0", "-6.0", "inf", "-inf", "NaN"] {
            cases.push((
                format!(r#"SheetFile(version: 1, cell: (16, 16), clips: [("walk", (frames: [0], fps: {fps}))])"#),
                vec!["'walk'", "fps"],
            ));
        }

        for (text, expected) in cases {
            let Err(err) = parse(&text) else {
                panic!("{text} must be rejected at parse time");
            };

            let message = err.to_string();
            assert!(message.contains("deion.sheet.ron"), "{text}: names the file: {message}");
            for needle in expected {
                assert!(message.contains(needle), "{text}: expected {needle:?} in {message}");
            }
        }
    }

    #[test]
    fn into_parts_derives_the_grid_from_png_dimensions_excluding_a_partial_trailing_cell() -> Result<(), AssetError> {
        let exact = parse(r#"SheetFile(version: 1, cell: (16, 16), clips: [("walk", (frames: [0, 7], fps: 8.0))])"#)?;
        let uneven = parse(r#"SheetFile(version: 1, cell: (30, 30))"#)?;

        let (grid, clips, filter) = exact.into_parts("deion.sheet.ron", 64, 32)?;
        // 100 / 30 = 3 whole cells per axis; the 10px remainder is dropped
        // rather than stretched over, so cells stay pixel-exact.
        let (partial, _, _) = uneven.into_parts("x.sheet.ron", 100, 100)?;

        assert_eq!((grid.cols, grid.rows), (4, 2));
        assert_eq!(grid.cell_count(), 8);
        assert_eq!(clips[0].1.frame_indices, vec![0, 7]);
        assert_eq!(filter, TextureFilter::Nearest);
        assert_eq!((partial.cols, partial.rows), (3, 3));
        assert_eq!(partial.uv_rect(1), [0.3, 0.0, 0.3, 0.3]);
        Ok(())
    }

    #[test]
    fn frame_index_past_the_grid_is_rejected_naming_the_clip() -> Result<(), AssetError> {
        let sheet = parse(r#"SheetFile(version: 1, cell: (16, 16), clips: [("walk", (frames: [0, 99], fps: 8.0))])"#)?;

        let err = sheet
            .into_parts("deion.sheet.ron", 64, 32)
            .expect_err("cell 99 does not exist in an 8-cell sheet");

        let message = err.to_string();
        assert!(message.contains("'walk'"), "{message}");
        assert!(message.contains("99"), "{message}");
        assert!(message.contains("deion.sheet.ron"), "{message}");
        Ok(())
    }

    #[test]
    fn sidecar_path_replaces_the_extension_never_appends() {
        // The contract every call site defers to: same stem.
        assert_eq!(sidecar_path_for("tiles.png"), PathBuf::from("tiles.sheet.ron"));
        assert_eq!(sidecar_path_for("sprites/deion_16.png"), PathBuf::from("sprites/deion_16.sheet.ron"));
        assert_ne!(sidecar_path_for("tiles.png"), PathBuf::from("tiles.png.sheet.ron"));
    }
}
