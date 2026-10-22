//! The transport: Play and Stop act on the preview only while the editor holds the music,
//! and draw disabled while a play session does.

use ui::{DrawCommand, Rect, UIContext};

use super::state::ComposerState;
use super::test_support::melody;
use super::transport::render_transport;
use super::ComposerAction;
use crate::test_support::{click_through, frame};
use crate::theme::EditorTheme;

const TRANSPORT: Rect = Rect { x: 0.0, y: 0.0, width: 900.0, height: 28.0 };

#[test]
fn play_asks_for_the_preview_while_editing_and_nothing_during_a_play_session() {
    let song = melody();
    let theme = EditorTheme::default();
    for (read_only, expected) in [(false, vec![ComposerAction::PreviewPlayStop]), (true, Vec::new())] {
        let mut state = ComposerState::default();
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();
        frame(&mut ui, &input, |ui| render_transport(ui, &theme, &mut state, TRANSPORT, &song, read_only));
        let play = ui
            .draw_list()
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::TextPlaceholder { text, position, .. } if text == "Play" => Some(*position),
                _ => None,
            })
            .expect("the Play button");
        let (pressed, released) = click_through(&mut ui, &mut input, play, |ui| {
            render_transport(ui, &theme, &mut state, TRANSPORT, &song, read_only)
        });
        let actions: Vec<ComposerAction> = pressed.into_iter().chain(released).collect();
        assert_eq!(actions, expected, "read-only: {read_only}");
    }
}
