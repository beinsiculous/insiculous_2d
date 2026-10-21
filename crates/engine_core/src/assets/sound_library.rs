//! The sound tier: each recorded sound a scene names, loaded once by its path.
//!
//! A scene names a sound the way it names a texture — a path relative to the asset base —
//! and [`AssetManager::resolve_sound`] turns it into the audio manager's handle the first
//! time something plays it, then serves the handle from a by-path cache for the rest of
//! the session. A path that fails is remembered with its error and warned about once, so
//! a missing file costs one log line, not one a frame. Changing the asset base clears
//! both maps with the texture dedupe: the handles stay loaded, a base change being a
//! project switch.
//!
//! Child module of `assets` so it can reach `AssetManager`'s private fields, the
//! arrangement `sprite_sheet.rs` uses.

use std::collections::HashMap;
use std::path::Path;

use audio::{AudioError, AudioManager, SoundHandle};

use super::AssetManager;

/// The handles of the sounds resolved so far, and the paths that failed.
#[derive(Debug, Default)]
pub(crate) struct SoundLibrary {
    loaded: HashMap<String, SoundHandle>,
    /// Why each failed path failed, kept so it is warned about once.
    failed: HashMap<String, String>,
}

impl SoundLibrary {
    pub(crate) fn clear(&mut self) {
        self.loaded.clear();
        self.failed.clear();
    }
}

impl AssetManager {
    /// The handle of the recorded sound at `path`, relative to the asset base like a
    /// texture: loaded through `audio` the first time, from the cache after. A path that
    /// failed once fails again without a read and without a second warning. Game code
    /// plays the handle through `ctx.audio`.
    ///
    /// ```no_run
    /// # use engine_core::prelude::*;
    /// # fn play(ctx: &mut GameContext) -> Result<(), audio::AudioError> {
    /// let jump = ctx.assets.resolve_sound("sounds/jump.wav", ctx.audio)?;
    /// ctx.audio.play(jump)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn resolve_sound(
        &mut self,
        path: &str,
        audio: &mut AudioManager,
    ) -> Result<SoundHandle, AudioError> {
        if let Some(handle) = self.sound_library.loaded.get(path) {
            return Ok(*handle);
        }
        if let Some(problem) = self.sound_library.failed.get(path) {
            return Err(AudioError::LoadError(format!(
                "'{path}' failed to load earlier: {problem}"
            )));
        }
        let relative = Path::new(path);
        let full_path = if relative.is_relative() {
            Path::new(&self.config.base_path).join(relative)
        } else {
            relative.to_path_buf()
        };
        match audio.load_sound(&full_path) {
            Ok(handle) => {
                self.sound_library.loaded.insert(path.to_string(), handle);
                Ok(handle)
            }
            Err(error) => {
                log::warn!("Sound '{path}' could not be loaded ({error}); it stays silent this session");
                self.sound_library.failed.insert(path.to_string(), error.to_string());
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::AssetConfig;

    /// A one-sample silent mono WAV.
    fn tiny_wav() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&38u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&44_100u32.to_le_bytes());
        bytes.extend_from_slice(&88_200u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&0i16.to_le_bytes());
        bytes
    }

    fn assets_at(base: &Path) -> AssetManager {
        AssetManager::headless(AssetConfig {
            base_path: base.to_string_lossy().to_string(),
            ..AssetConfig::default()
        })
    }

    #[test]
    fn a_sound_is_loaded_once_by_its_path_relative_to_the_asset_base() -> std::io::Result<()> {
        let base = tempfile::tempdir()?;
        std::fs::create_dir(base.path().join("sounds"))?;
        std::fs::write(base.path().join("sounds/blip.wav"), tiny_wav())?;
        let mut assets = assets_at(base.path());
        let mut audio = AudioManager::disabled();

        let first = assets.resolve_sound("sounds/blip.wav", &mut audio).expect("the file loads");
        std::fs::remove_file(base.path().join("sounds/blip.wav"))?;
        let second = assets.resolve_sound("sounds/blip.wav", &mut audio).expect("served from the cache");
        assert_eq!(first, second, "one handle a path, the file read once");
        Ok(())
    }

    #[test]
    fn a_failed_path_is_remembered_until_the_asset_base_changes() -> std::io::Result<()> {
        let base = tempfile::tempdir()?;
        let mut assets = assets_at(base.path());
        let mut audio = AudioManager::disabled();
        assert!(matches!(assets.resolve_sound("late.wav", &mut audio), Err(AudioError::IoError(_))));

        std::fs::write(base.path().join("late.wav"), tiny_wav())?;
        assert!(
            matches!(assets.resolve_sound("late.wav", &mut audio), Err(AudioError::LoadError(_))),
            "the failure is remembered, the file not read again"
        );

        assets.set_base_path(base.path().to_string_lossy().to_string());
        assert!(assets.resolve_sound("late.wav", &mut audio).is_ok(), "a base change forgets the failure");
        Ok(())
    }
}
