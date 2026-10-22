//! The note list: the active track's notes as rows of text, the roll's keyboard-first twin.
//!
//! Each row names the note, its key number, where it falls in bars and beats and in ticks,
//! its velocity and its length in ticks, so nothing rests on the drawing. A selected row
//! carries the accent fill and a leading "▸"; the keyboard's row a focus ring. A click
//! selects the row's note (Ctrl+click toggles it in the selection).

use glam::Vec2;
use music_midi::{NoteId, NoteImage, TempoMap};
use ui::{Rect, UIContext};

use super::state::ComposerState;
use super::{note_name, position_text, track_label, track_notes, ComposerAction, ComposerFocus};
use crate::editor_input::Modifiers;
use crate::song::SongDocument;
use crate::theme::EditorTheme;

/// One row of the list.
pub const ROW_HEIGHT: f32 = 20.0;
/// The list's heading row.
const HEADING_HEIGHT: f32 = 22.0;
/// The mark a selected row leads with, beside its fill.
pub const SELECTED_ROW_MARK: &str = "▸";

/// What the list reads besides its state.
pub struct NoteListContext<'a> {
    pub song: &'a SongDocument,
    pub read_only: bool,
    pub modifiers: Modifiers,
}

/// One row's text: "C4 · key 60 · bar 1 · beat 1 · tick 0 · velocity 100 · 240 ticks".
pub fn row_text(image: &NoteImage, tempo_map: &TempoMap) -> String {
    format!(
        "{} · key {} · {} · tick {} · velocity {} · {} ticks",
        note_name(image.key),
        image.key,
        position_text(tempo_map, image.tick),
        image.tick,
        image.velocity,
        image.duration_ticks,
    )
}

/// Draw the list and take its clicks.
pub fn render_note_list(
    ui: &mut UIContext,
    theme: &EditorTheme,
    state: &mut ComposerState,
    bounds: Rect,
    context: &NoteListContext,
) -> Vec<ComposerAction> {
    let mut actions = Vec::new();
    ui.rect(bounds, theme.surface_1);
    let notes = state
        .active_track
        .map(|track| track_notes(context.song, track))
        .unwrap_or_default();
    let heading = match state.active_track {
        Some(track) => format!("Notes · {} · {} notes", track_label(context.song, track), notes.len()),
        None => "Notes · no note track".to_string(),
    };
    ui.label_in_bounds_styled(
        &heading,
        Rect::new(bounds.x, bounds.y, bounds.width, HEADING_HEIGHT),
        ui::TextAlign::Left,
        theme.text_primary,
        theme.fonts.body,
        6.0,
    );
    let rows = Rect::new(
        bounds.x,
        bounds.y + HEADING_HEIGHT,
        bounds.width,
        (bounds.height - HEADING_HEIGHT).max(0.0),
    );
    state.note_list_row = state.note_list_row.min(notes.len().saturating_sub(1));
    if state.reveal_row {
        let top = state.note_list_row as f32 * ROW_HEIGHT;
        let offset = state.note_list_scroll.offset();
        if top < offset {
            state.note_list_scroll.scroll_to(top);
        } else if top + ROW_HEIGHT > offset + rows.height {
            state.note_list_scroll.scroll_to(top + ROW_HEIGHT - rows.height);
        }
        state.reveal_row = false;
    }
    state.note_list_scroll.end_frame(notes.len() as f32 * ROW_HEIGHT, rows.height);
    let offset = state
        .note_list_scroll
        .begin_frame(rows, ui.mouse_pos(), ui.scroll_delta(), rows.height);

    let tempo_map = context.song.data().tempo_map();
    let focused = state.focus == ComposerFocus::NoteList;
    ui.push_clip_rect(rows);
    for (index, image) in notes.iter().enumerate() {
        let row = Rect::new(rows.x, rows.y + index as f32 * ROW_HEIGHT - offset, rows.width, ROW_HEIGHT);
        if row.bottom() < rows.y || row.y > rows.bottom() {
            continue;
        }
        let selected = context.song.selection().contains(image.id);
        if selected {
            ui.rect(row, theme.selection_fill);
            ui.label_styled(
                SELECTED_ROW_MARK,
                Vec2::new(row.x + 4.0, row.y + ROW_HEIGHT * 0.7),
                theme.text_primary,
                theme.fonts.small,
            );
        }
        ui.label_in_bounds_styled(
            &row_text(image, &tempo_map),
            row,
            ui::TextAlign::Left,
            theme.text_primary,
            theme.fonts.small,
            18.0,
        );
        if focused && index == state.note_list_row {
            ui.rect_border(row, theme.focus_ring, 2.0, 0.0);
        }
        let clicked = ui
            .interact(ui::WidgetId::from_str_index("composer_note_row", index), row, !context.read_only)
            .clicked;
        if clicked && rows.contains(ui.mouse_pos()) {
            state.note_list_row = index;
            actions.push(ComposerAction::Focus(ComposerFocus::NoteList));
            actions.push(ComposerAction::Select(clicked_selection(context, image.id)));
        }
    }
    ui.pop_clip_rect();
    if notes.is_empty() && state.active_track.is_some() {
        ui.label_in_bounds_styled(
            "No notes yet: focus the roll, place the cursor and press Enter",
            Rect::new(rows.x, rows.y, rows.width, ROW_HEIGHT),
            ui::TextAlign::Left,
            theme.text_muted,
            theme.fonts.small,
            6.0,
        );
    }
    actions
}

/// The selection a click on `note` makes: the note alone, or with Ctrl the selection as it
/// stood with the note toggled.
fn clicked_selection(context: &NoteListContext, note: NoteId) -> Vec<NoteId> {
    if !context.modifiers.ctrl {
        return vec![note];
    }
    let selection = context.song.selection();
    let mut notes: Vec<NoteId> = selection.notes().filter(|selected| *selected != note).collect();
    if !selection.contains(note) {
        notes.push(note);
    }
    notes
}
