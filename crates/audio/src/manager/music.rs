//! Music playback and the volume buses — the second seam of
//! [`AudioManager`]: background-music lifecycle (play/stop/pause/resume)
//! plus the master/SFX/music volume accessors that re-derive live sink
//! volumes.

use std::io::Cursor;
use std::path::Path;

use rodio::{Decoder, Sink, Source};

use crate::error::{AudioError, AudioResult};

use super::{clamp_volume, AudioManager, PendingMusic};

impl AudioManager {
    /// Play background music from a file, looping forever.
    ///
    /// Only one music track can play at a time. Playing new music
    /// will stop the current track.
    pub fn play_music<P: AsRef<Path>>(&mut self, path: P) -> AudioResult<()> {
        self.play_music_with_volume(path, 1.0)
    }

    /// Play looping background music with a specific volume.
    pub fn play_music_with_volume<P: AsRef<Path>>(
        &mut self,
        path: P,
        volume: f32,
    ) -> AudioResult<()> {
        self.start_music(path.as_ref(), volume, true)
    }

    /// Play background music once (no looping), with a specific volume.
    ///
    /// The track plays to completion and then stops; use
    /// [`AudioManager::play_music`] for looping playback.
    pub fn play_music_once<P: AsRef<Path>>(
        &mut self,
        path: P,
        volume: f32,
    ) -> AudioResult<()> {
        self.start_music(path.as_ref(), volume, false)
    }

    /// Shared music startup: stops current music, opens and decodes the file,
    /// then starts playback (looping or one-shot).
    ///
    /// In disabled mode the file is still opened and decode-validated, but
    /// playback is a no-op: the call returns `Ok` while
    /// [`AudioManager::is_music_playing`] keeps reporting `false`. This keeps
    /// load errors observable on headless machines without pretending audio
    /// is audible. The request is remembered so
    /// [`AudioManager::enable_output`] can start it once a device connects
    /// (web: the first user gesture); the last request wins.
    fn start_music(&mut self, path: &Path, volume: f32, looping: bool) -> AudioResult<()> {
        // Stop current music if any
        self.stop_music();

        // Whole-file read through the VFS (works on web too; music files are
        // loaded eagerly like all audio — see TECH_DEBT on streaming).
        // I/O failures convert via `From<io::Error>`.
        let bytes = common::vfs::read(path)?;

        let source = Decoder::new(Cursor::new(bytes))
            .map_err(|e| AudioError::DecodeError(format!("{}: {}", path.display(), e)))?;

        // Disabled mode: file was validated above, playback is a no-op.
        // Record the request AFTER validation so missing/corrupt files still
        // error and never leave a doomed pending entry behind.
        let Some(output) = &self.output else {
            self.pending_music = Some(PendingMusic {
                path: path.to_path_buf(),
                volume,
                looping,
            });
            return Ok(());
        };

        let sink = Sink::try_new(&output.handle)
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        let base_volume = clamp_volume(volume);
        sink.set_volume(base_volume * self.music_volume * self.master_volume);
        if looping {
            sink.append(source.repeat_infinite());
        } else {
            sink.append(source);
        }

        self.music_sink = Some(sink);
        self.music_base_volume = base_volume;

        log::info!("Playing music: {} (looping: {})", path.display(), looping);

        Ok(())
    }

    /// Stop the current background music.
    ///
    /// Also clears any music request pending from disabled mode, so a
    /// stopped track cannot resurrect when a device connects later.
    pub fn stop_music(&mut self) {
        self.pending_music = None;
        if let Some(sink) = self.music_sink.take() {
            sink.stop();
        }
    }

    /// Start music recorded while disabled. Failures (e.g. the file vanished
    /// since the request) are logged, not returned: the device upgrade
    /// itself succeeded, and an `Err` from [`AudioManager::enable_output`]
    /// must mean "still disabled".
    pub(super) fn start_pending_music(&mut self) {
        let Some(pending) = self.pending_music.take() else {
            return;
        };
        if let Err(e) = self.start_music(&pending.path, pending.volume, pending.looping) {
            log::warn!("Pending music failed to start: {e}");
        }
    }

    /// Pause the current background music.
    pub fn pause_music(&mut self) {
        if let Some(ref sink) = self.music_sink {
            sink.pause();
        }
    }

    /// Resume the paused background music.
    pub fn resume_music(&mut self) {
        if let Some(ref sink) = self.music_sink {
            sink.play();
        }
    }

    /// Check if music is currently playing.
    ///
    /// Always `false` in disabled mode, even after a successful
    /// [`AudioManager::play_music`] call (playback is a no-op there; the
    /// request is pending until [`AudioManager::enable_output`] succeeds).
    #[must_use]
    pub fn is_music_playing(&self) -> bool {
        self.music_sink.as_ref().is_some_and(|s| !s.is_paused() && !s.empty())
    }

    /// Set the master volume (affects all audio, including sounds and music
    /// that are already playing).
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = clamp_volume(volume);
        self.update_all_volumes();
    }

    /// Get the current master volume.
    #[must_use]
    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Set the sound effects volume (re-applied to currently playing sounds).
    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = clamp_volume(volume);
        self.update_all_volumes();
    }

    /// Get the current sound effects volume.
    #[must_use]
    pub fn sfx_volume(&self) -> f32 {
        self.sfx_volume
    }

    /// Set the music volume (re-applied to currently playing music).
    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = clamp_volume(volume);
        self.update_all_volumes();
    }

    /// Get the current music volume.
    #[must_use]
    pub fn music_volume(&self) -> f32 {
        self.music_volume
    }

    /// Re-derive sink volumes for the music track and every live SFX
    /// instance from `base * bus * master`.
    fn update_all_volumes(&mut self) {
        if let Some(ref sink) = self.music_sink {
            sink.set_volume(self.music_base_volume * self.music_volume * self.master_volume);
        }
        for active in &self.active_sounds {
            active
                .sink
                .set_volume(active.base_volume * self.sfx_volume * self.master_volume);
        }
    }
}
