//! Songs — the third seam of [`AudioManager`]: a MIDI file and its sidecar played by the
//! conductor, its background on the music bus and its triggered instruments on the sfx
//! bus, with transport, beat queries and the triggers game code sends.
//!
//! **One music slot.** A song and a recorded track share it: `play_song` stops recorded
//! music and `play_music` stops a song. A request made before the web's first gesture is
//! pending, the last of either kind winning, and starts in
//! [`AudioManager::enable_output`]; while pending a song reports not playing and has no
//! position.
//!
//! **One conductor a song.** Each song is loaded into a conductor of its own on the game
//! thread, so the audio thread never allocates for a song change and no command resolved
//! against the old song's bindings can reach the new one: the old queue goes with the old
//! conductor. The flush floor carries over as the new conductor's first command after
//! `Play`.
//!
//! **Without a device** (a headless test, `GameHarness`, a machine with no output) a
//! playing song renders silently from [`AudioManager::update`] by the wall time elapsed
//! since the last update, so its position, its beats and its triggers behave as they
//! would through a device.
//!
//! **Commands that do not fit.** Transport, deactivations and flushes refused by a full
//! queue stay pending here and are retried, in order, from `update()` and before any
//! trigger is sent; a trigger never overtakes them.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use common::clock::Instant;
use music_midi::{
    BindingIndex, CommandRejected, ConductorCommand, ConductorHandle, PositionSnapshot, Song,
    SongData, SongFile, SourceId, TempoMap, TransportState,
};
use rodio::Sink;

use crate::conductor_source::{BusLevels, ConductorSource, SourceMeters, SourceReadings};
use crate::error::{AudioError, AudioResult};

use super::{AudioManager, PendingMusic};

mod edges;
pub(crate) mod load;
#[cfg(test)]
mod tests;
mod virtual_clock;

use edges::BeatEdges;
pub use edges::Subdivision;
use load::{read_song_files, Closure, DependencyCache, ReadySong};
use virtual_clock::{duration_of, frames_in, LONGEST_VIRTUAL_STEP};

/// The rate every song's conductor renders at; the device's mixer resamples it. A
/// position's `latency_samples` count at this rate.
pub const SONG_SAMPLE_RATE: u32 = 44_100;

/// Where a playing song is, read from what its conductor last published.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SongPosition {
    /// Counted from zero, as every position here is.
    pub bar: u64,
    pub beat: u64,
    pub beat_in_bar: u64,
    /// Seconds of song from its start to the next frame the conductor renders.
    pub seconds: f64,
    /// Samples rendered, not yet at the speaker — what this engine can count of them:
    /// the frames of the current block the device has not yet taken. The device's own
    /// buffer (a device period natively, up to a 2048-frame pull in a browser) lies
    /// beyond and is not known; the position runs at least this far ahead of the sound.
    pub latency_samples: u64,
}

/// The conductor's counters and the source's measurements, read value by value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SongSnapshot {
    pub conductor: PositionSnapshot,
    pub source: SourceReadings,
}

/// The manager's song state that outlives one song.
pub(super) struct SongDeck {
    playback: Option<SongPlayback>,
    cache: DependencyCache,
    asset_base: PathBuf,
    buses: Arc<BusLevels>,
    /// The highest epoch flushed, sent to every conductor after its `Play`.
    flush_floor: Option<u32>,
    dead_output_warnings: u64,
}

impl SongDeck {
    pub(super) fn new(music_volume: f32, sfx_volume: f32) -> Self {
        Self {
            playback: None,
            cache: DependencyCache::default(),
            asset_base: PathBuf::new(),
            buses: Arc::new(BusLevels::new(music_volume, sfx_volume)),
            flush_floor: None,
            dead_output_warnings: 0,
        }
    }

    pub(super) fn set_buses(&self, music_volume: f32, sfx_volume: f32) {
        self.buses.set(music_volume, sfx_volume);
    }

    /// The song's sink, for master volume changes.
    pub(super) fn sink(&self) -> Option<&Sink> {
        match &self.playback.as_ref()?.output {
            SongOutput::Device(sink) => Some(sink),
            _ => None,
        }
    }
}

enum SongOutput {
    Device(Sink),
    Virtual {
        source: Box<ConductorSource>,
        last_update: Instant,
        /// Wall time not yet rendered: less than one frame.
        carried: Duration,
    },
    /// The source was taken out of the manager's hands, as a device takes it: what a
    /// test drops to stand for a device dropping it.
    #[cfg(test)]
    Detached,
    /// The device dropped the source, or the source is gone.
    Gone,
}

struct SongPlayback {
    label: String,
    song: Arc<Song>,
    bindings: HashMap<String, BindingIndex>,
    handle: ConductorHandle,
    meters: Arc<SourceMeters>,
    output: SongOutput,
    /// What the game last asked of the transport; the conductor follows at its next block.
    requested: TransportState,
    pending_lifecycle: VecDeque<ConductorCommand>,
    edges: BeatEdges,
}

impl SongPlayback {
    fn tempo_map(&self) -> &TempoMap {
        self.song.background().tempo_map()
    }

    fn is_dead(&self) -> bool {
        matches!(self.output, SongOutput::Gone)
    }

    /// Whether the conductor was dropped — by a device that let its source go, or with
    /// the source itself. The handle is never cloned, so the conductor holds the only
    /// other reference to the position they share.
    fn conductor_is_gone(&self) -> bool {
        Arc::strong_count(self.handle.position()) < 2
    }

    /// Send `command` after every command already waiting, keeping it when the queue is
    /// full. `false` when the conductor is gone.
    fn send_lifecycle(&mut self, command: ConductorCommand) -> bool {
        self.pending_lifecycle.push_back(command);
        self.retry_lifecycle()
    }

    /// Send the waiting commands in order until one does not fit. `false` when the
    /// conductor is gone.
    fn retry_lifecycle(&mut self) -> bool {
        while let Some(command) = self.pending_lifecycle.front().copied() {
            match self.handle.send(command) {
                Ok(()) => {
                    self.pending_lifecycle.pop_front();
                }
                Err(CommandRejected::QueueFull) => return true,
                Err(CommandRejected::ConductorGone) => return false,
            }
        }
        true
    }
}

/// The action → binding table of `song`, built once so a trigger never searches.
fn binding_table(song: &Song) -> HashMap<String, BindingIndex> {
    song.file()
        .bindings
        .iter()
        .filter_map(|binding| {
            song.binding_index(&binding.action)
                .map(|index| (binding.action.clone(), index))
        })
        .collect()
}

impl AudioManager {
    /// The directory a song sidecar's SoundFont and sample paths are relative to: the
    /// game's asset directory. Until it is set they are relative to the working directory.
    pub fn set_asset_base(&mut self, asset_base: impl AsRef<Path>) {
        self.songs.asset_base = asset_base.as_ref().to_path_buf();
    }

    /// Play the song whose MIDI file is at `path` (its sidecar is `<path>.ron` beside it)
    /// from its start, replacing recorded music and any song. The whole closure — the
    /// MIDI file, the sidecar, every SoundFont and sample it names — is read, checked and
    /// built first, in disabled mode too; any failure is an error naming the song, the
    /// sidecar entry and the path, and leaves what is playing untouched.
    pub fn play_song(&mut self, path: impl AsRef<Path>) -> AudioResult<()> {
        let path = path.as_ref();
        let (file, data) = read_song_files(path)?;
        let label = path.display().to_string();
        let ready = self.prepare_ready(&label, file, data)?;
        self.install_song(ready);
        Ok(())
    }

    /// Build a song from a sidecar and MIDI data already in memory (the composer's
    /// unsaved document), its SoundFonts and samples loaded through the same cache as
    /// [`AudioManager::play_song`]'s, ready for [`AudioManager::play_song_in_memory`].
    pub fn prepare_song(
        &mut self,
        label: &str,
        file: SongFile,
        data: SongData,
    ) -> AudioResult<Arc<Song>> {
        Ok(self.prepare_ready(label, file, data)?.song)
    }

    /// Play a song built in memory, as [`AudioManager::play_song`] plays a file.
    pub fn play_song_in_memory(&mut self, song: Arc<Song>) -> AudioResult<()> {
        self.play_song_in_memory_at(song, 0)
    }

    /// [`AudioManager::play_song_in_memory`] from `tick`: its first block plays there,
    /// whether the song starts now or after the web's first gesture.
    pub fn play_song_in_memory_at(&mut self, song: Arc<Song>, tick: u64) -> AudioResult<()> {
        let mut ready = ReadySong::new("the song in memory", song)?;
        ready.start_tick = tick;
        self.install_song(ready);
        Ok(())
    }

    fn prepare_ready(
        &mut self,
        label: &str,
        file: SongFile,
        data: SongData,
    ) -> AudioResult<ReadySong> {
        let asset_base = self.songs.asset_base.clone();
        let closure = Closure {
            label,
            asset_base: &asset_base,
        };
        self.songs.cache.prepare(&closure, file, data)
    }

    fn install_song(&mut self, ready: ReadySong) {
        self.stop_music();
        self.stop_song();
        if self.output.is_none() && self.awaits_gesture {
            self.pending_music = Some(PendingMusic::Song(Box::new(ready)));
            return;
        }
        self.start_song(ready);
    }

    /// Start a ready song on the device, or on the virtual clock without one.
    pub(super) fn start_song(&mut self, ready: ReadySong) {
        let ReadySong {
            label,
            song,
            conductor,
            handle,
            start_tick,
        } = ready;
        let meters = Arc::new(SourceMeters::default());
        let source = ConductorSource::new(
            conductor,
            Arc::clone(&self.songs.buses),
            Arc::clone(&meters),
        );
        let output = self.song_output(source);
        let mut playback = SongPlayback {
            label,
            bindings: binding_table(&song),
            song,
            handle,
            meters,
            output,
            requested: TransportState::Playing,
            pending_lifecycle: VecDeque::new(),
            edges: BeatEdges::default(),
        };
        // A seek while stopped is where the play starts, in whichever block it lands.
        if start_tick > 0 {
            playback.send_lifecycle(ConductorCommand::Seek(start_tick));
        }
        playback.send_lifecycle(ConductorCommand::Play);
        if let Some(flush_epoch) = self.songs.flush_floor {
            playback.send_lifecycle(ConductorCommand::StopAllSources { flush_epoch });
        }
        log::info!("Playing song: {}", playback.label);
        self.songs.playback = Some(playback);
    }

    fn song_output(&self, source: ConductorSource) -> SongOutput {
        let Some(output) = &self.output else {
            return SongOutput::Virtual {
                source: Box::new(source),
                last_update: Instant::now(),
                carried: Duration::ZERO,
            };
        };
        match self.start_sink(output, source, 1.0, 1.0, false) {
            Ok(sink) => SongOutput::Device(sink),
            Err(error) => {
                log::warn!("The song cannot reach the device: {error}");
                SongOutput::Gone
            }
        }
    }

    /// Hand a song playing on the virtual clock to the device that just connected.
    pub(super) fn connect_song_output(&mut self) {
        let Some(playback) = self.songs.playback.as_mut() else {
            return;
        };
        if !matches!(playback.output, SongOutput::Virtual { .. }) {
            return;
        }
        let SongOutput::Virtual { source, .. } =
            std::mem::replace(&mut playback.output, SongOutput::Gone)
        else {
            return;
        };
        let output = self.song_output(*source);
        if let Some(playback) = self.songs.playback.as_mut() {
            playback.output = output;
        }
    }

    /// Stop the song and drop its conductor; a song pending the first gesture is
    /// forgotten too. Recorded music is untouched.
    pub fn stop_song(&mut self) {
        if matches!(self.pending_music, Some(PendingMusic::Song(_))) {
            self.pending_music = None;
        }
        if let Some(playback) = self.songs.playback.take() {
            if let SongOutput::Device(sink) = &playback.output {
                sink.stop();
            }
        }
    }

    /// Hold the song's clock and release every note; triggers are refused until
    /// [`AudioManager::resume_song`].
    pub fn pause_song(&mut self) {
        self.song_transport(ConductorCommand::Pause, TransportState::Paused);
    }

    /// Continue a paused song from where it was held.
    pub fn resume_song(&mut self) {
        self.song_transport(ConductorCommand::Resume, TransportState::Playing);
    }

    /// Move the song to the start of beat `beat` (from zero), releasing every triggered
    /// note. While paused it moves where the song resumes.
    pub fn seek_song(&mut self, beat: u64) {
        let Some(playback) = self.songs.playback.as_ref() else {
            return;
        };
        let tick = playback.tempo_map().tick_at_beat(beat);
        let requested = playback.requested;
        self.song_transport(ConductorCommand::Seek(tick), requested);
    }

    /// Loop the song at its end, or let it stop there; takes effect at the next seam.
    pub fn set_song_looping(&mut self, looping: bool) {
        self.song_lifecycle(ConductorCommand::SetLooping(looping));
    }

    fn song_transport(&mut self, command: ConductorCommand, requested: TransportState) {
        let Some(playback) = self.songs.playback.as_mut() else {
            return;
        };
        if playback.is_dead() {
            return;
        }
        let pausing = command == ConductorCommand::Pause;
        let resuming = command == ConductorCommand::Resume;
        if (pausing && playback.requested != TransportState::Playing)
            || (resuming && playback.requested != TransportState::Paused)
        {
            return;
        }
        playback.requested = requested;
        if !playback.send_lifecycle(command) {
            self.mark_song_output_dead();
        }
    }

    /// Whether a song is playing: started, not paused, not played to its end, and its
    /// output alive. `false` while a song waits for the first gesture.
    #[must_use]
    pub fn is_song_playing(&self) -> bool {
        self.songs.playback.as_ref().is_some_and(|playback| {
            !playback.is_dead()
                && playback.requested == TransportState::Playing
                && !playback.handle.position().has_ended()
        })
    }

    /// Whether the song played to its end without looping: what tells a song that
    /// finished from one whose output died. `false` without a song.
    #[must_use]
    pub fn song_ended(&self) -> bool {
        self.songs
            .playback
            .as_ref()
            .is_some_and(|playback| playback.handle.position().has_ended())
    }

    /// Where the song is, or `None` without a song, while one waits for the first
    /// gesture, or once its output died. The position is a block's start — where a
    /// scheduled trigger fires — so it runs up to 63 frames ahead of the frame playing,
    /// and `latency_samples` further.
    #[must_use]
    pub fn song_position(&self) -> Option<SongPosition> {
        let playback = self
            .songs
            .playback
            .as_ref()
            .filter(|playback| !playback.is_dead())?;
        let sample_position = playback.handle.position().sample_position();
        let tempo_map = playback.tempo_map();
        let tick = tempo_map.tick_at_sample(sample_position, SONG_SAMPLE_RATE);
        let musical = tempo_map.position_at_tick(tick);
        Some(SongPosition {
            bar: musical.bar,
            beat: musical.beat,
            beat_in_bar: musical.beat_in_bar,
            seconds: sample_position as f64 / f64::from(SONG_SAMPLE_RATE),
            latency_samples: playback.meters.readings().undelivered_frames,
        })
    }

    /// The conductor's counters and the source's measurements, while a song plays.
    #[must_use]
    pub fn song_snapshot(&self) -> Option<SongSnapshot> {
        let playback = self.songs.playback.as_ref()?;
        Some(SongSnapshot {
            conductor: playback.handle.position().snapshot(),
            source: playback.meters.readings(),
        })
    }

    /// Whether the last [`AudioManager::update`] saw the song cross a `subdivision`
    /// boundary: at most once an update, however long since the one before.
    #[must_use]
    pub fn beat_crossed(&self, subdivision: Subdivision) -> bool {
        self.songs
            .playback
            .as_ref()
            .is_some_and(|playback| playback.edges.crossed(subdivision))
    }

    /// Play the binding `action` names for `source`, stamped now. An action the song does
    /// not bind is an error naming it; so is no song, or one whose output died. While a
    /// song waits for the first gesture a known action is a silent `Ok`. A trigger waits
    /// behind any transport, deactivation or flush the queue has not yet taken, and is
    /// refused while one waits.
    pub fn trigger(&mut self, action: &str, source: SourceId) -> AudioResult<()> {
        if let Some(PendingMusic::Song(ready)) = &self.pending_music {
            return match ready.song.binding_index(action) {
                Some(_) => Ok(()),
                None => Err(AudioError::UnknownAction {
                    song: ready.label.clone(),
                    action: action.to_string(),
                }),
            };
        }
        let Some(playback) = self.songs.playback.as_mut() else {
            return Err(AudioError::SongNotPlaying);
        };
        let Some(binding) = playback.bindings.get(action).copied() else {
            return Err(AudioError::UnknownAction {
                song: playback.label.clone(),
                action: action.to_string(),
            });
        };
        if playback.is_dead() {
            return Err(AudioError::SongNotPlaying);
        }
        if !playback.retry_lifecycle() {
            self.mark_song_output_dead();
            return Err(AudioError::SongNotPlaying);
        }
        if !playback.pending_lifecycle.is_empty() {
            return Err(AudioError::CommandRefused(CommandRejected::QueueFull));
        }
        let command = ConductorCommand::Trigger {
            binding,
            source,
            stamp: Instant::now(),
        };
        match playback.handle.send(command) {
            Ok(()) => Ok(()),
            Err(CommandRejected::ConductorGone) => {
                self.mark_song_output_dead();
                Err(AudioError::SongNotPlaying)
            }
            Err(refused) => Err(AudioError::CommandRefused(refused)),
        }
    }

    /// End `source`'s activation: its notes released, its scheduled notes dropped. Kept
    /// and retried when the queue is full; a no-op without a song.
    pub fn deactivate_source(&mut self, source: SourceId) {
        self.song_lifecycle(ConductorCommand::DeactivateSource { source });
    }

    /// End every source activated before `flush_epoch` and refuse those epochs from now
    /// on, in this song and every song played after it.
    pub fn stop_all_sources(&mut self, flush_epoch: u32) {
        let floor = self
            .songs
            .flush_floor
            .map_or(flush_epoch, |floor| floor.max(flush_epoch));
        self.songs.flush_floor = Some(floor);
        self.song_lifecycle(ConductorCommand::StopAllSources { flush_epoch });
    }

    fn song_lifecycle(&mut self, command: ConductorCommand) {
        let Some(playback) = self.songs.playback.as_mut() else {
            return;
        };
        if !playback.is_dead() && !playback.send_lifecycle(command) {
            self.mark_song_output_dead();
        }
    }

    /// Advance the song by `now`: the virtual clock without a device, the waiting
    /// commands retried, a dead output noticed, the beat edges latched.
    pub(super) fn update_song(&mut self, now: Instant) {
        let Some(playback) = self.songs.playback.as_mut() else {
            return;
        };
        if playback.is_dead() {
            return;
        }
        let sent = playback.retry_lifecycle();
        if let SongOutput::Virtual {
            source,
            last_update,
            carried,
        } = &mut playback.output
        {
            let elapsed = now
                .saturating_duration_since(*last_update)
                .min(LONGEST_VIRTUAL_STEP);
            *last_update = now;
            let owed = *carried + elapsed;
            let frames = frames_in(owed);
            *carried = owed.saturating_sub(duration_of(frames));
            source.advance_frames(frames);
        }
        if !sent || playback.conductor_is_gone() {
            self.mark_song_output_dead();
            return;
        }
        let snapshot = playback.handle.position().snapshot();
        let tempo_map = playback.song.background().tempo_map();
        playback
            .edges
            .update(&snapshot, tempo_map, SONG_SAMPLE_RATE);
    }

    /// The song's output died — the device dropped its source, or its conductor is gone:
    /// the song stops, and the first time a warning says so.
    fn mark_song_output_dead(&mut self) {
        let Some(playback) = self.songs.playback.as_mut() else {
            return;
        };
        if playback.is_dead() {
            return;
        }
        playback.output = SongOutput::Gone;
        playback.edges = BeatEdges::default();
        self.songs.dead_output_warnings += 1;
        log::warn!(
            "Song '{}' stopped: its audio output is gone",
            playback.label
        );
    }
}
