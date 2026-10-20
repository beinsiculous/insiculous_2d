//! The background sequencer: a song's background score turned into channel messages at
//! the right samples, with no synthesizer in sight.
//!
//! A [`Sequencer`] owns the song clock (a sample position in the song) and drives a
//! [`SequencerSink`] — the conductor's background synthesizers, or a test's recorder —
//! with three things in order: messages, reconstructions and frames to render.
//!
//! **Events land on the sink's grain.** A SoundFont synthesizer renders in fixed blocks
//! and a message reaches the sound at the next block it starts, so the sequencer sends,
//! at the first frame of each grain, every event whose sample falls inside that grain —
//! up to a grain early, never late. The grain is counted from the last reconstruction,
//! which the sink starts on a fresh block, so an event exactly on a reconstruction (a
//! loop's downbeat, a seek's destination) sounds on its exact sample.
//!
//! **The loop seam is sample-exact.** A render that crosses the song's end is split
//! there: the frames before the seam, then a reconstruction at the loop start (every
//! note released, every channel's state rebuilt), then the frames after it. The loop
//! never rounds to a block, so it never drifts.
//!
//! **Reconstruction** (a seek, a wrap, a resume) folds the score's state events up to
//! and including the destination tick — so a seek to 0 keeps tick 0's tempo and
//! programs — and its note events before it; a note that spans the destination is not
//! sounded again, and its note-off, when it comes, is not sent.
//!
//! **Same-key overlaps coalesce per channel.** The synthesizer releases every voice of a
//! key with one note-off, so the sequencer counts the sounding instances of each
//! (channel, key) and sends the note-off only when the last one ends: a piano
//! reduction's pedalled repeat sustains as written.

use std::sync::Arc;

mod fold;
mod score;
#[cfg(test)]
mod tests;

pub use fold::{ChannelStateFold, CHANNEL_COUNT};
pub use score::{BackgroundScore, ChannelMessage, ScoreEvent};

const KEYS_PER_CHANNEL: usize = 128;

/// What a sequencer drives: a synthesizer, or a test's recorder.
pub trait SequencerSink {
    /// Send one channel message; it reaches the sound at the start of the next grain.
    fn send(&mut self, channel: u8, message: ChannelMessage);
    /// Release every sounding note and rebuild every channel's state from `state`, on a
    /// synthesizer whose next block starts on the next frame rendered.
    fn reconstruct(&mut self, state: &ChannelStateFold);
    /// Release every sounding note; the tails ring out.
    fn release_all(&mut self);
    /// Render the next `frames` frames with everything sent so far.
    fn render(&mut self, frames: usize);
}

/// Plays a [`BackgroundScore`] into a [`SequencerSink`]. See the module docs.
pub struct Sequencer {
    score: Arc<BackgroundScore>,
    sample_rate: u32,
    grain_frames: usize,
    looping: bool,
    end_sample: u64,
    /// The next event to send.
    cursor: usize,
    position_sample: u64,
    frames_into_grain: usize,
    /// Note-ons before this index were released by the last reconstruction, so their
    /// note-offs are not sent.
    released_before: usize,
    /// Sounding instances of each (channel, key), indexed `channel * 128 + key`.
    sounding: Vec<u16>,
    fold: Box<ChannelStateFold>,
    ended: bool,
    reconstructions: u64,
    loop_wraps: u64,
}

impl Sequencer {
    /// A sequencer at the start of `score`, sending events on a grain of `grain_frames`
    /// (at least one) at `sample_rate`. Nothing has been sent: the first
    /// [`Sequencer::seek_to_tick`] sets the channels up.
    pub fn new(
        score: Arc<BackgroundScore>,
        sample_rate: u32,
        grain_frames: usize,
        looping: bool,
    ) -> Self {
        let end_sample = score
            .tempo_map()
            .sample_at_tick(score.end_tick(), sample_rate);
        Self {
            score,
            sample_rate,
            grain_frames: grain_frames.max(1),
            looping,
            end_sample,
            cursor: 0,
            position_sample: 0,
            frames_into_grain: 0,
            released_before: 0,
            sounding: vec![0; CHANNEL_COUNT * KEYS_PER_CHANNEL],
            fold: Box::default(),
            ended: false,
            reconstructions: 0,
            loop_wraps: 0,
        }
    }

    pub fn score(&self) -> &BackgroundScore {
        &self.score
    }

    /// The sample of the song the next rendered frame plays.
    pub fn position_sample(&self) -> u64 {
        self.position_sample
    }

    /// The tick the next rendered frame plays: the last tick at or before it.
    pub fn position_tick(&self) -> u64 {
        self.score
            .tempo_map()
            .tick_at_sample(self.position_sample, self.sample_rate)
    }

    /// The song's length in samples; a looping song wraps here.
    pub fn end_sample(&self) -> u64 {
        self.end_sample
    }

    /// Whether a song that does not loop has played to its end.
    pub fn has_ended(&self) -> bool {
        self.ended
    }

    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// Loop at the song's end, or stop there. Takes effect at the next seam.
    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    /// Seeks and loop wraps so far.
    pub fn reconstructions(&self) -> u64 {
        self.reconstructions
    }

    pub fn loop_wraps(&self) -> u64 {
        self.loop_wraps
    }

    /// Move to `tick` (past the end means the end) and rebuild the channels there.
    pub fn seek_to_tick(&mut self, tick: u64, sink: &mut impl SequencerSink) {
        let tick = tick.min(self.score.end_tick());
        let events = self.score.events();
        // State events sort first within a tick, so this cursor lies after the
        // destination's state events and before its notes.
        let cursor = events.partition_point(|event| {
            event.tick < tick || (event.tick == tick && event.message.is_state())
        });
        let position_sample = self
            .score
            .tempo_map()
            .sample_at_tick(tick, self.sample_rate);
        self.reconstruct(cursor, position_sample, sink);
    }

    /// Move to `sample` of the song (past the end means the end) and rebuild the
    /// channels there: every event on or after it is sent again.
    pub fn seek_to_sample(&mut self, sample: u64, sink: &mut impl SequencerSink) {
        let sample = sample.min(self.end_sample);
        let cursor = self.score.events().partition_point(|event| {
            self.score
                .tempo_map()
                .sample_at_tick(event.tick, self.sample_rate)
                < sample
        });
        self.reconstruct(cursor, sample, sink);
    }

    /// Advance the clock by `frames`, sending what falls due and rendering every frame
    /// through `sink`. A song that has ended only renders its tails.
    pub fn advance(&mut self, frames: usize, sink: &mut impl SequencerSink) {
        let mut remaining = frames;
        while remaining > 0 {
            if self.position_sample >= self.end_sample && !self.ended {
                self.cross_seam(sink);
                continue;
            }
            if self.ended {
                sink.render(remaining);
                return;
            }
            if self.frames_into_grain == 0 {
                self.send_grain(sink);
            }
            let frames_to_seam =
                usize::try_from(self.end_sample - self.position_sample).unwrap_or(usize::MAX);
            let chunk = remaining
                .min(self.grain_frames - self.frames_into_grain)
                .min(frames_to_seam);
            sink.render(chunk);
            self.position_sample += chunk as u64;
            self.frames_into_grain = (self.frames_into_grain + chunk) % self.grain_frames;
            remaining -= chunk;
        }
    }

    fn cross_seam(&mut self, sink: &mut impl SequencerSink) {
        if self.looping && self.end_sample > 0 {
            self.loop_wraps += 1;
            self.seek_to_tick(0, sink);
        } else {
            sink.release_all();
            self.ended = true;
        }
    }

    /// Send every event due before the end of the grain starting now. An event on the
    /// seam itself is left: the seam releases every note and rebuilds every channel.
    fn send_grain(&mut self, sink: &mut impl SequencerSink) {
        let grain_end_sample =
            (self.position_sample + self.grain_frames as u64).min(self.end_sample);
        while let Some(event) = self.score.events().get(self.cursor).copied() {
            let event_sample = self
                .score
                .tempo_map()
                .sample_at_tick(event.tick, self.sample_rate);
            if event_sample >= grain_end_sample {
                break;
            }
            self.send(event, sink);
            self.cursor += 1;
        }
    }

    fn send(&mut self, event: ScoreEvent, sink: &mut impl SequencerSink) {
        let index = |key: u8| {
            usize::from(event.channel) % CHANNEL_COUNT * KEYS_PER_CHANNEL
                + usize::from(key) % KEYS_PER_CHANNEL
        };
        match event.message {
            ChannelMessage::NoteOn { key, .. } => {
                let count = &mut self.sounding[index(key)];
                *count = count.saturating_add(1);
                sink.send(event.channel, event.message);
            }
            ChannelMessage::NoteOff { key } => {
                if event.note_on_index() < self.released_before {
                    return;
                }
                let count = &mut self.sounding[index(key)];
                if *count == 0 {
                    return;
                }
                *count -= 1;
                if *count == 0 {
                    sink.send(event.channel, event.message);
                }
            }
            state => sink.send(event.channel, state),
        }
    }

    fn reconstruct(&mut self, cursor: usize, position_sample: u64, sink: &mut impl SequencerSink) {
        self.fold.clear();
        self.fold.fold(&self.score.events()[..cursor]);
        self.sounding.fill(0);
        self.cursor = cursor;
        self.released_before = cursor;
        self.position_sample = position_sample;
        self.frames_into_grain = 0;
        self.ended = false;
        self.reconstructions += 1;
        sink.reconstruct(&self.fold);
    }
}
