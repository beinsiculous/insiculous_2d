//! What the wrapped game's `init` asked the audio manager for — a song or a recorded
//! track — remembered and stopped at boot, since editing is silent until the composer plays,
//! and asked for again when a Play session starts with no composer song to take the slot. Stop
//! silences it with the rest of the slot; the next Play asks again. A game that starts its
//! music once at `init`, as the editor demo's platformer does, is therefore heard in Play.
//!
//! The request is replayed as the game made it, by path and nothing else: a song built in
//! memory (`play_song_in_memory`) has no path and is not replayed; a file the game removed
//! after it loaded fails the replay — the status bar says so and the session runs without it;
//! and what the game set after the request (`set_song_looping`, a seek) is not replayed. A game
//! that reads the audio's state for its own music control (`is_music_playing`) stays truthful
//! across the replay; one that trusts a flag it set at `init` does not.

use std::path::PathBuf;

use engine_core::prelude::AudioManager;

#[derive(Debug, Clone, PartialEq, Default)]
pub(super) enum InitMusic {
    #[default]
    None,
    /// A song, by the path `play_song` was given.
    Song(PathBuf),
    /// A recorded track, with the volume it was asked with.
    Track(audio::MusicRequest),
}

impl InitMusic {
    /// Read the slot right after the game's `init`: the song pending or playing, by the path it
    /// was asked for (a song built in memory has none), else the track pending or playing,
    /// else nothing.
    pub(super) fn capture(audio: &AudioManager) -> Self {
        if let Some(path) = audio.song_request() {
            return InitMusic::Song(path.to_path_buf());
        }
        match audio.music_request() {
            Some(request) => InitMusic::Track(request.clone()),
            None => InitMusic::None,
        }
    }

    /// Ask for it again, as the game did; the error names what would not play.
    pub(super) fn replay(&self, audio: &mut AudioManager) -> Result<(), String> {
        match self {
            InitMusic::None => Ok(()),
            InitMusic::Song(path) => audio.play_song(path).map_err(|error| error.to_string()),
            InitMusic::Track(request) => audio
                .play_music_with_volume(&request.path, request.volume)
                .map_err(|error| error.to_string()),
        }
    }
}
