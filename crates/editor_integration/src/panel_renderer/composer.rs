//! The composer panel's content: the `editor::composer` areas laid out for the panel's
//! width, the click that gives an area the keyboard, the focus ring on that area, and the
//! drop site for a `.wav`, a SoundFont or a song from Assets.
//!
//! It reads the song and the drag's preview and returns what was asked; the host applies
//! it (`editor_game/composer_keys.rs`), past the one refusal a play session raises. A
//! bottom panel is never the dock's narrow overlay, but the composer follows the same rule
//! as every panel: shown as one, its widgets live on the floating band.

use editor::composer::{
    drop_action, layout, note_list, piano_roll, sidebar, transport, ComposerAction, ComposerFocus,
    MovePreview, RollPoint,
};
use editor::{DragPayload, EditorContext, Modifiers, PanelId, SongDocument};
use ui::{Rect, UIContext};

/// What the composer's host lends its render.
pub(crate) struct ComposerView<'a> {
    pub song: Option<&'a SongDocument>,
    pub preview: Option<&'a MovePreview>,
    /// A play session runs: every field draws its value and no click takes the keyboard.
    pub read_only: bool,
    pub drag_live: bool,
    pub modifiers: Modifiers,
    /// Enter went down this frame, which makes the path field's commit a save.
    pub enter_pressed: bool,
    /// Where a never-saved song saves, and the song's own file, both asset-relative.
    pub default_path: &'a str,
    pub song_path: Option<&'a str>,
}

/// What the composer's render asked for this frame.
#[derive(Debug, Default)]
pub(crate) struct ComposerOutput {
    pub actions: Vec<ComposerAction>,
    /// The roll's tick and key under the pointer, which a live drag follows.
    pub pointer: Option<RollPoint>,
}

/// Draw the composer into `bounds` and take its input. See the module docs.
pub(crate) fn render_composer(
    editor: &mut EditorContext,
    ui: &mut UIContext,
    bounds: Rect,
    view: &ComposerView,
) -> ComposerOutput {
    let narrow_overlay = editor.dock_area.narrow_overlay() == Some(PanelId::COMPOSER);
    if narrow_overlay {
        ui.begin_overlay_in(ui::UiLayer::Floating, bounds);
    }
    let output = render_body(editor, ui, bounds, view);
    if narrow_overlay {
        ui.end_overlay();
    }
    output
}

fn render_body(editor: &mut EditorContext, ui: &mut UIContext, bounds: Rect, view: &ComposerView) -> ComposerOutput {
    let mut output = ComposerOutput::default();
    let ticks_per_quarter = view.song.map_or(1, |song| song.data().ticks_per_quarter());
    let first_pass = layout::compute(bounds, &editor.composer, ticks_per_quarter);
    if let (Some(roll), Some(song)) = (first_pass.roll, view.song) {
        piano_roll::scroll_roll(&mut editor.composer, ui, roll.bounds, song, view.modifiers);
    }
    let composer_layout = layout::compute(bounds, &editor.composer, ticks_per_quarter);
    if !view.read_only && ui.mouse_just_pressed() && !ui.is_input_blocked_at(ui.mouse_pos()) {
        if let Some(area) = area_at(&composer_layout, ui.mouse_pos()) {
            output.actions.push(ComposerAction::Focus(area));
        }
    }
    let theme = &editor.theme;
    let state = &mut editor.composer;

    match view.song {
        Some(song) => output.actions.extend(transport::render_transport(ui, theme, state, composer_layout.transport, song)),
        None => {
            ui.rect(composer_layout.transport, theme.surface_2);
            ui.label_in_bounds_styled(
                "No song open — File → New Song, or a song from the list",
                composer_layout.transport,
                ui::TextAlign::Left,
                theme.text_secondary,
                theme.fonts.small,
                8.0,
            );
        }
    }
    if let Some(tabs) = composer_layout.area_tabs {
        output.actions.extend(render_area_tabs(ui, theme, state, tabs));
    }
    if let (Some(roll), Some(song)) = (composer_layout.roll, view.song) {
        let context = piano_roll::RollContext {
            song,
            preview: view.preview,
            read_only: view.read_only,
            modifiers: view.modifiers,
            drag_live: view.drag_live,
        };
        let roll_output = piano_roll::render_piano_roll(ui, theme, state, &roll, &context);
        output.actions.extend(roll_output.actions);
        output.pointer = roll_output.pointer;
    }
    if let (Some(list), Some(song)) = (composer_layout.note_list, view.song) {
        let context = note_list::NoteListContext { song, read_only: view.read_only, modifiers: view.modifiers };
        output.actions.extend(note_list::render_note_list(ui, theme, state, list, &context));
    }
    if let Some(side) = composer_layout.sidebar {
        let context = sidebar::SidebarContext {
            song: view.song,
            read_only: view.read_only,
            enter_pressed: view.enter_pressed,
            default_path: view.default_path,
            song_path: view.song_path,
        };
        output
            .actions
            .extend(sidebar::render_sidebar(ui, theme, state, side, composer_layout.section_tabs, &context));
    }
    if let Some(focused) = area_rect(&composer_layout, state.focus) {
        ui.rect_border(focused, theme.focus_ring, 2.0, 0.0);
    }

    let accepts = |payload: &DragPayload| {
        matches!(payload, DragPayload::Sound { .. } | DragPayload::Song { .. } | DragPayload::SoundFont { .. })
    };
    if editor.drag_drop.dragging_payload().is_some_and(accepts) && bounds.contains(ui.mouse_pos()) {
        ui.rect_border(bounds, editor.theme.accent_blue, 2.0, 0.0);
    }
    if let Some((payload, _)) = editor.drag_drop.take_drop_in(bounds) {
        output.actions.push(drop_action(view.song, &payload));
    }
    output
}

/// The area of the composer under `point`, if any.
fn area_at(composer_layout: &layout::ComposerLayout, point: glam::Vec2) -> Option<ComposerFocus> {
    let areas = [
        (Some(composer_layout.transport), ComposerFocus::Transport),
        (composer_layout.roll.map(|roll| roll.bounds), ComposerFocus::PianoRoll),
        (composer_layout.note_list, ComposerFocus::NoteList),
        (composer_layout.sidebar, ComposerFocus::Sidebar),
        (composer_layout.section_tabs, ComposerFocus::Sidebar),
    ];
    areas
        .into_iter()
        .find(|(rect, _)| rect.is_some_and(|rect| rect.contains(point)))
        .map(|(_, area)| area)
}

/// The rectangle the focus ring goes round for `focus`, when that area is shown.
fn area_rect(composer_layout: &layout::ComposerLayout, focus: ComposerFocus) -> Option<Rect> {
    match focus {
        ComposerFocus::None => None,
        ComposerFocus::Transport => Some(composer_layout.transport),
        ComposerFocus::PianoRoll => composer_layout.roll.map(|roll| roll.bounds),
        ComposerFocus::NoteList => composer_layout.note_list,
        ComposerFocus::Sidebar => composer_layout.sidebar,
    }
}

/// The area tabs of the narrowest mode: one area shown at a time.
fn render_area_tabs(
    ui: &mut UIContext,
    theme: &editor::EditorTheme,
    state: &mut editor::composer::ComposerState,
    tabs: Rect,
) -> Vec<ComposerAction> {
    let mut actions = Vec::new();
    ui.rect(tabs, theme.surface_2);
    let areas = [ComposerFocus::PianoRoll, ComposerFocus::NoteList, ComposerFocus::Sidebar];
    let width = tabs.width / areas.len() as f32;
    for (index, area) in areas.into_iter().enumerate() {
        let tab = Rect::new(tabs.x + index as f32 * width, tabs.y, width, tabs.height);
        let label = if state.tabs_area == area {
            format!("▸ {}", area.label())
        } else {
            area.label().to_string()
        };
        if ui.button(("composer_area_tab", index), &label, tab) {
            state.tabs_area = area;
            actions.push(ComposerAction::Focus(area));
        }
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor::composer::layout;
    use glam::Vec2;

    const PANEL: Rect = Rect { x: 0.0, y: 400.0, width: 1280.0, height: 260.0 };

    fn view(song: &SongDocument, read_only: bool) -> ComposerView<'_> {
        ComposerView {
            song: Some(song),
            preview: None,
            read_only,
            drag_live: false,
            modifiers: Modifiers::default(),
            enter_pressed: false,
            default_path: "music/untitled.mid",
            song_path: None,
        }
    }

    /// Render one frame with the left button pressed at `point`.
    fn press(editor: &mut EditorContext, song: &SongDocument, read_only: bool, point: Vec2) -> ComposerOutput {
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();
        input.mouse_mut().update_position(point.x, point.y);
        input.mouse_mut().handle_button_press(input::prelude::MouseButton::Left);
        ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
        let output = render_composer(editor, &mut ui, PANEL, &view(song, read_only));
        ui.end_frame();
        output
    }

    #[test]
    fn a_press_gives_its_area_the_keyboard_but_takes_none_during_a_play_session() {
        let song = SongDocument::new_song().expect("a blank song");
        let mut editor = EditorContext::new();
        editor.composer.follow(0, Some(&song));
        let roll = layout::compute(PANEL, &editor.composer, 480).roll.expect("a roll").grid.center();

        let editing = press(&mut editor, &song, false, roll);
        assert!(editing.actions.contains(&ComposerAction::Focus(ComposerFocus::PianoRoll)), "{:?}", editing.actions);

        let playing = press(&mut editor, &song, true, roll);
        assert!(
            playing.actions.iter().all(|action| !matches!(action, ComposerAction::Focus(_) | ComposerAction::Select(_))),
            "read-only: no focus, no selection: {:?}",
            playing.actions
        );
    }

    #[test]
    fn a_wav_released_over_the_panel_asks_for_a_sample_instrument() {
        let song = SongDocument::new_song().expect("a blank song");
        let mut editor = EditorContext::new();
        editor.composer.follow(0, Some(&song));
        let centre = PANEL.center();
        editor.drag_drop.arm(DragPayload::Sound { path: "sounds/kick.wav".to_string() }, Vec2::ZERO);
        editor.drag_drop.begin_frame(Vec2::new(50.0, 50.0), true, false);
        editor.drag_drop.begin_frame(centre, false, true);

        let mut ui = UIContext::new();
        let input = input::InputHandler::new();
        ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
        let output = render_composer(&mut editor, &mut ui, PANEL, &view(&song, false));
        ui.end_frame();
        assert!(output.actions.iter().any(|action| matches!(
            action,
            ComposerAction::Edit(editor::SongEdit::AddInstrument { instrument }) if instrument.name == "kick"
        )), "{:?}", output.actions);
    }
}
