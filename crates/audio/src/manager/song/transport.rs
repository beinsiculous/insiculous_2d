//! The song's transport and what a game reads of it: stop, pause, resume, seek and
//! looping, and the queries — playing, ended, the position, the snapshot and the beat
//! edges the last update latched.

use music_midi::{ConductorCommand, TransportState};

use super::{
    AudioManager, PendingMusic, SongOutput, SongPosition, SongSnapshot, Subdivision,
    SONG_SAMPLE_RATE,
};

impl AudioManager {
    /// Stop the song and drop its conductor; a song pending the first gesture is
    /// forgotten too. Recorded music is untouched.
    pub fn stop_song(&mut self) {
        if matches!(self.pending_music, Some(PendingMusic::Song(_))) {
            self.pending_music = None;
        }
        self.songs.drop_pending_replacement();
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
    /// note. While paused it moves where the song resumes. The seek is no beat crossing:
    /// the edges count again from where it lands, and a parked replacement waits for it.
    pub fn seek_song(&mut self, beat: u64) {
        let Some(playback) = self.songs.playback.as_ref() else {
            return;
        };
        let tick = playback.tempo_map().tick_at_beat(beat);
        let requested = playback.requested;
        self.song_transport(ConductorCommand::Seek(tick), requested);
        let live = self
            .songs
            .playback
            .as_mut()
            .filter(|playback| !playback.is_dead());
        if let Some(playback) = live {
            let ordinal = playback.last_queued_ordinal();
            playback.edges.expect_seek(ordinal);
        }
    }

    /// Loop the song at its end, or let it stop there; takes effect at the next seam.
    pub fn set_song_looping(&mut self, looping: bool) {
        if let Some(playback) = self.songs.playback.as_mut() {
            playback.looping = looping;
        }
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
                && !playback.published_ended()
        })
    }

    /// Whether the song played to its end without looping: what tells a song that
    /// finished from one whose output died. `false` without a song.
    #[must_use]
    pub fn song_ended(&self) -> bool {
        self.songs
            .playback
            .as_ref()
            .is_some_and(|playback| playback.published_ended())
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
        let sample_position = playback.published_sample();
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
}
