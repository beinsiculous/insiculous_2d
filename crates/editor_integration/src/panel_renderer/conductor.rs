//! The Conductor panel's content: the `editor::conductor` areas laid out for the panel's
//! width, the click that gives an area the keyboard, the focus ring on that area, and the
//! drop site for a `.wav`, a SoundFont or a song from Assets.
//!
//! It reads the song and returns what was asked; the host applies it
//! (`editor_game/conductor_keys.rs`), past the one refusal a play session raises. A bottom
//! panel is never the dock's narrow overlay, but the Conductor panel follows the same rule as every
//! panel: shown as one, its widgets live on the floating band.

use editor::conductor::{drop_action, layout, sidebar, transport, ConductorAction, ConductorFocus};
use editor::{DragPayload, EditorContext, PanelId, SongDocument};
use ui::{Rect, UIContext};

/// What the Conductor panel's host lends its render.
pub(crate) struct ConductorView<'a> {
    pub song: Option<&'a SongDocument>,
    /// A play session runs: every field draws its value and no click takes the keyboard.
    pub read_only: bool,
    /// Enter went down this frame, which makes the path field's commit a save.
    pub enter_pressed: bool,
    /// Where a never-saved song saves, and the song's own file, both asset-relative.
    pub default_path: &'a str,
    pub song_path: Option<&'a str>,
    /// The song preview is sounding, so a binding's Trigger reaches the conductor.
    pub preview_audible: bool,
    /// The bank the song's sidecar names is a file under the asset folder.
    pub sound_font_present: bool,
}

/// Draw the Conductor panel into `bounds` and take its input, returning what it asked for this
/// frame. See the module docs.
pub(crate) fn render_conductor(
    editor: &mut EditorContext,
    ui: &mut UIContext,
    bounds: Rect,
    view: &ConductorView,
) -> Vec<ConductorAction> {
    let narrow_overlay = editor.dock_area.narrow_overlay() == Some(PanelId::CONDUCTOR);
    if narrow_overlay {
        ui.begin_overlay_in(ui::UiLayer::Floating, bounds);
    }
    let actions = render_body(editor, ui, bounds, view);
    if narrow_overlay {
        ui.end_overlay();
    }
    actions
}

fn render_body(editor: &mut EditorContext, ui: &mut UIContext, bounds: Rect, view: &ConductorView) -> Vec<ConductorAction> {
    let mut actions = Vec::new();
    let conductor_layout = layout::compute(bounds);
    if !view.read_only && ui.mouse_just_pressed() && !ui.is_input_blocked_at(ui.mouse_pos()) {
        if let Some(area) = area_at(&conductor_layout, ui.mouse_pos()) {
            actions.push(ConductorAction::Focus(area));
        }
    }
    let theme = &editor.theme;
    let state = &mut editor.conductor;

    match view.song {
        Some(song) => actions.extend(transport::render_transport(ui, theme, state, conductor_layout.transport, song, view.read_only)),
        None => {
            ui.rect(conductor_layout.transport, theme.surface_2);
            ui.label_in_bounds_styled(
                "No song open — a song from the list, or File → Open Song...",
                conductor_layout.transport,
                ui::TextAlign::Left,
                theme.text_secondary,
                theme.fonts.small,
                8.0,
            );
        }
    }
    let context = sidebar::SidebarContext {
        song: view.song,
        read_only: view.read_only,
        enter_pressed: view.enter_pressed,
        default_path: view.default_path,
        song_path: view.song_path,
        preview_audible: view.preview_audible,
        sound_font_present: view.sound_font_present,
    };
    actions.extend(sidebar::render_sidebar(
        ui,
        theme,
        state,
        conductor_layout.sidebar,
        conductor_layout.section_tabs,
        &context,
    ));
    if let Some(focused) = area_rect(&conductor_layout, state.focus) {
        ui.rect_border(focused, theme.focus_ring, 2.0, 0.0);
    }

    let accepts = |payload: &DragPayload| {
        matches!(payload, DragPayload::Sound { .. } | DragPayload::Song { .. } | DragPayload::SoundFont { .. })
    };
    if editor.drag_drop.dragging_payload().is_some_and(accepts) && bounds.contains(ui.mouse_pos()) {
        ui.rect_border(bounds, editor.theme.accent_blue, 2.0, 0.0);
    }
    if let Some((payload, _)) = editor.drag_drop.take_drop_in(bounds) {
        actions.push(drop_action(view.song, &payload));
    }
    actions
}

/// The area of the Conductor panel under `point`, if any: the transport, or the sidebar with its
/// section tabs.
fn area_at(conductor_layout: &layout::ConductorLayout, point: glam::Vec2) -> Option<ConductorFocus> {
    if conductor_layout.transport.contains(point) {
        return Some(ConductorFocus::Transport);
    }
    let on_tabs = conductor_layout.section_tabs.is_some_and(|tabs| tabs.contains(point));
    (on_tabs || conductor_layout.sidebar.contains(point)).then_some(ConductorFocus::Sidebar)
}

/// The rectangle the focus ring goes round for `focus`.
fn area_rect(conductor_layout: &layout::ConductorLayout, focus: ConductorFocus) -> Option<Rect> {
    match focus {
        ConductorFocus::None => None,
        ConductorFocus::Transport => Some(conductor_layout.transport),
        ConductorFocus::Sidebar => Some(conductor_layout.sidebar),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor::conductor::layout;
    use glam::Vec2;

    const PANEL: Rect = Rect { x: 0.0, y: 400.0, width: 1280.0, height: 260.0 };

    /// A song read from MIDI bytes: the music crate's Napolitan fixture, on a sidecar of its own.
    fn napolitan_song() -> SongDocument {
        const NAPOLITAN: &[u8] = include_bytes!("../../../music_midi/fixtures/dansenapolitaine.mid");
        SongDocument::from_smf(NAPOLITAN, editor::song::default_song_file()).expect("the fixture reads")
    }

    fn view(song: &SongDocument, read_only: bool) -> ConductorView<'_> {
        ConductorView {
            song: Some(song),
            read_only,
            enter_pressed: false,
            default_path: "music/untitled.mid",
            song_path: None,
            preview_audible: false,
            sound_font_present: true,
        }
    }

    /// Render one frame with the left button pressed at `point`.
    fn press(editor: &mut EditorContext, song: &SongDocument, read_only: bool, point: Vec2) -> Vec<ConductorAction> {
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();
        input.mouse_mut().update_position(point.x, point.y);
        input.mouse_mut().handle_button_press(input::prelude::MouseButton::Left);
        ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
        let actions = render_conductor(editor, &mut ui, PANEL, &view(song, read_only));
        ui.end_frame();
        actions
    }

    #[test]
    fn a_press_gives_its_area_the_keyboard_but_takes_none_during_a_play_session() {
        let song = napolitan_song();
        let mut editor = EditorContext::new();
        editor.conductor.follow(0, Some(&song));
        let conductor_layout = layout::compute(PANEL);
        let sidebar = conductor_layout.sidebar.center();
        let transport = Vec2::new(conductor_layout.transport.right() - 20.0, conductor_layout.transport.center().y);

        let editing = press(&mut editor, &song, false, sidebar);
        assert!(editing.contains(&ConductorAction::Focus(ConductorFocus::Sidebar)), "{editing:?}");
        let on_transport = press(&mut editor, &song, false, transport);
        assert!(on_transport.contains(&ConductorAction::Focus(ConductorFocus::Transport)), "{on_transport:?}");

        let playing = press(&mut editor, &song, true, sidebar);
        assert!(
            playing.iter().all(|action| !matches!(action, ConductorAction::Focus(_))),
            "read-only: no focus: {playing:?}"
        );
    }

    #[test]
    fn a_wav_released_over_the_panel_asks_for_a_sample_instrument() {
        let song = napolitan_song();
        let mut editor = EditorContext::new();
        editor.conductor.follow(0, Some(&song));
        let centre = PANEL.center();
        editor.drag_drop.arm(DragPayload::Sound { path: "sounds/kick.wav".to_string() }, Vec2::ZERO);
        editor.drag_drop.begin_frame(Vec2::new(50.0, 50.0), true, false);
        editor.drag_drop.begin_frame(centre, false, true);

        let mut ui = UIContext::new();
        let input = input::InputHandler::new();
        ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
        let actions = render_conductor(&mut editor, &mut ui, PANEL, &view(&song, false));
        ui.end_frame();
        assert!(actions.iter().any(|action| matches!(
            action,
            ConductorAction::Edit(editor::SongEdit::AddInstrument { instrument }) if instrument.name == "kick"
        )), "{actions:?}");
    }
}
