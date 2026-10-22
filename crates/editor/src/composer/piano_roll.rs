//! The piano roll: the active track's notes on a grid of keys and beats.
//!
//! A press on a note selects it (a Ctrl+press toggles it in the selection as the selection
//! stood before the press, and never drags) and begins a drag of the selected notes; a
//! press on empty grid clears the selection and puts the cursor there, where Enter or
//! Insert adds a note. While a drag is live the roll draws the dragged notes where the
//! host's [`MovePreview`] puts them, the document untouched until release. The wheel
//! scrolls the keys, Shift+wheel the ticks, Ctrl+wheel zooms.

use glam::Vec2;
use music_midi::NoteId;
use ui::{Rect, UIContext};

use super::layout::{BeatLineKind, PianoRollGeometry};
use super::state::ComposerState;
use super::{note_name, track_notes, ComposerAction, ComposerFocus, MovePreview, RollPoint};
use crate::editor_input::Modifiers;
use crate::song::SongDocument;
use crate::theme::EditorTheme;

/// What one wheel notch of Ctrl+wheel multiplies the zoom by.
const ZOOM_PER_NOTCH: f32 = 1.25;
/// The quarter notes of empty grid the roll scrolls past the song's last note.
const TRAILING_QUARTERS: u64 = 16;
/// A selected note's border, twice a resting one's: one of its two non-colour marks.
const SELECTED_BORDER_WIDTH: f32 = 2.0;
/// The glyph a selected note carries at its start, its other non-colour mark.
pub const SELECTED_GLYPH: &str = "▸";
/// The keys of the twelve that are black.
const BLACK_KEYS: [u8; 5] = [1, 3, 6, 8, 10];

/// What the roll reads besides its state.
pub struct RollContext<'a> {
    pub song: &'a SongDocument,
    pub preview: Option<&'a MovePreview>,
    pub read_only: bool,
    pub modifiers: Modifiers,
    pub drag_live: bool,
}

/// What the roll did this frame.
#[derive(Debug, Default)]
pub struct RollOutput {
    pub actions: Vec<ComposerAction>,
    /// The tick and key under the pointer — always, while a drag is live, so the drag
    /// follows a pointer that left the roll.
    pub pointer: Option<RollPoint>,
}

/// Take this frame's wheel over the roll — zoom, then the two scrolls — and bring the
/// cursor into view when a key moved it. Call before [`super::layout::compute`], which
/// reads the offsets.
pub fn scroll_roll(
    state: &mut ComposerState,
    ui: &UIContext,
    roll: Rect,
    song: &SongDocument,
    modifiers: Modifiers,
) {
    let ticks_per_quarter = u64::from(song.data().ticks_per_quarter().max(1));
    let mouse = ui.mouse_pos();
    let wheel = ui.scroll_delta().clamp(-1.0, 1.0);
    let over_roll = roll.contains(mouse);
    if modifiers.ctrl && over_roll && wheel != 0.0 {
        state.pixels_per_quarter = (state.pixels_per_quarter * ZOOM_PER_NOTCH.powf(wheel))
            .clamp(super::state::MINIMUM_PIXELS_PER_QUARTER, super::state::MAXIMUM_PIXELS_PER_QUARTER);
    }
    let pixels_per_tick = state.pixels_per_quarter / ticks_per_quarter as f32;
    let last_tick = song_end_tick(song) + ticks_per_quarter * TRAILING_QUARTERS;
    let grid_width = (roll.width - super::layout::KEY_LABEL_WIDTH).max(0.0);
    let content_width = last_tick as f32 * pixels_per_tick;
    let content_height = f32::from(super::layout::KEY_COUNT) * super::layout::KEY_HEIGHT;
    if !state.roll_centered && roll.height > 0.0 {
        let cursor_top = (127.0 - f32::from(state.cursor.key)) * super::layout::KEY_HEIGHT;
        state.roll_vertical.scroll_to(cursor_top - roll.height / 2.0);
        state.roll_centered = true;
    }
    if state.reveal_cursor {
        reveal_cursor(state, roll.height, grid_width, pixels_per_tick);
        state.reveal_cursor = false;
    }
    let (horizontal_wheel, vertical_wheel) = match (modifiers.ctrl, modifiers.shift) {
        (true, _) => (0.0, 0.0),
        (false, true) => (wheel, 0.0),
        (false, false) => (0.0, wheel),
    };
    state.roll_horizontal.end_frame(content_width, grid_width);
    state.roll_horizontal.begin_frame(roll, mouse, horizontal_wheel, grid_width);
    state.roll_vertical.end_frame(content_height, roll.height);
    state.roll_vertical.begin_frame(roll, mouse, vertical_wheel, roll.height);
}

/// Scroll just far enough that the cursor's cell is inside the grid.
fn reveal_cursor(state: &mut ComposerState, height: f32, width: f32, pixels_per_tick: f32) {
    let top = (127.0 - f32::from(state.cursor.key)) * super::layout::KEY_HEIGHT;
    let vertical = state.roll_vertical.offset();
    if top < vertical {
        state.roll_vertical.scroll_to(top);
    } else if top + super::layout::KEY_HEIGHT > vertical + height {
        state.roll_vertical.scroll_to(top + super::layout::KEY_HEIGHT - height);
    }
    let left = state.cursor.tick as f32 * pixels_per_tick;
    let horizontal = state.roll_horizontal.offset();
    if left < horizontal {
        state.roll_horizontal.scroll_to(left);
    } else if left > horizontal + width * 0.9 {
        state.roll_horizontal.scroll_to(left - width * 0.5);
    }
}

/// The tick the song's last note ends on, or 0 for a song with none.
pub fn song_end_tick(song: &SongDocument) -> u64 {
    song.data()
        .tracks()
        .iter()
        .flat_map(|track| track.units())
        .flat_map(|unit| unit.notes.iter().map(move |note| unit.tick.saturating_add(note.duration_ticks)))
        .max()
        .unwrap_or(0)
}

/// Draw the roll and take its presses. See the module docs.
pub fn render_piano_roll(
    ui: &mut UIContext,
    theme: &EditorTheme,
    state: &mut ComposerState,
    geometry: &PianoRollGeometry,
    context: &RollContext,
) -> RollOutput {
    let mut output = RollOutput::default();
    ui.rect(geometry.bounds, theme.surface_0);
    ui.push_clip_rect(geometry.bounds);
    draw_key_rows(ui, theme, geometry, state);
    let tempo_map = context.song.data().tempo_map();
    for line in geometry.beat_lines(&tempo_map) {
        let (color, width) = match line.kind {
            BeatLineKind::Bar => (theme.text_muted, 1.0),
            BeatLineKind::Beat => (theme.grid_primary, 1.0),
            BeatLineKind::Subdivision => (theme.grid_secondary, 1.0),
        };
        ui.line(
            Vec2::new(line.x, geometry.grid.y),
            Vec2::new(line.x, geometry.grid.bottom()),
            color,
            width,
        );
    }

    let Some(track) = state.active_track else {
        ui.label_in_bounds_styled(
            "This song has no note track",
            geometry.grid,
            ui::TextAlign::Center,
            theme.text_secondary,
            theme.fonts.body,
            0.0,
        );
        ui.pop_clip_rect();
        return output;
    };
    let empty_preview = MovePreview::default();
    let preview = context.preview.unwrap_or(&empty_preview);
    let notes = track_notes(context.song, track);
    let mut placed = Vec::with_capacity(notes.len());
    for image in &notes {
        let (tick, key) = preview.place(image);
        let bounds = geometry.note_rect(tick, image.duration_ticks, key);
        placed.push((image.id, bounds));
        ui.rect(bounds, theme.note_fill);
        if context.song.selection().contains(image.id) {
            ui.rect_border(bounds, theme.note_selected_border, SELECTED_BORDER_WIDTH, 0.0);
            ui.label_in_bounds_styled(
                SELECTED_GLYPH,
                bounds,
                ui::TextAlign::Left,
                theme.note_selected_border,
                theme.fonts.small,
                1.0,
            );
        }
    }

    let cursor_rect = Rect::new(
        geometry.tick_to_x(state.cursor.tick),
        geometry.pitch_to_y(state.cursor.key),
        (sixteenth_ticks(context.song) as f32 * geometry.pixels_per_tick).max(3.0),
        geometry.key_height,
    );
    if state.focus == ComposerFocus::PianoRoll {
        let under_cursor = notes
            .iter()
            .find(|image| image.tick == state.cursor.tick && image.key == state.cursor.key)
            .and_then(|image| placed.iter().find(|(id, _)| *id == image.id))
            .map(|(_, bounds)| *bounds);
        ui.rect_border(under_cursor.unwrap_or(cursor_rect), theme.focus_ring, 2.0, 0.0);
    }
    let playhead_x = geometry.tick_to_x(0);
    ui.line(
        Vec2::new(playhead_x, geometry.grid.y),
        Vec2::new(playhead_x, geometry.grid.bottom()),
        theme.playhead,
        2.0,
    );
    ui.pop_clip_rect();

    let mouse = ui.mouse_pos();
    let pointer = RollPoint {
        tick: geometry.x_to_tick(mouse.x),
        key: geometry.y_to_pitch(mouse.y),
    };
    if context.drag_live || geometry.grid.contains(mouse) {
        output.pointer = Some(pointer);
    }
    let result = ui.interact("composer_piano_roll", geometry.grid, !context.read_only);
    if context.read_only || result.state != ui::WidgetState::Active || !ui.mouse_just_pressed() {
        return output;
    }
    output.actions.push(ComposerAction::Focus(ComposerFocus::PianoRoll));
    let hit = placed.iter().rev().find(|(_, bounds)| bounds.contains(mouse)).map(|(id, _)| *id);
    output.actions.extend(press_actions(context, track, hit, pointer));
    if hit.is_none() && pointer.tick >= 0 && (0..=127).contains(&pointer.key) {
        let step = sixteenth_ticks(context.song) as i64;
        state.cursor.tick = (pointer.tick / step * step) as u64;
        state.cursor.key = pointer.key as u8;
    }
    output
}

/// What a press on `hit` (or on empty grid) asks for. A Ctrl+press reads the selection as
/// it stood before the press and never begins a drag.
fn press_actions(
    context: &RollContext,
    track: usize,
    hit: Option<NoteId>,
    point: RollPoint,
) -> Vec<ComposerAction> {
    let selection = context.song.selection();
    match hit {
        Some(note) if context.modifiers.ctrl => {
            let mut notes: Vec<NoteId> = selection.notes().filter(|selected| *selected != note).collect();
            if !selection.contains(note) {
                notes.push(note);
            }
            vec![ComposerAction::Select(notes)]
        }
        Some(note) if selection.contains(note) => vec![ComposerAction::BeginDrag {
            notes: super::state::selected_on_track(context.song, track),
            point,
        }],
        Some(note) => vec![
            ComposerAction::Select(vec![note]),
            ComposerAction::BeginDrag { notes: vec![note], point },
        ],
        None if context.modifiers.ctrl => Vec::new(),
        None => vec![ComposerAction::Select(Vec::new())],
    }
}

/// A sixteenth note in the song's ticks: the cursor's and the nudge's step.
pub fn sixteenth_ticks(song: &SongDocument) -> u64 {
    (u64::from(song.data().ticks_per_quarter()) / 4).max(1)
}

/// The black keys' rows and the key names down the left edge — every C, and the cursor's
/// key while the roll holds the keyboard.
fn draw_key_rows(ui: &mut UIContext, theme: &EditorTheme, geometry: &PianoRollGeometry, state: &ComposerState) {
    for key in 0..=super::HIGHEST_KEY {
        let top = geometry.pitch_to_y(key);
        if top > geometry.bounds.bottom() || top + geometry.key_height < geometry.bounds.y {
            continue;
        }
        let row = Rect::new(geometry.bounds.x, top, geometry.bounds.width, geometry.key_height);
        if BLACK_KEYS.contains(&(key % 12)) {
            ui.rect(row, theme.key_black);
        }
        let cursor_key = state.focus == ComposerFocus::PianoRoll && key == state.cursor.key;
        if key % 12 == 0 || cursor_key {
            let label = Rect::new(geometry.keyboard.x, top, geometry.keyboard.width, geometry.key_height);
            let color = if cursor_key { theme.focus_ring } else { theme.text_secondary };
            ui.label_in_bounds_styled(&note_name(key), label, ui::TextAlign::Left, color, theme.fonts.small, 2.0);
        }
    }
    ui.line(
        Vec2::new(geometry.grid.x, geometry.bounds.y),
        Vec2::new(geometry.grid.x, geometry.bounds.bottom()),
        theme.separator,
        1.0,
    );
}
