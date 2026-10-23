//! The sidebar: the Songs list opened from the keyboard (Open Song…) with the open song's
//! row inert, the path field's Enter as a save and its blur as nothing — and Save Song As…
//! bringing the field into view before focusing it — a sample instrument's provenance and
//! the song's own: the mark while it is missing, the one edit an author field commits, a
//! refused save's request focusing the song's Author; a binding's Trigger live only while
//! the preview sounds; and a scrub of a number field landing every frame.

use glam::Vec2;
use input::prelude::KeyCode;
use music_midi::InstrumentSource;
use ui::{DrawCommand, Rect, UIContext};

use super::sidebar::{path_field_id, render_sidebar, song_author_field_id, SidebarContext, ROW_HEIGHT};
use super::state::{ComposerFocus, ComposerState, ControlRequest, SidebarSection};
use super::test_support::melody;
use super::{drop_action, ComposerAction};
use crate::drag_drop::DragPayload;
use crate::song::{Gesture, SongDocument, SongEdit};
use crate::test_support::{click_through, frame, move_to, press_at, release, type_key};
use crate::theme::EditorTheme;

const SIDEBAR: Rect = Rect { x: 0.0, y: 0.0, width: 300.0, height: 2000.0 };
const TABS: Rect = Rect { x: 0.0, y: 2000.0, width: 300.0, height: 24.0 };

fn context<'a>(song: Option<&'a SongDocument>, enter_pressed: bool, song_path: Option<&'a str>) -> SidebarContext<'a> {
    SidebarContext {
        song,
        read_only: false,
        enter_pressed,
        default_path: "music/untitled.mid",
        song_path,
        preview_audible: false,
        sound_font_present: true,
    }
}

fn sidebar_state() -> ComposerState {
    let mut state = ComposerState::default();
    state.focus = ComposerFocus::Sidebar;
    state.songs = vec!["ai/music/theme.mid".to_string(), "music/theme.mid".to_string()];
    state.songs_scanned = true;
    state
}

#[test]
fn open_song_puts_the_cursor_on_the_songs_list_and_enter_opens_any_row_but_the_open_one() {
    let song = melody();
    let theme = EditorTheme::default();
    let mut state = sidebar_state();
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();
    let draw = |ui: &mut UIContext, state: &mut ComposerState| {
        render_sidebar(ui, &theme, state, SIDEBAR, None, &context(Some(&song), false, Some("music/theme.mid")))
    };

    state.request = Some(ControlRequest::FocusSongsList);
    frame(&mut ui, &input, |ui| draw(ui, &mut state));
    state.request = Some(ControlRequest::ActivateSidebar);
    let first = frame(&mut ui, &input, |ui| draw(ui, &mut state));
    assert_eq!(
        first,
        vec![ComposerAction::OpenSong("ai/music/theme.mid".to_string())],
        "the first row, by its folder-qualified path"
    );

    state.sidebar_control += 1;
    state.request = Some(ControlRequest::ActivateSidebar);
    let open_row = frame(&mut ui, &input, |ui| draw(ui, &mut state));
    assert!(open_row.is_empty(), "the open song's own row is not a target: {open_row:?}");
}

#[test]
fn the_path_fields_enter_saves_there_and_leaving_it_any_other_way_saves_nothing() {
    let song = melody();
    let theme = EditorTheme::default();
    let mut state = sidebar_state();
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    state.request = Some(ControlRequest::FocusPathField);
    frame(&mut ui, &input, |ui| render_sidebar(ui, &theme, &mut state, SIDEBAR, None, &context(Some(&song), false, None)));
    assert!(ui.wants_keyboard(), "Save Song As… puts the keyboard in the path field");
    let saved = type_key(&mut ui, &mut input, KeyCode::Enter, |ui| {
        render_sidebar(ui, &theme, &mut state, SIDEBAR, None, &context(Some(&song), true, None))
    });
    assert_eq!(saved, vec![ComposerAction::SaveSongTo("music/untitled.mid".to_string())]);

    state.request = Some(ControlRequest::FocusPathField);
    frame(&mut ui, &input, |ui| render_sidebar(ui, &theme, &mut state, SIDEBAR, None, &context(Some(&song), false, None)));
    let left = type_key(&mut ui, &mut input, KeyCode::Tab, |ui| {
        render_sidebar(ui, &theme, &mut state, SIDEBAR, None, &context(Some(&song), false, None))
    });
    assert!(left.is_empty(), "a commit with no Enter only leaves the field: {left:?}");
}

#[test]
fn a_sample_without_provenance_shows_the_mark_and_an_author_commits_the_whole_instrument() {
    let mut song = melody();
    let wav = crate::drag_drop::DragPayload::Sound { path: "sounds/kick.wav".to_string() };
    let ComposerAction::Edit(add) = super::drop_action(Some(&song), &wav) else {
        panic!("a .wav adds an instrument");
    };
    song.apply(add, Gesture::Discrete).expect("the sample lands");
    let theme = EditorTheme::default();
    let mut state = sidebar_state();
    state.sidebar_section = SidebarSection::Instruments;
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let draw = |ui: &mut UIContext, state: &mut ComposerState, enter: bool| {
        render_sidebar(ui, &theme, state, SIDEBAR, Some(TABS), &context(Some(&song), enter, None))
    };

    frame(&mut ui, &input, |ui| draw(ui, &mut state, false));
    let marked = ui.draw_list().commands().iter().any(|command| {
        matches!(command, DrawCommand::TextPlaceholder { text, .. } if text.contains("needed before Save"))
    });
    assert!(marked, "a sample with no author or license says so");

    // Root note, gain, playback, then the author field.
    state.sidebar_control = 3;
    state.request = Some(ControlRequest::ActivateSidebar);
    frame(&mut ui, &input, |ui| draw(ui, &mut state, false));
    type_key(&mut ui, &mut input, KeyCode::KeyJ, |ui| draw(ui, &mut state, false));
    let committed = type_key(&mut ui, &mut input, KeyCode::Enter, |ui| draw(ui, &mut state, true));
    let instrument = song.file().instruments[0].clone();
    match committed.as_slice() {
        [ComposerAction::Edit(SongEdit::SetInstrument { name, instrument: edited })] => {
            assert_eq!(name, &instrument.name);
            assert_eq!(edited.source, instrument.source, "the source rides along unchanged");
            let provenance = edited.provenance.as_ref().expect("the author is set");
            assert_eq!((provenance.author.as_str(), provenance.license.as_str()), ("j", ""));
        }
        other => panic!("one SetInstrument carrying the instrument: {other:?}"),
    }
}

/// A sidebar scrolled past the Song section holds nothing of the path field; Save Song
/// As… must bring the field back before focusing it, or a focused field the walk never
/// draws owns the keyboard with nothing to receive the keys.
#[test]
fn save_song_as_scrolls_the_sidebar_to_its_path_field_before_focusing_it() {
    let song = melody();
    let theme = EditorTheme::default();
    let mut state = sidebar_state();
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let short = Rect { x: 0.0, y: 0.0, width: 300.0, height: 120.0 };
    let draw = |ui: &mut UIContext, state: &mut ComposerState, enter: bool| {
        render_sidebar(ui, &theme, state, short, None, &context(Some(&song), enter, None))
    };

    frame(&mut ui, &input, |ui| draw(ui, &mut state, false));
    state.sidebar_scroll.scroll_to(1000.0);
    frame(&mut ui, &input, |ui| draw(ui, &mut state, false));
    assert!(state.sidebar_scroll.offset() > ROW_HEIGHT, "the path row is scrolled out: {}", state.sidebar_scroll.offset());

    state.request = Some(ControlRequest::FocusPathField);
    frame(&mut ui, &input, |ui| draw(ui, &mut state, false));
    assert_eq!(state.sidebar_scroll.offset(), 0.0, "Save Song As… scrolls to the field");
    assert_eq!(state.sidebar_control, 0, "and puts the cursor on it");
    assert!(ui.is_focused(path_field_id()));
    let saved = type_key(&mut ui, &mut input, KeyCode::Enter, |ui| draw(ui, &mut state, true));
    assert_eq!(saved, vec![ComposerAction::SaveSongTo("music/untitled.mid".to_string())], "the drawn field takes Enter");
}

/// Save Song As… from a sidebar showing another section: the field is the Song section's,
/// so the section switches before the field is focused.
#[test]
fn save_song_as_switches_the_sidebar_to_the_song_section_before_focusing_the_path_field() {
    let song = melody();
    let theme = EditorTheme::default();
    let mut state = sidebar_state();
    state.sidebar_section = SidebarSection::Instruments;
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let draw = |ui: &mut UIContext, state: &mut ComposerState, enter: bool| {
        render_sidebar(ui, &theme, state, SIDEBAR, Some(TABS), &context(Some(&song), enter, None))
    };

    state.request = Some(ControlRequest::FocusPathField);
    frame(&mut ui, &input, |ui| draw(ui, &mut state, false));
    assert_eq!(state.sidebar_section, SidebarSection::Song);
    assert!(ui.is_focused(path_field_id()));
    let saved = type_key(&mut ui, &mut input, KeyCode::Enter, |ui| draw(ui, &mut state, true));
    assert_eq!(saved, vec![ComposerAction::SaveSongTo("music/untitled.mid".to_string())]);
}

/// The root note and the gain a number field's frame carries, and whether it is the
/// gesture's first.
fn sample_fields(actions: &[ComposerAction]) -> (u8, f32, bool) {
    let [ComposerAction::FieldEdit { edit: SongEdit::SetInstrument { instrument, .. }, first }] = actions else {
        panic!("one frame of a field's gesture: {actions:?}");
    };
    match instrument.source {
        InstrumentSource::Sample { root_note, gain, .. } => (root_note, gain, *first),
        _ => panic!("a sample"),
    }
}

/// Where a field's label was drawn this frame; a press at the sidebar's right edge on that
/// row lands in the field's input.
fn label_position(ui: &UIContext, label: &str) -> Vec2 {
    ui.draw_list()
        .commands()
        .iter()
        .find_map(|command| match command {
            DrawCommand::TextPlaceholder { text, position, .. } if text == label => Some(*position),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the {label} row"))
}

/// A number field's gesture is the walk's to open and close: its first changed frame is a
/// discrete edit, later frames of the same field continue it, and it closes on the field's
/// commit (a scrub's release reports the input value with only the commit flag, so a host
/// waiting for the commit would see the document's own value back), when the field loses
/// focus with no scrub running, and when a frame does not draw it — so a scrub undoes as
/// one, two fields never undo as one, and no ui-wide commit flag is shared with the
/// inspector.
#[test]
fn a_number_fields_gesture_opens_discrete_continues_while_live_and_closes_on_release_focus_loss_or_an_undrawn_row() {
    let mut song = melody();
    let wav = DragPayload::Sound { path: "sounds/kick.wav".to_string() };
    let ComposerAction::Edit(add) = drop_action(Some(&song), &wav) else {
        panic!("a .wav adds an instrument");
    };
    song.apply(add, Gesture::Discrete).expect("the sample lands");
    let theme = EditorTheme::default();
    let mut state = sidebar_state();
    state.sidebar_section = SidebarSection::Instruments;
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let draw = |ui: &mut UIContext, state: &mut ComposerState| {
        render_sidebar(ui, &theme, state, SIDEBAR, Some(TABS), &context(Some(&song), false, None))
    };
    let close = |a: f32, b: f32| (a - b).abs() < 1e-3;
    let gain_id = ui::WidgetId::from_str_index("composer_instrument_gain", 0);

    frame(&mut ui, &input, |ui| draw(ui, &mut state));
    let on_gain = Vec2::new(SIDEBAR.right() - 40.0, label_position(&ui, "Gain").y);
    let on_root = Vec2::new(SIDEBAR.right() - 40.0, label_position(&ui, "Root note").y);

    // A scrub: the first moved frame opens the gesture, the next continues it, the release
    // closes it.
    let pressed = press_at(&mut ui, &mut input, on_gain, |ui| draw(ui, &mut state));
    assert!(pressed.is_empty(), "the press only arms: {pressed:?}");
    let opened = sample_fields(&move_to(&mut ui, &mut input, on_gain + Vec2::new(20.0, 0.0), |ui| draw(ui, &mut state)));
    assert!(close(opened.1, 2.0) && opened.2, "20 px at 0.05 a pixel from 1.0 opens: {opened:?}");
    assert_eq!(state.field_gesture, Some(gain_id));
    let continued = sample_fields(&move_to(&mut ui, &mut input, on_gain + Vec2::new(40.0, 0.0), |ui| draw(ui, &mut state)));
    assert!(close(continued.1, 3.0) && !continued.2, "the next frame continues: {continued:?}");
    let released = release(&mut ui, &mut input, |ui| draw(ui, &mut state));
    assert!(released.is_empty(), "the release reports no change of its own: {released:?}");
    assert_eq!(state.field_gesture, None, "and closes the gesture");

    // Arrows in the focused field: the first nudge opens, the second continues.
    click_through(&mut ui, &mut input, on_gain, |ui| draw(ui, &mut state));
    assert!(ui.is_focused(gain_id), "a click focuses the field");
    let nudged = sample_fields(&type_key(&mut ui, &mut input, KeyCode::ArrowUp, |ui| draw(ui, &mut state)));
    assert!(close(nudged.1, 1.05) && nudged.2, "{nudged:?}");
    let nudged_again = sample_fields(&type_key(&mut ui, &mut input, KeyCode::ArrowUp, |ui| draw(ui, &mut state)));
    assert!(close(nudged_again.1, 1.10) && !nudged_again.2, "{nudged_again:?}");

    // Focus moving to another field closes it: the root's first nudge is its own entry.
    click_through(&mut ui, &mut input, on_root, |ui| draw(ui, &mut state));
    assert_eq!(state.field_gesture, None, "the gain field lost focus with no scrub running");
    let root_nudged = sample_fields(&type_key(&mut ui, &mut input, KeyCode::ArrowUp, |ui| draw(ui, &mut state)));
    assert!(root_nudged.0 == 61 && root_nudged.2, "{root_nudged:?}");

    // A frame that does not draw the field closes it too.
    click_through(&mut ui, &mut input, on_gain, |ui| draw(ui, &mut state));
    let reopened = sample_fields(&type_key(&mut ui, &mut input, KeyCode::ArrowUp, |ui| draw(ui, &mut state)));
    assert!(reopened.2, "{reopened:?}");
    assert_eq!(state.field_gesture, Some(gain_id));
    state.sidebar_section = SidebarSection::Bindings;
    frame(&mut ui, &input, |ui| draw(ui, &mut state));
    assert_eq!(state.field_gesture, None, "an undrawn field's gesture is closed");
    state.sidebar_section = SidebarSection::Instruments;
    let after = sample_fields(&type_key(&mut ui, &mut input, KeyCode::ArrowUp, |ui| draw(ui, &mut state)));
    assert!(after.2, "the next nudge is a gesture of its own: {after:?}");
}

#[test]
fn a_song_without_its_own_provenance_shows_the_mark_and_its_author_commits_one_set_provenance() {
    let mut song = melody();
    song.apply(
        SongEdit::SetProvenance {
            provenance: music_midi::Provenance { author: String::new(), license: "MIT".to_string() },
        },
        Gesture::Discrete,
    )
    .expect("an empty author is an edit like any other");
    let theme = EditorTheme::default();
    let mut state = sidebar_state();
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let draw = |ui: &mut UIContext, state: &mut ComposerState, enter: bool| {
        render_sidebar(ui, &theme, state, SIDEBAR, None, &context(Some(&song), enter, None))
    };

    frame(&mut ui, &input, |ui| draw(ui, &mut state, false));
    assert!(drawn_text(&ui).contains("Song author and license: needed before Save"), "a song with no author says so");

    state.request = Some(ControlRequest::FocusSongAuthor);
    frame(&mut ui, &input, |ui| draw(ui, &mut state, false));
    assert!(ui.is_focused(song_author_field_id()), "a refused save's request focuses the Author");
    assert_eq!(state.sidebar_control, 1, "the cursor is on the Author row");
    type_key(&mut ui, &mut input, KeyCode::KeyJ, |ui| draw(ui, &mut state, false));
    let committed = type_key(&mut ui, &mut input, KeyCode::Enter, |ui| draw(ui, &mut state, true));
    assert_eq!(
        committed,
        vec![ComposerAction::Edit(SongEdit::SetProvenance {
            provenance: music_midi::Provenance { author: "j".to_string(), license: "MIT".to_string() },
        })],
        "the license rides along unchanged"
    );
}

#[test]
fn a_bindings_trigger_is_live_only_while_the_preview_sounds_and_enter_still_reaches_the_host() {
    let song = song_with_a_binding();
    let theme = EditorTheme::default();
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    for audible in [true, false] {
        let mut state = sidebar_state();
        state.sidebar_section = SidebarSection::Bindings;
        let mut sidebar = context(Some(&song), false, None);
        sidebar.preview_audible = audible;
        frame(&mut ui, &input, |ui| render_sidebar(ui, &theme, &mut state, SIDEBAR, Some(TABS), &sidebar));
        let trigger = label_position(&ui, "Trigger jump");
        let (pressed, released) = click_through(&mut ui, &mut input, trigger, |ui| {
            render_sidebar(ui, &theme, &mut state, SIDEBAR, Some(TABS), &sidebar)
        });
        let clicked: Vec<ComposerAction> = pressed.into_iter().chain(released).collect();
        let expected = if audible { vec![ComposerAction::PreviewTrigger(0)] } else { Vec::new() };
        assert_eq!(clicked, expected, "a click reaches a live Trigger only (audible: {audible})");

        state.sidebar_control = 0;
        state.request = Some(ControlRequest::ActivateSidebar);
        let entered = frame(&mut ui, &input, |ui| render_sidebar(ui, &theme, &mut state, SIDEBAR, Some(TABS), &sidebar));
        assert_eq!(entered, vec![ComposerAction::PreviewTrigger(0)], "Enter reaches the host, which says why when it is not live");
    }
}

/// Every text the last frame drew, joined by spaces as a reader would run the lines together.
fn drawn_text(ui: &UIContext) -> String {
    ui.draw_list()
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::TextPlaceholder { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn every_note_wraps_inside_a_narrow_sidebar_so_none_is_cut_at_its_edge() {
    let song = SongDocument::new_song().expect("a blank song");
    let theme = EditorTheme::default();
    let mut state = sidebar_state();
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();
    let narrow = Rect { x: 0.0, y: 0.0, width: 180.0, height: 2000.0 };
    frame(&mut ui, &input, |ui| render_sidebar(ui, &theme, &mut state, narrow, None, &context(Some(&song), false, None)));

    assert!(drawn_text(&ui).contains("Song author and license: needed before Save"));
    assert!(drawn_text(&ui).contains("Enter saves here (Save Song As…); under music/ or ai/music/"));
    for command in ui.draw_list().commands() {
        if let DrawCommand::TextPlaceholder { text, position, .. } = command {
            let right = position.x + ui.measure_text_styled(text, theme.fonts.small).x;
            let single_word = !text.contains(' ');
            assert!(right <= narrow.width || single_word, "'{text}' runs to {right} past {}", narrow.width);
        }
    }
}

#[test]
fn walking_down_to_the_last_control_of_a_short_sidebar_rings_it_wholly_inside() {
    let song = melody();
    let theme = EditorTheme::default();
    let mut state = sidebar_state();
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();
    let short = Rect { x: 0.0, y: 0.0, width: 300.0, height: 120.0 };
    let draw = |ui: &mut UIContext, state: &mut ComposerState| {
        render_sidebar(ui, &theme, state, short, None, &context(Some(&song), false, None))
    };
    frame(&mut ui, &input, |ui| draw(ui, &mut state));

    for control in 1..state.sidebar_control_count {
        state.sidebar_control = control;
        state.reveal_control = true;
        frame(&mut ui, &input, |ui| draw(ui, &mut state));
        frame(&mut ui, &input, |ui| draw(ui, &mut state));
        let ring = ui.draw_list().commands().iter().find_map(|command| match command {
            DrawCommand::RectBorder { bounds, color, .. } if *color == theme.focus_ring => Some(*bounds),
            _ => None,
        });
        let ring = ring.unwrap_or_else(|| panic!("control {control} has no ring once revealed"));
        assert!(ring.y >= short.y && ring.bottom() <= short.bottom(), "control {control}'s ring {ring:?} is not inside");
    }
}

#[test]
fn a_provenance_field_committed_with_its_own_value_is_no_edit() {
    let song = melody();
    let theme = EditorTheme::default();
    let mut state = sidebar_state();
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let draw = |ui: &mut UIContext, state: &mut ComposerState, enter: bool| {
        render_sidebar(ui, &theme, state, SIDEBAR, None, &context(Some(&song), enter, None))
    };
    state.request = Some(ControlRequest::FocusSongAuthor);
    frame(&mut ui, &input, |ui| draw(ui, &mut state, false));
    assert!(ui.is_focused(song_author_field_id()));
    let committed = type_key(&mut ui, &mut input, KeyCode::Enter, |ui| draw(ui, &mut state, true));
    assert_eq!(committed, Vec::new(), "Enter on an untouched Author changes nothing");
}

/// The melody with a part on its melody track, a SoundFont instrument and a binding.
fn song_with_a_binding() -> SongDocument {
    let mut song = melody();
    for edit in [
        SongEdit::AddPart {
            part: music_midi::Part {
                name: "lead".to_string(),
                track: music_midi::TrackSelector::Index(1),
                mode: music_midi::PartMode::Loop,
                quantize: music_midi::Quantize::Off,
            },
        },
        SongEdit::AddInstrument {
            instrument: music_midi::Instrument {
                name: "organ".to_string(),
                source: InstrumentSource::SoundFont { path: "soundfonts/cast.sf2".to_string(), bank: 0, preset: 19 },
                provenance: None,
            },
        },
        SongEdit::SetBinding {
            binding: music_midi::Binding { action: "jump".to_string(), part: "lead".to_string(), instrument: "organ".to_string() },
        },
    ] {
        song.apply(edit, Gesture::Discrete).expect("the fixture builds");
    }
    song
}

#[test]
fn left_and_right_at_the_full_width_move_the_cursor_to_each_sections_first_control_and_wrap() {
    let song = song_with_a_binding();
    let theme = EditorTheme::default();
    let mut state = sidebar_state();
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();
    let draw = |ui: &mut UIContext, state: &mut ComposerState| {
        render_sidebar(ui, &theme, state, SIDEBAR, None, &context(Some(&song), false, None))
    };
    frame(&mut ui, &input, |ui| draw(ui, &mut state));
    let starts = state.sidebar_section_starts.clone();
    assert_eq!(starts.len(), 4, "every section is drawn: {starts:?}");
    let start_of = |section: SidebarSection| starts.iter().find(|(drawn, _)| *drawn == section).map(|(_, start)| *start);

    for section in [SidebarSection::Parts, SidebarSection::Instruments, SidebarSection::Bindings, SidebarSection::Song] {
        state.step_sidebar_section(true);
        frame(&mut ui, &input, |ui| draw(ui, &mut state));
        assert_eq!((state.sidebar_section, Some(state.sidebar_control)), (section, start_of(section)));
    }
    state.step_sidebar_section(false);
    assert_eq!(Some(state.sidebar_control), start_of(SidebarSection::Bindings), "Left from the top wraps to the last section");
    frame(&mut ui, &input, |ui| draw(ui, &mut state));
    let ring = ui.draw_list().commands().iter().find_map(|command| match command {
        DrawCommand::RectBorder { bounds, color, .. } if *color == theme.focus_ring => Some(*bounds),
        _ => None,
    });
    let trigger = label_position(&ui, "Trigger jump");
    assert!(ring.is_some_and(|ring| ring.contains(trigger)), "the ring is on the Bindings section's first control");
}

#[test]
fn left_and_right_behind_tabs_change_the_tab_and_put_the_cursor_at_its_top() {
    let song = song_with_a_binding();
    let theme = EditorTheme::default();
    let mut state = sidebar_state();
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();
    frame(&mut ui, &input, |ui| render_sidebar(ui, &theme, &mut state, SIDEBAR, Some(TABS), &context(Some(&song), false, None)));
    assert_eq!(state.sidebar_section_starts.len(), 1, "one section behind the tabs");
    state.sidebar_control = 3;
    state.step_sidebar_section(false);
    assert_eq!((state.sidebar_section, state.sidebar_control), (SidebarSection::Bindings, 0));
}
