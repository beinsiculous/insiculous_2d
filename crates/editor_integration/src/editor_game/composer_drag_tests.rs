//! The composer's drag through the real `EditorGame`: thirty frames preview the move with
//! the document untouched and the release lands one entry; Escape, hiding the panel and
//! a replaced song each drop it with nothing applied and nothing older touched; history
//! operations, a track switch and the API wait for it.

use editor::composer::{track_notes, ComposerAction, ComposerFocus, ComposerKeyAction, NoteImage, RollPoint};
use editor::{Modifiers, PanelId};
use winit::keyboard::KeyCode;

use super::shortcuts::KeyRoute;
use super::test_support::{api_line, assert_ok, composer_session, melody_song, DummyGame};
use super::EditorGame;

fn melody_notes(editor: &EditorGame<DummyGame>) -> Vec<NoteImage> {
    track_notes(editor.song.as_ref().expect("a song"), 1)
}

fn undo_count(editor: &EditorGame<DummyGame>) -> usize {
    editor.song.as_ref().map_or(0, |song| song.history().undo_count())
}

fn press(key: KeyCode, editor: &mut EditorGame<DummyGame>) {
    let route = editor.route_editor_key(key, false, Modifiers::default());
    match route {
        KeyRoute::Composer(action) => editor.run_composer_key(action),
        other => panic!("{key:?} should be the composer's: {other:?}"),
    }
}

#[test]
fn a_thirty_frame_diagonal_drag_previews_with_the_song_untouched_and_releases_as_one_entry() {
    let folder = tempfile::tempdir().expect("an asset folder");
    let mut editor = composer_session(folder.path(), melody_song());
    let before = melody_notes(&editor);
    let entries = undo_count(&editor);
    let dragged = vec![before[0].id, before[1].id];
    editor.begin_composer_drag(dragged, RollPoint { tick: 0, key: 60 });
    for frame in 1..=30 {
        let pointer = RollPoint { tick: frame * 16, key: 60 + (frame / 10) as i32 };
        editor.update_composer_drag(Some(pointer), true);
        assert_eq!(melody_notes(&editor), before, "frame {frame}: the preview never touches the song");
    }
    let preview = editor.composer_drag.as_ref().expect("live").preview().clone();
    assert_eq!((preview.tick_delta, preview.key_delta), (480, 3), "the total delta, snapped to a sixteenth");

    editor.update_composer_drag(None, false);
    assert!(editor.composer_drag.is_none(), "the release ends the drag");
    assert_eq!(undo_count(&editor), entries + 1, "thirty frames are one entry");
    let after = melody_notes(&editor);
    let moved: Vec<(u64, u8)> = after.iter().map(|image| (image.tick, image.key)).collect();
    assert_eq!(moved, vec![(480, 63), (960, 67), (960, 67)], "C4 and E4 moved a quarter and three keys");

    editor.undo_song_with_feedback();
    assert_eq!(melody_notes(&editor), before, "one undo puts both back");
}

#[test]
fn escape_drops_a_drag_applying_nothing_and_touching_no_older_entry() {
    let folder = tempfile::tempdir().expect("an asset folder");
    let mut editor = composer_session(folder.path(), melody_song());
    editor.undo_song_with_feedback();
    let before = melody_notes(&editor);
    let entries = undo_count(&editor);
    assert!(editor.song.as_ref().is_some_and(|song| song.history().can_redo()), "a redo entry to keep");

    editor.begin_composer_drag(vec![before[0].id], RollPoint { tick: 0, key: 60 });
    editor.update_composer_drag(Some(RollPoint { tick: 960, key: 72 }), true);
    press(KeyCode::Escape, &mut editor);
    assert!(editor.composer_drag.is_none());
    assert_eq!(editor.editor.composer.focus, ComposerFocus::PianoRoll, "Escape takes the drag first, then the focus");
    assert_eq!(melody_notes(&editor), before, "nothing applied");
    assert_eq!(undo_count(&editor), entries, "no undo entry");
    assert!(editor.song.as_ref().is_some_and(|song| song.history().can_redo()), "the older redo entry stands");

    editor.update_composer_drag(Some(RollPoint { tick: 480, key: 64 }), true);
    editor.update_composer_drag(None, false);
    assert_eq!(melody_notes(&editor), before, "a move after Escape applies nothing");

    editor.begin_composer_drag(vec![before[0].id], RollPoint { tick: 0, key: 60 });
    editor.update_composer_drag(Some(RollPoint { tick: 30, key: 60 }), true);
    editor.update_composer_drag(None, false);
    assert_eq!(undo_count(&editor), entries, "a drag released where it began commits nothing");
}

#[test]
fn a_dragged_chord_clamps_at_the_songs_edges_and_keeps_its_intervals() {
    let folder = tempfile::tempdir().expect("an asset folder");
    let mut editor = composer_session(folder.path(), melody_song());
    let notes = melody_notes(&editor);
    let all: Vec<_> = notes.iter().map(|image| image.id).collect();
    editor.begin_composer_drag(all, RollPoint { tick: 480, key: 64 });
    editor.update_composer_drag(Some(RollPoint { tick: -5_000, key: 300 }), true);
    let preview = editor.composer_drag.as_ref().expect("live").preview().clone();
    assert_eq!((preview.tick_delta, preview.key_delta), (0, 60), "C4 stays at tick 0; G4 reaches 127");
    editor.update_composer_drag(None, false);
    let keys: Vec<u8> = melody_notes(&editor).iter().map(|image| image.key).collect();
    assert_eq!(keys, vec![120, 124, 127], "the same intervals, a fifth from the top");
}

#[test]
fn undo_a_track_switch_and_the_api_wait_for_a_live_drag() {
    let folder = tempfile::tempdir().expect("an asset folder");
    let mut editor = composer_session(folder.path(), melody_song());
    let (sender, receiver) = std::sync::mpsc::channel();
    editor.api.receiver = Some(receiver);
    sender.send("song".to_string()).expect("a queued line");
    let first = melody_notes(&editor)[0];
    let entries = undo_count(&editor);

    editor.begin_composer_drag(vec![first.id], RollPoint { tick: 0, key: 60 });
    editor.run_composer_key(ComposerKeyAction::Undo);
    assert_eq!(undo_count(&editor), entries, "Ctrl+Z mid-drag undoes nothing");
    assert_eq!(editor.editor.status_bar.message(), Some("Finish or Escape the drag first"));
    editor.run_composer_actions(vec![ComposerAction::SetActiveTrack(0)]);
    assert_eq!(editor.editor.composer.active_track, Some(1), "no track switch mid-drag");
    assert!(editor.take_api_lines().is_empty(), "no API line lands between a drag's frames");

    editor.update_composer_drag(None, false);
    assert_eq!(editor.take_api_lines(), vec!["song".to_string()], "the line waited, it was not lost");
}

#[test]
fn hiding_the_panel_mid_drag_drops_the_drag_and_frees_undo_and_the_api() {
    let folder = tempfile::tempdir().expect("an asset folder");
    let mut editor = composer_session(folder.path(), melody_song());
    let before = melody_notes(&editor);
    editor.begin_composer_drag(vec![before[0].id], RollPoint { tick: 0, key: 60 });
    editor.update_composer_drag(Some(RollPoint { tick: 480, key: 61 }), true);

    editor.editor.dock_area.set_panel_visible(PanelId::COMPOSER, false);
    let ui = ui::UIContext::new();
    editor.settle_composer(&ui);
    assert!(editor.composer_drag.is_none(), "the drag went with the panel");
    assert_eq!(editor.editor.composer.focus, ComposerFocus::None, "and so did the keyboard");
    assert_eq!(melody_notes(&editor), before);
    let entries = undo_count(&editor);
    editor.undo_song_with_feedback();
    assert_eq!(undo_count(&editor), entries - 1, "undo runs again");
}

#[test]
fn a_song_replaced_under_a_drag_drops_it_and_no_old_note_reaches_the_new_song() {
    let folder = tempfile::tempdir().expect("an asset folder");
    let mut editor = composer_session(folder.path(), melody_song());
    let first = melody_notes(&editor)[0];
    editor.begin_composer_drag(vec![first.id], RollPoint { tick: 0, key: 60 });
    editor.update_composer_drag(Some(RollPoint { tick: 480, key: 60 }), true);

    let mut world = ecs::World::new();
    assert_ok(&api_line(&mut editor, &mut world, "song new discard"));
    assert!(editor.composer_drag.is_none(), "the old song's drag is gone");
    editor.update_composer_drag(None, false);
    let song = editor.song.as_ref().expect("the new song");
    assert_eq!(song.history().undo_count(), 0, "no entry in the new song");
    assert_eq!(song.data().note_count(), 0);
}
