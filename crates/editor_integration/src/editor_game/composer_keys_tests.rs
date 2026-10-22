//! The composer's host through the real `EditorGame`: who the keys go to, which document
//! undo undoes, the play session's read-only rule, the active track, the path field's
//! saves and the overwrite prompt, the Songs list and the drops, and the preview's refusal.

use std::path::Path;

use ecs::World;
use editor::composer::{drop_action, track_notes, ComposerAction, ComposerFocus, ComposerKeyAction, Direction};
use editor::{ConfirmChoice, DragPayload, EditorAction, Modifiers, PlayControlAction, SongEdit};
use winit::keyboard::KeyCode;

use super::scene_confirm::PendingSceneAction;
use super::shortcuts::KeyRoute;
use super::test_support::{
    api_line, assert_ok, composer_session, dirty_editor, melody_song, second_track_holds_the_notes,
    timing_only_song, DummyGame,
};
use super::EditorGame;
use super::test_support::press_play;

const PLAIN: Modifiers = Modifiers { ctrl: false, shift: false };

fn folder() -> tempfile::TempDir {
    tempfile::tempdir().expect("an asset folder")
}

fn json(response: &str) -> serde_json::Value {
    serde_json::from_str(response).expect("one line of JSON")
}

fn note_count(editor: &EditorGame<DummyGame>) -> usize {
    editor.song.as_ref().map_or(0, |song| song.data().note_count())
}

fn status(editor: &EditorGame<DummyGame>) -> String {
    editor.editor.status_bar.message().unwrap_or_default().to_string()
}

#[test]
fn composer_keys_never_reach_the_game_while_it_is_focused_and_do_once_it_is_not() {
    let folder = folder();
    let mut editor = composer_session(folder.path(), melody_song());
    let ctrl = Modifiers { ctrl: true, shift: false };
    let cases = [
        (KeyCode::ArrowLeft, PLAIN, KeyRoute::Composer(ComposerKeyAction::MoveCursor(Direction::Left))),
        (KeyCode::Space, PLAIN, KeyRoute::Composer(ComposerKeyAction::PreviewToggle)),
        (KeyCode::KeyG, PLAIN, KeyRoute::Consumed),
        (KeyCode::KeyK, ctrl, KeyRoute::Consumed),
        (KeyCode::KeyS, ctrl, KeyRoute::Editor { action: EditorAction::SaveSong, shift: false }),
        (KeyCode::KeyS, Modifiers { ctrl: true, shift: true }, KeyRoute::Editor { action: EditorAction::SaveSongAs, shift: true }),
    ];
    for (key, modifiers, route) in cases {
        assert_eq!(editor.route_editor_key(key, false, modifiers), route, "{key:?} while focused");
    }
    editor.run_composer_key(ComposerKeyAction::Escape);
    assert_eq!(editor.editor.composer.focus, ComposerFocus::None);
    assert_eq!(editor.route_editor_key(KeyCode::KeyK, false, PLAIN), KeyRoute::ForwardToGame, "once unfocused");
    assert_eq!(
        editor.route_editor_key(KeyCode::KeyS, false, ctrl),
        KeyRoute::Editor { action: EditorAction::Save, shift: false },
        "Ctrl+S is the scene's once the composer lets go"
    );
    assert_eq!(
        editor.route_editor_key(KeyCode::ArrowLeft, false, PLAIN),
        KeyRoute::Editor { action: EditorAction::NudgeLeft, shift: false }
    );
}

#[test]
fn undo_belongs_to_the_focused_document_and_a_click_in_the_scene_moves_it_there() {
    let folder = folder();
    let mut world = World::new();
    let mut editor = dirty_editor(&mut world);
    editor.asset_base = folder.path().to_path_buf();
    editor.song = Some(melody_song());
    editor.follow_song();
    editor.editor.dock_area.set_bounds(ui::Rect::new(0.0, 24.0, 1280.0, 672.0));
    editor.show_composer(ComposerFocus::PianoRoll);
    assert!(editor.command_history.can_undo(), "the scene has an entry to undo");

    editor.undo_with_feedback(&mut world);
    assert_eq!(note_count(&editor), 2, "focused, Edit → Undo undoes the song's last add");
    assert!(editor.command_history.can_undo(), "and leaves the scene alone");
    assert_eq!(status(&editor), "Undid AddNote (song)");

    let mut input = input::InputHandler::new();
    let mut ui = ui::UIContext::new();
    super::test_support::press_mouse(&mut input, glam::Vec2::new(640.0, 150.0));
    ui.begin_frame(&input, glam::Vec2::new(1280.0, 720.0));
    editor.settle_composer(&ui);
    assert_eq!(editor.editor.composer.focus, ComposerFocus::None, "a press in the scene takes the keyboard back");
    editor.undo_with_feedback(&mut world);
    assert!(!editor.command_history.can_undo(), "now Undo undoes the scene");
    assert_eq!(note_count(&editor), 2, "and not the song");
}

#[test]
fn a_menu_click_keeps_the_composers_keyboard_and_a_press_in_the_panel_too() {
    let folder = folder();
    let mut editor = composer_session(folder.path(), melody_song());
    let composer = editor
        .editor
        .dock_area
        .get_panel(editor::PanelId::COMPOSER)
        .expect("the composer")
        .content_bounds();
    let mut input = input::InputHandler::new();
    let mut ui = ui::UIContext::new();
    super::test_support::press_mouse(&mut input, composer.center());
    ui.begin_frame(&input, glam::Vec2::new(1280.0, 720.0));
    editor.settle_composer(&ui);
    assert_eq!(editor.editor.composer.focus, ComposerFocus::PianoRoll, "inside the panel");

    let mut input = input::InputHandler::new();
    let mut ui = ui::UIContext::new();
    super::test_support::press_mouse(&mut input, glam::Vec2::new(30.0, 10.0));
    ui.begin_frame(&input, glam::Vec2::new(1280.0, 720.0));
    editor.settle_composer(&ui);
    assert_eq!(editor.editor.composer.focus, ComposerFocus::PianoRoll, "the menu bar is chrome");
}

#[test]
fn playing_and_paused_refuse_every_composer_mutation_and_clear_the_keyboard() {
    let folder = folder();
    let mut world = World::new();
    let mut editor = composer_session(folder.path(), melody_song());
    assert_ok(&api_line(&mut editor, &mut world, "song save music/theme.mid"));
    editor.show_composer(ComposerFocus::PianoRoll);
    let first = track_notes(editor.song.as_ref().expect("a song"), 1)[0];
    let entries = editor.song.as_ref().map(|song| song.history().undo_count());

    press_play(&mut editor, &mut world);
    for state in ["Playing", "Paused"] {
        assert_eq!(editor.editor.composer.focus, ComposerFocus::None, "{state}: session entry clears the focus");
        assert_eq!(
            editor.route_editor_key(KeyCode::ArrowLeft, false, PLAIN),
            if state == "Playing" { KeyRoute::ForwardToGame } else { KeyRoute::Editor { action: EditorAction::NudgeLeft, shift: false } },
            "{state}: the arrows are not the composer's"
        );
        editor.show_composer(ComposerFocus::PianoRoll);
        let wav = DragPayload::Sound { path: "sounds/kick.wav".to_string() };
        let drop = drop_action(editor.song.as_ref(), &wav);
        editor.run_composer_actions(
            vec![
                ComposerAction::Focus(ComposerFocus::PianoRoll),
                ComposerAction::Edit(SongEdit::SetLoop { looping: false }),
                drop,
                ComposerAction::BeginDrag { notes: vec![first.id], point: editor::composer::RollPoint { tick: 0, key: 60 } },
            ]
        );
        editor.run_composer_key(ComposerKeyAction::Activate);
        editor.run_composer_key(ComposerKeyAction::Undo);
        assert_eq!(editor.editor.composer.focus, ComposerFocus::None, "{state}: no click or menu takes the keyboard");
        assert!(editor.composer_drag.is_none(), "{state}: no drag begins");
        let song = editor.song.as_ref().expect("a song");
        assert!(song.file().looping && song.file().instruments.is_empty(), "{state}: the sidecar is untouched");
        assert_eq!(Some(song.history().undo_count()), entries, "{state}: nothing recorded or undone");
        assert_eq!(song.data().note_count(), 3);
        editor.handle_play_action(PlayControlAction::Pause, &mut world);
    }
}

#[test]
fn the_song_opens_on_its_note_track_enter_adds_there_and_a_switch_clears_the_selection() {
    let folder = folder();
    let mut editor = composer_session(folder.path(), second_track_holds_the_notes());
    assert_eq!(editor.editor.composer.active_track, Some(2), "the lead, not the empty pad");
    editor.editor.composer.cursor = editor::composer::RollCursor { tick: 960, key: 74 };
    editor.run_composer_key(ComposerKeyAction::Activate);
    let song = editor.song.as_ref().expect("a song");
    let lead = track_notes(song, 2);
    assert_eq!(lead.len(), 2, "Enter added to the shown track");
    assert!(song.selection().contains(lead[1].id), "and selected the new note");
    assert_eq!(editor.editor.composer.cursor.tick, 1200, "the cursor moved past it");

    editor.run_composer_actions(vec![ComposerAction::SetActiveTrack(1)]);
    assert_eq!(editor.editor.composer.active_track, Some(1));
    assert!(editor.song.as_ref().is_some_and(|song| song.selection().is_empty()), "no hidden note stays selected");
    editor.run_composer_actions(vec![ComposerAction::SetActiveTrack(0)]);
    assert_eq!(editor.editor.composer.active_track, Some(1), "the timing track is never shown");

    let mut timing = composer_session(folder.path(), timing_only_song());
    assert_eq!(timing.editor.composer.active_track, None);
    timing.run_composer_key(ComposerKeyAction::Activate);
    assert_eq!(note_count(&timing), 0, "a file of timing alone refuses adds");
    assert!(status(&timing).contains("no note track"), "{}", status(&timing));
}

#[test]
fn save_as_onto_another_song_parks_cancel_keeps_both_and_overwrite_completes_in_one_click() {
    let folder = folder();
    let mut world = World::new();
    for final_file in ["music/other.mid", "music/sidecar_only.mid.ron"] {
        let mut editor = composer_session(folder.path(), melody_song());
        common::vfs::write(&folder.path().join(final_file), b"another song").expect("its file");
        let target = final_file.trim_end_matches(".ron");

        // The field's Enter: the key the field owns is consumed, then its commit parks.
        assert_eq!(editor.route_editor_key(KeyCode::Enter, true, PLAIN), KeyRoute::Consumed);
        editor.save_song_from_field(target);
        let parked = PendingSceneAction::OverwriteSong(folder.path().join(target));
        assert_eq!(editor.scene_confirm.pending_action, Some(parked.clone()), "{final_file}: another song");
        assert_eq!(editor.scene_confirm.pending_choice, None, "the same Enter does not answer the prompt");

        editor.resolve_song_choice(parked.clone(), ConfirmChoice::Cancel);
        assert_eq!(common::vfs::read(&folder.path().join(final_file)).ok(), Some(b"another song".to_vec()));
        assert!(editor.song.as_ref().is_some_and(|song| song.path().is_none() && song.is_dirty()), "the song unmoved");

        let refused = json(&api_line(&mut editor, &mut world, &format!("song save {target}")));
        assert_eq!(refused["error"]["kind"], "refused", "{refused}");
        assert!(refused["error"]["message"].as_str().is_some_and(|message| message.contains("overwrite")));

        editor.save_song_from_field(target);
        editor.resolve_song_choice(parked, ConfirmChoice::Confirm);
        assert_eq!(editor.scene_confirm.pending_action, None, "one click closes the prompt");
        assert_eq!(status(&editor), format!("Song saved · {target}"), "the status bar names the document");
        let song = editor.song.as_ref().expect("a song");
        assert_eq!(song.path(), Some(folder.path().join(target).as_path()));
        let written = common::vfs::read(&folder.path().join(target)).expect("the .mid");
        assert_eq!(written, song.data().write_smf().expect("exports"), "Overwrite replaced the file");
    }
    let mut editor = composer_session(folder.path(), melody_song());
    assert_ok(&api_line(&mut editor, &mut world, "song save music/other.mid overwrite"));

    // The song's own file spelt another way is still its own: no prompt.
    editor.save_song_from_field("music/./other.mid");
    assert_eq!(editor.scene_confirm.pending_action, None, "music/./other.mid is the open song's file");
    assert_eq!(status(&editor), "Song saved · music/other.mid");
}

/// The Songs list and the transport are walked with no song open — Open Song… is how
/// the first song opens — while the roll's cursor, which reads the song, stays put.
#[test]
fn the_songs_list_and_the_transport_are_walked_by_the_arrows_with_no_song_open() {
    let folder = folder();
    let mut editor = composer_session(folder.path(), melody_song());
    editor.song = None;
    editor.song_generation += 1;
    editor.follow_song();
    editor.dispatch_song_action(EditorAction::OpenSong);
    assert_eq!(editor.editor.composer.focus, ComposerFocus::Sidebar);
    // The render counts the controls; headless, the count is the three rows a list shows.
    editor.editor.composer.sidebar_control_count = 3;
    editor.run_composer_key(ComposerKeyAction::MoveCursor(Direction::Down));
    editor.run_composer_key(ComposerKeyAction::MoveCursor(Direction::Down));
    assert_eq!(editor.editor.composer.sidebar_control, 2, "Down reaches the third row with no song");
    editor.run_composer_key(ComposerKeyAction::MoveCursor(Direction::Right));
    assert_eq!(editor.editor.composer.sidebar_section, editor::composer::SidebarSection::Parts);

    editor.editor.composer.focus_area(ComposerFocus::Transport);
    editor.editor.composer.transport_control_count = 5;
    editor.run_composer_key(ComposerKeyAction::MoveCursor(Direction::Right));
    assert_eq!(editor.editor.composer.transport_control, 1);

    editor.editor.composer.focus_area(ComposerFocus::PianoRoll);
    editor.run_composer_key(ComposerKeyAction::MoveCursor(Direction::Right));
    assert_eq!(editor.editor.composer.cursor.tick, 0, "the roll's cursor reads the song, and there is none");
}

/// A number field's first frame opens an entry of its own — sealing one left open — and
/// its later frames continue it; a discrete edit after the gesture is another entry.
/// Ctrl+Z after a scrub restores the value before it, not the frame before the release.
#[test]
fn a_number_fields_first_frame_opens_an_entry_its_later_frames_continue_it_and_an_edit_after_is_another() {
    let folder = folder();
    let mut editor = composer_session(folder.path(), melody_song());
    let wav = DragPayload::Sound { path: "sounds/kick.wav".to_string() };
    let add = drop_action(editor.song.as_ref(), &wav);
    editor.run_composer_actions(vec![add]);
    // This crate names no instrument type: the edit is built in the API's JSON grammar.
    let instrument_json = |editor: &EditorGame<DummyGame>| -> serde_json::Value {
        let instrument = &editor.song.as_ref().expect("a song").file().instruments[0];
        serde_json::to_value(instrument).expect("an instrument serializes")
    };
    let kick = |editor: &EditorGame<DummyGame>, gain: f32| -> SongEdit {
        let mut instrument = instrument_json(editor);
        instrument["source"]["Sample"]["gain"] = serde_json::json!(gain);
        let name = instrument["name"].clone();
        serde_json::from_value(serde_json::json!({ "SetInstrument": { "name": name, "instrument": instrument } }))
            .expect("the edit's JSON grammar")
    };
    let gain_of = |editor: &EditorGame<DummyGame>| -> f64 {
        instrument_json(editor)["source"]["Sample"]["gain"].as_f64().expect("a sample's gain")
    };

    let frame = |editor: &EditorGame<DummyGame>, gain: f32, first: bool| ComposerAction::FieldEdit { edit: kick(editor, gain), first };
    let first_gesture = vec![frame(&editor, 1.5, true), frame(&editor, 2.0, false)];
    editor.run_composer_actions(first_gesture);
    // A second gesture while the first was never closed by a commit: its first frame seals it.
    editor.run_composer_actions(vec![frame(&editor, 3.0, true)]);
    editor.run_composer_actions(vec![ComposerAction::Edit(kick(&editor, 3.5))]);
    assert_eq!(gain_of(&editor), 3.5);

    editor.run_composer_key(ComposerKeyAction::Undo);
    assert_eq!(gain_of(&editor), 3.0, "the edit after the gestures was its own entry");
    editor.run_composer_key(ComposerKeyAction::Undo);
    assert_eq!(gain_of(&editor), 2.0, "the second gesture was its own entry");
    editor.run_composer_key(ComposerKeyAction::Undo);
    assert_eq!(gain_of(&editor), 1.0, "the first gesture's frames undid as one, back to before the first");
}

#[test]
fn the_path_fields_enter_saves_there_an_outside_path_is_refused_and_an_orphan_sidecar_moves_the_default_on() {
    let folder = folder();
    let mut editor = composer_session(folder.path(), melody_song());
    common::vfs::write(&folder.path().join("music/untitled.mid.ron"), b"orphan").expect("an orphan sidecar");
    assert_eq!(editor.default_song_path(), folder.path().join("music/untitled_2.mid"), "never over an orphan");

    editor.save_song_from_field("sounds/outside.mid");
    assert!(status(&editor).contains("outside the song folders"), "{}", status(&editor));
    assert!(editor.song.as_ref().is_some_and(|song| song.path().is_none()));
    editor.save_song_from_field("music/levels/boss.mid");
    assert_eq!(
        editor.song.as_ref().and_then(|song| song.path().map(Path::to_path_buf)),
        Some(folder.path().join("music/levels/boss.mid"))
    );
    assert!(editor.editor.composer.songs.contains(&"music/levels/boss.mid".to_string()), "a save rescans the list");
}

#[test]
fn a_songs_row_parks_behind_the_dirty_prompt_and_a_failed_open_after_discard_keeps_the_song() {
    let folder = folder();
    let mut editor = composer_session(folder.path(), melody_song());
    editor.run_composer_actions(vec![ComposerAction::OpenSong("music/missing.mid".to_string())]);
    let parked = PendingSceneAction::OpenSong(folder.path().join("music/missing.mid"));
    assert_eq!(editor.scene_confirm.pending_action, Some(parked.clone()), "the unsaved song asks first");
    editor.resolve_song_choice(parked, ConfirmChoice::Alt);
    assert_eq!(note_count(&editor), 3, "the failed open kept the song it would have replaced");
    assert!(status(&editor).contains("Song open failed"), "{}", status(&editor));
}

#[test]
fn each_drop_lands_its_instrument_or_its_open_and_an_mp3_is_refused_naming_wav() {
    let folder = folder();
    let mut editor = composer_session(folder.path(), melody_song());
    for path in ["sounds/kick.wav", "sounds/kick.wav"] {
        let drop = drop_action(editor.song.as_ref(), &DragPayload::Sound { path: path.to_string() });
        editor.run_composer_actions(vec![drop]);
    }
    let drop = drop_action(editor.song.as_ref(), &DragPayload::SoundFont { path: "soundfonts/organ.sf2".to_string() });
    editor.run_composer_actions(vec![drop]);
    let names: Vec<String> = editor
        .song
        .as_ref()
        .map(|song| song.file().instruments.iter().map(|instrument| instrument.name.clone()).collect())
        .unwrap_or_default();
    assert_eq!(names, ["kick", "kick_2", "organ"]);

    let mp3 = drop_action(editor.song.as_ref(), &DragPayload::Sound { path: "music/loop.mp3".to_string() });
    editor.run_composer_actions(vec![mp3]);
    assert!(status(&editor).contains("WAV"), "{}", status(&editor));

    let song = drop_action(editor.song.as_ref(), &DragPayload::Song { path: "music/theme.mid".to_string() });
    editor.run_composer_actions(vec![song]);
    assert_eq!(
        editor.scene_confirm.pending_action,
        Some(PendingSceneAction::OpenSong(folder.path().join("music/theme.mid"))),
        "a dropped song opens through the dirty prompt like any open"
    );
}

#[test]
fn a_held_shift_arrow_nudge_is_one_entry_until_the_key_is_released() {
    let folder = folder();
    let mut editor = composer_session(folder.path(), melody_song());
    let first = track_notes(editor.song.as_ref().expect("a song"), 1)[0];
    editor.song.as_mut().expect("a song").select([first.id]);
    let entries = editor.song.as_ref().map_or(0, |song| song.history().undo_count());
    for _ in 0..3 {
        editor.run_composer_key(ComposerKeyAction::Nudge(Direction::Right));
    }
    editor.song.as_mut().expect("a song").break_merge();
    editor.run_composer_key(ComposerKeyAction::Nudge(Direction::Up));
    let song = editor.song.as_ref().expect("a song");
    assert_eq!(song.history().undo_count(), entries + 2, "one held key, then another");
    let moved = song.data().note_image(first.id).expect("still there");
    assert_eq!((moved.tick, moved.key), (360, 61), "three sixteenths right, a semitone up");
}
