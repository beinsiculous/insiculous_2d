//! The conductor as a rodio source: the song's background and its instruments rendered a
//! block at a time, the bus gains applied per block, and a soft peak limiter on the
//! combined stream before it reaches the device.
//!
//! **One block per render.** Every [`BLOCK_FRAMES`] frames the source renders one block
//! through [`Conductor::render_at`] with the instant the block is rendered at, so each
//! block drains its commands at its own clock; the frames are then handed out
//! interleaved, left then right. A browser pulling 2048 frames at once still drains 32
//! blocks at 32 instants.
//!
//! **The buses.** The music and sfx gains cross from the game thread as f32 bit patterns
//! in two atomics read once a block — the background is mixed at the music gain and the
//! instruments at the sfx gain inside the conductor — and master is the sink's volume, as
//! for every other sound.
//!
//! **The limiter** lowers the gain ahead of a peak and lets it recover after, using the
//! block it has rendered as its look-ahead, so the combined stream after the bus gains
//! never passes [`LIMITER_THRESHOLD`] (see [`limiter`]). It is a safety net for a storm of
//! voices, not a mastering stage, and every sample it turns down is counted.
//!
//! Nothing here allocates once the source is built: the block buffers are sized then.
//! The source never ends by itself; stopping its sink drops it, and the conductor with it.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use common::clock::Instant;
use music_midi::{BusGains, Conductor, BLOCK_FRAMES};

mod limiter;
#[cfg(test)]
mod tests;

use limiter::Limiter;
pub use limiter::{LIMITER_ATTACK_SECONDS, LIMITER_RELEASE_SECONDS, LIMITER_THRESHOLD};

/// The music and sfx bus gains, shared between the game thread and the source.
#[derive(Debug)]
pub(crate) struct BusLevels {
    music_bits: AtomicU32,
    sfx_bits: AtomicU32,
}

impl BusLevels {
    pub(crate) fn new(music: f32, sfx: f32) -> Self {
        Self {
            music_bits: AtomicU32::new(music.to_bits()),
            sfx_bits: AtomicU32::new(sfx.to_bits()),
        }
    }

    pub(crate) fn set(&self, music: f32, sfx: f32) {
        self.music_bits.store(music.to_bits(), Ordering::Release);
        self.sfx_bits.store(sfx.to_bits(), Ordering::Release);
    }

    pub(crate) fn gains(&self) -> BusGains {
        BusGains {
            music: f32::from_bits(self.music_bits.load(Ordering::Acquire)),
            sfx: f32::from_bits(self.sfx_bits.load(Ordering::Acquire)),
        }
    }
}

/// What the source measures, published for the game thread. Each value is whole; one
/// read beside another may straddle a block.
#[derive(Debug, Default)]
pub(crate) struct SourceMeters {
    limited_samples: AtomicU64,
    /// Frames of the current block rendered and not yet handed to the device.
    undelivered_frames: AtomicU64,
    render_nanoseconds_total: AtomicU64,
    render_nanoseconds_longest: AtomicU64,
    drain_gap_nanoseconds_longest: AtomicU64,
}

/// The source's meters as plain values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceReadings {
    /// Output samples the limiter changed.
    pub limited_samples: u64,
    /// Frames this engine has rendered that the device has not yet taken: at most one
    /// block. The device's own buffer lies beyond them and is not counted.
    pub undelivered_frames: u64,
    pub render_nanoseconds_total: u64,
    pub render_nanoseconds_longest: u64,
    /// The longest wall-clock gap between two blocks: how long a command can wait to be
    /// drained (a device period natively, a 2048-frame pull in a browser).
    pub drain_gap_nanoseconds_longest: u64,
}

impl SourceMeters {
    pub(crate) fn readings(&self) -> SourceReadings {
        SourceReadings {
            limited_samples: self.limited_samples.load(Ordering::Acquire),
            undelivered_frames: self.undelivered_frames.load(Ordering::Acquire),
            render_nanoseconds_total: self.render_nanoseconds_total.load(Ordering::Acquire),
            render_nanoseconds_longest: self.render_nanoseconds_longest.load(Ordering::Acquire),
            drain_gap_nanoseconds_longest: self
                .drain_gap_nanoseconds_longest
                .load(Ordering::Acquire),
        }
    }

    /// Swap zero into the two maxima and return what they held: the longest render and
    /// the longest drain gap since the last take. A `fetch_max` that loaded the old maximum
    /// just before the swap compares against it, so a smaller spike in that instant can be
    /// lost; nothing larger is.
    pub(crate) fn take_maxima(&self) -> (u64, u64) {
        (
            self.render_nanoseconds_longest.swap(0, Ordering::AcqRel),
            self.drain_gap_nanoseconds_longest.swap(0, Ordering::AcqRel),
        )
    }
}

/// See the module docs.
pub(crate) struct ConductorSource {
    conductor: Conductor,
    buses: Arc<BusLevels>,
    meters: Arc<SourceMeters>,
    limiter: Limiter,
    left: Vec<f32>,
    right: Vec<f32>,
    /// The next interleaved sample to hand out; `2 × BLOCK_FRAMES` means the block is
    /// spent.
    next_sample: usize,
    previous_block_at: Option<Instant>,
}

impl ConductorSource {
    /// A source over `conductor`, its song already loaded and its transport sent.
    pub(crate) fn new(
        conductor: Conductor,
        buses: Arc<BusLevels>,
        meters: Arc<SourceMeters>,
    ) -> Self {
        let limiter = Limiter::new(conductor.config().sample_rate, BLOCK_FRAMES);
        Self {
            conductor,
            buses,
            meters,
            limiter,
            left: vec![0.0; BLOCK_FRAMES],
            right: vec![0.0; BLOCK_FRAMES],
            next_sample: 2 * BLOCK_FRAMES,
            previous_block_at: None,
        }
    }

    /// Render whole blocks for `frames` of output and discard them: the clock of a
    /// manager with no device, which still drains commands and publishes the position.
    pub(crate) fn advance_frames(&mut self, frames: u64) {
        let mut remaining = frames;
        while remaining > 0 {
            let undelivered = (2 * BLOCK_FRAMES - self.next_sample) / 2;
            if undelivered == 0 {
                self.render_block(Instant::now());
                continue;
            }
            let taken = (undelivered as u64).min(remaining);
            self.next_sample += 2 * taken as usize;
            remaining -= taken;
        }
        self.publish_undelivered();
    }

    fn render_block(&mut self, now: Instant) {
        if let Some(previous) = self.previous_block_at.replace(now) {
            let gap = now.saturating_duration_since(previous).as_nanos() as u64;
            self.meters
                .drain_gap_nanoseconds_longest
                .fetch_max(gap, Ordering::Relaxed);
        }
        let gains = self.buses.gains();
        self.conductor
            .render_at(now, &mut self.left, &mut self.right, gains);
        let limited = self.limiter.apply(&mut self.left, &mut self.right);
        if limited > 0 {
            self.meters
                .limited_samples
                .fetch_add(limited, Ordering::Relaxed);
        }
        let nanoseconds = now.elapsed().as_nanos() as u64;
        self.meters
            .render_nanoseconds_total
            .fetch_add(nanoseconds, Ordering::Relaxed);
        self.meters
            .render_nanoseconds_longest
            .fetch_max(nanoseconds, Ordering::Relaxed);
        self.next_sample = 0;
    }

    fn publish_undelivered(&self) {
        let undelivered = (2 * BLOCK_FRAMES - self.next_sample) / 2;
        self.meters
            .undelivered_frames
            .store(undelivered as u64, Ordering::Relaxed);
    }
}

impl Iterator for ConductorSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.next_sample == 2 * BLOCK_FRAMES {
            self.render_block(Instant::now());
        }
        let frame = self.next_sample / 2;
        let sample = if self.next_sample.is_multiple_of(2) {
            self.left[frame]
        } else {
            self.right[frame]
        };
        self.next_sample += 1;
        if self.next_sample.is_multiple_of(2) {
            self.publish_undelivered();
        }
        Some(sample)
    }
}

impl rodio::Source for ConductorSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        self.conductor.config().sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
