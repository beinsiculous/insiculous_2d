//! The song preview and the play session's music: everything the editor asks of the one
//! music slot, pushed as intents by pure methods and drained once a frame in
//! `finish_frame`, the one frame phase that reaches the audio manager.
//!
//! **The preview plays the document.** Its Play prepares the song as the document stands
//! and starts it at a tick, the tick carried by the song so it starts there whether now
//! or after the web's first gesture. Its state follows the song: once a position was read,
//! a song that stops playing ends the preview — silently at the score's end, with a word on
//! the status bar otherwise; before then, no position with a device connected means the
//! song never reached it, and no position without one is the web waiting for its first
//! click. Any change to the document — an edit, an undo or a redo, from the composer or the
//! API — stops it, as another song does: the prepared score is the old document.
//!
//! **Play hears the saved song.** Scene Play arms a pending play instead of starting the
//! session: the drain flushes the sources and plays the saved file, the one slot replacing
//! the preview, and the next frame starts the session, so a game's first update hears the
//! song. A saved song that fails to load says so on the status bar and the session starts
//! all the same, without the editor's music: the editor's Play is never hostage to a
//! song's bank. Intents pushed while it is armed are dropped.
//!
//! **Editing is silent until the composer plays.** The editor stops whatever music the
//! wrapped game's `init` asked for, and Stop flushes the sources and stops the slot
//! whoever filled it — the saved song, the game's own music, a recorded track — so the only
//! music heard while editing is the composer's preview, started and stopped by its
//! transport.

use std::path::{Path, PathBuf};

use audio::{SongPosition, SONG_SAMPLE_RATE};
use editor::composer::ControlRequest;
use editor::SongDocument;
use engine_core::prelude::{AudioManager, SoundSources};
use engine_core::Game;

use super::EditorGame;

/// What the status bar says when a preview ends other than at the score's end.
const STOPPED_EARLY: &str = "Song preview: playback stopped early";
const NEVER_REACHED_THE_DEVICE: &str = "Song preview: the song could not reach the device";
const STOPPED_BY_THE_EDIT: &str = "Song preview stopped by the edit";

/// Whether the preview is playing, as the composer shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum PreviewState {
    #[default]
    Stopped,
    Playing,
}

/// One thing the editor asks of the music slot, applied by the frame's drain in order.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum SongAudioIntent {
    /// Prepare the document as it stands and play it from `from_tick`.
    PlayDocument { from_tick: u64 },
    Stop,
    /// Move the preview to the start of a whole beat, counted from zero.
    Seek { beat: u64 },
    /// Play a binding's part as its action would, from the menu's source.
    Trigger { binding: String },
    PauseSession,
    ResumeSession,
    /// The session's end: every source flushed, and the slot stopped when the editor holds it.
    StopAll,
}

/// Which of the editor's songs holds the one music slot, as the drain last left it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SlotHolder {
    #[default]
    Nobody,
    Preview,
    /// The saved song the session's Play started.
    Session,
}

/// Scene Play, armed until the drain plays the saved song.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PendingPlay {
    /// The saved song to play; `None` with no song open, when Play touches no music.
    song: Option<PathBuf>,
    /// Whether the drain played it, so the next frame starts the session.
    ready: bool,
}

/// See the module docs.
#[derive(Debug, Default)]
pub(super) struct SongPreview {
    state: PreviewState,
    playhead: Option<u64>,
    /// The song generation and history revision the playing preview was prepared from.
    generation: u64,
    revision: u64,
    pending_play: Option<PendingPlay>,
    /// Whether the song in the slot has published a position since it started: before,
    /// it may be waiting for the web's first click.
    position_seen: bool,
    slot: SlotHolder,
    intents: Vec<SongAudioIntent>,
}

/// What the manager says of the slot, read once a frame after the intents.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SlotReadings {
    pub position: Option<SongPosition>,
    /// Whether the conductor has rendered a block: before, the position reads the song's
    /// zero, wherever it starts.
    pub published: bool,
    pub playing: bool,
    pub ended: bool,
    pub enabled: bool,
}

impl SlotReadings {
    pub(super) fn read(audio: &AudioManager) -> Self {
        Self {
            position: audio.song_position(),
            published: audio
                .song_snapshot()
                .is_some_and(|snapshot| snapshot.conductor.blocks_rendered > 0),
            playing: audio.is_song_playing(),
            ended: audio.song_ended(),
            enabled: audio.is_enabled(),
        }
    }
}

/// What following the slot found the preview doing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum PreviewPoll {
    /// Sounding at `seconds` of song, the samples this engine holds taken off.
    Sounding { seconds: f64 },
    /// Waiting for the web's first click, or for the song's first block.
    Waiting,
    /// Ended this frame: with the status bar's word, or silently at the score's end.
    Ended(Option<&'static str>),
}

/// What a frame's drain did that a caller may check.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct DrainReport {
    /// The armed Play's song, when this drain ran it: `Ok`, or why it could not play.
    pub session_play: Option<Result<(), String>>,
}

impl SongPreview {
    pub(super) fn is_playing(&self) -> bool {
        self.state == PreviewState::Playing
    }

    /// Playing and heard: a position was read since it started. A song waiting for the
    /// web's first click is playing but not audible.
    pub(super) fn is_audible(&self) -> bool {
        self.is_playing() && self.position_seen
    }

    pub(super) fn is_armed(&self) -> bool {
        self.pending_play.is_some()
    }

    /// Whether the session's saved song is waiting for the web's first click, when a
    /// pause or a resume would part the session from its sound.
    pub(super) fn session_song_waits(&self) -> bool {
        self.slot == SlotHolder::Session && !self.position_seen
    }

    /// Queue `intent`; dropped while scene Play is armed, so the saved song is what the
    /// session hears. `false` when dropped.
    pub(super) fn push(&mut self, intent: SongAudioIntent) -> bool {
        if self.is_armed() {
            return false;
        }
        self.intents.push(intent);
        true
    }

    fn park(&mut self) {
        self.state = PreviewState::Stopped;
        self.playhead = None;
    }

    /// Follow the slot's song after this frame's intents; the preview ends when its song
    /// stopped playing, or never reached a connected device.
    pub(super) fn poll(&mut self, readings: SlotReadings) -> PreviewPoll {
        if readings.position.is_some() {
            self.position_seen = true;
        }
        if self.position_seen && !readings.playing {
            self.end_in_slot();
            return PreviewPoll::Ended((!readings.ended).then_some(STOPPED_EARLY));
        }
        match readings.position {
            Some(_) if !readings.published => PreviewPoll::Waiting,
            Some(position) => {
                let latency_seconds = position.latency_samples as f64 / f64::from(SONG_SAMPLE_RATE);
                PreviewPoll::Sounding {
                    seconds: (position.seconds - latency_seconds).max(0.0),
                }
            }
            None if readings.enabled => {
                self.end_in_slot();
                PreviewPoll::Ended(Some(NEVER_REACHED_THE_DEVICE))
            }
            None => PreviewPoll::Waiting,
        }
    }

    /// The preview's song is gone from the slot, with no stop sent: it ended there.
    fn end_in_slot(&mut self) {
        self.park();
        self.slot = SlotHolder::Nobody;
        self.position_seen = false;
    }
}

impl<G: Game> EditorGame<G> {
    /// Play the preview from `from_tick`: the composer's Play and Space from the roll's
    /// cursor, the API's `song play` from a beat. Refused during a play session, with no
    /// song, and while scene Play is starting.
    pub(super) fn preview_play(&mut self, from_tick: u64) -> Result<(), String> {
        if let Some(refusal) = self.song_change_refusal() {
            return Err(refusal.to_string());
        }
        if self.song.is_none() {
            return Err("no song is open".to_string());
        }
        if !self.song_preview.push(SongAudioIntent::PlayDocument { from_tick }) {
            return Err("Play is starting the saved song".to_string());
        }
        self.song_preview.state = PreviewState::Playing;
        self.song_preview.playhead = None;
        self.sync_preview_view();
        Ok(())
    }

    /// Stop the preview; nothing while it is stopped.
    pub(super) fn preview_stop(&mut self) {
        if !self.song_preview.is_playing() {
            return;
        }
        self.song_preview.push(SongAudioIntent::Stop);
        self.song_preview.park();
        self.sync_preview_view();
    }

    /// Space and the transport: stop a playing preview, else play it from the roll's
    /// cursor — from the top when the cursor stands at or past the last note's end, where
    /// nothing would sound (Enter leaves it there after adding a note); a refusal goes to
    /// the status bar.
    pub(super) fn preview_play_stop(&mut self) {
        if self.song_preview.is_playing() {
            self.preview_stop();
            return;
        }
        let cursor = self.editor.composer.cursor.tick;
        let end = self.song.as_ref().map_or(0, editor::composer::piano_roll::song_end_tick);
        let tick = if cursor >= end { 0 } else { cursor };
        if let Err(refusal) = self.preview_play(tick) {
            self.editor.status_bar.show_error(format!("Song preview: {refusal}"));
        }
    }

    /// Move a playing preview to a whole beat. Before its song has published a position
    /// it may be waiting for the web's first click, when a seek reaches nothing, so the
    /// play is issued again from the beat instead.
    pub(super) fn preview_seek(&mut self, beat: u64) -> Result<(), String> {
        if !self.song_preview.is_playing() {
            return Err("the preview is not playing — `song play <beat>` starts it there".to_string());
        }
        if self.song_preview.position_seen {
            self.song_preview.push(SongAudioIntent::Seek { beat });
            return Ok(());
        }
        let tick = self
            .song
            .as_ref()
            .map_or(0, |song| song.data().tempo_map().tick_at_beat(beat));
        self.preview_play(tick)
    }

    /// Play a binding's part as `action` would, while the preview sounds: before its first
    /// position a trigger is a silent `Ok` the conductor never hears.
    pub(super) fn preview_trigger(&mut self, action: &str) -> Result<(), String> {
        if !self.song_preview.is_audible() {
            return Err("triggers sound while the song plays — press Play first".to_string());
        }
        self.song_preview.push(SongAudioIntent::Trigger {
            binding: action.to_string(),
        });
        Ok(())
    }

    /// A Trigger button: the binding at `index` of the sidecar's bindings.
    pub(super) fn preview_trigger_binding(&mut self, index: usize) {
        let action = self
            .song
            .as_ref()
            .and_then(|song| song.file().bindings.get(index))
            .map(|binding| binding.action.clone());
        let Some(action) = action else {
            return;
        };
        if let Err(refusal) = self.preview_trigger(&action) {
            self.editor.status_bar.show_error(format!("Trigger {action}: {refusal}"));
        }
    }

    /// Scene Play from Editing, the song's prompt answered: park the preview — its slot is
    /// about to be taken, so no stop is sent — and arm the saved song's play.
    pub(super) fn arm_play(&mut self) {
        let song = self
            .song
            .as_ref()
            .and_then(SongDocument::path)
            .map(Path::to_path_buf);
        let preview = &mut self.song_preview;
        preview.park();
        preview.pending_play = Some(PendingPlay { song, ready: false });
        self.sync_preview_view();
    }

    /// Stop while Play is armed: drop it and stop what the editor started. `true` when
    /// one was armed.
    pub(super) fn cancel_armed_play(&mut self) -> bool {
        if self.song_preview.pending_play.take().is_none() {
            return false;
        }
        self.song_preview.push(SongAudioIntent::StopAll);
        true
    }

    /// Frame start: begin the session whose song the last drain played.
    pub(super) fn start_armed_play(&mut self, world: &mut ecs::World) {
        if !self
            .song_preview
            .pending_play
            .as_ref()
            .is_some_and(|pending| pending.ready)
        {
            return;
        }
        self.song_preview.pending_play = None;
        if self.editor.is_editing() {
            self.start_play_session(world);
        }
    }

    /// The frame's drain, then the slot followed as the manager reads after it.
    pub(super) fn apply_song_intents(
        &mut self,
        audio: &mut AudioManager,
        sources: &mut SoundSources,
    ) -> DrainReport {
        let report = self.drain_song_intents(audio, sources);
        self.follow_song_slot(SlotReadings::read(audio));
        report
    }

    /// Stop a preview whose document moved, apply the intents in order, and run an armed
    /// Play's song last.
    pub(super) fn drain_song_intents(
        &mut self,
        audio: &mut AudioManager,
        sources: &mut SoundSources,
    ) -> DrainReport {
        self.stop_preview_moved_under_it();
        for intent in std::mem::take(&mut self.song_preview.intents) {
            self.apply_song_intent(intent, audio, sources);
        }
        let report = DrainReport {
            session_play: self.play_armed_song(audio, sources),
        };
        self.sync_preview_view();
        report
    }

    /// A playing preview whose song was replaced, or whose document changed, stops: the
    /// prepared score is the old one, and live changes to a running score are not offered.
    fn stop_preview_moved_under_it(&mut self) {
        let preview = &self.song_preview;
        let replaying = preview
            .intents
            .iter()
            .any(|intent| matches!(intent, SongAudioIntent::PlayDocument { .. }));
        if !preview.is_playing() || preview.slot != SlotHolder::Preview || replaying {
            return;
        }
        let replaced = preview.generation != self.song_generation;
        let revision = self.song.as_ref().map(|song| song.history().revision());
        if !replaced && revision == Some(preview.revision) {
            return;
        }
        self.song_preview.intents.insert(0, SongAudioIntent::Stop);
        self.song_preview.park();
        if !replaced {
            self.editor.status_bar.show_message(STOPPED_BY_THE_EDIT);
        }
    }

    fn apply_song_intent(
        &mut self,
        intent: SongAudioIntent,
        audio: &mut AudioManager,
        sources: &mut SoundSources,
    ) {
        let preview_holds_slot = self.song_preview.slot == SlotHolder::Preview;
        let session_holds_slot = self.song_preview.slot == SlotHolder::Session;
        match intent {
            SongAudioIntent::PlayDocument { from_tick } => self.play_document(from_tick, audio),
            SongAudioIntent::Stop if preview_holds_slot => {
                audio.stop_song();
                self.song_preview.slot = SlotHolder::Nobody;
            }
            SongAudioIntent::Seek { beat } if preview_holds_slot => audio.seek_song(beat),
            SongAudioIntent::Trigger { binding } => {
                if let Err(error) = audio.trigger(&binding, sources.menu()) {
                    self.editor.status_bar.show_error(format!("Trigger {binding}: {error}"));
                }
            }
            SongAudioIntent::PauseSession if session_holds_slot => audio.pause_song(),
            SongAudioIntent::ResumeSession if session_holds_slot => audio.resume_song(),
            SongAudioIntent::StopAll => {
                sources.flush(audio);
                // Editing is silent until the composer plays: whoever filled the slot during
                // the session — the saved song, the game's own music, a recorded track — stops.
                audio.stop_song();
                audio.stop_music();
                self.song_preview.slot = SlotHolder::Nobody;
                self.song_preview.position_seen = false;
                self.song_preview.park();
            }
            SongAudioIntent::Stop
            | SongAudioIntent::Seek { .. }
            | SongAudioIntent::PauseSession
            | SongAudioIntent::ResumeSession => {}
        }
    }

    /// Prepare the document as it stands and play it from `from_tick`; a refusal says why
    /// on the status bar and leaves the preview stopped.
    fn play_document(&mut self, from_tick: u64, audio: &mut AudioManager) {
        let Some(song) = self.song.as_ref() else {
            self.song_preview.park();
            return;
        };
        let label = song
            .path()
            .map_or_else(|| "the song in memory".to_string(), |path| path.display().to_string());
        let started = audio
            .prepare_song(&label, song.file().clone(), song.data().clone())
            .and_then(|prepared| audio.play_song_in_memory_at(prepared, from_tick));
        let revision = song.history().revision();
        match started {
            Ok(()) => {
                let preview = &mut self.song_preview;
                preview.slot = SlotHolder::Preview;
                preview.position_seen = false;
                preview.generation = self.song_generation;
                preview.revision = revision;
            }
            Err(error) => {
                // A preview already sounding would part from the state the composer shows.
                if self.song_preview.slot == SlotHolder::Preview {
                    audio.stop_song();
                    self.song_preview.slot = SlotHolder::Nobody;
                }
                self.song_preview.park();
                self.editor
                    .status_bar
                    .show_error(format!("Song preview could not play: {error}"));
            }
        }
    }

    /// An armed Play's music, last of the drain: the sources flushed and, with a song
    /// open, its saved file played — loaded and checked whole before the playing song is
    /// touched. A song that fails to load never blocks Play: the session starts without the
    /// editor's music — a parked preview still sounding is stopped, so the session owns
    /// nothing it did not start; the game's own music is the game's to start — and the
    /// error stays on the status bar.
    fn play_armed_song(
        &mut self,
        audio: &mut AudioManager,
        sources: &mut SoundSources,
    ) -> Option<Result<(), String>> {
        let pending = self.song_preview.pending_play.as_ref().filter(|pending| !pending.ready)?;
        let song = pending.song.clone();
        sources.flush(audio);
        let played = match &song {
            Some(path) => audio.play_song(path).map_err(|error| error.to_string()),
            None => Ok(()),
        };
        let preview = &mut self.song_preview;
        match &played {
            Ok(()) => {
                if let Some(pending) = preview.pending_play.as_mut() {
                    pending.ready = true;
                }
                if song.is_some() {
                    preview.slot = SlotHolder::Session;
                    preview.position_seen = false;
                }
            }
            Err(error) => {
                if let Some(pending) = preview.pending_play.as_mut() {
                    pending.ready = true;
                }
                // `play_song` refused before touching the slot, so a parked preview would
                // otherwise play on under a session that owns nothing.
                if preview.slot == SlotHolder::Preview {
                    audio.stop_song();
                    preview.slot = SlotHolder::Nobody;
                }
                self.editor.status_bar.show_error(format!(
                    "Play could not start the song: {error}; the game runs without it"
                ));
            }
        }
        Some(played)
    }

    /// Follow the slot once a frame: the session's song only for its first position;
    /// the preview, while Editing, for its playhead and for how it ended.
    pub(super) fn follow_song_slot(&mut self, readings: SlotReadings) {
        let preview = &mut self.song_preview;
        if preview.slot == SlotHolder::Session && readings.position.is_some() {
            preview.position_seen = true;
        }
        if preview.slot != SlotHolder::Preview || !preview.is_playing() || !self.editor.is_editing() {
            self.sync_preview_view();
            return;
        }
        match preview.poll(readings) {
            PreviewPoll::Sounding { seconds } => {
                preview.playhead = self
                    .song
                    .as_ref()
                    .map(|song| song.data().tempo_map().tick_at_seconds(seconds));
            }
            PreviewPoll::Waiting => preview.playhead = None,
            PreviewPoll::Ended(word) => {
                if let Some(word) = word {
                    self.editor.status_bar.show_error(word);
                }
            }
        }
        self.sync_preview_view();
    }

    /// Write what the composer shows of the preview.
    fn sync_preview_view(&mut self) {
        self.editor.composer.preview_playing = self.song_preview.is_playing();
        self.editor.composer.playhead_tick = self.song_preview.playhead;
    }

    /// Whether the open song lacks the author or the license Save requires of it.
    pub(super) fn song_lacks_provenance(&self) -> bool {
        self.song.as_ref().is_some_and(|song| {
            song.file().provenance.as_ref().is_none_or(|provenance| {
                provenance.author.trim().is_empty() || provenance.license.trim().is_empty()
            })
        })
    }

    /// After a save from the GUI was refused: when the song lacks its own author or
    /// license, open the Song section with the Author field focused, as Save As opens the
    /// path field, so the way out is on screen.
    pub(super) fn reveal_missing_song_provenance(&mut self) {
        if !self.song_lacks_provenance() {
            return;
        }
        self.editor.composer.sidebar_section = editor::composer::SidebarSection::Song;
        self.show_composer(editor::composer::ComposerFocus::Sidebar);
        self.editor.composer.request = Some(ControlRequest::FocusSongAuthor);
    }
}
