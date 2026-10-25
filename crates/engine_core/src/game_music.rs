//! A game's song, the rules every game that plays one carries: requested at `init` with its
//! failure logged once, re-aimed at every match start by the identity of the song in the
//! slot, jammed from the pause menu, pulsed on the beat, and reported every five seconds.
//!
//! What stays in each game is what the game names: the song's path and its bindings, the
//! moments that trigger them (`ctx.sources.trigger`, counted by the game), the counts it
//! appends to the telemetry line, and where its beat visual lands. The one trigger this
//! module sends itself is the jam, on the menu's source. There is no recorded fallback: a
//! song that will not play leaves the game silent, every trigger refused and counted.

use std::path::Path;

use audio::Subdivision;

use crate::contexts::GameContext;
use crate::menu_input::MenuInput;

/// How many frames the frame-time ring holds: five seconds at 60 fps.
const FRAME_RING_LENGTH: usize = 300;
/// Seconds between two telemetry lines.
const TELEMETRY_SECONDS: f32 = 5.0;

/// Which beat a frame crossed, for a game's beat visual.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeatPulse {
    /// A bar line: the full pulse.
    Bar,
    /// Another beat of the bar.
    Beat,
}

/// See the module docs.
#[derive(Debug, Clone)]
pub struct GameMusic {
    song_path: &'static str,
    log_prefix: &'static str,
    telemetry_env: &'static str,
    /// Telemetry at `info` when asked for (`?telemetry=1` on the web, the environment
    /// variable natively), else `debug`.
    telemetry_at_info: bool,
    song_failed: bool,
    /// Requests that failed: the first is logged at `error`, the rest at `debug`.
    failures: u64,
    /// The refusals already reported by the once-a-second debug line.
    refused_seen: u64,
    frame_times: FrameRing,
    /// Whether the pause menu was up last frame: the frame it opens jams nothing.
    was_paused: bool,
    jam_units: u64,
    pulsed: u64,
    pulsed_this_update: bool,
    seconds_since_telemetry: f32,
    seconds_since_refusals: f32,
}

impl GameMusic {
    /// `song_path` relative to the asset base (`"ai/music/ai_tong.mid"`), `log_prefix` the
    /// game's name in its lines (`"tong"`), `telemetry_env` the native switch (`"TONG_TELEMETRY"`).
    pub fn new(song_path: &'static str, log_prefix: &'static str, telemetry_env: &'static str) -> Self {
        Self {
            song_path,
            log_prefix,
            telemetry_env,
            telemetry_at_info: false,
            song_failed: false,
            failures: 0,
            refused_seen: 0,
            frame_times: FrameRing::default(),
            was_paused: false,
            jam_units: 0,
            pulsed: 0,
            pulsed_this_update: false,
            seconds_since_telemetry: 0.0,
            seconds_since_refusals: 0.0,
        }
    }

    /// The joined path `start` requests and `restart` compares the slot against.
    pub fn song_path(&self, ctx: &GameContext) -> String {
        format!("{}/{}", ctx.assets.base_path(), self.song_path)
    }

    /// Request the song — on the web it waits for the first click or key and starts then —
    /// and drop whatever the boot left in the source's maxima.
    pub fn start(&mut self, ctx: &mut GameContext) {
        self.telemetry_at_info = telemetry_asked(self.telemetry_env);
        self.request(ctx);
        ctx.audio.take_song_maxima();
    }

    fn request(&mut self, ctx: &mut GameContext) {
        match ctx.audio.play_song(self.song_path(ctx)) {
            Ok(()) => self.song_failed = false,
            Err(error) => {
                self.song_failed = true;
                self.failures += 1;
                if self.failures == 1 {
                    log::error!("{}: the song could not play: {error}", self.log_prefix);
                } else {
                    log::debug!("{}: the song could not play again ({} times): {error}", self.log_prefix, self.failures);
                }
            }
        }
    }

    /// A match starts, decided by the song in the slot: none — a preview that ended took it,
    /// or the output died — requests the game's song again; the game's own, compared as a
    /// whole path, goes back to its top and reports on the next update; another song is left
    /// alone, since in the editor a song the composer saved is the session's music and a
    /// match is how it is auditioned. Also clears the pause latch, so a Restart taken from
    /// the pause menu does not read the next pause's opening frame as an open menu.
    pub fn restart(&mut self, ctx: &mut GameContext) {
        self.was_paused = false;
        let own = self.song_path(ctx);
        let in_slot = ctx.audio.song_label().map(str::to_string);
        match in_slot {
            None => self.request(ctx),
            Some(label) if Path::new(&label) == Path::new(&own) => {
                ctx.audio.seek_song(0);
                self.seconds_since_telemetry = TELEMETRY_SECONDS;
            }
            Some(_) => {}
        }
    }

    /// The pause menu's frame: a move up or down — read from the input the menu itself
    /// navigates by, never from its selection, which follows a resting pointer — triggers
    /// `action` on `ctx.sources.menu()`. The frame the menu opens jams nothing, nor do
    /// confirm, cancel and the pointer. Called every gameplay frame with the menu's live
    /// state — a frame it is skipped on reads as the menu never having closed. Returns
    /// whether a move asked for a unit; whether the host accepted the trigger is
    /// `ctx.sources.refused_triggers()`'s to say, as for every other count on the
    /// telemetry line.
    pub fn follow_pause(&mut self, ctx: &mut GameContext, paused: bool, action: &str) -> bool {
        let was_paused = std::mem::replace(&mut self.was_paused, paused);
        if !(paused && was_paused) {
            return false;
        }
        let input = MenuInput::read(ctx.input);
        if !(input.up || input.down) {
            return false;
        }
        self.jam_units += 1;
        let source = ctx.sources.menu();
        ctx.sources.trigger(ctx.audio, action, source);
        true
    }

    /// The beat this update crossed, bar first; at most one pulse an update, however often
    /// it is asked. The game applies its own visual, and asks only while its world runs —
    /// a frozen grid discards what it is given.
    pub fn beat_pulse(&mut self, ctx: &mut GameContext) -> Option<BeatPulse> {
        if self.pulsed_this_update {
            return None;
        }
        let pulse = if ctx.audio.beat_crossed(Subdivision::Bar) {
            BeatPulse::Bar
        } else if ctx.audio.beat_crossed(Subdivision::Beat) {
            BeatPulse::Beat
        } else {
            return None;
        };
        self.pulsed_this_update = true;
        self.pulsed += 1;
        Some(pulse)
    }

    /// Every frame, before the game's own logic: the frame time into the ring, the telemetry
    /// line every five seconds with the game's own counts appended (`counts` is called only
    /// when a line prints), the refusals once a second at debug.
    pub fn update(&mut self, ctx: &mut GameContext, counts: impl FnOnce() -> String) {
        self.pulsed_this_update = false;
        self.frame_times.push(ctx.delta_time);
        self.seconds_since_telemetry += ctx.delta_time;
        self.seconds_since_refusals += ctx.delta_time;
        if self.seconds_since_telemetry >= TELEMETRY_SECONDS {
            self.print_telemetry(ctx, counts);
        }
        if self.seconds_since_refusals >= 1.0 {
            self.seconds_since_refusals = 0.0;
            let refused = ctx.sources.refused_triggers();
            if refused != self.refused_seen {
                log::debug!("{}: {} triggers refused in all", self.log_prefix, refused);
                self.refused_seen = refused;
            }
        }
    }

    /// One `conductor=on` line, its window's maxima taken, with the game's own counts.
    fn print_telemetry(&mut self, ctx: &mut GameContext, counts: impl FnOnce() -> String) {
        self.seconds_since_telemetry = 0.0;
        let Some(mut snapshot) = ctx.audio.song_snapshot() else {
            log::debug!("{}: no song in the slot", self.log_prefix);
            return;
        };
        let maxima = ctx.audio.take_song_maxima();
        snapshot.source.render_nanoseconds_longest = maxima.render_nanoseconds_longest;
        snapshot.source.drain_gap_nanoseconds_longest = maxima.drain_gap_nanoseconds_longest;
        let line = format!(
            "{} frame_p95={:.2}ms frame_mean={:.2}ms {} refused={}",
            snapshot.log_line(),
            self.frame_times.p95_milliseconds(),
            self.frame_times.mean_milliseconds(),
            counts(),
            ctx.sources.refused_triggers(),
        );
        if self.telemetry_at_info {
            log::info!("{line}");
        } else {
            log::debug!("{line}");
        }
    }

    /// The last request of the song failed; every trigger since is a counted refusal.
    pub fn song_failed(&self) -> bool {
        self.song_failed
    }

    /// Requests that failed, `start`'s and every `restart`'s.
    pub fn failures(&self) -> u64 {
        self.failures
    }

    /// Units the pause menu jammed.
    pub fn jam_units(&self) -> u64 {
        self.jam_units
    }

    /// Beat pulses handed to the game.
    pub fn pulsed(&self) -> u64 {
        self.pulsed
    }
}

#[cfg(target_arch = "wasm32")]
fn telemetry_asked(_environment_variable: &str) -> bool {
    crate::web::query_param("telemetry").as_deref() == Some("1")
}

#[cfg(not(target_arch = "wasm32"))]
fn telemetry_asked(environment_variable: &str) -> bool {
    std::env::var(environment_variable).as_deref() == Ok("1")
}

/// The last frames' `delta_time`, for the p95 and the mean a telemetry line prints.
#[derive(Debug, Clone)]
struct FrameRing {
    seconds: [f32; FRAME_RING_LENGTH],
    next: usize,
    filled: usize,
}

impl Default for FrameRing {
    fn default() -> Self { Self { seconds: [0.0; FRAME_RING_LENGTH], next: 0, filled: 0 } }
}

impl FrameRing {
    fn push(&mut self, delta_seconds: f32) {
        self.seconds[self.next] = delta_seconds;
        self.next = (self.next + 1) % FRAME_RING_LENGTH;
        self.filled = (self.filled + 1).min(FRAME_RING_LENGTH);
    }

    /// The 95th percentile frame time in milliseconds; zero with no frame yet.
    fn p95_milliseconds(&self) -> f32 {
        let mut sorted = self.seconds[..self.filled].to_vec();
        if sorted.is_empty() {
            return 0.0;
        }
        sorted.sort_by(f32::total_cmp);
        let index = ((sorted.len() as f32 * 0.95).ceil() as usize).clamp(1, sorted.len()) - 1;
        sorted[index] * 1000.0
    }

    fn mean_milliseconds(&self) -> f32 {
        if self.filled == 0 {
            return 0.0;
        }
        self.seconds[..self.filled].iter().sum::<f32>() / self.filled as f32 * 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_reports_its_ninety_fifth_percentile_and_mean_over_its_last_frames_only() {
        let mut ring = FrameRing::default();
        assert_eq!(ring.p95_milliseconds(), 0.0);
        for _ in 0..FRAME_RING_LENGTH {
            ring.push(1.0);
        }
        for frame in 0..FRAME_RING_LENGTH {
            ring.push(if frame >= FRAME_RING_LENGTH - 15 { 0.050 } else { 0.010 });
        }
        assert!((ring.p95_milliseconds() - 10.0).abs() < 1e-3, "{}", ring.p95_milliseconds());
        ring.push(0.050);
        assert!((ring.p95_milliseconds() - 50.0).abs() < 1e-3, "sixteen slow frames of 300 reach the p95");
        assert!(ring.mean_milliseconds() < 20.0, "the second-long frames have left the ring");
    }
}
