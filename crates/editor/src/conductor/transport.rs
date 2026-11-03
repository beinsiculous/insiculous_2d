//! The transport row: the active track's selector, the preview's Play and Stop, and the
//! playhead's position with the tempo and meter, all in text.
//!
//! Play starts the host's preview from the top and Stop stops it; each does nothing while
//! the preview is already where it would put it, and both draw disabled while a play session
//! holds the music. The text says where the preview is. The arrows walk the controls left
//! to right and Enter acts on the one reached.

use ui::{Rect, UIContext};

use super::state::{next_track, ConductorState, ControlRequest};
use super::{position_text, track_label, ConductorAction, ConductorFocus};
use crate::song::SongDocument;
use crate::theme::EditorTheme;

const SELECTOR_WIDTH: f32 = 180.0;
const BUTTON_WIDTH: f32 = 56.0;
const GAP: f32 = 6.0;

/// The transport's controls, in the order the arrows walk them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportControl {
    TrackSelector,
    Play,
    Stop,
}

pub const TRANSPORT_CONTROLS: [TransportControl; 3] = [
    TransportControl::TrackSelector,
    TransportControl::Play,
    TransportControl::Stop,
];

/// Where the preview is, with the tempo and meter there, as the transport says it:
/// stopped, "Playhead bar 1 · beat 1 · tick 0 · 120 BPM · 4/4"; playing, "Playing · bar 2 ·
/// beat 3 · 120 BPM · 4/4"; and "Playing · waiting for the first click" while a song waits
/// for the web's first gesture.
pub fn status_text(song: &SongDocument, state: &ConductorState) -> String {
    let tick = match (state.preview_playing, state.playhead_tick) {
        (true, None) => return "Playing · waiting for the first click".to_string(),
        (true, Some(tick)) => tick,
        (false, _) => 0,
    };
    let data = song.data();
    let tempo_map = data.tempo_map();
    let beats_per_minute = 60_000_000.0 / f64::from(tempo_map.microseconds_per_quarter_at(tick).max(1));
    let (numerator, denominator) = data
        .time_signatures()
        .iter()
        .take_while(|signature| signature.tick <= tick)
        .last()
        .map(|signature| (signature.numerator, signature.denominator))
        .unwrap_or((4, 4));
    let meter = format!("{beats_per_minute:.0} BPM · {numerator}/{denominator}");
    if state.preview_playing {
        format!("Playing · {} · {meter}", position_text(&tempo_map, tick))
    } else {
        format!("Playhead {} · tick 0 · {meter}", position_text(&tempo_map, 0))
    }
}

/// Draw the transport and take its clicks and its keyboard activation. `read_only` is a play
/// session: the game's music owns the one slot, so the preview's controls are drawn disabled.
pub fn render_transport(
    ui: &mut UIContext,
    theme: &EditorTheme,
    state: &mut ConductorState,
    bounds: Rect,
    song: &SongDocument,
    read_only: bool,
) -> Vec<ConductorAction> {
    let mut actions = Vec::new();
    ui.rect(bounds, theme.surface_2);
    ui.push_clip_rect(bounds);
    state.transport_control_count = TRANSPORT_CONTROLS.len();
    state.transport_control = state.transport_control.min(TRANSPORT_CONTROLS.len() - 1);
    let activate = state.request == Some(ControlRequest::ActivateTransport);
    if activate {
        state.request = None;
    }
    let focused = state.focus == ConductorFocus::Transport;
    let row_y = bounds.y + 3.0;
    let height = (bounds.height - 6.0).max(0.0);
    let mut x = bounds.x + GAP;
    for (index, control) in TRANSPORT_CONTROLS.into_iter().enumerate() {
        let width = match control {
            TransportControl::TrackSelector => SELECTOR_WIDTH,
            TransportControl::Play | TransportControl::Stop => BUTTON_WIDTH,
        };
        let button = Rect::new(x, row_y, width, height);
        x += width + GAP;
        let (label, enabled) = match control {
            TransportControl::TrackSelector => (
                state
                    .active_track
                    .map(|track| track_label(song, track))
                    .unwrap_or_else(|| "No note track".to_string()),
                state.active_track.is_some(),
            ),
            TransportControl::Play => ("Play".to_string(), !read_only),
            TransportControl::Stop => ("Stop".to_string(), !read_only),
        };
        let clicked = ui.button_styled(("conductor_transport", index), &label, button, enabled);
        match control {
            TransportControl::Play | TransportControl::Stop if read_only => {
                ui.tooltip(button, "The game plays the saved song during a play session")
            }
            TransportControl::Play => ui.tooltip(button, "Play the song from the top (Space)"),
            TransportControl::Stop => ui.tooltip(button, "Stop the song (Space)"),
            TransportControl::TrackSelector => {
                ui.tooltip(button, "The track Add a part builds on; click for the next")
            }
        }
        if focused && index == state.transport_control {
            ui.rect_border(button, theme.focus_ring, 2.0, 2.0);
        }
        let activated = activate && index == state.transport_control;
        if clicked || activated {
            if clicked {
                state.transport_control = index;
            }
            actions.extend(act(state, song, control));
        }
    }
    ui.label_in_bounds_styled(
        &status_text(song, state),
        Rect::new(x, bounds.y, (bounds.right() - x).max(0.0), bounds.height),
        ui::TextAlign::Left,
        theme.text_secondary,
        theme.fonts.small,
        GAP,
    );
    ui.pop_clip_rect();
    actions
}

/// What a transport control does when pressed or activated.
fn act(state: &ConductorState, song: &SongDocument, control: TransportControl) -> Vec<ConductorAction> {
    match control {
        TransportControl::TrackSelector => next_track(song, state.active_track, true)
            .map(ConductorAction::SetActiveTrack)
            .into_iter()
            .collect(),
        TransportControl::Play if !state.preview_playing => vec![ConductorAction::PreviewPlayStop],
        TransportControl::Stop if state.preview_playing => vec![ConductorAction::PreviewPlayStop],
        TransportControl::Play | TransportControl::Stop => Vec::new(),
    }
}
