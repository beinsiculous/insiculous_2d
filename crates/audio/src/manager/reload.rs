//! A loaded sound's bytes replaced behind its handle, so every caller holding the handle
//! plays the new sound from its next play on.
//!
//! What already plays decides for itself: a one-shot finishes on the bytes its sink was
//! started with; a loop takes the new bytes at its next boundary through its
//! [`LoopSlot`](crate::loop_source::LoopSlot), its pass in progress never cut — unless the
//! new bytes change the channel count or the sample rate, which the boundary cannot carry
//! (see `loop_source.rs`), and then it is restarted on the new bytes at once under its own
//! playback id, volume, pan and speed. A loop held while disabled decodes the bytes current
//! when the device arrives, so it needs nothing.

use std::io::Cursor;
use std::sync::Arc;

use rodio::{Decoder, Source};

use crate::error::{AudioError, AudioResult};
use crate::sound::{PlaybackId, SoundFormat, SoundFormatChange, SoundHandle, SoundSettings};

use super::playback::HeldPlayback;
use super::AudioManager;

impl AudioManager {
    /// Replace the bytes behind `handle` with `bytes`, decoded once here, on the calling
    /// thread, and pulled to their end — streamed and dropped when no loop will take them,
    /// kept as the buffer the loops read when one will, so the audio thread decodes nothing
    /// at the boundary and a reload nothing plays holds no decoded copy: a stream that does
    /// not open, or yields no sample, is refused with [`AudioError::DecodeError`] and the old
    /// bytes kept; a body shorter than before is the sound's new length. An unknown handle
    /// is [`AudioError::InvalidHandle`].
    ///
    /// Returns the format change when the new bytes have another channel count or sample
    /// rate than the old — every loop of the sound was then restarted on them at once —
    /// and `None` when every loop takes them at its next boundary.
    pub fn reload_sound(&mut self, handle: SoundHandle, bytes: Vec<u8>) -> AudioResult<Option<SoundFormatChange>> {
        let Some(sound) = self.sounds.get(&handle.id) else {
            return Err(AudioError::InvalidHandle(handle.id));
        };
        let bytes: Arc<[u8]> = Arc::from(bytes);
        let decoder = Decoder::new(Cursor::new(Arc::clone(&bytes)))
            .map_err(|error| AudioError::DecodeError(error.to_string()))?;
        let after = SoundFormat { channels: decoder.channels(), sample_rate: decoder.sample_rate() };
        let before = header_format(&sound.bytes);

        let mut waiting_slots = Vec::new();
        let mut restarts = Vec::new();
        for active in self.active_sounds.iter().filter(|active| active.handle == handle) {
            match &active.slot {
                Some(slot) if active.format == after => waiting_slots.push(Arc::clone(slot)),
                Some(_) => restarts.push(active.playback),
                None => {}
            }
        }
        if waiting_slots.is_empty() {
            // Nothing will read a buffer: the samples are counted as they stream and dropped,
            // so a long recording never sits decoded in memory for a reload nothing plays.
            if decoder.count() == 0 {
                return Err(no_sample());
            }
        } else {
            let replacement = decoder.buffered();
            if replacement.clone().count() == 0 {
                return Err(no_sample());
            }
            for slot in waiting_slots {
                match slot.lock() {
                    Ok(mut waiting) => *waiting = Some(replacement.clone()),
                    Err(_) => log::warn!("a loop of sound {} keeps its old bytes: its slot was poisoned", handle.id),
                }
            }
        }
        if let Some(sound) = self.sounds.get_mut(&handle.id) {
            sound.bytes = bytes;
        }
        for playback in restarts {
            self.restart_loop(playback);
        }
        Ok(before.filter(|before| *before != after).map(|before| SoundFormatChange { before, after }))
    }

    /// Start `playback`'s sound again from the current bytes under the same id, volume, pan
    /// and speed, and only then stop the sink it was playing on; a start that fails is
    /// logged and the old sink plays on, on the bytes it had, rather than falling silent
    /// under an id its holder still trusts.
    fn restart_loop(&mut self, playback: PlaybackId) {
        let Some(index) = self.active_sounds.iter().position(|active| active.playback == playback) else {
            return;
        };
        let held = {
            let old = &self.active_sounds[index];
            let settings = SoundSettings {
                volume: old.base_volume,
                speed: old.sink.speed(),
                looping: true,
                pan: old.pan.get(),
            };
            HeldPlayback { playback, handle: old.handle, settings, pan: Arc::clone(&old.pan) }
        };
        // The new entry lands at the end, so the old one's index holds.
        match self.start_playback(held) {
            Ok(()) => {
                let old = self.active_sounds.remove(index);
                old.sink.stop();
            }
            Err(error) => log::warn!("loop {} keeps its old sound: it did not restart on the reloaded one ({error})", playback.id()),
        }
    }
}

/// A header with no body behind it: `Decoder::new` reads the header alone, so the samples
/// are pulled before the bytes are taken.
fn no_sample() -> AudioError {
    AudioError::DecodeError("the stream holds no sample".to_string())
}

/// The format the header of bytes already loaded declares; `None` only if they no longer
/// open, which their load ruled out.
fn header_format(bytes: &Arc<[u8]>) -> Option<SoundFormat> {
    let decoder = Decoder::new(Cursor::new(Arc::clone(bytes))).ok()?;
    Some(SoundFormat { channels: decoder.channels(), sample_rate: decoder.sample_rate() })
}
