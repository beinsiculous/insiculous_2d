//! The Song section's SoundFont row and the Parts section's Add-track row: the row cycles
//! the project's banks one edit a click, draws disabled naming the sidecar's bank while the
//! project has none, and warns while that bank is no file; the Add-track row adds a track
//! on Enter, playing the program its number field holds.

use glam::Vec2;
use input::prelude::KeyCode;
use ui::{DrawCommand, Rect, UIContext};

use super::sidebar::{render_sidebar, SidebarContext};
use super::sound_fonts::SoundFontScan;
use super::state::{ComposerFocus, ComposerState, ControlRequest, SidebarSection};
use super::test_support::melody;
use super::ComposerAction;
use crate::song::{Gesture, SongDocument, SongEdit};
use crate::test_support::{click_through, frame, type_key};
use crate::theme::EditorTheme;

const SIDEBAR: Rect = Rect { x: 0.0, y: 0.0, width: 320.0, height: 2000.0 };
const TABS: Rect = Rect { x: 0.0, y: 2000.0, width: 320.0, height: 24.0 };

fn context(song: &SongDocument, enter_pressed: bool, sound_font_present: bool) -> SidebarContext<'_> {
    SidebarContext {
        song: Some(song),
        read_only: false,
        enter_pressed,
        default_path: "music/untitled.mid",
        song_path: None,
        preview_audible: false,
        sound_font_present,
    }
}

fn state_with(banks: &[&str], links_only: bool) -> ComposerState {
    let mut state = ComposerState::default();
    state.focus = ComposerFocus::Sidebar;
    state.songs_scanned = true;
    state.sound_fonts = SoundFontScan {
        banks: banks.iter().map(|bank| bank.to_string()).collect(),
        links_only,
    };
    state
}

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

fn text_position(ui: &UIContext, wanted: &str) -> Vec2 {
    ui.draw_list()
        .commands()
        .iter()
        .find_map(|command| match command {
            DrawCommand::TextPlaceholder { text, position, .. } if text == wanted => Some(*position),
            _ => None,
        })
        .unwrap_or_else(|| panic!("'{wanted}' was drawn"))
}

/// A melody whose background is on `bank`.
fn melody_on(bank: &str) -> SongDocument {
    let mut song = melody();
    song.apply(SongEdit::SetSoundFont { path: bank.to_string() }, Gesture::Discrete)
        .expect("sets the bank");
    song
}

#[test]
fn a_click_on_the_row_picks_the_next_bank_as_one_undoable_edit() {
    let mut song = melody_on("soundfonts/a.sf2");
    let theme = EditorTheme::default();
    let mut state = state_with(&["soundfonts/a.sf2", "soundfonts/b.sf2"], false);
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let label = "SoundFont: soundfonts/a.sf2";

    frame(&mut ui, &input, |ui| render_sidebar(ui, &theme, &mut state, SIDEBAR, None, &context(&song, false, true)));
    assert!(!drawn_text(&ui).contains("No SoundFont"), "a bank that exists warns of nothing");
    let at = text_position(&ui, label) + Vec2::new(4.0, 4.0);
    let (_, clicked) = click_through(&mut ui, &mut input, at, |ui| {
        render_sidebar(ui, &theme, &mut state, SIDEBAR, None, &context(&song, false, true))
    });

    let picked = SongEdit::SetSoundFont { path: "soundfonts/b.sf2".to_string() };
    assert_eq!(clicked, vec![ComposerAction::Edit(picked.clone())]);
    let entries = song.history().undo_count();
    song.apply(picked, Gesture::Discrete).expect("sets the bank");
    assert_eq!(song.history().undo_count(), entries + 1);
    song.undo().expect("undoes");
    assert_eq!(song.file().soundfont, "soundfonts/a.sf2");
}

#[test]
fn with_no_bank_the_row_is_disabled_naming_the_sidecars_and_the_warning_says_how_to_add_one() {
    let song = melody_on("soundfonts/cast.sf2");
    let theme = EditorTheme::default();
    let mut state = state_with(&[], false);
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    frame(&mut ui, &input, |ui| render_sidebar(ui, &theme, &mut state, SIDEBAR, None, &context(&song, false, false)));
    let text = drawn_text(&ui);
    assert!(
        text.contains("No SoundFont at soundfonts/cast.sf2") && text.contains("put an .sf2 under soundfonts/"),
        "{text}"
    );
    assert!(!text.contains("not links"));
    let at = text_position(&ui, "SoundFont: soundfonts/cast.sf2") + Vec2::new(4.0, 4.0);
    let (_, clicked) = click_through(&mut ui, &mut input, at, |ui| {
        render_sidebar(ui, &theme, &mut state, SIDEBAR, None, &context(&song, false, false))
    });
    assert!(clicked.is_empty(), "nothing to cycle over: {clicked:?}");

    let mut linked = state_with(&[], true);
    frame(&mut ui, &input, |ui| render_sidebar(ui, &theme, &mut linked, SIDEBAR, None, &context(&song, false, false)));
    assert!(drawn_text(&ui).contains("banks must be files, not links"));
}

#[test]
fn the_warning_clears_once_the_bank_it_names_is_there() {
    let song = melody_on("soundfonts/a.sf2");
    let theme = EditorTheme::default();
    let mut state = state_with(&[], false);
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();
    frame(&mut ui, &input, |ui| render_sidebar(ui, &theme, &mut state, SIDEBAR, None, &context(&song, false, false)));
    assert!(drawn_text(&ui).contains("No SoundFont at soundfonts/a.sf2"));

    state.sound_fonts.banks = vec!["soundfonts/a.sf2".to_string()];
    frame(&mut ui, &input, |ui| render_sidebar(ui, &theme, &mut state, SIDEBAR, None, &context(&song, false, true)));
    assert!(!drawn_text(&ui).contains("No SoundFont"));
}

#[test]
fn enter_in_the_add_track_name_adds_one_track_playing_the_programs_number() {
    let song = melody();
    let theme = EditorTheme::default();
    let mut state = state_with(&[], false);
    state.sidebar_section = SidebarSection::Parts;
    state.new_track_program = 57;
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let draw = |ui: &mut UIContext, state: &mut ComposerState, enter: bool| {
        render_sidebar(ui, &theme, state, SIDEBAR, Some(TABS), &context(&song, enter, true))
    };

    frame(&mut ui, &input, |ui| draw(ui, &mut state, false));
    assert!(drawn_text(&ui).contains("Add track"));
    // No part and no shown track: the program field, then the name field.
    state.sidebar_control = 1;
    state.request = Some(ControlRequest::ActivateSidebar);
    frame(&mut ui, &input, |ui| draw(ui, &mut state, false));
    type_key(&mut ui, &mut input, KeyCode::KeyB, |ui| draw(ui, &mut state, false));
    let added = type_key(&mut ui, &mut input, KeyCode::Enter, |ui| draw(ui, &mut state, true));

    assert_eq!(
        added,
        vec![ComposerAction::Edit(SongEdit::AddTrack { name: "b".to_string(), channel: None, program: 57 })]
    );
}
