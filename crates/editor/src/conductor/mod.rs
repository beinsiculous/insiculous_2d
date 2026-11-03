//! The Conductor panel: the song document's panel — a transport, and a sidebar holding the song's
//! file, its parts, instruments and bindings. The score is MIDI Evil's: the Conductor panel maps
//! the `.mid` it exports and never edits its notes.
//!
//! Everything here is data and drawing. The host (`editor_integration`) owns the song, the
//! drag in flight and what each action does: a render returns [`ConductorAction`]s and never
//! touches the document, so every change still goes through `SongDocument::apply`, past
//! the host's one refusal for a play session. One renderer serves both states — while a
//! play session runs the host passes `read_only`, and every field draws its value instead
//! of its control.

pub mod keys;
pub mod layout;
pub mod sidebar;
pub mod state;
pub mod transport;

#[cfg(test)]
mod keys_tests;
#[cfg(test)]
mod layout_tests;
#[cfg(test)]
mod sidebar_tests;
#[cfg(test)]
mod sound_font_row_tests;
#[cfg(test)]
mod state_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod transport_tests;

use music_midi::{Instrument, InstrumentSource, SamplePlayback, TempoMap};

use crate::drag_drop::DragPayload;
use crate::song::{SongDocument, SongEdit};

pub use keys::{resolve as resolve_conductor_key, ConductorKeyAction, ConductorKeyRoute, Direction};
pub use layout::{ConductorLayout, ConductorMode};
pub use state::{ConductorFocus, ConductorState, ControlRequest, SidebarSection};

/// Something the Conductor panel's render or keys ask the host to do.
#[derive(Debug, Clone, PartialEq)]
pub enum ConductorAction {
    /// Give an area the keyboard (a click in it).
    Focus(ConductorFocus),
    /// One edit through the document.
    Edit(SongEdit),
    /// One frame of a number field's gesture — a scrub, a held arrow. The `first` frame
    /// seals whatever entry was open and starts the gesture's own; the rest continue it,
    /// so the gesture is one undo entry. The sidebar's walk decides which frame is which
    /// from the field's own focus, scrub and drawn state, so no ui-wide flag is read.
    FieldEdit { edit: SongEdit, first: bool },
    SetActiveTrack(usize),
    /// Open the song at this asset-relative path (a Songs row, a dropped `.mid`).
    OpenSong(String),
    /// Save to the path the field holds, as typed (the path field's Enter).
    SaveSongTo(String),
    /// Start the song preview from the top, or stop it: the transport's Play while it is
    /// stopped, its Stop while it plays.
    PreviewPlayStop,
    /// Play the binding at this index of the sidecar's bindings, as its action would.
    PreviewTrigger(usize),
    /// Tell the user why something was not done, on the status bar.
    Refused(String),
}

/// A key's name in scientific pitch, middle C (60) being C4.
pub fn note_name(key: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = i32::from(key / 12) - 1;
    format!("{}{octave}", NAMES[usize::from(key % 12)])
}

/// Where a tick falls, as a musician counts it: "bar 1 · beat 1", both from one.
pub fn position_text(tempo_map: &TempoMap, tick: u64) -> String {
    let position = tempo_map.position_at_tick(tick);
    format!("bar {} · beat {}", position.bar + 1, position.beat_in_bar + 1)
}

/// A track as the selector and the sidebar name it: its index (what a part's `Index`
/// selector takes) and its name, when it has one.
pub fn track_label(song: &SongDocument, track: usize) -> String {
    match song.data().tracks().get(track).and_then(|song_track| song_track.name()) {
        Some(name) => format!("Track {track} · {name}"),
        None => format!("Track {track}"),
    }
}

/// The name a new part or instrument takes: `base`, else `base_2`, `base_3` … — the
/// sidecar refuses a repeated name.
pub fn unique_name<'a>(base: &str, taken: impl Iterator<Item = &'a str> + Clone) -> String {
    let free = |candidate: &str| !taken.clone().any(|name| name == candidate);
    if free(base) {
        return base.to_string();
    }
    (2..)
        .map(|number| format!("{base}_{number}"))
        .find(|candidate| free(candidate))
        .unwrap_or_else(|| base.to_string())
}

/// What a payload dropped on the Conductor panel asks for. A `.wav` becomes a one-shot sample
/// instrument rooted on middle C at gain 1.0, named by its file's stem (suffixed while the
/// song has that name); a SoundFont an instrument on its bank 0, preset 0; a song is
/// opened. Any other recorded sound is refused naming WAV — the sample player pitches
/// WAV only — as is anything else, and a sample or a SoundFont with no song open.
pub fn drop_action(song: Option<&SongDocument>, payload: &DragPayload) -> ConductorAction {
    let source = match payload {
        DragPayload::Song { path } => return ConductorAction::OpenSong(path.clone()),
        DragPayload::Sound { path } if is_wav(path) => InstrumentSource::Sample {
            path: path.clone(),
            root_note: 60,
            gain: 1.0,
            playback: SamplePlayback::OneShot,
        },
        DragPayload::Sound { path } => {
            return ConductorAction::Refused(format!(
                "{path} is not a WAV: a sample instrument is pitched from a .wav file"
            ))
        }
        DragPayload::SoundFont { path } => InstrumentSource::SoundFont {
            path: path.clone(),
            bank: 0,
            preset: 0,
        },
        DragPayload::Texture { .. } | DragPayload::Script { .. } => {
            return ConductorAction::Refused(
                "The Conductor panel takes a .wav, a .sf2 or a .mid from Assets".to_string(),
            )
        }
    };
    let Some(song) = song else {
        return ConductorAction::Refused("Open a song first (File → Open Song...)".to_string());
    };
    let path = match &source {
        InstrumentSource::Sample { path, .. } | InstrumentSource::SoundFont { path, .. } => path,
    };
    let stem = std::path::Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "instrument".to_string());
    let name = unique_name(&stem, song.file().instruments.iter().map(|instrument| instrument.name.as_str()));
    ConductorAction::Edit(SongEdit::AddInstrument {
        instrument: Instrument {
            name,
            source,
            provenance: None,
        },
    })
}

fn is_wav(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
}
