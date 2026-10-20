//! The rodio `Source` that carries the [`Mixer`] onto the audio thread.
//!
//! The game thread keeps a bounded `SyncSender` (sending with `try_send`, so it never
//! blocks) and an `Arc<SpikeStats>`; the source owns the receiver and the mixer. Commands
//! are drained once per 64-frame block, before the block renders, so a command's effect
//! lands on a block boundary. No lock is shared between the threads.

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::sync::Arc;
use std::time::Duration;

use rodio::Source;

use common::clock::Instant;

use crate::mixer::{Mixer, SpikeCommand, BLOCK_FRAMES};

/// Commands the channel holds before `try_send` reports it full.
pub const COMMAND_CAPACITY: usize = 256;

/// What the audio thread reports back, readable from any thread at any time.
#[derive(Debug, Default)]
pub struct SpikeStats {
    pub frames_rendered: AtomicU64,
    pub blocks_rendered: AtomicU64,
    pub render_nanoseconds_total: AtomicU64,
    pub render_nanoseconds_longest: AtomicU64,
    pub commands_applied: AtomicU64,
    pub non_finite_samples: AtomicU64,
    pub peak_sounding_notes: AtomicUsize,
    pub peak_sample_voices: AtomicUsize,
    /// `f32` bits: the largest sample-to-sample step of the mixed output.
    pub largest_step_bits: AtomicU32,
    /// The longest wall-clock wait between two block renders: how long a command can sit
    /// in the channel before it is drained. On the web the backend pulls whole 2048-frame
    /// buffers, so this is a buffer period, not a block.
    pub longest_drain_gap_nanoseconds: AtomicU64,
    /// Seeks and loop wraps of the background score, and the longest state rebuild.
    pub reconstructions: AtomicU64,
    pub longest_reconstruction_nanoseconds: AtomicU64,
}

impl SpikeStats {
    pub fn largest_step(&self) -> f32 {
        f32::from_bits(self.largest_step_bits.load(Ordering::Relaxed))
    }

    pub fn mean_render_microseconds(&self) -> f64 {
        let blocks = self.blocks_rendered.load(Ordering::Relaxed).max(1);
        self.render_nanoseconds_total.load(Ordering::Relaxed) as f64 / blocks as f64 / 1000.0
    }

    pub fn longest_render_microseconds(&self) -> f64 {
        self.render_nanoseconds_longest.load(Ordering::Relaxed) as f64 / 1000.0
    }

    pub fn longest_drain_gap_milliseconds(&self) -> f64 {
        self.longest_drain_gap_nanoseconds.load(Ordering::Relaxed) as f64 / 1_000_000.0
    }
}

/// The game thread's end of the channel.
#[derive(Clone)]
pub struct SpikeSender {
    sender: SyncSender<SpikeCommand>,
    pub stats: Arc<SpikeStats>,
    rejected: Arc<AtomicU64>,
}

impl SpikeSender {
    /// Queue a command without blocking. A full channel drops the command and counts it.
    pub fn send(&self, command: SpikeCommand) -> bool {
        match self.sender.try_send(command) {
            Ok(()) => true,
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                self.rejected.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    pub fn rejected_commands(&self) -> u64 {
        self.rejected.load(Ordering::Relaxed)
    }
}

pub struct SpikeSource {
    mixer: Mixer,
    receiver: Receiver<SpikeCommand>,
    stats: Arc<SpikeStats>,
    sample_rate: u32,
    left: [f32; BLOCK_FRAMES],
    right: [f32; BLOCK_FRAMES],
    /// The next interleaved sample to hand out; `2 * BLOCK_FRAMES` means the block is spent.
    next_sample: usize,
    previous_block_at: Option<Instant>,
}

/// A source and the sender that drives it.
pub fn spike_channel(mixer: Mixer, sample_rate: u32) -> (SpikeSource, SpikeSender) {
    let (sender, receiver) = std::sync::mpsc::sync_channel(COMMAND_CAPACITY);
    let stats = Arc::new(SpikeStats::default());
    let source = SpikeSource {
        mixer,
        receiver,
        stats: Arc::clone(&stats),
        sample_rate,
        left: [0.0; BLOCK_FRAMES],
        right: [0.0; BLOCK_FRAMES],
        next_sample: 2 * BLOCK_FRAMES,
        previous_block_at: None,
    };
    (source, SpikeSender { sender, stats, rejected: Arc::new(AtomicU64::new(0)) })
}

impl SpikeSource {
    fn render_next_block(&mut self) {
        let now = Instant::now();
        if let Some(previous) = self.previous_block_at.replace(now) {
            let gap = now.duration_since(previous).as_nanos() as u64;
            self.stats.longest_drain_gap_nanoseconds.fetch_max(gap, Ordering::Relaxed);
        }
        while let Ok(command) = self.receiver.try_recv() {
            self.mixer.apply(command);
            self.stats.commands_applied.fetch_add(1, Ordering::Relaxed);
        }
        let started = Instant::now();
        let report = self.mixer.render_block(&mut self.left, &mut self.right);
        let nanoseconds = started.elapsed().as_nanos() as u64;
        let stats = &self.stats;
        stats.frames_rendered.fetch_add(BLOCK_FRAMES as u64, Ordering::Relaxed);
        stats.blocks_rendered.fetch_add(1, Ordering::Relaxed);
        stats.render_nanoseconds_total.fetch_add(nanoseconds, Ordering::Relaxed);
        stats.render_nanoseconds_longest.fetch_max(nanoseconds, Ordering::Relaxed);
        stats.non_finite_samples.fetch_add(u64::from(report.non_finite_samples), Ordering::Relaxed);
        stats.peak_sounding_notes.fetch_max(report.sounding_notes, Ordering::Relaxed);
        stats.peak_sample_voices.fetch_max(report.sample_voices, Ordering::Relaxed);
        if report.largest_step > stats.largest_step() {
            stats.largest_step_bits.store(report.largest_step.to_bits(), Ordering::Relaxed);
        }
        if let Some(background) = self.mixer.background.as_ref() {
            let counters = background.counters();
            stats.reconstructions.store(counters.reconstructions, Ordering::Relaxed);
            stats.longest_reconstruction_nanoseconds
                .store((counters.longest_reconstruction_microseconds * 1000.0) as u64, Ordering::Relaxed);
        }
        self.next_sample = 0;
    }
}

impl Iterator for SpikeSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.next_sample == 2 * BLOCK_FRAMES {
            if self.mixer.is_stopped() {
                return None;
            }
            self.render_next_block();
            if self.mixer.is_stopped() {
                return None;
            }
        }
        let frame = self.next_sample / 2;
        let sample = if self.next_sample.is_multiple_of(2) { self.left[frame] } else { self.right[frame] };
        self.next_sample += 1;
        Some(sample)
    }
}

impl Source for SpikeSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
