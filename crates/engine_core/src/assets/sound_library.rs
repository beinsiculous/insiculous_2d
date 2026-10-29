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
//! A host that watches the files reloads a path when it changes on disk
//! ([`AssetManager::reload_sound`]): a loaded one is re-read behind the handle it already
//! has, so every caller and emitter holding it stays valid; a failed one is forgotten and
//! tried again — the one way a remembered failure is lifted short of a base change.
//!
//! Child module of `assets` so it can reach `AssetManager`'s private fields, the
//! arrangement `sprite_sheet.rs` uses.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use audio::{AudioError, AudioManager, SoundFormatChange, SoundHandle};

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

/// What [`AssetManager::reload_sound`] did with a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundReload {
    /// The path was loaded, and its handle now plays the file as it is; the format change,
    /// when there was one, restarted its loops at once.
    Reloaded { format_change: Option<SoundFormatChange> },
    /// The path had failed, and now loads: its handle is served from here on.
    NowLoads,
    /// Nothing has played the path yet; the first play reads the file as it is then.
    NotLoaded,
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
        match audio.load_sound(self.sound_file(path)) {
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

    /// Take the file at `path` as it is on disk now. A loaded path is re-read through the
    /// asset base and its handle's bytes replaced (`AudioManager::reload_sound`): the handle
    /// is unchanged, and a file that does not read or decode keeps the old sound and returns
    /// the error. A failed path is forgotten and resolved again, a new failure remembered
    /// again. A path in neither has nothing to reload.
    pub fn reload_sound(&mut self, path: &str, audio: &mut AudioManager) -> Result<SoundReload, AudioError> {
        if let Some(handle) = self.sound_library.loaded.get(path).copied() {
            let bytes = common::vfs::read(&self.sound_file(path))?;
            let format_change = audio.reload_sound(handle, bytes)?;
            return Ok(SoundReload::Reloaded { format_change });
        }
        if self.sound_library.failed.remove(path).is_some() {
            self.resolve_sound(path, audio)?;
            return Ok(SoundReload::NowLoads);
        }
        Ok(SoundReload::NotLoaded)
    }

    /// Every path the sound tier has loaded or failed, as it was named — what a host watches
    /// to reload.
    pub fn sound_paths(&self) -> impl Iterator<Item = &str> {
        self.sound_library.loaded.keys().chain(self.sound_library.failed.keys()).map(String::as_str)
    }

    /// The paths whose load failed: a file there is expected to be missing.
    pub fn failed_sound_paths(&self) -> impl Iterator<Item = &str> {
        self.sound_library.failed.keys().map(String::as_str)
    }

    /// `path` as a file: relative to the asset base like a texture, or as given when absolute.
    fn sound_file(&self, path: &str) -> PathBuf {
        let relative = Path::new(path);
        if relative.is_relative() {
            Path::new(&self.config.base_path).join(relative)
        } else {
            relative.to_path_buf()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::AssetConfig;

    /// A silent mono WAV `samples` samples long.
    fn wav(samples: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + 2 * samples).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&44_100u32.to_le_bytes());
        bytes.extend_from_slice(&88_200u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(2 * samples).to_le_bytes());
        for _ in 0..samples {
            bytes.extend_from_slice(&0i16.to_le_bytes());
        }
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
        std::fs::write(base.path().join("sounds/blip.wav"), wav(1))?;
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

        std::fs::write(base.path().join("late.wav"), wav(1))?;
        assert!(
            matches!(assets.resolve_sound("late.wav", &mut audio), Err(AudioError::LoadError(_))),
            "the failure is remembered, the file not read again"
        );

        assets.set_base_path(base.path().to_string_lossy().to_string());
        assert!(assets.resolve_sound("late.wav", &mut audio).is_ok(), "a base change forgets the failure");
        Ok(())
    }

    #[test]
    fn a_changed_file_is_served_from_the_same_handle() -> std::io::Result<()> {
        let base = tempfile::tempdir()?;
        std::fs::write(base.path().join("hum.wav"), wav(1))?;
        let mut assets = assets_at(base.path());
        let mut audio = AudioManager::disabled();
        let handle = assets.resolve_sound("hum.wav", &mut audio).expect("the file loads");

        std::fs::write(base.path().join("hum.wav"), wav(441))?;
        assert_eq!(
            assets.reload_sound("hum.wav", &mut audio).expect("the new file decodes"),
            SoundReload::Reloaded { format_change: None }
        );
        assert_eq!(assets.resolve_sound("hum.wav", &mut audio).expect("still loaded"), handle, "the same handle");
        assert!(audio.play(handle).is_ok(), "which every holder still plays");
        Ok(())
    }

    #[test]
    fn a_failed_path_whose_file_appears_is_retried_once_and_serves() -> std::io::Result<()> {
        let base = tempfile::tempdir()?;
        let mut assets = assets_at(base.path());
        let mut audio = AudioManager::disabled();
        assert_eq!(assets.reload_sound("never.wav", &mut audio).expect("nothing to do"), SoundReload::NotLoaded);
        assert!(assets.resolve_sound("late.wav", &mut audio).is_err());
        assert_eq!(assets.failed_sound_paths().collect::<Vec<_>>(), ["late.wav"]);

        std::fs::write(base.path().join("late.wav"), wav(1))?;
        assert_eq!(assets.reload_sound("late.wav", &mut audio).expect("it loads now"), SoundReload::NowLoads);
        let handle = assets.resolve_sound("late.wav", &mut audio).expect("served from here on");
        assert!(audio.play(handle).is_ok());
        assert_eq!(assets.sound_paths().collect::<Vec<_>>(), ["late.wav"], "loaded, no longer failed");
        assert_eq!(assets.failed_sound_paths().count(), 0);
        Ok(())
    }

    #[test]
    fn a_reload_that_fails_to_decode_keeps_the_path_loaded_and_returns_the_error() -> std::io::Result<()> {
        let base = tempfile::tempdir()?;
        std::fs::write(base.path().join("hum.wav"), wav(1))?;
        let mut assets = assets_at(base.path());
        let mut audio = AudioManager::disabled();
        let handle = assets.resolve_sound("hum.wav", &mut audio).expect("the file loads");

        std::fs::write(base.path().join("hum.wav"), b"half a recording")?;
        assert!(matches!(assets.reload_sound("hum.wav", &mut audio), Err(AudioError::DecodeError(_))));
        assert_eq!(assets.resolve_sound("hum.wav", &mut audio).expect("kept"), handle, "the old sound stays loaded");
        assert_eq!(assets.failed_sound_paths().count(), 0, "a loaded path never becomes a failed one");
        Ok(())
    }
}
