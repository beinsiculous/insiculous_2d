//! A sample slot: one sample instrument's voices, for every source that triggers it.
//!
//! **No lanes.** A sample has no MIDI channels; the slot keeps its voices directly, each
//! with its own position and its own [`NoteTicket`], so two sources playing one sample —
//! or one source striking a key twice — hold independent voices, and a note-off ends only
//! the voice its ticket names.
//!
//! **One-shot or gated.** A one-shot voice ignores its note-off and plays to its
//! pitch-scaled end; a gated voice fades from its note-off. A one-shot is sounding until
//! its end, so it counts against the cap all that time.
//!
//! **Bounded twice.** At most `live_voice_cap` voices are live: a note-on past it fades
//! the oldest live voice out. Fading voices are bounded too: at the live cap plus the fade
//! budget the newest note-on is refused, so a burst of triggers inside one block never
//! piles up voices and never cuts one mid-fade, which would click. The trigger engine
//! asks for [`SampleSlot::free_voices`] first and refuses the whole trigger, cursor kept,
//! like every other overflow. The voice list is allocated to that ceiling when the slot
//! is built and never grows.
//!
//! **Forced termination** — a steal, a deactivation, a seek, a wrap, a pause, a stop —
//! fades every voice it ends, one-shots included, over the same 5 ms as a gated note-off.

use super::ledger::NoteTicket;
use crate::pcm::OwnedPcm;
use crate::sample_voice::{step_frames, SampleVoice};
use crate::song_file::SamplePlayback;
use crate::sources::SourceId;
use crate::SAMPLE_FADE_SECONDS;

#[cfg(test)]
mod tests;

/// The loudest MIDI velocity, which plays a sample at its instrument's gain.
const LOUDEST_VELOCITY: f32 = 127.0;

/// What a sample instrument plays: its sample and how.
#[derive(Debug, Clone)]
pub(crate) struct SampleInstrument {
    pub pcm: OwnedPcm,
    pub root_note: u8,
    pub gain: f32,
    pub playback: SamplePlayback,
}

/// How one sample slot is built: the conductor's rate, the two voice bounds, and the level
/// every voice plays at before its instrument's gain and its velocity.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SampleShape {
    pub output_rate: u32,
    pub live_voice_cap: usize,
    pub fade_budget: usize,
    pub voice_level: f32,
}

/// Running totals.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SampleCounters {
    pub voices_started: u64,
    /// Live voices faded out to make room under the live cap.
    pub steals: u64,
    /// Note-ons refused at the ceiling: a scheduled note whose start found no room.
    pub refusals: u64,
    /// The most voices, fading ones included, held at once.
    pub peak_voices: usize,
}

/// One sample instrument's voices. See the module docs.
pub struct SampleSlot {
    instrument: SampleInstrument,
    output_rate: u32,
    fade_frames: u32,
    live_voice_cap: usize,
    voice_ceiling: usize,
    voice_level: f32,
    /// Oldest first; never longer than `voice_ceiling`.
    voices: Vec<SampleVoice>,
    next_ticket: u64,
    counters: SampleCounters,
}

impl SampleSlot {
    pub(crate) fn new(instrument: SampleInstrument, shape: SampleShape) -> Self {
        let live_voice_cap = shape.live_voice_cap.max(1);
        let voice_ceiling = live_voice_cap + shape.fade_budget.max(1);
        let fade_frames = (SAMPLE_FADE_SECONDS * f64::from(shape.output_rate)).round() as u32;
        Self {
            instrument,
            output_rate: shape.output_rate,
            fade_frames: fade_frames.max(1),
            live_voice_cap,
            voice_ceiling,
            voice_level: shape.voice_level,
            voices: Vec::with_capacity(voice_ceiling),
            next_ticket: 0,
            counters: SampleCounters::default(),
        }
    }

    /// Start a voice at `key` for `source`; the ticket ends that voice. `None` at the
    /// ceiling: the newest note is refused, and no voice is cut.
    pub fn note_on(&mut self, source: SourceId, key: u8, velocity: u8) -> Option<NoteTicket> {
        if self.voices.len() >= self.voice_ceiling {
            self.counters.refusals += 1;
            return None;
        }
        let ticket = NoteTicket::numbered(self.next_ticket);
        self.next_ticket += 1;
        if self.live_voice_count() >= self.live_voice_cap {
            if let Some(oldest) = self.voices.iter_mut().find(|voice| !voice.is_fading()) {
                oldest.fade(self.fade_frames);
                self.counters.steals += 1;
            }
        }
        let instrument = &self.instrument;
        let step = step_frames(
            instrument.root_note,
            key,
            instrument.pcm.sample_rate(),
            self.output_rate,
        );
        let gain = self.voice_level * instrument.gain * f32::from(velocity) / LOUDEST_VELOCITY;
        self.voices
            .push(SampleVoice::new(source, ticket, step, gain));
        self.counters.voices_started += 1;
        self.counters.peak_voices = self.counters.peak_voices.max(self.voices.len());
        Some(ticket)
    }

    /// End the voice `ticket` started: a gated voice fades, a one-shot plays on.
    pub fn note_off(&mut self, ticket: NoteTicket) {
        if self.instrument.playback == SamplePlayback::OneShot {
            return;
        }
        let fade_frames = self.fade_frames;
        if let Some(voice) = self.voices.iter_mut().find(|voice| voice.ticket == ticket) {
            voice.fade(fade_frames);
        }
    }

    /// Whether the voice `ticket` started still plays and is not fading.
    pub fn is_sounding(&self, ticket: NoteTicket) -> bool {
        self.voices
            .iter()
            .any(|voice| voice.ticket == ticket && !voice.is_fading())
    }

    /// Fade every voice `source` holds.
    pub fn release_source(&mut self, source: SourceId) {
        let fade_frames = self.fade_frames;
        for voice in self
            .voices
            .iter_mut()
            .filter(|voice| voice.source == source)
        {
            voice.fade(fade_frames);
        }
    }

    /// Fade every voice.
    pub fn release_all(&mut self) {
        let fade_frames = self.fade_frames;
        for voice in &mut self.voices {
            voice.fade(fade_frames);
        }
    }

    /// Add every voice into `left` and `right` (equal lengths); a voice that ends is
    /// dropped.
    pub fn render_add(&mut self, left: &mut [f32], right: &mut [f32]) {
        let (pcm, fade_frames) = (&self.instrument.pcm, self.fade_frames);
        self.voices
            .retain_mut(|voice| voice.render_add(pcm, fade_frames, left, right));
    }

    /// Voices not fading.
    pub fn live_voice_count(&self) -> usize {
        self.voices
            .iter()
            .filter(|voice| !voice.is_fading())
            .count()
    }

    /// Every voice still playing, fading ones included.
    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    /// Note-ons the slot can take before the ceiling refuses one.
    pub fn free_voices(&self) -> usize {
        self.voice_ceiling.saturating_sub(self.voices.len())
    }

    /// The most voices the slot ever holds: the live cap plus the fade budget.
    pub fn voice_ceiling(&self) -> usize {
        self.voice_ceiling
    }

    /// Output frames a fade lasts.
    pub fn fade_frames(&self) -> u32 {
        self.fade_frames
    }

    /// How many output frames a voice at `key` plays before its pitch-scaled end, unfaded.
    pub fn duration_frames(&self, key: u8) -> f64 {
        let instrument = &self.instrument;
        let step = step_frames(
            instrument.root_note,
            key,
            instrument.pcm.sample_rate(),
            self.output_rate,
        );
        (instrument.pcm.frame_count() - 1) as f64 / step
    }

    pub fn pcm(&self) -> &OwnedPcm {
        &self.instrument.pcm
    }

    pub fn counters(&self) -> SampleCounters {
        self.counters
    }
}
