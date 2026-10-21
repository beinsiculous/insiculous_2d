//! Audio manager for loading and playing sounds.
//!
//! Split along the crate's three seams: this module owns the device/output
//! connection and SFX playback; the `playback` child module owns each playing
//! instance of a sound (stopped, re-mixed or held while disabled on its own);
//! the `music` child module owns recorded music and the volume buses; the `song`
//! child module owns songs played by the conductor. Tests live in the `tests`
//! child modules.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use common::clamp_volume;

use crate::error::{AudioError, AudioResult};
use crate::pan_source::{PanLevel, PanSource};
use crate::sound::{Bus, PlaybackId, SoundHandle, SoundSettings};

mod music;
mod playback;
pub(crate) mod song;

#[cfg(test)]
mod tests;

use playback::HeldPlayback;
use song::load::ReadySong;
use song::SongDeck;

/// Pure multiplication for a sound's effective playback volume.
pub(crate) fn effective_volume(base: f32, bus: f32, master: f32) -> f32 {
    base * bus * master
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
    /// Which playback this is (used by [`AudioManager::stop_playback`]).
    playback: PlaybackId,
    /// Which loaded sound this instance plays (used by [`AudioManager::stop`]).
    handle: SoundHandle,
    /// The per-sound volume from `SoundSettings`, kept so bus volume changes
    /// (`set_sfx_volume` / `set_master_volume`) can re-derive the sink volume.
    base_volume: f32,
    /// The pan the playing source reads once a frame.
    pan: Arc<PanLevel>,
}

/// A caller-built source playing under a bus (see [`AudioManager::play_source`]).
/// The sink is kept because dropping a rodio `Sink` stops its source.
struct ActiveSource {
    sink: Sink,
    bus: Bus,
}

/// Music requested while no output device was connected (web pre-gesture),
/// replayed by [`AudioManager::enable_output`] — one slot for both kinds, so the
/// last request wins across them. A recorded track stores the path, not the
/// bytes: the VFS re-read is cheap (an in-memory map on the web) and
/// `Decoder::new` only parses headers, so no full decode happens at replay
/// time. A song is kept built, so nothing can refuse it once the gesture comes.
enum PendingMusic {
    Recorded {
        path: PathBuf,
        volume: f32,
        looping: bool,
    },
    Song(Box<ReadySong>),
}

/// Live connection to an audio output device.
pub(super) struct AudioOutput {
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
/// still load and validate, and sound and recorded-music playback calls succeed
/// as no-ops. A song is the exception: a headless disabled manager plays it
/// silently on a virtual clock advanced by `update()` (so `is_song_playing()`
/// is `true` and its position moves), while a gesture-gated one holds it
/// pending. This keeps games runnable on headless machines and in CI, and keeps
/// a game's beat logic testable there.
pub struct AudioManager {
    /// Output device connection. `None` means disabled mode — sounds and
    /// recorded music no-op, a song runs on the virtual clock.
    output: Option<AudioOutput>,
    /// Cached sound data by handle.
    sounds: HashMap<u32, SoundData>,
    /// Next sound id to hand out. Instance-local so handle ids are unique
    /// within this manager and deterministic across managers.
    next_sound_id: u32,
    /// Currently active sound instances.
    active_sounds: Vec<ActiveSound>,
    /// Next playback id to hand out; manager-local like `next_sound_id`.
    next_playback_id: u64,
    /// Playbacks requested while disabled: a loop waits for a device (and reports
    /// playing until stopped), a one-shot is reported ended at the next `update`.
    held_playbacks: Vec<HeldPlayback>,
    /// Caller-built sources still playing.
    active_sources: Vec<ActiveSource>,
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
    /// The song playing, and what outlives it.
    songs: SongDeck,
    /// Disabled until a user gesture connects a device (the web), rather than
    /// running without one: a song requested now waits instead of playing on
    /// the virtual clock.
    awaits_gesture: bool,
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
            Self::awaiting_gesture()
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

    /// A disabled manager that waits for a user gesture to connect a device, as
    /// the web's does: a song requested before then is pending, not playing.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn awaiting_gesture() -> Self {
        let mut manager = Self::disabled();
        manager.awaits_gesture = true;
        manager
    }

    /// Whether an output device is connected. `false` means sounds and recorded
    /// music no-op; a song still runs on the virtual clock (see the type docs).
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
    /// This is the web upgrade path: browsers refuse an `AudioContext`
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
        self.awaits_gesture = false;

        self.connect_song_output();
        self.start_pending_music();
        self.start_held_loops();

        Ok(())
    }

    fn with_output(output: Option<AudioOutput>) -> Self {
        Self {
            output,
            sounds: HashMap::new(),
            next_sound_id: 1,
            active_sounds: Vec::new(),
            next_playback_id: 1,
            held_playbacks: Vec::new(),
            active_sources: Vec::new(),
            music_sink: None,
            pending_music: None,
            music_base_volume: 1.0,
            master_volume: 1.0,
            sfx_volume: 1.0,
            music_volume: 1.0,
            songs: SongDeck::new(1.0, 1.0),
            awaits_gesture: false,
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
    pub fn play(&mut self, handle: SoundHandle) -> AudioResult<PlaybackId> {
        self.play_with_settings(&handle, SoundSettings::default())
    }

    /// Play a sound with custom settings, returning the playback so it can be stopped,
    /// re-mixed and asked about on its own.
    ///
    /// Volume is clamped to 0.0..=1.0, speed floored at 0.1 and pan clamped to
    /// −1.0..=1.0 at the point of use, so directly-set `SoundSettings` fields cannot
    /// bypass the valid ranges. A disabled manager still validates the handle and mints
    /// the playback: a loop is held until a device connects (see
    /// [`AudioManager::is_playing`]), a one-shot ends at the next `update`.
    pub fn play_with_settings(
        &mut self,
        handle: &SoundHandle,
        settings: SoundSettings,
    ) -> AudioResult<PlaybackId> {
        if !self.sounds.contains_key(&handle.id) {
            return Err(AudioError::InvalidHandle(handle.id));
        }
        let playback = PlaybackId { id: self.next_playback_id };
        self.next_playback_id += 1;
        let held = HeldPlayback {
            playback,
            handle: *handle,
            pan: Arc::new(PanLevel::new(settings.pan)),
            settings,
        };
        if self.output.is_none() {
            self.held_playbacks.push(held);
        } else {
            self.start_playback(held)?;
        }
        Ok(playback)
    }

    /// Decode `held`'s sound and start it on the device. The pan wraps the repeat, never
    /// the reverse: rodio's repeat buffers its input's samples, and a pan inside it would
    /// freeze at the first pass's value.
    fn start_playback(&mut self, held: HeldPlayback) -> AudioResult<()> {
        let sound_data = self.sounds.get(&held.handle.id)
            .ok_or(AudioError::InvalidHandle(held.handle.id))?;
        let Some(output) = &self.output else {
            return Ok(());
        };

        // Decode straight from the shared cached bytes — no buffer copy.
        let cursor = Cursor::new(Arc::clone(&sound_data.bytes));
        let source = Decoder::new(cursor)
            .map_err(|e| AudioError::DecodeError(e.to_string()))?;

        let base_volume = clamp_volume(held.settings.volume);
        let pan = Arc::clone(&held.pan);
        let sink = if held.settings.looping {
            let looped = PanSource::new(source.repeat_infinite(), pan);
            self.start_sink(output, looped, base_volume, self.sfx_volume, false)?
        } else {
            self.start_sink(output, PanSource::new(source, pan), base_volume, self.sfx_volume, false)?
        };
        sink.set_speed(clamp_speed(held.settings.speed));

        self.active_sounds.push(ActiveSound {
            sink,
            playback: held.playback,
            handle: held.handle,
            base_volume,
            pan: held.pan,
        });

        Ok(())
    }

    /// Play a source the caller built (a synthesizer, a sequencer) on `bus`
    /// until its iterator ends.
    ///
    /// The source's samples are played as they come; the sink volume is the
    /// bus volume times master and follows later changes to either. Nothing
    /// here stops the source early: it plays until it returns `None`, and
    /// [`AudioManager::update`] then frees its sink. A disabled manager drops
    /// the source without pulling a sample and returns `Ok`, like every other
    /// playback call.
    pub fn play_source<S>(&mut self, source: S, bus: Bus) -> AudioResult<()>
    where
        S: Source<Item = f32> + Send + 'static,
    {
        let Some(output) = &self.output else {
            return Ok(());
        };
        let sink = self.start_sink(output, source, 1.0, self.bus_volume(bus), false)?;
        self.active_sources.push(ActiveSource { sink, bus });
        Ok(())
    }

    fn bus_volume(&self, bus: Bus) -> f32 {
        match bus {
            Bus::Music => self.music_volume,
            Bus::Sfx => self.sfx_volume,
        }
    }

    /// Create and configure a sink for playback under the given bus volume.
    pub(super) fn start_sink<S>(
        &self,
        output: &AudioOutput,
        source: S,
        base: f32,
        bus: f32,
        looping: bool,
    ) -> AudioResult<Sink>
    where
        S: Source + Send + 'static,
        S::Item: rodio::Sample + Send + Sync,
        f32: rodio::cpal::FromSample<S::Item>,
    {
        let sink = Sink::try_new(&output.handle)
            .map_err(|e| AudioError::StreamError(e.to_string()))?;
        sink.set_volume(effective_volume(base, bus, self.master_volume));
        if looping {
            sink.append(source.repeat_infinite());
        } else {
            sink.append(source);
        }
        Ok(sink)
    }

    /// Stop all currently playing instances of the given sound, a loop held while
    /// disabled included.
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
        self.held_playbacks.retain(|held| held.handle != handle);
    }

    /// Clean up finished sound instances and advance the song.
    ///
    /// Call this once per frame: it frees the sinks of sounds that have finished,
    /// retries song commands a full queue refused, notices a song whose output
    /// died, latches the song's beat edges and, without a device, advances a
    /// playing song by the wall time since the last call.
    pub fn update(&mut self) {
        self.update_at(common::clock::Instant::now());
    }

    /// [`AudioManager::update`] at `now`, so a test drives the song's clock.
    pub(crate) fn update_at(&mut self, now: common::clock::Instant) {
        self.active_sounds.retain(|active| !active.sink.empty());
        self.held_playbacks.retain(|held| held.settings.looping);
        self.active_sources.retain(|active| !active.sink.empty());
        self.update_song(now);
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
