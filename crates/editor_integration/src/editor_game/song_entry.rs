//! The music entry points' host: which song is "the" song, a MIDI file opened at its first
//! part's track, and a sound emitter's binding opened at its part's track.
//!
//! **Which song is "the" song**, in order: during a play session, the song in the music
//! slot — read where the song phase reads the slot and kept here each frame, since the
//! inspector's pass has no audio; else the song open in the Conductor panel, which is what the next
//! Play plays (an open song is the editor's claim on the slot); else the song the game's
//! `init` asked for, which plays when no song is open; else the project's Songs list —
//! scanned first when the Conductor panel never drew it — whose one song is it, whose several the
//! emitter's Song row chooses among, and whose none is an error. Every path is relative to the asset base, as the
//! editor's open requests are.
//!
//! The pair is read at the resolved path through `editor::external_tools::read_song_pair`,
//! never through the song document's open, which may write.

use std::path::{Path, PathBuf};

use ecs::{SoundEmitter, SoundRef, World};
use editor::console::Severity;
use editor::external_tools::{
    binding_location, first_part_location, is_midi_path, midi_sidecar_path, read_song_pair, Location, OpenRequest,
};
use engine_core::prelude::AudioManager;
use engine_core::Game;

use super::init_music::InitMusic;
use super::EditorGame;

/// The error when the rule finds no song at all.
pub(super) const NO_SONG: &str = "No song in this project: a song is a .mid under music/ or ai/music/";

/// Where the rule found the song.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SongByRule {
    /// One song, by its path relative to the asset base.
    Song(PathBuf),
    /// Nothing plays or is open and the project holds several songs: the Song row chooses.
    Choice(Vec<String>),
    /// No song anywhere.
    None,
}

impl<G: Game> EditorGame<G> {
    /// The song phase's reading, each frame: the path of the song pending or playing in
    /// the slot, kept for the rule.
    pub(super) fn keep_slot_song(&mut self, audio: &AudioManager) {
        self.slot_song = audio.song_request().map(Path::to_path_buf);
    }

    /// The rule's answer, the Songs list scanned first when the Conductor panel never drew it.
    pub(super) fn song_by_rule(&mut self) -> SongByRule {
        let init_song = match &self.init_music {
            InitMusic::Song(path) => Some(path.as_path()),
            InitMusic::None | InitMusic::Track(_) => None,
        };
        let earlier = self
            .slot_song
            .as_deref()
            .filter(|_| self.editor.in_play_session())
            .or_else(|| self.song.as_ref().and_then(|song| song.path()))
            .or(init_song);
        if let Some(path) = earlier {
            return SongByRule::Song(PathBuf::from(self.asset_relative(path)));
        }
        if !self.editor.conductor.songs_scanned {
            self.editor.conductor.rescan_songs(&self.asset_base);
        }
        match self.editor.conductor.songs.as_slice() {
            [] => SongByRule::None,
            [only] => SongByRule::Song(PathBuf::from(only)),
            several => SongByRule::Choice(several.to_vec()),
        }
    }

    /// The song the rule names — among several, the Song row's choice, else the first;
    /// [`NO_SONG`] when there is none.
    pub(super) fn the_song(&mut self) -> Result<PathBuf, String> {
        match self.song_by_rule() {
            SongByRule::Song(path) => Ok(path),
            SongByRule::Choice(songs) => {
                let chosen = self.editor.edit_binding.chosen_song.as_ref().filter(|song| songs.contains(song));
                chosen.or_else(|| songs.first()).map(PathBuf::from).ok_or_else(|| NO_SONG.to_string())
            }
            SongByRule::None => Err(NO_SONG.to_string()),
        }
    }

    /// Before the panels draw: the songs the emitter's Song row offers. The rule runs only
    /// while the primary selection names a binding the button would open, so a project's
    /// songs are listed only when asked; the web draws no button and offers nothing.
    pub(super) fn offer_song_choices(&mut self, world: &World) {
        let names_a_binding = self
            .editor
            .selection
            .primary()
            .and_then(|entity| world.get::<SoundEmitter>(entity))
            .is_some_and(|emitter| matches!(&emitter.sound, SoundRef::Binding(action) if !action.trim().is_empty()));
        let offered = names_a_binding && cfg!(not(target_arch = "wasm32"));
        self.editor.edit_binding.song_choices = match offered.then(|| self.song_by_rule()) {
            Some(SongByRule::Choice(songs)) => songs,
            _ => Vec::new(),
        };
    }

    /// The binding the emitter's button asked for, queued as a request for the song's MIDI
    /// file at its part's track; why it cannot be is an error naming the sidecar.
    pub(super) fn open_requested_binding(&mut self) {
        let Some(action) = self.editor.edit_binding.requested.take() else { return };
        match self.binding_request(&action) {
            Ok(request) => self.editor.open_requests.push(request),
            Err(message) => self.report(Severity::Error, message, None),
        }
    }

    fn binding_request(&mut self, action: &str) -> Result<OpenRequest, String> {
        let song = self.the_song()?;
        let (file, data) = read_song_pair(&self.resolve_asset_path(&song)).map_err(|error| error.to_string())?;
        let location = binding_location(&file, &data, action)
            .map_err(|error| format!("{} {error}", midi_sidecar_path(&song).display()))?;
        Ok(OpenRequest { path: song, location })
    }

    /// A MIDI file asked to open at no location opens at its first part's track; any other
    /// request passes untouched. A sidecar that cannot place it — missing, unparseable, or
    /// declaring no part — leaves it at no location, and the console says why.
    pub(super) fn at_first_part_track(&mut self, request: OpenRequest) -> OpenRequest {
        if request.location != Location::None || !is_midi_path(&request.path) {
            return request;
        }
        let sidecar = midi_sidecar_path(&request.path);
        let placed = read_song_pair(&self.resolve_asset_path(&request.path))
            .map_err(|error| error.to_string())
            .and_then(|(file, data)| {
                first_part_location(&file, &data).map_err(|problem| format!("{}: {problem}", sidecar.display()))
            });
        let reason = match placed {
            Ok(Some(location)) => return OpenRequest { location, ..request },
            Ok(None) => format!("{} declares no parts", sidecar.display()),
            Err(reason) => reason,
        };
        let name = request.path.file_name().map_or_else(|| request.path.display().to_string(), |name| name.to_string_lossy().into_owned());
        self.report(Severity::Info, format!("{name} opens at no track: {reason}"), None);
        request
    }
}
