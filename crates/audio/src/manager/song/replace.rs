//! Replacing the playing song at a bar line: a new song prepared off the frame, parked
//! beside the playing one, and swapped in by `update` the first time it is due, at the
//! same bar and the same fraction of it.
//!
//! **The worker.** [`AudioManager::replace_song_at_seam`] reads the two files on the
//! frame, so a missing one is an immediate error, and hands them with a copy of the
//! decoded-file cache to a thread that prepares the song exactly as `play_song` does —
//! only a file that changed is decoded again. The result comes back over a channel with
//! the copy, whose decodes the frame's cache takes in — a failed preparation's too, with the
//! paths it attempted — so `play_song` never waits on a
//! preparation. On wasm32 there is no thread: the preparation runs at the call and its
//! result takes the same path.
//!
//! **Request order wins.** Every request takes the next id and drops the preparation
//! already parked; a result older than the latest request, or for a playback since
//! replaced or stopped, is dropped unread, so a slow bank preparation never overwrites a
//! newer one, whichever finishes first.
//!
//! **When it is due.** A running transport takes it on the first update after it
//! arrived whose edges cross a bar, forward or by the loop's wrap — a seek the host sent
//! never counts: nothing swaps while it is in flight, and where it lands a staged refusal
//! is taken (a running song's un-staged preparation still waits for its next bar line). A
//! paused song takes it at once,
//! at its held position; a song stopped at its end takes it at once, stopped at the new
//! end; a running song that reaches its end with one pending takes it there, playing on
//! when the place maps inside the new song.
//!
//! **Where it lands.** The old position's bar, and the fraction of that bar played, go to
//! the tick at the same fraction of the same bar in the new song, in this order: the
//! fraction clamps inside the new bar (a partial last bar included); a looping transport
//! takes the bar modulo the new song's bar count; a running transport that does not loop,
//! whose bar the new song does not have, refuses the swap — the old song plays on and the
//! preparation stays staged for the next stationary transport or seek.
//!
//! **What it costs.** The position runs ahead of what is heard by the device's buffer, so
//! at the frame after the bar line the old source has already rendered the downbeat: the
//! swap cuts it, and the new source starts past the notes that began on the bar line —
//! the new score's downbeat is skipped once per replacement, and the triggers of the swap
//! frame, sent to the old source, go with it. The host re-sends its own transport and its
//! waiting lifecycle commands to the new source; nothing crosses the audio thread but the
//! source replacement `play_song` already makes.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use common::clock::Instant;
use music_midi::{ConductorCommand, PositionSnapshot, Song, TransportState};

use crate::conductor_source::{ConductorSource, SourceMeters};
use crate::error::{AudioError, AudioResult};

use super::load::{read_song_files, Closure, DependencyCache, ReadySong};
use super::{
    binding_table, AudioManager, Landing, SongDeck, SongOutput, SongPosition, Subdivision,
    SONG_SAMPLE_RATE,
};

mod landing;

use landing::{position_at, same_place, start_commands, Bars, Start};

/// What became of a replacement, in the order it happened; drained by
/// [`AudioManager::take_song_replacements`].
#[derive(Debug)]
pub enum SongReplacement {
    /// The new song took over in bar `bar` of the old one and plays on from `position`.
    Swapped { bar: u64, position: SongPosition },
    /// The running song is in a bar the new song does not have and does not loop: the old
    /// song plays on, and the preparation waits for a pause, the song's end or a seek.
    Refused { reason: String },
    /// The new song could not be prepared — the error names the song, the sidecar entry
    /// and the path, as `play_song`'s does — and the old one plays on.
    Failed(AudioError),
    /// A preparation superseded by a newer request, or dropped with its playback by
    /// `play_song`, `stop_song` or a dead output.
    Dropped,
}

/// A song prepared for the playing one's bar line, parked until it is due.
pub(super) struct PreparedPlayback {
    request_id: u64,
    ready: ReadySong,
    /// Refused while running: it waits for a stationary transport or a seek.
    staged: bool,
}

/// A worker's answer.
pub(super) struct Preparation {
    request_id: u64,
    generation: u64,
    result: AudioResult<ReadySong>,
    /// The worker's copy of the cache, holding what it decoded; none for a song built in
    /// memory.
    cache: Option<DependencyCache>,
}

/// What an update read of a live song: when, the snapshot, and whether it was the host's
/// seek landing.
pub(super) struct Reading {
    pub(super) now: Instant,
    pub(super) snapshot: PositionSnapshot,
    pub(super) seek_landed: bool,
}

/// The deck's side of replacement, outliving every song.
pub(super) struct Replacements {
    latest_request: u64,
    latest_generation: u64,
    sender: Sender<Preparation>,
    receiver: Receiver<Preparation>,
    in_flight: usize,
    /// Taken off the channel, in order, waiting for this update to take them in.
    arrived: VecDeque<Preparation>,
    outcomes: Vec<SongReplacement>,
}

impl Replacements {
    pub(super) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            latest_request: 0,
            latest_generation: 0,
            sender,
            receiver,
            in_flight: 0,
            arrived: VecDeque::new(),
            outcomes: Vec::new(),
        }
    }

    /// The generation of a playback `start_song` builds: a result prepared for another
    /// is dropped.
    pub(super) fn next_generation(&mut self) -> u64 {
        self.latest_generation += 1;
        self.latest_generation
    }
}

impl SongDeck {
    /// Drop the parked preparation, if any, off the frame, and report it.
    pub(super) fn drop_pending_replacement(&mut self) {
        let Some(prepared) = self
            .playback
            .as_mut()
            .and_then(|playback| playback.pending_replacement.take())
        else {
            return;
        };
        self.replacements.outcomes.push(SongReplacement::Dropped);
        retire(prepared);
    }
}

/// Drop `value` on a thread of its own, so a song's decoded banks and its conductor are
/// freed off the frame; on wasm32, which has no thread, here.
fn retire<T: Send + 'static>(value: T) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let spawned = std::thread::Builder::new()
            .name("song retirement".to_string())
            .spawn(move || drop(value));
        if let Err(error) = spawned {
            log::warn!("A retired song was dropped on the frame: {error}");
        }
    }
    #[cfg(target_arch = "wasm32")]
    drop(value);
}

/// What a worker runs: the song prepared, and the cache copy it prepared with.
type PreparationJob = Box<dyn FnOnce() -> (AudioResult<ReadySong>, Option<DependencyCache>) + Send>;

/// The moment a parked preparation is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Moment {
    /// A running transport crossed a bar line.
    Running,
    /// Paused: at its held position.
    Paused,
    /// Stopped at its end when the preparation arrived.
    StoppedAtEnd,
    /// Ran to its end with the preparation parked.
    ReachedEnd,
}

impl AudioManager {
    /// Replace the playing song with the one at `path` (its sidecar `<path>.ron`) at its
    /// next bar line, keeping its place, its transport and its label: see the module docs.
    /// The two files are read now — a missing or malformed one is an error and nothing
    /// changes; the closure is prepared off the frame and the outcome drained with
    /// [`AudioManager::take_song_replacements`]. `SongNotPlaying` without a live song.
    pub fn replace_song_at_seam(&mut self, path: impl AsRef<Path>) -> AudioResult<()> {
        let path = path.as_ref();
        let generation = self.live_song_generation()?;
        let (file, data) = read_song_files(path)?;
        let label = path.display().to_string();
        let asset_base = self.songs.asset_base.clone();
        let mut cache = self.songs.cache.for_worker();
        self.dispatch_preparation(
            generation,
            Box::new(move || {
                let closure = Closure {
                    label: &label,
                    asset_base: &asset_base,
                };
                let result = cache.prepare(&closure, file, data);
                (result, Some(cache))
            }),
        );
        Ok(())
    }

    /// [`AudioManager::replace_song_at_seam`] for a song built in memory: its conductor is
    /// loaded off the frame.
    pub fn replace_song_in_memory_at_seam(&mut self, song: Arc<Song>) -> AudioResult<()> {
        let generation = self.live_song_generation()?;
        let label = self
            .songs
            .playback
            .as_ref()
            .map_or_else(String::new, |playback| playback.label.clone());
        self.dispatch_preparation(
            generation,
            Box::new(move || (ReadySong::new(&label, song), None)),
        );
        Ok(())
    }

    /// Every replacement outcome since the last call, oldest first; the list is emptied.
    pub fn take_song_replacements(&mut self) -> Vec<SongReplacement> {
        std::mem::take(&mut self.songs.replacements.outcomes)
    }

    fn live_song_generation(&self) -> AudioResult<u64> {
        self.songs
            .playback
            .as_ref()
            .filter(|playback| !playback.is_dead())
            .map(|playback| playback.generation)
            .ok_or(AudioError::SongNotPlaying)
    }

    fn dispatch_preparation(&mut self, generation: u64, job: PreparationJob) {
        self.songs.drop_pending_replacement();
        let replacements = &mut self.songs.replacements;
        replacements.latest_request += 1;
        let request_id = replacements.latest_request;
        let sender = replacements.sender.clone();
        let run = move || {
            let (result, cache) = job();
            let preparation = Preparation {
                request_id,
                generation,
                result,
                cache,
            };
            if sender.send(preparation).is_err() {
                log::debug!("A song preparation finished after its manager was dropped");
            }
        };
        replacements.in_flight += 1;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let spawned = std::thread::Builder::new()
                .name("song preparation".to_string())
                .spawn(run);
            if let Err(error) = spawned {
                replacements.in_flight -= 1;
                replacements
                    .outcomes
                    .push(SongReplacement::Failed(AudioError::IoError(error)));
            }
        }
        #[cfg(target_arch = "wasm32")]
        run();
    }

    /// Take in what arrived, then swap what is due: a parked preparation at its moment,
    /// one that arrived this update only if the transport is stationary.
    pub(super) fn settle_replacement(&mut self, reading: Option<Reading>) {
        if let Some(reading) = &reading {
            if reading.seek_landed {
                self.swap_at_seek(reading);
            } else {
                self.swap_if_due(reading, false);
            }
        }
        let parked = self.take_in_preparations();
        if let (true, Some(reading)) = (parked, &reading) {
            self.swap_if_due(reading, true);
        }
    }

    /// A host seek landed with a preparation parked: a stationary transport, which the seek
    /// in flight held back, takes it now — a paused song where the seek landed, a song
    /// stopped at its end (whose seek moved nothing) stopped at the new end — and so does a
    /// staged refusal, where the seek landed (refused again when that bar is missing too);
    /// a running song's un-staged preparation waits for its next bar line, as it would have
    /// without the seek.
    fn swap_at_seek(&mut self, reading: &Reading) {
        let Some(playback) = self.songs.playback.as_mut() else {
            return;
        };
        let Some(prepared) = playback.pending_replacement.as_mut() else {
            return;
        };
        let moment = if playback.requested == TransportState::Paused {
            Moment::Paused
        } else if reading.snapshot.ended {
            Moment::StoppedAtEnd
        } else if prepared.staged {
            Moment::Running
        } else {
            return;
        };
        prepared.staged = false;
        self.swap(moment, reading);
    }

    /// Every result off the channel, kept or dropped; `true` when one was parked.
    fn take_in_preparations(&mut self) -> bool {
        let replacements = &mut self.songs.replacements;
        for preparation in replacements.receiver.try_iter() {
            replacements.in_flight = replacements.in_flight.saturating_sub(1);
            replacements.arrived.push_back(preparation);
        }
        let mut parked = false;
        while let Some(preparation) = self.songs.replacements.arrived.pop_front() {
            parked |= self.take_in(preparation);
        }
        parked
    }

    fn take_in(&mut self, preparation: Preparation) -> bool {
        let Preparation {
            request_id,
            generation,
            result,
            cache,
        } = preparation;
        let deck = &mut self.songs;
        let playback = deck
            .playback
            .as_mut()
            .filter(|playback| !playback.is_dead() && playback.generation == generation);
        let Some(playback) = playback.filter(|_| request_id == deck.replacements.latest_request)
        else {
            deck.replacements.outcomes.push(SongReplacement::Dropped);
            retire((result, cache));
            return false;
        };
        match result {
            Ok(ready) => {
                if let Some(cache) = cache {
                    deck.cache.merge(cache);
                }
                playback.pending_replacement = Some(PreparedPlayback {
                    request_id,
                    ready,
                    staged: false,
                });
                true
            }
            Err(error) => {
                if let Some(cache) = cache {
                    deck.cache.merge_failed(cache);
                }
                deck.replacements
                    .outcomes
                    .push(SongReplacement::Failed(error));
                false
            }
        }
    }

    fn swap_if_due(&mut self, reading: &Reading, arrived_now: bool) {
        let snapshot = &reading.snapshot;
        let Some(playback) = self.songs.playback.as_ref() else {
            return;
        };
        let Some(prepared) = playback.pending_replacement.as_ref() else {
            return;
        };
        // A seek in flight may land anywhere, and the old conductor holds it: the swap
        // waits, and the landing takes the preparation where the seek went.
        if playback.edges.seek_in_flight() {
            return;
        }
        let moment = if snapshot.ended {
            if arrived_now {
                Moment::StoppedAtEnd
            } else {
                Moment::ReachedEnd
            }
        } else if playback.requested == TransportState::Paused {
            Moment::Paused
        } else if !arrived_now
            && !prepared.staged
            && snapshot.transport == TransportState::Playing
            && playback.edges.crossed(Subdivision::Bar)
        {
            Moment::Running
        } else {
            return;
        };
        self.swap(moment, reading);
    }

    /// Put the parked preparation in the playing song's place as one transition, or, for a
    /// running song past the new song's last bar, refuse and keep it staged.
    fn swap(&mut self, moment: Moment, reading: &Reading) {
        let snapshot = &reading.snapshot;
        let Some(mut playback) = self.songs.playback.take() else {
            return;
        };
        let Some(mut prepared) = playback.pending_replacement.take() else {
            self.songs.playback = Some(playback);
            return;
        };
        let old = Bars::of(&playback.song);
        let new = Bars::of(&prepared.ready.song);
        let old_tick = old
            .map
            .tick_at_sample(snapshot.sample_position, SONG_SAMPLE_RATE);
        let (bar, mapped) = same_place(old, new, old_tick, playback.looping);
        let start = match (moment, mapped) {
            (Moment::Running, Some(tick)) | (Moment::ReachedEnd, Some(tick)) => {
                Start::Playing(tick)
            }
            (Moment::Running, None) => {
                let reason = format!(
                    "'{}' is in bar {bar}, which the new song does not have, and does not loop; \
                     it plays on, and the new song waits for a pause, its end or a seek",
                    playback.label
                );
                prepared.staged = true;
                playback.pending_replacement = Some(prepared);
                self.songs.playback = Some(playback);
                self.songs
                    .replacements
                    .outcomes
                    .push(SongReplacement::Refused { reason });
                return;
            }
            (Moment::Paused, mapped) => Start::Paused(mapped.unwrap_or(new.end_tick)),
            (Moment::StoppedAtEnd, _) | (Moment::ReachedEnd, None) => Start::StoppedAtEnd,
        };
        let landing_tick = match start {
            Start::Playing(tick) | Start::Paused(tick) => tick,
            Start::StoppedAtEnd => new.end_tick,
        };
        let commands = start_commands(start, playback.looping, new.end_tick);
        let position = position_at(new.map, landing_tick);
        let landed = PositionSnapshot {
            sample_position: new.map.sample_at_tick(landing_tick, SONG_SAMPLE_RATE),
            ..PositionSnapshot::default()
        };

        let PreparedPlayback {
            request_id, ready, ..
        } = prepared;
        let ReadySong {
            song,
            conductor,
            handle,
            ..
        } = ready;
        let meters = Arc::new(SourceMeters::default());
        let source = ConductorSource::new(
            conductor,
            Arc::clone(&self.songs.buses),
            Arc::clone(&meters),
        );
        let mut output = self.song_output(source);
        // The old source has played up to this update; the new one's clock runs from it.
        if let SongOutput::Virtual { last_update, .. } = &mut output {
            *last_update = reading.now;
        }
        let old_output = std::mem::replace(&mut playback.output, output);
        if let SongOutput::Device(sink) = &old_output {
            sink.stop();
        }
        let old_song = std::mem::replace(&mut playback.song, song);
        let old_bindings = std::mem::replace(&mut playback.bindings, binding_table(&playback.song));
        let old_handle = std::mem::replace(&mut playback.handle, handle);
        let old_meters = std::mem::replace(&mut playback.meters, meters);
        playback.edges = playback.edges.handed_over(&landed);
        playback.landing = Some(Landing {
            sample_position: landed.sample_position,
            ended: start == Start::StoppedAtEnd,
        });
        // The new conductor counts its commands from nothing.
        playback.commands_sent = 0;

        let waiting = std::mem::take(&mut playback.pending_lifecycle);
        let flush = self
            .songs
            .flush_floor
            .map(|flush_epoch| ConductorCommand::StopAllSources { flush_epoch });
        let mut sent = true;
        for command in commands.into_iter().chain(flush).chain(waiting) {
            sent &= playback.send_lifecycle(command);
        }
        log::info!(
            "Song '{}' replaced in bar {bar} (request {request_id})",
            playback.label
        );
        self.songs.playback = Some(playback);
        self.songs
            .replacements
            .outcomes
            .push(SongReplacement::Swapped { bar, position });
        retire((old_output, old_song, old_bindings, old_handle, old_meters));
        if !sent {
            self.mark_song_output_dead();
        }
    }
}

#[cfg(test)]
impl AudioManager {
    /// Block until every preparation in flight has arrived — each within a minute, or the
    /// test fails — and leave them newest request first for the next `update` to take in,
    /// as though the newest had finished first.
    pub(crate) fn wait_for_preparations_newest_first(&mut self) {
        let replacements = &mut self.songs.replacements;
        for _ in 0..replacements.in_flight {
            let preparation = replacements
                .receiver
                .recv_timeout(std::time::Duration::from_secs(60))
                .expect("a preparation arrives within a minute");
            replacements.arrived.push_back(preparation);
        }
        replacements.in_flight = 0;
        replacements
            .arrived
            .make_contiguous()
            .sort_by_key(|preparation| std::cmp::Reverse(preparation.request_id));
    }

    /// The parked preparation's request id, if one is parked.
    pub(crate) fn parked_replacement(&self) -> Option<(u64, bool)> {
        self.songs
            .playback
            .as_ref()
            .and_then(|playback| playback.pending_replacement.as_ref())
            .map(|prepared| (prepared.request_id, prepared.staged))
    }
}
