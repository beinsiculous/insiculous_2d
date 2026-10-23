//! What a game reads about the song for its own logic and its telemetry: the label of the
//! song in the slot, the source's maxima taken window by window, and the one line every
//! `conductor=on` report prints, so the example and the games never format it twice.

use std::path::Path;

use super::{AudioManager, PendingMusic, SongSnapshot};

/// The source's two maxima since the last take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SongMaxima {
    pub render_nanoseconds_longest: u64,
    pub drain_gap_nanoseconds_longest: u64,
}

impl AudioManager {
    /// The label the song in the slot was requested with — the path's display for
    /// [`AudioManager::play_song`] — while it is pending the first gesture, playing or
    /// paused. `None` without a song, and for one that played to its end or whose output
    /// died: nothing reaps those, and to a caller deciding whether to request music they
    /// are no song.
    #[must_use]
    pub fn song_label(&self) -> Option<&str> {
        if let Some(PendingMusic::Song(ready)) = &self.pending_music {
            return Some(&ready.label);
        }
        self.songs
            .playback
            .as_ref()
            .filter(|playback| !playback.is_dead() && !playback.handle.position().has_ended())
            .map(|playback| playback.label.as_str())
    }

    /// The path the song in the slot was asked for by — pending, playing or paused, the same
    /// filter as [`AudioManager::song_label`] — so a host that stopped it can ask for it again
    /// as the game did. `None` for a song built in memory, which only its builder can replay.
    #[must_use]
    pub fn song_request(&self) -> Option<&Path> {
        if let Some(PendingMusic::Song(ready)) = &self.pending_music {
            return ready.path.as_deref();
        }
        self.songs
            .playback
            .as_ref()
            .filter(|playback| !playback.is_dead() && !playback.handle.position().has_ended())
            .and_then(|playback| playback.path.as_deref())
    }

    /// The longest render and the longest drain gap since the last take, both swapped to
    /// zero, so each report prints its own window and a hitch at boot does not sit in
    /// every later line. Zeros without a song.
    pub fn take_song_maxima(&mut self) -> SongMaxima {
        let Some(playback) = self.songs.playback.as_ref() else {
            return SongMaxima::default();
        };
        let (render, drain_gap) = playback.meters.take_maxima();
        SongMaxima {
            render_nanoseconds_longest: render,
            drain_gap_nanoseconds_longest: drain_gap,
        }
    }
}

impl SongSnapshot {
    /// The snapshot as one `conductor=on key=value …` line: the render cost, the drain gap,
    /// the transport and every counter. Position and frame times are the caller's to
    /// append; a caller printing windowed maxima puts [`SongMaxima`] into `source` first.
    #[must_use]
    pub fn log_line(&self) -> String {
        let (conductor, source) = (&self.conductor, &self.source);
        let blocks = conductor.blocks_rendered.max(1);
        format!(
            "conductor=on blocks={} block_mean={:.1}us block_longest={:.1}us drain_gap_longest={:.2}ms \
             transport={:?} wraps={} reconstructions={} non_finite={} applied={} rejected={} \
             triggers_accepted={} triggers_rejected={} triggers_late={} triggers_stale={} \
             triggers_silent={} live_sources={} pending={} sample_voices={} limited_samples={}",
            conductor.blocks_rendered,
            source.render_nanoseconds_total as f64 / 1000.0 / blocks as f64,
            source.render_nanoseconds_longest as f64 / 1000.0,
            source.drain_gap_nanoseconds_longest as f64 / 1_000_000.0,
            conductor.transport,
            conductor.loop_wraps,
            conductor.reconstructions,
            conductor.non_finite_samples,
            conductor.commands_applied,
            conductor.commands_rejected,
            conductor.accepted_triggers,
            conductor.rejected_triggers,
            conductor.late_triggers,
            conductor.stale_triggers,
            conductor.silent_triggers,
            conductor.live_sources,
            conductor.pending_events,
            conductor.sample_voices,
            source.limited_samples,
        )
    }
}
