//! The arithmetic of a song played with no device: wall time turned into whole frames at
//! the song sample rate, the remainder carried to the next update.

use std::time::Duration;

use super::SONG_SAMPLE_RATE;

/// The longest wall time one `update()` advances a song with no device: a longer gap —
/// a debugger pause, a stalled test — advances this much, as a device resuming from where
/// it rendered would not catch up either.
pub(super) const LONGEST_VIRTUAL_STEP: Duration = Duration::from_secs(1);

const NANOSECONDS_PER_SECOND: u128 = 1_000_000_000;

/// Whole frames at the song sample rate in `duration`.
pub(super) fn frames_in(duration: Duration) -> u64 {
    (duration.as_nanos() * u128::from(SONG_SAMPLE_RATE) / NANOSECONDS_PER_SECOND) as u64
}

/// How long `frames` frames last at the song sample rate, rounded down.
pub(super) fn duration_of(frames: u64) -> Duration {
    let nanoseconds = u128::from(frames) * NANOSECONDS_PER_SECOND / u128::from(SONG_SAMPLE_RATE);
    Duration::from_nanos(u64::try_from(nanoseconds).unwrap_or(u64::MAX))
}
