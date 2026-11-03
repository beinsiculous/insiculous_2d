//! What the composer remembers between frames: which of its areas holds the keyboard, the
//! one track the transport shows, the sidebar's scroll and keyboard cursors, and the songs
//! found in the two song folders.
//!
//! The state follows the document it was built against: the host bumps a generation each
//! time the song is replaced, created or closed, and [`ComposerState::follow`] resets
//! everything that pointed into the old song when the two differ.

use std::path::Path;

use ui::WidgetId;

use crate::external_tools::{scan_songs, scan_sound_fonts, SoundFontScan};
use crate::scroll::ScrollState;
use crate::song::SongDocument;

/// Which of the composer's areas holds the keyboard. `None` is the composer unfocused:
/// every key goes to the editor and the game as before.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComposerFocus {
    #[default]
    None,
    Transport,
    Sidebar,
}

/// The order Tab walks the areas in.
const FOCUS_CYCLE: [ComposerFocus; 2] = [ComposerFocus::Transport, ComposerFocus::Sidebar];

impl ComposerFocus {
    /// Whether the composer holds the keyboard at all.
    pub fn is_held(self) -> bool {
        self != ComposerFocus::None
    }

    /// The area Tab moves to; from `None`, the first.
    pub fn next(self) -> Self {
        let index = FOCUS_CYCLE.iter().position(|area| *area == self);
        match index {
            Some(index) => FOCUS_CYCLE[(index + 1) % FOCUS_CYCLE.len()],
            None => FOCUS_CYCLE[0],
        }
    }

    /// The area Shift+Tab moves to; from `None`, the last.
    pub fn previous(self) -> Self {
        let index = FOCUS_CYCLE.iter().position(|area| *area == self);
        match index {
            Some(index) => FOCUS_CYCLE[(index + FOCUS_CYCLE.len() - 1) % FOCUS_CYCLE.len()],
            None => FOCUS_CYCLE[FOCUS_CYCLE.len() - 1],
        }
    }

    /// The area's name, as the status bar and a tab say it.
    pub fn label(self) -> &'static str {
        match self {
            ComposerFocus::None => "Composer",
            ComposerFocus::Transport => "Transport",
            ComposerFocus::Sidebar => "Song",
        }
    }
}

/// The sidebar's four sections; below the full width it shows one at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarSection {
    #[default]
    Song,
    Parts,
    Instruments,
    Bindings,
}

impl SidebarSection {
    pub const ALL: [SidebarSection; 4] = [
        SidebarSection::Song,
        SidebarSection::Parts,
        SidebarSection::Instruments,
        SidebarSection::Bindings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SidebarSection::Song => "Song",
            SidebarSection::Parts => "Parts",
            SidebarSection::Instruments => "Instruments",
            SidebarSection::Bindings => "Bindings",
        }
    }
}

/// A keyboard request to a control the next render draws: the render knows which
/// control sits at a cursor index; the key handler, which runs before it, does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlRequest {
    /// Act on the transport control at the transport cursor, as a click would.
    ActivateTransport,
    /// Act on the sidebar control at the sidebar cursor, as a click would.
    ActivateSidebar,
    /// Put the keyboard in the song's path field (Save Song As…).
    FocusPathField,
    /// Put the keyboard in the song's Author field: a save refused for the song's own
    /// provenance shows where it is filled.
    FocusSongAuthor,
    /// Put the sidebar's cursor on the Songs list's first row (Open Song…).
    FocusSongsList,
}

/// See the module docs.
#[derive(Debug, Clone)]
pub struct ComposerState {
    pub focus: ComposerFocus,
    /// The track the transport shows and "Add a part" builds on, an index into
    /// `SongData::tracks`; only a track with a channel, and `None` for a song with none.
    pub active_track: Option<usize>,
    /// The song generation everything here was built against.
    generation: Option<u64>,
    pub sidebar_scroll: ScrollState,
    /// The transport control the arrows reached.
    pub transport_control: usize,
    /// How many controls the transport drew last frame, so the arrows stop at the last.
    pub transport_control_count: usize,
    /// The sidebar control the arrows reached.
    pub sidebar_control: usize,
    /// How many controls the sidebar drew last frame.
    pub sidebar_control_count: usize,
    /// Bring the sidebar's cursor into view on the next frame: a key moved it.
    pub reveal_control: bool,
    /// The one sidebar section shown below the full width.
    pub sidebar_section: SidebarSection,
    /// Each section the sidebar drew last frame with the index of its first control, in
    /// order: every section at the full width, the tabs' one below it.
    pub sidebar_section_starts: Vec<(SidebarSection, usize)>,
    /// The sidebar number field whose gesture is open: its later frames continue the song
    /// entry its first frame opened. Closed by the field's commit, by its losing focus with
    /// no scrub running, and by a frame that does not draw it.
    pub field_gesture: Option<WidgetId>,
    pub request: Option<ControlRequest>,
    /// Every MIDI file under the song folders, by its asset-relative path.
    pub songs: Vec<String>,
    /// Whether `songs` and `sound_fonts` are current; cleared once when the panel closes,
    /// so opening it rescans — the sound emitter's song rule shares the list and its scan.
    pub songs_scanned: bool,
    /// Whether the panel drew since the host last saw it hidden: the close the clear above
    /// waits for, once, rather than every hidden frame — and what tells a draw that the
    /// scan it finds was the rule's, made while the panel was hidden, so it scans again.
    pub panel_drawn: bool,
    /// Every bank under `soundfonts/`, which the Song section's SoundFont row cycles.
    pub sound_fonts: SoundFontScan,
    /// Whether the host's song preview is playing, written by the host each frame: the
    /// transport's Play and Stop act on it.
    pub preview_playing: bool,
    /// The tick the preview is sounding, written by the host each frame while it plays;
    /// `None` while stopped and while a song waits for the web's first click.
    pub playhead_tick: Option<u64>,
}

impl Default for ComposerState {
    fn default() -> Self {
        Self {
            focus: ComposerFocus::None,
            active_track: None,
            generation: None,
            sidebar_scroll: ScrollState::default(),
            transport_control: 0,
            transport_control_count: 0,
            sidebar_control: 0,
            sidebar_control_count: 0,
            reveal_control: false,
            sidebar_section: SidebarSection::Song,
            sidebar_section_starts: Vec::new(),
            field_gesture: None,
            request: None,
            songs: Vec::new(),
            songs_scanned: false,
            panel_drawn: false,
            sound_fonts: SoundFontScan::default(),
            preview_playing: false,
            playhead_tick: None,
        }
    }
}

impl ComposerState {
    /// Reset what pointed into another song when `generation` is not the one this state
    /// was built against; returns whether it reset. The songs and banks lists are the
    /// folders', not the song's, and survive.
    pub fn follow(&mut self, generation: u64, song: Option<&SongDocument>) -> bool {
        if self.generation == Some(generation) {
            return false;
        }
        let songs = std::mem::take(&mut self.songs);
        let sound_fonts = std::mem::take(&mut self.sound_fonts);
        let songs_scanned = self.songs_scanned;
        let panel_drawn = self.panel_drawn;
        *self = Self {
            generation: Some(generation),
            active_track: song.and_then(default_track),
            songs,
            sound_fonts,
            songs_scanned,
            panel_drawn,
            ..Self::default()
        };
        true
    }

    /// Drop the keyboard: every key goes back to the editor and the game.
    pub fn clear_focus(&mut self) {
        self.focus = ComposerFocus::None;
        self.request = None;
    }

    /// Left and Right in the sidebar: the next section, or the one before, wrapping. With
    /// every section drawn the cursor moves to that section's first control — a section
    /// with none is passed over — and behind tabs the tab changes and the cursor goes to its
    /// top.
    pub fn step_sidebar_section(&mut self, forward: bool) {
        let all_drawn = self.sidebar_section_starts.len() > 1;
        if !all_drawn {
            let sections = SidebarSection::ALL;
            let index = sections.iter().position(|section| *section == self.sidebar_section).unwrap_or(0);
            let next = (if forward { index + 1 } else { index + sections.len() - 1 }) % sections.len();
            self.sidebar_section = sections[next];
            self.sidebar_control = 0;
            return;
        }
        // Each drawn section with at least one control, and where its controls begin.
        let starts = &self.sidebar_section_starts;
        let mut stops: Vec<(SidebarSection, usize)> = Vec::new();
        for (index, (section, start)) in starts.iter().enumerate() {
            let end = starts.get(index + 1).map_or(self.sidebar_control_count, |(_, next)| *next);
            if *start < end {
                stops.push((*section, *start));
            }
        }
        let Some(current) = stops.iter().rposition(|(_, start)| *start <= self.sidebar_control) else {
            return;
        };
        let next = (if forward { current + 1 } else { current + stops.len() - 1 }) % stops.len();
        let (section, start) = stops[next];
        self.sidebar_section = section;
        self.sidebar_control = start;
    }

    /// Find every song in the two song folders under `asset_base` again, and every bank.
    /// Record a draw of the panel; whether its lists want a scan first: never scanned, or
    /// scanned while the panel was hidden — the sound emitter's song rule shares the list —
    /// since opening the panel is when the folders are looked at again.
    pub fn drawn(&mut self) -> bool {
        let wants_scan = !self.songs_scanned || !self.panel_drawn;
        self.panel_drawn = true;
        wants_scan
    }

    pub fn rescan_songs(&mut self, asset_base: &Path) {
        self.songs = scan_songs(asset_base);
        self.sound_fonts = scan_sound_fonts(asset_base);
        self.songs_scanned = true;
    }

    /// Keep `active_track` a track the song still has with a channel: an adopted score may
    /// have fewer tracks than the one it replaced, and a stale index let "Add a part" build
    /// one on a track that was not there.
    pub fn reconcile_active_track(&mut self, song: Option<&SongDocument>) {
        let Some(song) = song else {
            self.active_track = None;
            return;
        };
        let still_selectable = self.active_track.is_some_and(|track| {
            song.data().tracks().get(track).is_some_and(|song_track| song_track.channel().is_some())
        });
        if !still_selectable {
            self.active_track = default_track(song);
        }
    }
}

/// The track a song opens on: the first holding a note, else the first with a channel to
/// write notes on; `None` for a song whose tracks are all timing.
pub fn default_track(song: &SongDocument) -> Option<usize> {
    let tracks = song.data().tracks();
    tracks
        .iter()
        .position(|track| track.note_count() > 0 && track.channel().is_some())
        .or_else(|| tracks.iter().position(|track| track.channel().is_some()))
}

/// The tracks the selector cycles over: those with a channel. A type-1 file's timing
/// track has none, so a note never lands on it.
pub fn selectable_tracks(song: &SongDocument) -> Vec<usize> {
    song.data()
        .tracks()
        .iter()
        .enumerate()
        .filter(|(_, track)| track.channel().is_some())
        .map(|(index, _)| index)
        .collect()
}

/// The selectable track after `current`, wrapping; the first when `current` is none.
pub fn next_track(song: &SongDocument, current: Option<usize>, forward: bool) -> Option<usize> {
    let tracks = selectable_tracks(song);
    let position = current.and_then(|track| tracks.iter().position(|index| *index == track));
    let count = tracks.len();
    if count == 0 {
        return None;
    }
    let next = match (position, forward) {
        (Some(position), true) => (position + 1) % count,
        (Some(position), false) => (position + count - 1) % count,
        (None, _) => 0,
    };
    Some(tracks[next])
}
