//! The criteria's assertions, run against the committed fixtures only (the sine
//! SoundFont, the sine WAVs, the Mutopia files), so they need no downloaded bank.

mod criterion_1_seam;
mod criterion_2_isolation;
mod criterion_3_samples;
mod score_playback;

use std::path::PathBuf;
use std::sync::Arc;

use rustysynth::SoundFont;

use crate::bank::load_sound_font;
use crate::instrument_slot::{InstrumentSlot, SlotCapacity};

pub(crate) const SAMPLE_RATE: u32 = 44_100;

pub(crate) fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../crates/music_midi/fixtures").join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

pub(crate) fn sine_bank() -> Arc<SoundFont> {
    load_sound_font(&fixture("sine.sf2")).expect("the sine fixture is a valid SoundFont")
}

pub(crate) fn sine_slot(bank: &Arc<SoundFont>, sounding_cap: usize, maximum_polyphony: usize) -> InstrumentSlot {
    InstrumentSlot::new(bank, SAMPLE_RATE as i32, SlotCapacity { sounding_cap, maximum_polyphony }, 0, 0)
        .expect("a slot over the sine bank")
}

/// The frequency of `key` on the sine bank (an A4 sample at root 69).
pub(crate) fn hertz_of(key: i32) -> f64 {
    440.0 * 2f64.powf(f64::from(key - 69) / 12.0)
}

/// Render `frames` frames of a slot, left channel only.
pub(crate) fn render_slot(slot: &mut InstrumentSlot, frames: usize) -> Vec<f32> {
    let mut left = vec![0.0; frames];
    let mut right = vec![0.0; frames];
    slot.render(&mut left, &mut right);
    left
}

pub(crate) fn root_mean_square(samples: &[f32]) -> f32 {
    (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len().max(1) as f32).sqrt()
}

/// The amplitude of one frequency in `samples` (Goertzel over a Hann window), so one
/// note's level can be read out of a mix of others.
pub(crate) fn amplitude_at(samples: &[f32], hertz: f64) -> f64 {
    let length = samples.len() as f64;
    let coefficient = 2.0 * (2.0 * std::f64::consts::PI * hertz / f64::from(SAMPLE_RATE)).cos();
    let (mut previous, mut before_previous) = (0.0_f64, 0.0_f64);
    for (index, sample) in samples.iter().enumerate() {
        let window = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * index as f64 / length).cos();
        let current = f64::from(*sample) * window + coefficient * previous - before_previous;
        before_previous = previous;
        previous = current;
    }
    let power = previous * previous + before_previous * before_previous - coefficient * previous * before_previous;
    power.max(0.0).sqrt() * 4.0 / length
}
