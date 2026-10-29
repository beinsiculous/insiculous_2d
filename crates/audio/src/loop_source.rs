//! A looping recorded sound that can be handed new bytes while it plays.
//!
//! rodio's `Repeat` keeps two `Buffered` clones of one decode — the pass playing and the
//! pass to restart from — and at the end of a pass makes the second the first again, so
//! it can never be given another source. [`LoopSource`] keeps the same two fields and a
//! [`LoopSlot`] beside them: when the pass playing runs out it tries the slot, and a
//! replacement found there becomes the pass to restart from, so this pass and every later
//! one is the new sound. The pass that was playing is never cut.
//!
//! **The audio thread never waits.** The slot is `try_lock`ed, only at a pass's end: while
//! the frame thread holds it, the old sound plays once more and the slot is tried again
//! at the next boundary. The replacement is decoded and buffered on the frame thread
//! before it goes in.
//!
//! The swap assumes the replacement has the old sound's channel count and sample rate:
//! rodio rebuilds its resampler only at a span's end, from what the source reports there,
//! and this source reports the pass to restart from until the slot is taken — so a new
//! format would play one span converted as the old. The manager restarts the sink for a
//! format change instead of filling the slot.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::source::Buffered;
use rodio::{Sample, Source};

/// The replacement a looping sound takes at its next boundary, shared between the frame
/// thread that fills it and the audio thread that empties it.
pub(crate) type LoopSlot<I> = Arc<Mutex<Option<Buffered<I>>>>;

/// See the module docs.
pub(crate) struct LoopSource<I>
where
    I: Source,
    I::Item: Sample,
{
    inner: Buffered<I>,
    next: Buffered<I>,
    slot: LoopSlot<I>,
}

impl<I> LoopSource<I>
where
    I: Source,
    I::Item: Sample,
{
    /// Repeat `input` until a replacement arrives in `slot`.
    pub(crate) fn new(input: I, slot: LoopSlot<I>) -> Self {
        let next = input.buffered();
        Self { inner: next.clone(), next, slot }
    }

    /// The pass to restart from: the slot's replacement when one waits and the slot is
    /// free, else the one already kept.
    fn take_replacement(&mut self) {
        let Ok(mut slot) = self.slot.try_lock() else {
            return;
        };
        if let Some(replacement) = slot.take() {
            self.next = replacement;
        }
    }
}

impl<I> Iterator for LoopSource<I>
where
    I: Source,
    I::Item: Sample,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        if let Some(sample) = self.inner.next() {
            return Some(sample);
        }
        self.take_replacement();
        self.inner = self.next.clone();
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

impl<I> Source for LoopSource<I>
where
    I: Source,
    I::Item: Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        match self.inner.current_frame_len() {
            Some(0) => self.next.current_frame_len(),
            length => length,
        }
    }

    fn channels(&self) -> u16 {
        match self.inner.current_frame_len() {
            Some(0) => self.next.channels(),
            _ => self.inner.channels(),
        }
    }

    fn sample_rate(&self) -> u32 {
        match self.inner.current_frame_len() {
            Some(0) => self.next.sample_rate(),
            _ => self.inner.sample_rate(),
        }
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
