//! The composer's host: who holds the keyboard, what its key table's rows do, what its
//! render's actions do, the File menu's song items, and the frame-start rules that take
//! the keyboard away.
//!
//! Every change to the song goes through [`EditorGame::apply_composer_edit`], past the one
//! refusal a play session raises, into `SongDocument::apply`. Focus is explicit: a click
//! in the panel, Ctrl+Shift+M or a File item gives it; a press elsewhere in the dock,
//! hiding or collapsing the panel, Reset Layout, entering a play session and Escape take
//! it away — and a menu click does not, so Edit → Undo after a click in the sidebar undoes
//! the song.

use std::path::Path;

use editor::composer::{
    self, ComposerAction, ComposerFocus, ComposerKeyAction, ComposerKeyRoute, ComposerState,
    ControlRequest, Direction, SidebarSection,
};
use editor::{EditorAction, Gesture, Modifiers, PanelId, SongEdit};
use engine_core::contexts::GameContext;
use engine_core::Game;
use winit::keyboard::KeyCode;

use super::panel_press::PanelPress;
use super::scene_confirm::PendingSceneAction;
use super::shortcuts::KeyRoute;
use super::EditorGame;
use crate::panel_renderer::composer::{render_composer, ComposerView};

impl<G: Game> EditorGame<G> {
    pub(super) fn composer_holds_focus(&self) -> bool {
        self.editor.composer.focus.is_held()
    }

    /// Why no composer edit may land now, if one may not: the song cannot change during
    /// a play session (Playing or Paused). The host's song refusal, shared with the API.
    pub(super) fn composer_write_refusal(&self) -> Option<&'static str> {
        self.song_change_refusal()
    }

    /// One edit through the document. A refused edit lands nothing and says why on the
    /// status bar; returns whether it landed.
    pub(super) fn apply_composer_edit(&mut self, edit: SongEdit, gesture: Gesture) -> bool {
        if let Some(refusal) = self.composer_write_refusal() {
            self.editor.status_bar.show_error(refusal);
            return false;
        }
        let Some(song) = self.song.as_mut() else {
            self.editor.status_bar.show_error("No song is open");
            return false;
        };
        match song.apply(edit, gesture) {
            Ok(()) => {
                self.sync_dirty_mirror();
                true
            }
            Err(error) => {
                self.editor.status_bar.show_error(format!("Song edit refused: {error}"));
                false
            }
        }
    }

    /// Take the keyboard from the composer.
    pub(super) fn leave_composer(&mut self) {
        self.editor.composer.clear_focus();
    }

    /// Show the composer, expanded, and give `area` the keyboard — unless a play session
    /// runs, when the panel is read-only and keys belong to the game.
    pub(super) fn show_composer(&mut self, area: ComposerFocus) {
        self.editor.dock_area.set_panel_visible(PanelId::COMPOSER, true);
        self.editor.dock_area.set_panel_collapsed(PanelId::COMPOSER, false);
        if self.composer_write_refusal().is_none() {
            self.editor.composer.focus = area;
        }
    }

    /// Reset the composer's state when the song it was built against was replaced.
    pub(super) fn follow_song(&mut self) {
        self.editor.composer.follow(self.song_generation, self.song.as_ref());
    }

    /// Frame start, before any chrome handles this frame's press: follow the song, then
    /// drop the keyboard when the panel is gone or a press of either button
    /// landed elsewhere in the dock by `panel_press`'s rule — a narrow-mode overlay over the
    /// panel included, never a menu, whose dropdown is chrome, nor anything under a dialog.
    pub(super) fn settle_composer(&mut self, ui: &ui::UIContext) {
        self.follow_song();
        let Some(press) = self.panel_press(ui, PanelId::COMPOSER) else {
            self.leave_composer();
            // Once per close: cleared every hidden frame, the list the emitter's song rule
            // shares would be scanned again each frame the composer stays hidden.
            if self.editor.composer.panel_drawn {
                self.editor.composer.songs_scanned = false;
                self.editor.composer.panel_drawn = false;
            }
            return;
        };
        if press == PanelPress::Elsewhere && self.composer_holds_focus() {
            self.leave_composer();
        }
    }

    /// How a key goes while the composer holds the keyboard; `None` hands it to the
    /// editor's own table. A key neither table binds is swallowed, never the game's.
    pub(super) fn composer_key_route(&self, key: KeyCode, modifiers: Modifiers) -> Option<KeyRoute> {
        if !self.composer_holds_focus() {
            return None;
        }
        match composer::resolve_composer_key(key, modifiers) {
            ComposerKeyRoute::Composer(action) => Some(KeyRoute::Composer(action)),
            ComposerKeyRoute::Swallowed => Some(KeyRoute::Consumed),
            ComposerKeyRoute::Editor => {
                let bound = self.editor.input_mapping.resolve(key, modifiers.ctrl, modifiers.shift);
                bound.is_none().then_some(KeyRoute::Consumed)
            }
        }
    }

    /// What one row of the composer's key table does.
    pub(super) fn run_composer_key(&mut self, action: ComposerKeyAction) {
        let state = &mut self.editor.composer;
        match action {
            ComposerKeyAction::Escape => self.leave_composer(),
            ComposerKeyAction::NextArea => state.focus = state.focus.next(),
            ComposerKeyAction::PreviousArea => state.focus = state.focus.previous(),
            ComposerKeyAction::MoveCursor(direction) => {
                move_control_cursor(state, direction);
                state.reveal_control = state.focus == ComposerFocus::Sidebar;
            }
            ComposerKeyAction::Activate => match state.focus {
                ComposerFocus::Transport => state.request = Some(ControlRequest::ActivateTransport),
                ComposerFocus::Sidebar => state.request = Some(ControlRequest::ActivateSidebar),
                ComposerFocus::None => {}
            },
            ComposerKeyAction::Undo => self.undo_song_with_feedback(),
            ComposerKeyAction::Redo => self.redo_song_with_feedback(),
            ComposerKeyAction::PreviewToggle => self.preview_play_stop(),
        }
    }

    /// Show `track` in the transport: the track "Add a part" builds on.
    fn set_active_track(&mut self, track: usize) {
        let Some(song) = self.song.as_ref() else {
            return;
        };
        if song.data().tracks().get(track).and_then(|song_track| song_track.channel()).is_none() {
            self.editor.status_bar.show_error("That track holds timing only; it has no notes to show");
            return;
        }
        if self.editor.composer.active_track != Some(track) {
            self.editor.composer.active_track = Some(track);
            let label = composer::track_label(song, track);
            self.editor.status_bar.show_message(format!("Showing {label}"));
        }
    }

    /// The song's undo, for the composer's Ctrl+Z and for Edit → Undo while it holds the
    /// keyboard; the status bar names the document.
    pub(super) fn undo_song_with_feedback(&mut self) {
        self.song_history_step(true);
    }

    pub(super) fn redo_song_with_feedback(&mut self) {
        self.song_history_step(false);
    }

    fn song_history_step(&mut self, undo: bool) {
        if let Some(refusal) = self.composer_write_refusal() {
            self.editor.status_bar.show_error(refusal);
            return;
        }
        let Some(song) = self.song.as_mut() else {
            self.editor.status_bar.show_message("No song is open");
            return;
        };
        let (verb, result) = if undo {
            ("Undid", song.undo())
        } else {
            ("Redid", song.redo())
        };
        match result {
            Some(name) => self.editor.status_bar.show_message(format!("{verb} {name} (song)")),
            None => self.editor.status_bar.show_message(format!("Nothing to {} (song)", if undo { "undo" } else { "redo" })),
        }
        self.sync_dirty_mirror();
    }

    /// The File menu's song items, `FocusComposer` and the preview toggle.
    pub(super) fn dispatch_song_action(&mut self, action: EditorAction) {
        match action {
            EditorAction::OpenSong => {
                self.editor.composer.sidebar_section = SidebarSection::Song;
                self.editor.composer.rescan_songs(&self.asset_base);
                self.show_composer(ComposerFocus::Sidebar);
                self.editor.composer.request = Some(ControlRequest::FocusSongsList);
            }
            EditorAction::SaveSong => {
                let result = self.save_song(None).map(drop);
                let refused = result.is_err();
                self.report_song_failure("Song save", result);
                if refused {
                    self.reveal_missing_song_provenance();
                }
            }
            EditorAction::SaveSongAs if self.song.is_some() => {
                self.editor.composer.sidebar_section = SidebarSection::Song;
                self.show_composer(ComposerFocus::Sidebar);
                self.editor.composer.request = Some(ControlRequest::FocusPathField);
            }
            EditorAction::SaveSongAs => self.editor.status_bar.show_error("No song is open to save"),
            EditorAction::CloseSong => {
                if self.request_document_action(PendingSceneAction::CloseSong) {
                    self.perform_song_action(PendingSceneAction::CloseSong);
                }
            }
            EditorAction::FocusComposer => self.show_composer(ComposerFocus::Sidebar),
            EditorAction::SongPlayStop => self.preview_play_stop(),
            other => log::error!("{other:?} is not a song action"),
        }
    }

    /// The composer panel's frame: render it with the song, then run what it asked for.
    pub(super) fn render_composer_panel(&mut self, ctx: &mut GameContext, bounds: common::Rect) {
        self.follow_song();
        // A score adopted this frame may have fewer tracks than the one shown; the render
        // must not offer "Add a part" on an index that is gone.
        self.editor.composer.reconcile_active_track(self.song.as_ref());
        if self.editor.composer.drawn() {
            self.editor.composer.rescan_songs(&self.asset_base);
        }
        let default_path = self.asset_relative(&self.default_song_path());
        let song_path = self.song.as_ref().and_then(|song| song.path()).map(|path| self.asset_relative(path));
        let keyboard = ctx.input.keyboard();
        let enter_pressed = keyboard.is_key_just_pressed(KeyCode::Enter) || keyboard.is_key_just_pressed(KeyCode::NumpadEnter);
        let view = ComposerView {
            song: self.song.as_ref(),
            read_only: self.composer_write_refusal().is_some(),
            enter_pressed,
            default_path: &default_path,
            song_path: song_path.as_deref(),
            preview_audible: self.song_preview.is_audible(),
            sound_font_present: self.song.as_ref().is_none_or(|song| {
                editor::external_tools::sound_font_exists(&self.asset_base, &song.file().soundfont)
            }),
        };
        let actions = render_composer(&mut self.editor, ctx.ui, bounds, &view);
        self.run_composer_actions(actions);
    }

    /// Run what the composer's render asked for, in order.
    pub(super) fn run_composer_actions(&mut self, actions: Vec<ComposerAction>) {
        for action in actions {
            match action {
                ComposerAction::Focus(area) if self.composer_write_refusal().is_none() => {
                    self.editor.composer.focus = area;
                }
                ComposerAction::Focus(_) => {}
                ComposerAction::Edit(edit) => {
                    self.apply_composer_edit(edit, Gesture::Discrete);
                }
                ComposerAction::FieldEdit { edit, first } => {
                    if first {
                        if let Some(song) = self.song.as_mut() {
                            song.break_merge();
                        }
                    }
                    self.apply_composer_edit(edit, Gesture::Continuing);
                }
                ComposerAction::SetActiveTrack(track) => self.set_active_track(track),
                ComposerAction::OpenSong(relative) => {
                    let pending = PendingSceneAction::OpenSong(self.resolve_asset_path(Path::new(&relative)));
                    if self.request_document_action(pending.clone()) {
                        self.perform_song_action(pending);
                    }
                    self.show_composer(ComposerFocus::Sidebar);
                }
                ComposerAction::SaveSongTo(typed) => self.save_song_from_field(&typed),
                ComposerAction::PreviewPlayStop => self.preview_play_stop(),
                ComposerAction::PreviewTrigger(index) => self.preview_trigger_binding(index),
                ComposerAction::Refused(message) => self.editor.status_bar.show_error(message),
            }
        }
    }

    /// The path field's Enter: Save As to what it holds, relative to the asset folder. A
    /// path outside the song folders or not a `.mid` is refused by the save; another
    /// song's file parks behind the overwrite prompt.
    pub(super) fn save_song_from_field(&mut self, typed: &str) {
        let typed = typed.trim();
        if typed.is_empty() {
            self.editor.status_bar.show_error("Type a path under music/ or ai/music/ to save the song there");
            return;
        }
        let target = self.resolve_asset_path(Path::new(typed));
        if let Err(error) = self.save_song(Some(target)) {
            self.editor.status_bar.show_error(format!("Song save failed: {error}"));
            self.reveal_missing_song_provenance();
        }
    }

    /// `path` relative to the asset folder with `/` separators, as the composer shows it.
    pub(super) fn asset_relative(&self, path: &Path) -> String {
        let relative = path.strip_prefix(&self.asset_base).unwrap_or(path);
        relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// An arrow: the transport's control, or the sidebar's control and its section. Neither
/// needs a song — the Songs list is walked with none open.
fn move_control_cursor(state: &mut ComposerState, direction: Direction) {
    match (state.focus, direction) {
        (ComposerFocus::Transport, Direction::Left) => state.transport_control = state.transport_control.saturating_sub(1),
        (ComposerFocus::Transport, Direction::Right) => {
            state.transport_control = (state.transport_control + 1).min(state.transport_control_count.saturating_sub(1));
        }
        (ComposerFocus::Sidebar, Direction::Up) => state.sidebar_control = state.sidebar_control.saturating_sub(1),
        (ComposerFocus::Sidebar, Direction::Down) => {
            state.sidebar_control = (state.sidebar_control + 1).min(state.sidebar_control_count.saturating_sub(1));
        }
        (ComposerFocus::Sidebar, Direction::Left | Direction::Right) => {
            state.step_sidebar_section(direction == Direction::Right);
        }
        _ => {}
    }
}
