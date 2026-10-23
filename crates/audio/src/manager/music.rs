//! Recorded music and the volume buses — the second seam of
//! [`AudioManager`]: background-music lifecycle (play/stop/pause/resume)
//! plus the master/SFX/music volume accessors that re-derive live sink
//! volumes and hand the music and sfx gains to a playing song. Recorded
//! music shares one slot with a song: playing either stops the other.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use rodio::Decoder;

use crate::error::{AudioError, AudioResult};

use super::{clamp_volume, effective_volume, AudioManager, PendingMusic};

/// The recorded track last asked for, as it was asked for: what a host that stopped it
/// plays again. `play_music` always loops, so `looping` records the request rather than
/// offering a choice.
#[derive(Debug, Clone, PartialEq)]
pub struct MusicRequest {
    pub path: PathBuf,
    pub volume: f32,
    pub looping: bool,
}

impl AudioManager {
    /// Play background music from a file, looping forever.
    ///
    /// Only one music track can play at a time. Playing new music
    /// will stop the current track, and a song.
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
        // Whole-file read through the VFS (works on web too; music files are
        // loaded eagerly like all audio — no streaming, a by-design limit
        // documented in this crate's CLAUDE.md).
        // I/O failures convert via `From<io::Error>`.
        let bytes = common::vfs::read(path)?;

        let source = Decoder::new(Cursor::new(bytes))
            .map_err(|e| AudioError::DecodeError(format!("{}: {}", path.display(), e)))?;

        // Only now, with the file read and decoded, is the current music stopped: a
        // track that fails to load leaves what plays alone, as a song that fails to
        // load does. The slot is shared, so a playing or pending song stops too.
        self.stop_music();
        self.stop_song();
        self.music_request = Some(MusicRequest {
            path: path.to_path_buf(),
            volume,
            looping,
        });

        // Disabled mode: file was validated above, playback is a no-op.
        // Record the request AFTER validation so missing/corrupt files still
        // error and never leave a doomed pending entry behind.
        let Some(output) = &self.output else {
            self.pending_music = Some(PendingMusic::Recorded {
                path: path.to_path_buf(),
                volume,
                looping,
            });
            return Ok(());
        };

        let base_volume = clamp_volume(volume);
        let sink = self.start_sink(output, source, base_volume, self.music_volume, looping)?;

        self.music_sink = Some(sink);
        self.music_base_volume = base_volume;

        log::info!("Playing music: {} (looping: {})", path.display(), looping);

        Ok(())
    }

    /// Stop the current background music.
    ///
    /// Also clears a recorded track pending from disabled mode, so a
    /// stopped track cannot resurrect when a device connects later. A song
    /// is [`AudioManager::stop_song`]'s.
    pub fn stop_music(&mut self) {
        self.music_request = None;
        if matches!(self.pending_music, Some(PendingMusic::Recorded { .. })) {
            self.pending_music = None;
        }
        if let Some(sink) = self.music_sink.take() {
            sink.stop();
        }
    }

    /// Start music recorded while disabled. Failures (e.g. the file vanished
    /// since the request) are logged, not returned: the device upgrade
    /// itself succeeded, and an `Err` from [`AudioManager::enable_output`]
    /// must mean "still disabled".
    pub(super) fn start_pending_music(&mut self) {
        match self.pending_music.take() {
            None => {}
            Some(PendingMusic::Recorded {
                path,
                volume,
                looping,
            }) => {
                if let Err(e) = self.start_music(&path, volume, looping) {
                    log::warn!("Pending music failed to start: {e}");
                }
            }
            Some(PendingMusic::Song(ready)) => self.start_song(*ready),
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

    /// The recorded track last asked for — pending the web's gesture, or playing, paused
    /// included — with the volume it was asked with, so a host that stopped it can ask again as
    /// the game did. `None` after `stop_music`, after a song took the slot, and once a track's
    /// sink has emptied.
    #[must_use]
    pub fn music_request(&self) -> Option<&MusicRequest> {
        let pending = matches!(self.pending_music, Some(PendingMusic::Recorded { .. }));
        let sounding = self.music_sink.as_ref().is_some_and(|sink| !sink.empty());
        (pending || sounding).then_some(self.music_request.as_ref()).flatten()
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

    /// Re-derive sink volumes for the music track, every live SFX instance
    /// and every caller-built source from `base * bus * master`.
    fn update_all_volumes(&mut self) {
        self.songs.set_buses(self.music_volume, self.sfx_volume);
        if let Some(sink) = self.songs.sink() {
            sink.set_volume(effective_volume(1.0, 1.0, self.master_volume));
        }
        if let Some(ref sink) = self.music_sink {
            sink.set_volume(effective_volume(self.music_base_volume, self.music_volume, self.master_volume));
        }
        for active in &self.active_sounds {
            active
                .sink
                .set_volume(effective_volume(active.base_volume, self.sfx_volume, self.master_volume));
        }
        for active in &self.active_sources {
            active
                .sink
                .set_volume(effective_volume(1.0, self.bus_volume(active.bus), self.master_volume));
        }
    }
}
