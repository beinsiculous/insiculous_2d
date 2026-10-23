//! The sidebar: the song's file and the songs beside it, then what its sidecar holds —
//! parts, instruments, bindings — each block with its fields and a remove button.
//!
//! No file picker exists, so the sidebar is the picker. The path field is Save As's input:
//! its Enter saves there (the host refuses a path outside the song folders and asks before
//! replacing another song), and when it loses focus without one it shows the song's path
//! again. The song's own Author and License follow it — Save needs both, and the mark says
//! so from the moment a song without them opens; each commit is one `SetProvenance`. The
//! Songs list is every `.mid` under `music/` and `ai/music/`; a click opens one, asking
//! first over unsaved edits. Every control is a row the arrows reach and Enter acts on;
//! each instrument edit is one `SetInstrument` carrying the whole instrument.

mod blocks;
mod walk;

use music_midi::Provenance;
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

/// The song's Author field, which a save refused for the song's provenance puts the
/// keyboard in.
pub fn song_author_field_id() -> WidgetId {
    WidgetId::hashed("composer_song_author")
}

/// The Author field's place among the sidebar's controls: right below the path field.
const SONG_AUTHOR_CONTROL: usize = 1;

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
    /// Whether the song preview is sounding, so a binding's Trigger reaches the conductor.
    pub preview_audible: bool,
    /// Whether the bank the song's sidecar names is a file under the asset folder.
    pub sound_font_present: bool,
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
            | ControlRequest::FocusSongAuthor
            | ControlRequest::FocusSongsList,
        ) => state.request.take(),
        _ => None,
    };
    let focus_path_field = request == Some(ControlRequest::FocusPathField) && !context.read_only;
    let focus_author = request == Some(ControlRequest::FocusSongAuthor) && !context.read_only && context.song.is_some();
    if focus_path_field || focus_author {
        // Both fields are among the Song section's first controls, at the content's top. A
        // sidebar showing another section, or scrolled past them, would focus a row the
        // walk never draws, and a focused field nothing renders owns the keyboard with
        // nothing to receive it.
        state.sidebar_section = SidebarSection::Song;
        state.sidebar_scroll.scroll_to(0.0);
        state.sidebar_control = if focus_author { SONG_AUTHOR_CONTROL } else { 0 };
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
    if let (true, Some(song)) = (focus_author, context.song) {
        let author = song.file().provenance.as_ref().map(|provenance| provenance.author.clone()).unwrap_or_default();
        walk.ui.focus_text_input(song_author_field_id(), &author);
    }
    let sections: Vec<SidebarSection> = match section_tabs {
        Some(_) => vec![state.sidebar_section],
        None => SidebarSection::ALL.to_vec(),
    };
    let mut section_starts = Vec::with_capacity(sections.len());
    for section in sections {
        if section == SidebarSection::Song || context.song.is_some() {
            section_starts.push((section, walk.next_control()));
        }
        match (section, context.song) {
            (SidebarSection::Song, _) => song_section(&mut walk, state, context),
            (_, None) => {}
            (SidebarSection::Parts, Some(song)) => {
                let draft = blocks::TrackDraft { program: &mut state.new_track_program, enter_pressed: context.enter_pressed };
                blocks::parts(&mut walk, song, state.active_track, draft)
            }
            (SidebarSection::Instruments, Some(song)) => blocks::instruments(&mut walk, song),
            (SidebarSection::Bindings, Some(song)) => blocks::bindings(&mut walk, song, context.preview_audible),
        }
    }
    let end = walk.finish(&mut actions);
    state.sidebar_section_starts = section_starts;
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

/// The song's name and state, the path field, its author and license, the Loop switch and
/// the Songs list.
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
            song_provenance_fields(walk, song);
            sound_font_row(walk, song, state, context);
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

/// The background's bank: a row that cycles the project's banks, one `SetSoundFont` a
/// click, drawn disabled naming the sidecar's bank while the project has none, and a
/// warning while the bank it names is no file under the asset folder.
fn sound_font_row(walk: &mut SidebarWalk, song: &SongDocument, state: &ComposerState, context: &SidebarContext) {
    let current = &song.file().soundfont;
    let banks = &state.sound_fonts.banks;
    let label = format!("SoundFont: {current}");
    if walk.button(WidgetId::hashed("composer_song_sound_font"), &label, !banks.is_empty()) {
        let next = banks
            .iter()
            .position(|bank| bank == current)
            .map_or(0, |index| (index + 1) % banks.len());
        if let Some(bank) = banks.get(next) {
            walk.push(ComposerAction::Edit(SongEdit::SetSoundFont { path: bank.clone() }));
        }
    }
    walk.tooltip("The bank the song's background plays on: click for the next one under soundfonts/");
    if !context.sound_font_present {
        walk.warning(&format!(
            "No SoundFont at {current} — put an .sf2 under soundfonts/ and pick it on the SoundFont row"
        ));
        if state.sound_fonts.links_only {
            walk.warning("soundfonts/ holds no listed bank: banks must be files, not links");
        }
    }
}

/// The song's own author and license, as a sample instrument's are: each commit is one
/// `SetProvenance`, and the mark shows while either is empty.
fn song_provenance_fields(walk: &mut SidebarWalk, song: &SongDocument) {
    let provenance = song.file().provenance.clone().unwrap_or(Provenance {
        author: String::new(),
        license: String::new(),
    });
    let author = walk.field(song_author_field_id(), "Author", &provenance.author);
    let license = walk.field(WidgetId::hashed("composer_song_license"), "License", &provenance.license);
    if provenance.author.trim().is_empty() || provenance.license.trim().is_empty() {
        walk.warning("Song author and license: needed before Save");
    }
    if author.is_none() && license.is_none() {
        return;
    }
    let committed = Provenance {
        author: author.unwrap_or_else(|| provenance.author.clone()),
        license: license.unwrap_or_else(|| provenance.license.clone()),
    };
    // A field left as it was — Enter on it, or a press away — commits its own value back,
    // which is no edit: it would dirty the song and stop a playing preview for nothing.
    if committed != provenance {
        walk.push(ComposerAction::Edit(SongEdit::SetProvenance { provenance: committed }));
    }
}
