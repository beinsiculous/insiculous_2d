//! The sidebar: the song's file and the songs beside it, then what its sidecar holds —
//! parts, instruments, bindings — each block with its fields and a remove button.
//!
//! No file picker exists, so the sidebar is the picker. The path field is Save As's input:
//! its Enter saves there (the host refuses a path outside the song folders and asks before
//! replacing another song), and when it loses focus without one it shows the song's path
//! again. The Songs list is every `.mid` under `music/` and `ai/music/`; a click opens one,
//! asking first over unsaved edits. Every control is a row the arrows reach and Enter acts
//! on; each instrument edit is one `SetInstrument` carrying the whole instrument.

mod blocks;
mod walk;

use ui::{Rect, UIContext, WidgetId};

use super::state::{ComposerState, ControlRequest, SidebarSection};
use super::{ComposerAction, ComposerFocus};
use crate::song::{SongDocument, SongEdit};
use crate::theme::EditorTheme;

pub use walk::ROW_HEIGHT;
use walk::SidebarWalk;

/// The path field's widget, which Save Song As… puts the keyboard in.
pub fn path_field_id() -> WidgetId {
    WidgetId::hashed("composer_song_path")
}

/// What the sidebar reads besides its state.
pub struct SidebarContext<'a> {
    pub song: Option<&'a SongDocument>,
    pub read_only: bool,
    /// Whether Enter went down this frame: the path field's commit on that frame is a save,
    /// any other commit (a click away, Tab) only leaves the field.
    pub enter_pressed: bool,
    /// The path a never-saved song would take, asset-relative.
    pub default_path: &'a str,
    /// The song's own path, asset-relative, once it has one.
    pub song_path: Option<&'a str>,
}

/// Draw the sections the mode shows — all of them when `section_tabs` is `None`, else the
/// tabs and the chosen one — and take their clicks and keyboard activation.
pub fn render_sidebar(
    ui: &mut UIContext,
    theme: &EditorTheme,
    state: &mut ComposerState,
    bounds: Rect,
    section_tabs: Option<Rect>,
    context: &SidebarContext,
) -> Vec<ComposerAction> {
    let mut actions = Vec::new();
    if let Some(tabs) = section_tabs {
        actions.extend(render_section_tabs(ui, theme, state, tabs));
    }
    ui.rect(bounds, theme.surface_1);
    let request = match state.request {
        Some(
            ControlRequest::ActivateSidebar
            | ControlRequest::FocusPathField
            | ControlRequest::FocusSongsList,
        ) => state.request.take(),
        _ => None,
    };
    let focus_path_field = request == Some(ControlRequest::FocusPathField) && !context.read_only;
    if focus_path_field {
        // The field is the Song section's first control, at the content's top. A sidebar
        // showing another section, or scrolled past it, would focus a row the walk never
        // draws, and a focused field nothing renders owns the keyboard with nothing to
        // receive it.
        state.sidebar_section = SidebarSection::Song;
        state.sidebar_scroll.scroll_to(0.0);
        state.sidebar_control = 0;
    }
    if request == Some(ControlRequest::FocusSongsList) {
        state.sidebar_section = SidebarSection::Song;
    }
    let offset = state
        .sidebar_scroll
        .begin_frame(bounds, ui.mouse_pos(), ui.scroll_delta(), bounds.height);
    let mut walk = SidebarWalk::new(ui, theme, bounds, offset, state, request, context.read_only);
    if focus_path_field {
        let path = context.song_path.unwrap_or(context.default_path).to_string();
        walk.ui.focus_text_input(path_field_id(), &path);
    }
    let sections: Vec<SidebarSection> = match section_tabs {
        Some(_) => vec![state.sidebar_section],
        None => SidebarSection::ALL.to_vec(),
    };
    for section in sections {
        match (section, context.song) {
            (SidebarSection::Song, _) => song_section(&mut walk, state, context),
            (_, None) => {}
            (SidebarSection::Parts, Some(song)) => blocks::parts(&mut walk, song, state.active_track),
            (SidebarSection::Instruments, Some(song)) => blocks::instruments(&mut walk, song),
            (SidebarSection::Bindings, Some(song)) => blocks::bindings(&mut walk, song),
        }
    }
    let end = walk.finish(&mut actions);
    if let Some(top) = end.reveal_top {
        let offset = state.sidebar_scroll.offset();
        if top < offset {
            state.sidebar_scroll.scroll_to(top);
        } else if top + ROW_HEIGHT > offset + bounds.height {
            state.sidebar_scroll.scroll_to(top + ROW_HEIGHT - bounds.height);
        }
    }
    state.reveal_control = false;
    state.sidebar_scroll.end_frame(end.content_height, bounds.height);
    state.sidebar_control_count = end.control_count;
    state.sidebar_control = end.cursor.min(end.control_count.saturating_sub(1));
    state.field_gesture = end.field_gesture;
    actions
}

/// The section tabs below the full width; Left and Right in the sidebar switch them too.
fn render_section_tabs(
    ui: &mut UIContext,
    theme: &EditorTheme,
    state: &mut ComposerState,
    tabs: Rect,
) -> Vec<ComposerAction> {
    let mut actions = Vec::new();
    ui.rect(tabs, theme.surface_2);
    let width = tabs.width / SidebarSection::ALL.len() as f32;
    for (index, section) in SidebarSection::ALL.into_iter().enumerate() {
        let tab = Rect::new(tabs.x + index as f32 * width, tabs.y, width, tabs.height);
        let label = if section == state.sidebar_section {
            format!("▸ {}", section.label())
        } else {
            section.label().to_string()
        };
        if ui.button(("composer_section_tab", index), &label, tab) {
            state.sidebar_section = section;
            state.sidebar_control = 0;
            actions.push(ComposerAction::Focus(ComposerFocus::Sidebar));
        }
    }
    actions
}

/// The song's name and state, the path field, the Loop switch and the Songs list.
fn song_section(walk: &mut SidebarWalk, state: &ComposerState, context: &SidebarContext) {
    match context.song {
        Some(song) => {
            let name = context.song_path.unwrap_or("Untitled song");
            let mark = if song.is_dirty() { " · unsaved" } else { " · saved" };
            walk.heading(&format!("Song · {name}{mark}"));
            let shown = context.song_path.unwrap_or(context.default_path);
            if let Some(typed) = walk.field(path_field_id(), "Path", shown) {
                if context.enter_pressed {
                    walk.push(ComposerAction::SaveSongTo(typed));
                }
            }
            walk.note("Enter saves here (Save Song As…); under music/ or ai/music/");
            let looping = song.file().looping;
            let label = if looping { "Song loops: on" } else { "Song loops: off" };
            if walk.button(WidgetId::hashed("composer_song_loop"), label, true) {
                walk.push(ComposerAction::Edit(SongEdit::SetLoop { looping: !looping }));
            }
            if let crate::song::EditingPolicy::Refused { reason } = song.editing() {
                walk.warning(&format!("Not editable: {reason}"));
            }
        }
        None => {
            walk.heading("No song open");
            walk.note("File → New Song, or open one below");
        }
    }
    walk.heading("Songs");
    walk.mark_songs_list();
    if state.songs.is_empty() {
        walk.note("No .mid under music/ or ai/music/");
    }
    for (index, path) in state.songs.iter().enumerate() {
        let is_open = context.song_path == Some(path.as_str());
        let label = if is_open { format!("{path} · open") } else { path.clone() };
        let pressed = walk.button(WidgetId::from_str_index("composer_song_row", index), &label, !is_open);
        if pressed && !is_open && !context.read_only {
            walk.push(ComposerAction::OpenSong(path.clone()));
        }
    }
}
