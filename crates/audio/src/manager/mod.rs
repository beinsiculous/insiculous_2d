//! Audio manager for loading and playing sounds.
//!
//! Split along the crate's two seams: this module owns the device/output
//! connection and SFX playback; the `music` child module owns music
//! playback and the volume buses. Tests live in the `tests` child module.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use crate::error::{AudioError, AudioResult};
use crate::sound::{SoundHandle, SoundSettings};

mod music;

#[cfg(test)]
mod tests;

/// Clamp a volume value to the valid 0.0..=1.0 range.
fn clamp_volume(volume: f32) -> f32 {
    volume.clamp(0.0, 1.0)
}

/// Floor a playback speed at 0.1 (rodio misbehaves at zero/negative speeds).
fn clamp_speed(speed: f32) -> f32 {
    speed.max(0.1)
}

/// Cached sound data that can be played multiple times.
struct SoundData {
    /// Raw audio bytes for replay. `Arc<[u8]>` implements `AsRef<[u8]>`, so a
    /// `Cursor<Arc<[u8]>>` can feed a decoder without copying the buffer.
    bytes: Arc<[u8]>,
}

/// Active sound playback instance.
struct ActiveSound {
    sink: Sink,
    /// Which loaded sound this instance plays (used by [`AudioManager::stop`]).
    handle: SoundHandle,
    /// The per-sound volume from `SoundSettings`, kept so bus volume changes
    /// (`set_sfx_volume` / `set_master_volume`) can re-derive the sink volume.
    base_volume: f32,
}

/// Music requested while no output device was connected (web pre-gesture),
/// replayed by [`AudioManager::enable_output`]. Stores the path, not the
/// bytes: the VFS re-read is cheap (an in-memory map on the web) and
/// `Decoder::new` only parses headers, so no full decode happens at replay
/// time.
struct PendingMusic {
    path: PathBuf,
    volume: f32,
    looping: bool,
}

/// Live connection to an audio output device.
struct AudioOutput {
    /// Audio output stream (must be kept alive).
    _stream: OutputStream,
    /// Handle to the output stream for creating sinks.
    handle: OutputStreamHandle,
}

/// Manages audio playback for the game engine.
///
/// The AudioManager handles:
/// - Loading and caching sound files
/// - Playing sounds with configurable settings
/// - Managing active sound instances
/// - Background music playback (looping or one-shot)
///
/// A manager can run in *disabled* mode (no audio device available): sounds
/// still load and validate, playback calls succeed as no-ops. This keeps
/// games runnable on headless machines and in CI.
pub struct AudioManager {
    /// Output device connection. `None` means disabled mode — playback no-ops.
    output: Option<AudioOutput>,
    /// Cached sound data by handle.
    sounds: HashMap<u32, SoundData>,
    /// Next sound id to hand out. Instance-local so handle ids are unique
    /// within this manager and deterministic across managers.
    next_sound_id: u32,
    /// Currently active sound instances.
    active_sounds: Vec<ActiveSound>,
    /// Current background music sink.
    music_sink: Option<Sink>,
    /// Music requested while disabled, started when a device connects.
    pending_music: Option<PendingMusic>,
    /// Per-track volume of the current music, kept so bus volume changes can
    /// re-derive the music sink volume.
    music_base_volume: f32,
    /// Master volume for all sounds.
    master_volume: f32,
    /// Volume for sound effects.
    sfx_volume: f32,
    /// Volume for background music.
    music_volume: f32,
}

impl AudioManager {
    /// Create a new audio manager.
    ///
    /// This initializes the audio device and output stream.
    pub fn new() -> AudioResult<Self> {
        let mut manager = Self::disabled();
        manager.enable_output()?;
        Ok(manager)
    }

    /// Create a disabled audio manager that has no output device.
    ///
    /// Sounds can still be loaded (and are decode-validated); all playback
    /// calls succeed silently. Use this when audio hardware is unavailable.
    pub fn disabled() -> Self {
        Self::with_output(None)
    }

    /// Create an audio manager, falling back to disabled mode if no audio
    /// device is available. Never fails — the game keeps running either way.
    ///
    /// On the web this always starts disabled: browsers refuse (or silently
    /// suspend) an `AudioContext` created outside a user gesture, and a
    /// successful `try_default()` does NOT prove the context is running.
    /// The engine upgrades to a real device on the first user gesture via
    /// [`AudioManager::enable_output`].
    pub fn new_or_disabled() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            log::info!("Web audio starts disabled; first user gesture upgrades it");
            Self::disabled()
        }
        #[cfg(not(target_arch = "wasm32"))]
        match Self::new() {
            Ok(manager) => manager,
            Err(e) => {
                log::warn!("Failed to initialize audio: {}. Audio will be disabled.", e);
                Self::disabled()
            }
        }
    }

    /// Whether an output device is connected. `false` means playback no-ops.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.output.is_some()
    }

    /// Connect a disabled manager to a real output device.
    ///
    /// No-op `Ok` when already enabled. On success every sound loaded while
    /// disabled stays playable (handles, ids, and bus volumes are untouched)
    /// and music requested while disabled starts playing. On `Err` the
    /// manager is unchanged: still disabled, still fully functional as a
    /// no-op, and any pending music request is retained for a later attempt.
    ///
    /// This is the web (H7) upgrade path: browsers refuse an `AudioContext`
    /// created outside a user gesture, so the engine calls this from the
    /// first activation gesture. A successful `OutputStream::try_default()`
    /// at startup does NOT prove the context is running — only construction
    /// inside a gesture handler does.
    pub fn enable_output(&mut self) -> AudioResult<()> {
        if self.output.is_some() {
            return Ok(());
        }

        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| AudioError::DeviceInitError(e.to_string()))?;

        self.output = Some(AudioOutput {
            _stream: stream,
            handle: stream_handle,
        });
        log::debug!("Audio output enabled");

        self.start_pending_music();

        Ok(())
    }

    fn with_output(output: Option<AudioOutput>) -> Self {
        Self {
            output,
            sounds: HashMap::new(),
            next_sound_id: 1,
            active_sounds: Vec::new(),
            music_sink: None,
            pending_music: None,
            music_base_volume: 1.0,
            master_volume: 1.0,
            sfx_volume: 1.0,
            music_volume: 1.0,
        }
    }

    /// Load a sound from a file path.
    ///
    /// The sound is cached and can be played multiple times.
    /// Supports WAV, MP3, OGG, and FLAC formats.
    ///
    /// File read failures return [`AudioError::IoError`]; undecodable data
    /// returns [`AudioError::DecodeError`].
    pub fn load_sound<P: AsRef<Path>>(&mut self, path: P) -> AudioResult<SoundHandle> {
        let path = path.as_ref();

        // Read the entire file into memory for replay support. Goes through
        // the VFS so path-based loads also work on the web (prefetched map).
        // I/O failures convert via `From<io::Error>`.
        let bytes = common::vfs::read(path)?;

        let handle = self.load_sound_from_bytes(bytes).map_err(|e| match e {
            // Re-attach the file path for decode diagnostics.
            AudioError::DecodeError(msg) => {
                AudioError::DecodeError(format!("{}: {}", path.display(), msg))
            }
            other => other,
        })?;

        log::debug!("Loaded sound: {} (handle: {})", path.display(), handle.id);

        Ok(handle)
    }

    /// Load a sound from raw bytes.
    ///
    /// Useful for embedded audio or procedurally generated sounds.
    pub fn load_sound_from_bytes(&mut self, bytes: Vec<u8>) -> AudioResult<SoundHandle> {
        let bytes: Arc<[u8]> = Arc::from(bytes);

        // Validate that the audio can be decoded. Cloning the Arc is cheap
        // (reference count bump, no buffer copy). rodio's Decoder requires a
        // `'static` reader, so a borrowed `Cursor<&[u8]>` cannot be used.
        Decoder::new(Cursor::new(Arc::clone(&bytes)))
            .map_err(|e| AudioError::DecodeError(e.to_string()))?;

        let handle = SoundHandle::from_id(self.next_sound_id);
        self.next_sound_id += 1;
        self.sounds.insert(handle.id, SoundData { bytes });

        log::debug!("Loaded sound from bytes (handle: {})", handle.id);

        Ok(handle)
    }

    /// Play a sound with default settings.
    pub fn play(&mut self, handle: SoundHandle) -> AudioResult<()> {
        self.play_with_settings(&handle, SoundSettings::default())
    }

    /// Play a sound with custom settings.
    ///
    /// Volume is clamped to 0.0..=1.0 and speed floored at 0.1 here, so
    /// directly-set `SoundSettings` fields cannot bypass the valid ranges.
    pub fn play_with_settings(
        &mut self,
        handle: &SoundHandle,
        settings: SoundSettings,
    ) -> AudioResult<()> {
        let sound_data = self.sounds.get(&handle.id)
            .ok_or(AudioError::InvalidHandle(handle.id))?;

        // Disabled mode: handle was validated above, playback is a no-op.
        let Some(output) = &self.output else {
            return Ok(());
        };

        let sink = Sink::try_new(&output.handle)
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        // Decode straight from the shared cached bytes — no buffer copy.
        let cursor = Cursor::new(Arc::clone(&sound_data.bytes));
        let source = Decoder::new(cursor)
            .map_err(|e| AudioError::DecodeError(e.to_string()))?;

        // Clamp at point of use: SoundSettings fields are public, so builder
        // clamps can be bypassed.
        let base_volume = clamp_volume(settings.volume);
        sink.set_volume(base_volume * self.sfx_volume * self.master_volume);
        sink.set_speed(clamp_speed(settings.speed));

        if settings.looping {
            sink.append(source.repeat_infinite());
        } else {
            sink.append(source);
        }

        self.active_sounds.push(ActiveSound {
            sink,
            handle: *handle,
            base_volume,
        });

        Ok(())
    }

    /// Stop all currently playing instances of the given sound.
    ///
    /// Instances of other sounds and the music track are unaffected.
    /// Unknown handles or handles with no active instances are a no-op.
    pub fn stop(&mut self, handle: SoundHandle) {
        self.active_sounds.retain(|active| {
            if active.handle == handle {
                active.sink.stop();
                false
            } else {
                true
            }
        });
    }

    /// Clean up finished sound instances.
    ///
    /// Call this periodically (e.g., once per frame) to free resources
    /// from sounds that have finished playing.
    pub fn update(&mut self) {
        self.active_sounds.retain(|active| !active.sink.empty());
    }

    /// Unload a sound from the cache.
    ///
    /// Already-playing instances of the sound continue to completion (each
    /// playback holds its own reference to the audio data); only future
    /// `play` calls with this handle will fail.
    pub fn unload(&mut self, handle: SoundHandle) {
        self.sounds.remove(&handle.id);
    }
}
