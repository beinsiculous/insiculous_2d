//! The transport row: the active track's selector, the preview's Play and Stop, the zoom,
//! and the playhead's position with the tempo and meter, all in text.
//!
//! The preview does not exist yet: Play and Stop draw disabled with [`PREVIEW_REFUSAL`]
//! as their tooltip, the playhead stays at tick 0, and their keyboard activation answers
//! with the same refusal. The arrows walk the controls left to right and Enter acts on
//! the one reached.

use ui::{Rect, UIContext};

use super::state::{next_track, ComposerState, ControlRequest, MAXIMUM_PIXELS_PER_QUARTER, MINIMUM_PIXELS_PER_QUARTER};
use super::{position_text, track_label, ComposerAction, ComposerFocus, PREVIEW_REFUSAL};
use crate::song::SongDocument;
use crate::theme::EditorTheme;

/// What one press of a zoom button multiplies the zoom by.
const ZOOM_STEP: f32 = 1.5;
const SELECTOR_WIDTH: f32 = 180.0;
const BUTTON_WIDTH: f32 = 56.0;
const ZOOM_BUTTON_WIDTH: f32 = 72.0;
const GAP: f32 = 6.0;

/// The transport's controls, in the order the arrows walk them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportControl {
    TrackSelector,
    Play,
    Stop,
    ZoomOut,
    ZoomIn,
}

pub const TRANSPORT_CONTROLS: [TransportControl; 5] = [
    TransportControl::TrackSelector,
    TransportControl::Play,
    TransportControl::Stop,
    TransportControl::ZoomOut,
    TransportControl::ZoomIn,
];

/// The tempo and meter at tick 0 and the playhead's place, as the transport says them:
/// "Playhead bar 1 · beat 1 · tick 0 · 120 BPM · 4/4".
pub fn status_text(song: &SongDocument) -> String {
    let data = song.data();
    let tempo_map = data.tempo_map();
    let beats_per_minute = 60_000_000.0 / f64::from(tempo_map.microseconds_per_quarter_at(0).max(1));
    let (numerator, denominator) = data
        .time_signatures()
        .first()
        .filter(|signature| signature.tick == 0)
        .map(|signature| (signature.numerator, signature.denominator))
        .unwrap_or((4, 4));
    format!(
        "Playhead {} · tick 0 · {beats_per_minute:.0} BPM · {numerator}/{denominator}",
        position_text(&tempo_map, 0)
    )
}

/// Draw the transport and take its clicks and its keyboard activation.
pub fn render_transport(
    ui: &mut UIContext,
    theme: &EditorTheme,
    state: &mut ComposerState,
    bounds: Rect,
    song: &SongDocument,
) -> Vec<ComposerAction> {
    let mut actions = Vec::new();
    ui.rect(bounds, theme.surface_2);
    ui.push_clip_rect(bounds);
    state.transport_control_count = TRANSPORT_CONTROLS.len();
    state.transport_control = state.transport_control.min(TRANSPORT_CONTROLS.len() - 1);
    let activate = state.request == Some(ControlRequest::ActivateTransport);
    if activate {
        state.request = None;
    }
    let focused = state.focus == ComposerFocus::Transport;
    let row_y = bounds.y + 3.0;
    let height = (bounds.height - 6.0).max(0.0);
    let mut x = bounds.x + GAP;
    for (index, control) in TRANSPORT_CONTROLS.into_iter().enumerate() {
        let width = match control {
            TransportControl::TrackSelector => SELECTOR_WIDTH,
            TransportControl::ZoomOut | TransportControl::ZoomIn => ZOOM_BUTTON_WIDTH,
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
            TransportControl::Play => ("Play".to_string(), false),
            TransportControl::Stop => ("Stop".to_string(), false),
            TransportControl::ZoomOut => ("Zoom out".to_string(), true),
            TransportControl::ZoomIn => ("Zoom in".to_string(), true),
        };
        let clicked = ui.button_styled(("composer_transport", index), &label, button, enabled);
        match control {
            TransportControl::Play | TransportControl::Stop => ui.tooltip(button, PREVIEW_REFUSAL),
            TransportControl::TrackSelector => {
                ui.tooltip(button, "The track the roll and the notes show; click for the next")
            }
            TransportControl::ZoomOut | TransportControl::ZoomIn => {}
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
        &status_text(song),
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
fn act(state: &mut ComposerState, song: &SongDocument, control: TransportControl) -> Vec<ComposerAction> {
    match control {
        TransportControl::TrackSelector => next_track(song, state.active_track, true)
            .map(ComposerAction::SetActiveTrack)
            .into_iter()
            .collect(),
        TransportControl::Play | TransportControl::Stop => vec![ComposerAction::PreviewRefused],
        TransportControl::ZoomOut => {
            state.pixels_per_quarter = (state.pixels_per_quarter / ZOOM_STEP).max(MINIMUM_PIXELS_PER_QUARTER);
            Vec::new()
        }
        TransportControl::ZoomIn => {
            state.pixels_per_quarter = (state.pixels_per_quarter * ZOOM_STEP).min(MAXIMUM_PIXELS_PER_QUARTER);
            Vec::new()
        }
    }
}
