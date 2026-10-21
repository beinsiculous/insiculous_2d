//! The peak limiter: a gain that falls ahead of a loud peak and recovers after it, so the
//! combined stream never passes [`LIMITER_THRESHOLD`] and an overload sounds quieter
//! rather than distorted.
//!
//! **One block of look-ahead.** The source renders a block before it hands any of it out,
//! so the limiter sees the block's peaks before they play. The gain each frame needs — the
//! threshold over the louder of its two channels — is spread backwards at the attack rate,
//! so the gain is already down when a peak arrives, then forwards at the release rate from
//! the gain the last block ended on. A peak in a block's first frames, which the attack
//! cannot reach from the block before, takes its drop at once: the bound always holds, the
//! smoothness within a block.

/// The level no limited sample passes (to within f32 rounding of `threshold / peak × peak`).
pub const LIMITER_THRESHOLD: f32 = 0.9;

/// How long the gain takes to fall all the way, in seconds: long enough not to click,
/// shorter than a block so a peak is met inside the look-ahead.
pub const LIMITER_ATTACK_SECONDS: f32 = 0.001;

/// How long the gain takes to recover most of the way (one time constant), in seconds.
pub const LIMITER_RELEASE_SECONDS: f32 = 0.08;

/// A reduction this small is none: the release approaches unity without arriving, and the
/// counter would never stop.
const SMALLEST_REDUCTION: f32 = 0.0001;

// Clipping the waveform instead — a stateless soft knee bending each sample under the
// ceiling — kept every sample under full scale, but a key held on a sample instrument drove
// the mix to four times full scale and the knee turned it into distortion.
/// See the module docs. The per-frame gains are allocated when the limiter is built.
pub(crate) struct Limiter {
    /// One minus the gain. Kept rather than the gain because the release multiplies it
    /// towards zero, where f32 keeps its precision; stepped towards one in f32 the gain
    /// stalls short of unity.
    reduction: f32,
    attack_step: f32,
    release_decay: f32,
    gains: Vec<f32>,
}

impl Limiter {
    pub(crate) fn new(sample_rate: u32, block_frames: usize) -> Self {
        let rate = sample_rate.max(1) as f32;
        Self {
            reduction: 0.0,
            attack_step: 1.0 / (LIMITER_ATTACK_SECONDS * rate),
            release_decay: (-1.0 / (LIMITER_RELEASE_SECONDS * rate)).exp(),
            gains: vec![1.0; block_frames],
        }
    }

    /// Limit one block in place, `left` and `right` of equal length and at most the
    /// block. Returns the samples it changed.
    pub(crate) fn apply(&mut self, left: &mut [f32], right: &mut [f32]) -> u64 {
        let frames = left.len().min(right.len()).min(self.gains.len());
        if frames == 0 {
            return 0;
        }
        let gains = &mut self.gains[..frames];
        for (needed, (left_sample, right_sample)) in gains.iter_mut().zip(left.iter().zip(right.iter())) {
            let peak = left_sample.abs().max(right_sample.abs());
            *needed = if peak > LIMITER_THRESHOLD {
                LIMITER_THRESHOLD / peak
            } else {
                1.0
            };
        }
        for frame in (0..frames - 1).rev() {
            gains[frame] = gains[frame].min(gains[frame + 1] + self.attack_step);
        }
        let mut reduction = self.reduction;
        let mut changed = 0;
        for (needed, (left_sample, right_sample)) in gains.iter().zip(left.iter_mut().zip(right.iter_mut())) {
            reduction = (reduction * self.release_decay).max(1.0 - needed);
            if reduction < SMALLEST_REDUCTION {
                reduction = 0.0;
            }
            if reduction > 0.0 {
                // `1 - (1 - needed)` can round above `needed`; the bound is `needed`'s.
                let gain = needed.min(1.0 - reduction);
                *left_sample *= gain;
                *right_sample *= gain;
                changed += 2;
            }
        }
        self.reduction = reduction;
        changed
    }

    /// The gain the last block ended on.
    #[cfg(test)]
    pub(crate) fn gain(&self) -> f32 {
        1.0 - self.reduction
    }
}
