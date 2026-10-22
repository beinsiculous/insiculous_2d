//! The piano roll: the clamp that keeps a dragged chord whole and inside the song, the one
//! batch a move commits, what a press on a note asks for, the selection's two non-colour
//! marks in the draw list, and what a drop on the composer becomes.

use glam::Vec2;
use ui::{DrawCommand, Rect, UIContext};

use super::piano_roll::{render_piano_roll, RollContext, RollOutput, SELECTED_GLYPH};
use super::state::ComposerState;
use super::test_support::melody;
use super::{
    clamp_move, drop_action, moved_notes_edit, note_images, track_notes, ComposerAction, PianoRollGeometry,
    LAST_WRITABLE_TICK,
};
use crate::drag_drop::DragPayload;
use crate::editor_input::Modifiers;
use crate::song::{Gesture, SongDocument, SongEdit};
use crate::test_support::{frame, press_at};
use crate::theme::EditorTheme;

/// A roll over the melody's track, scrolled so its three notes are on screen.
fn roll_state() -> ComposerState {
    let mut state = ComposerState::default();
    state.active_track = Some(1);
    state.roll_vertical.scroll_to(500.0);
    state
}

fn geometry(state: &ComposerState) -> PianoRollGeometry {
    PianoRollGeometry::new(Rect::new(0.0, 0.0, 800.0, 400.0), state, 480)
}

fn render(ui: &mut UIContext, song: &SongDocument, state: &mut ComposerState, ctrl: bool) -> RollOutput {
    let theme = EditorTheme::default();
    let geometry = geometry(state);
    let context = RollContext {
        song,
        preview: None,
        read_only: false,
        modifiers: Modifiers { ctrl, shift: false },
        drag_live: false,
    };
    render_piano_roll(ui, &theme, state, &geometry, &context)
}

/// The centre of the note at `index` on the melody's track.
fn note_centre(song: &SongDocument, state: &ComposerState, index: usize) -> Vec2 {
    let image = track_notes(song, 1)[index];
    geometry(state).note_rect(image.tick, image.duration_ticks, image.key).center()
}

#[test]
fn a_dragged_chord_is_clamped_whole_so_it_keeps_its_intervals_inside_the_song() {
    let mut song = melody();
    song.apply(super::test_support::add_note(1, 0, 64), Gesture::Discrete).expect("a chord");
    let chord = note_images(&song, track_notes(&song, 1).iter().filter(|image| image.tick == 0).map(|image| image.id));
    assert_eq!(chord.len(), 2);
    assert_eq!(clamp_move(&chord, -500, 0), (0, 0), "tick 0 is the floor for the earliest note");
    assert_eq!(clamp_move(&chord, 0, 100), (0, 63), "E4 reaches 127; C4 keeps its third below");
    assert_eq!(clamp_move(&chord, 0, -100), (0, -60), "C4 reaches 0");

    let mut late = melody();
    let last = *track_notes(&late, 1).last().expect("three notes");
    late.apply(
        SongEdit::MoveNotes {
            notes: vec![last.id.number()],
            tick_delta: (LAST_WRITABLE_TICK - last.tick - last.duration_ticks - 1_000) as i64,
        },
        Gesture::Discrete,
    )
    .expect("near the end");
    let near_end = note_images(&late, [last.id]);
    let (tick_delta, _) = clamp_move(&near_end, 10_000, 0);
    let moved = moved_notes_edit(&near_end, tick_delta, 0).expect("it still moves");
    late.apply(moved, Gesture::Discrete).expect("a clamped drag commits");
    let end = note_images(&late, [last.id])[0];
    assert_eq!(end.tick + end.duration_ticks, LAST_WRITABLE_TICK, "it ends exactly at the limit");
}

#[test]
fn a_move_commits_one_batch_of_the_time_shift_then_a_pitch_per_starting_key() {
    let song = melody();
    let images = note_images(&song, track_notes(&song, 1).iter().take(2).map(|image| image.id));
    assert_eq!(moved_notes_edit(&images, 0, 0), None, "an unmoved drag commits nothing");
    let (first, second) = (images[0].id.number(), images[1].id.number());
    assert_eq!(
        moved_notes_edit(&images, 240, 2),
        Some(SongEdit::Batch(vec![
            SongEdit::MoveNotes { notes: vec![first, second], tick_delta: 240 },
            SongEdit::SetPitch { notes: vec![first], key: 62 },
            SongEdit::SetPitch { notes: vec![second], key: 66 },
        ])),
        "a diagonal drag across two starting keys, in key order"
    );
    let mut applied = melody();
    let edit = moved_notes_edit(&images, 240, 2).expect("an edit");
    applied.apply(edit, Gesture::Discrete).expect("the batch lands");
    assert_eq!(applied.history().undo_count(), 4, "one entry over the three adds");
}

#[test]
fn a_press_on_a_note_selects_and_drags_it_and_a_ctrl_press_toggles_without_a_drag() {
    let mut song = melody();
    let notes = track_notes(&song, 1);
    let mut state = roll_state();
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    let centre = note_centre(&song, &state, 0);
    let output = press_at(&mut ui, &mut input, centre, |ui| render(ui, &song, &mut state, false));
    assert!(matches!(
        output.actions.as_slice(),
        [ComposerAction::Focus(_), ComposerAction::Select(selected), ComposerAction::BeginDrag { notes: dragged, .. }]
            if selected == &vec![notes[0].id] && dragged == &vec![notes[0].id]
    ), "{:?}", output.actions);

    song.select([notes[0].id]);
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let second = note_centre(&song, &state, 1);
    let output = press_at(&mut ui, &mut input, second, |ui| render(ui, &song, &mut state, true));
    assert_eq!(
        output.actions[1..],
        [ComposerAction::Select(vec![notes[0].id, notes[1].id])],
        "Ctrl adds to the selection as it stood before the press, and no drag begins"
    );
}

#[test]
fn a_selected_note_carries_a_thicker_border_and_a_glyph_besides_its_colour() {
    let mut song = melody();
    let first = track_notes(&song, 1)[0].id;
    song.select([first]);
    let mut state = roll_state();
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();
    frame(&mut ui, &input, |ui| render(ui, &song, &mut state, false));
    let theme = EditorTheme::default();
    let commands = ui.draw_list().commands();
    let borders = commands
        .iter()
        .filter(|command| {
            matches!(command, DrawCommand::RectBorder { color, width, .. }
                if *color == theme.note_selected_border && *width >= 2.0)
        })
        .count();
    assert_eq!(borders, 1, "the one selected note's border");
    let glyphs = commands
        .iter()
        .filter(|command| match command {
            DrawCommand::TextPlaceholder { text, .. } => text == SELECTED_GLYPH,
            _ => false,
        })
        .count();
    assert_eq!(glyphs, 1, "and its glyph");
}

#[test]
fn a_drop_becomes_an_instrument_an_open_or_a_refusal_naming_why() {
    let mut song = melody();
    let wav = DragPayload::Sound { path: "sounds/kick.wav".to_string() };
    let first = drop_action(Some(&song), &wav);
    let ComposerAction::Edit(edit) = first else {
        panic!("a .wav adds an instrument: {first:?}");
    };
    song.apply(edit, Gesture::Discrete).expect("the instrument lands");
    match drop_action(Some(&song), &wav) {
        ComposerAction::Edit(SongEdit::AddInstrument { instrument }) => {
            assert_eq!(instrument.name, "kick_2", "the song already has a kick");
            assert!(instrument.provenance.is_none(), "provenance is the author's to give");
        }
        other => panic!("{other:?}"),
    }
    match drop_action(Some(&song), &DragPayload::Sound { path: "music/loop.mp3".to_string() }) {
        ComposerAction::Refused(reason) => assert!(reason.contains("WAV"), "{reason}"),
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        drop_action(Some(&song), &DragPayload::SoundFont { path: "soundfonts/organ.sf2".to_string() }),
        ComposerAction::Edit(SongEdit::AddInstrument { instrument })
            if instrument.name == "organ"
                && instrument.source == music_midi::InstrumentSource::SoundFont {
                    path: "soundfonts/organ.sf2".to_string(),
                    bank: 0,
                    preset: 0
                }
    ));
    assert_eq!(
        drop_action(Some(&song), &DragPayload::Song { path: "music/theme.mid".to_string() }),
        ComposerAction::OpenSong("music/theme.mid".to_string())
    );
    assert!(matches!(drop_action(None, &wav), ComposerAction::Refused(_)), "no song to add to");
}
