//! The composer's host: who holds the keyboard, what its key table's rows do, what its
//! render's actions do, the File menu's song items, and the frame-start rules that take
//! the keyboard away.
//!
//! Every change to the song goes through [`EditorGame::apply_composer_edit`], past the one
//! refusal a play session raises, into `SongDocument::apply`. Focus is explicit: a click
//! in the panel, Ctrl+Shift+M or a File item gives it; a press elsewhere in the dock,
//! hiding or collapsing the panel, Reset Layout, entering a play session and Escape take
//! it away — each of them dropping a live drag — and a menu click does not, so Edit → Undo
//! after a click in the roll undoes the song.

use std::path::Path;

use editor::composer::{
    self, piano_roll::sixteenth_ticks, ComposerAction, ComposerFocus, ComposerKeyAction,
    ComposerKeyRoute, ComposerState, ControlRequest, Direction, SidebarSection,
};
use editor::{EditorAction, Gesture, Modifiers, PanelId, SongDocument, SongEdit};
use engine_core::contexts::GameContext;
use engine_core::Game;
use winit::keyboard::KeyCode;

use super::scene_confirm::PendingSceneAction;
use super::shortcuts::KeyRoute;
use super::EditorGame;
use crate::panel_renderer::composer::{render_composer, ComposerView};

/// The velocity `+` and `-` add and take away.
const VELOCITY_STEP: i32 = 8;
/// The velocity a note added from the keyboard plays at.
const NEW_NOTE_VELOCITY: u8 = 100;

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

    /// Take the keyboard from the composer and drop a live drag, applying nothing.
    pub(super) fn leave_composer(&mut self) {
        self.drop_composer_drag();
        self.editor.composer.clear_focus();
    }

    /// Show the composer, expanded, and give `area` the keyboard — unless a play session
    /// runs, when the panel is read-only and keys belong to the game.
    pub(super) fn show_composer(&mut self, area: ComposerFocus) {
        self.editor.dock_area.set_panel_visible(PanelId::COMPOSER, true);
        self.editor.dock_area.set_panel_collapsed(PanelId::COMPOSER, false);
        if self.composer_write_refusal().is_none() {
            self.editor.composer.focus_area(area);
        }
    }

    /// Reset the composer's state when the song it was built against was replaced, and
    /// drop a drag whose notes were the old song's.
    pub(super) fn follow_song(&mut self) {
        if self.editor.composer.follow(self.song_generation, self.song.as_ref()) {
            self.composer_drag = None;
        }
    }

    /// Frame start, before any chrome handles this frame's press: follow the song, then
    /// drop the keyboard (and a drag) when the panel is gone or a press landed elsewhere in
    /// the dock — never on a menu, whose dropdown is chrome, nor under a dialog.
    pub(super) fn settle_composer(&mut self, ui: &ui::UIContext) {
        self.follow_song();
        let panel = self
            .editor
            .dock_area
            .get_panel(PanelId::COMPOSER)
            .filter(|panel| panel.visible && !panel.collapsed)
            .map(|panel| panel.content_bounds());
        let Some(content) = panel else {
            self.leave_composer();
            self.editor.composer.songs_scanned = false;
            return;
        };
        let header = editor::layout::HEADER_HEIGHT;
        let panel_bounds = ui::Rect::new(content.x, content.y - header, content.width, content.height + header);
        let mouse = ui.mouse_pos();
        let dialog_up = self.scene_confirm.pending_action.is_some() || self.stop_confirm.pending;
        let pressed_elsewhere = ui.mouse_just_pressed()
            && !panel_bounds.contains(mouse)
            && self.editor.dock_area.bounds().contains(mouse)
            && !self.editor.menu_bar.is_open()
            && !dialog_up;
        if pressed_elsewhere && (self.composer_holds_focus() || self.composer_drag.is_some()) {
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
        let focus = self.editor.composer.focus;
        match action {
            ComposerKeyAction::Escape => {
                if !self.drop_composer_drag() {
                    self.leave_composer();
                }
            }
            ComposerKeyAction::NextArea => self.editor.composer.focus_area(focus.next()),
            ComposerKeyAction::PreviousArea => self.editor.composer.focus_area(focus.previous()),
            ComposerKeyAction::MoveCursor(direction) => self.move_composer_cursor(direction),
            ComposerKeyAction::Nudge(direction) => self.nudge_notes(direction),
            ComposerKeyAction::Activate => match focus {
                ComposerFocus::Transport => self.editor.composer.request = Some(ControlRequest::ActivateTransport),
                ComposerFocus::Sidebar => self.editor.composer.request = Some(ControlRequest::ActivateSidebar),
                _ => self.add_note_at_cursor(),
            },
            ComposerKeyAction::DeleteNotes => {
                let notes = self.selected_note_images();
                if !notes.is_empty() && !self.refuse_during_drag() {
                    let notes = notes.iter().map(|image| image.id.number()).collect();
                    self.apply_composer_edit(SongEdit::DeleteNotes { notes }, Gesture::Discrete);
                }
            }
            ComposerKeyAction::Shorten => self.resize_notes(-1),
            ComposerKeyAction::Lengthen => self.resize_notes(1),
            ComposerKeyAction::VelocityUp => self.change_velocity(VELOCITY_STEP),
            ComposerKeyAction::VelocityDown => self.change_velocity(-VELOCITY_STEP),
            ComposerKeyAction::Undo => self.undo_song_with_feedback(),
            ComposerKeyAction::Redo => self.redo_song_with_feedback(),
            ComposerKeyAction::SelectAll => {
                if let (Some(song), Some(track)) = (self.song.as_mut(), self.editor.composer.active_track) {
                    let notes: Vec<_> = composer::track_notes(song, track).iter().map(|image| image.id).collect();
                    song.select(notes);
                }
            }
            ComposerKeyAction::PreviewToggle => self.preview_play_stop(),
        }
    }

    /// An arrow: the transport's and the sidebar's cursors need no song (the Songs list is
    /// walked with none open); the roll's and the note list's read it.
    fn move_composer_cursor(&mut self, direction: Direction) {
        let state = &mut self.editor.composer;
        if !move_control_cursor(state, direction) {
            if let Some(song) = self.song.as_mut() {
                move_note_cursor(state, song, direction);
            }
        }
        state.reveal_cursor = state.focus == ComposerFocus::PianoRoll;
        state.reveal_control = state.focus == ComposerFocus::Sidebar;
    }

    /// The selected notes the roll shows: on the active track, as they are now.
    fn selected_note_images(&self) -> Vec<editor::composer::NoteImage> {
        match (self.song.as_ref(), self.editor.composer.active_track) {
            (Some(song), Some(track)) => composer::note_images(song, composer::state::selected_on_track(song, track)),
            _ => Vec::new(),
        }
    }

    /// Shift+arrow: the selected notes a sixteenth in time or a semitone in pitch, as the
    /// drag's batch shape, merging while the key is held; the key's release seals it.
    fn nudge_notes(&mut self, direction: Direction) {
        let images = self.selected_note_images();
        let Some(step) = self.song.as_ref().map(sixteenth_ticks) else {
            return;
        };
        if images.is_empty() || self.refuse_during_drag() {
            return;
        }
        let step = i64::try_from(step).unwrap_or(1);
        let (tick_delta, key_delta) = match direction {
            Direction::Left => (-step, 0),
            Direction::Right => (step, 0),
            Direction::Up => (0, 1),
            Direction::Down => (0, -1),
        };
        let (tick_delta, key_delta) = composer::clamp_move(&images, tick_delta, key_delta);
        if let Some(edit) = composer::moved_notes_edit(&images, tick_delta, key_delta) {
            self.apply_composer_edit(edit, Gesture::Continuing);
        }
    }

    /// `[` and `]`: every selected note a sixteenth shorter or longer, never shorter than
    /// a sixteenth — one batch of one resize per distinct length.
    fn resize_notes(&mut self, sixteenths: i64) {
        let images = self.selected_note_images();
        let Some(step) = self.song.as_ref().map(sixteenth_ticks) else {
            return;
        };
        let mut lengths: Vec<u64> = images.iter().map(|image| image.duration_ticks).collect();
        lengths.sort_unstable();
        lengths.dedup();
        let members: Vec<SongEdit> = lengths
            .into_iter()
            .map(|length| SongEdit::ResizeNotes {
                notes: images.iter().filter(|image| image.duration_ticks == length).map(|image| image.id.number()).collect(),
                duration_ticks: length.saturating_add_signed(sixteenths * step as i64).max(step),
            })
            .collect();
        if !members.is_empty() && !self.refuse_during_drag() {
            self.apply_composer_edit(SongEdit::Batch(members), Gesture::Discrete);
        }
    }

    /// `+` and `-`: every selected note's velocity by `delta`, kept from 1 to 127.
    fn change_velocity(&mut self, delta: i32) {
        let images = self.selected_note_images();
        let mut velocities: Vec<u8> = images.iter().map(|image| image.velocity).collect();
        velocities.sort_unstable();
        velocities.dedup();
        let members: Vec<SongEdit> = velocities
            .into_iter()
            .map(|velocity| SongEdit::SetVelocity {
                notes: images.iter().filter(|image| image.velocity == velocity).map(|image| image.id.number()).collect(),
                velocity: (i32::from(velocity) + delta).clamp(1, 127) as u8,
            })
            .collect();
        if !members.is_empty() && !self.refuse_during_drag() {
            self.apply_composer_edit(SongEdit::Batch(members), Gesture::Discrete);
        }
    }

    /// Enter or Insert: an eighth at the roll's cursor on the active track, selected, with
    /// the cursor moved past it so the next Enter writes the next note.
    fn add_note_at_cursor(&mut self) {
        let Some(track) = self.editor.composer.active_track else {
            self.editor.status_bar.show_error("This song has no note track to add a note to");
            return;
        };
        let Some(step) = self.song.as_ref().map(sixteenth_ticks) else {
            return;
        };
        let cursor = self.editor.composer.cursor;
        let edit = SongEdit::AddNote {
            track,
            tick: cursor.tick,
            key: cursor.key,
            velocity: NEW_NOTE_VELOCITY,
            duration_ticks: step * 2,
        };
        if self.refuse_during_drag() || !self.apply_composer_edit(edit, Gesture::Discrete) {
            return;
        }
        if let Some(song) = self.song.as_mut() {
            let added = composer::track_notes(song, track)
                .into_iter()
                .filter(|image| image.tick == cursor.tick && image.key == cursor.key)
                .map(|image| image.id)
                .max();
            song.select(added);
        }
        self.editor.composer.cursor.tick = cursor.tick.saturating_add(step * 2);
        self.editor.composer.reveal_cursor = true;
    }

    /// Show `track` in the roll and the list. Switching clears the selection, so no key
    /// edits a note the roll is not showing; refused while a drag is live.
    fn set_active_track(&mut self, track: usize) {
        if self.refuse_during_drag() {
            return;
        }
        let Some(song) = self.song.as_mut() else {
            return;
        };
        if song.data().tracks().get(track).and_then(|song_track| song_track.channel()).is_none() {
            self.editor.status_bar.show_error("That track holds timing only; it has no notes to show");
            return;
        }
        if self.editor.composer.active_track != Some(track) {
            self.editor.composer.active_track = Some(track);
            self.editor.composer.note_list_row = 0;
            song.select([]);
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
        if self.refuse_during_drag() {
            return;
        }
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
            Ok(Some(name)) => self.editor.status_bar.show_message(format!("{verb} {name} (song)")),
            Ok(None) => self.editor.status_bar.show_message(format!("Nothing to {} (song)", if undo { "undo" } else { "redo" })),
            Err(error) => self.editor.status_bar.show_error(format!("Song history failed: {error}")),
        }
        self.sync_dirty_mirror();
    }

    /// The File menu's song items, `FocusComposer` and the preview toggle.
    pub(super) fn dispatch_song_action(&mut self, action: EditorAction) {
        match action {
            EditorAction::NewSong => {
                if self.request_document_action(PendingSceneAction::NewSong) {
                    self.perform_song_action(PendingSceneAction::NewSong);
                }
                self.show_composer(ComposerFocus::PianoRoll);
            }
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
            EditorAction::FocusComposer => self.show_composer(ComposerFocus::PianoRoll),
            EditorAction::SongPlayStop => self.preview_play_stop(),
            other => log::error!("{other:?} is not a song action"),
        }
    }

    /// The composer panel's frame: render it with the song and the drag's preview, then
    /// run what it asked for and carry the drag one frame on.
    pub(super) fn render_composer_panel(&mut self, ctx: &mut GameContext, bounds: common::Rect) {
        self.follow_song();
        if !self.editor.composer.songs_scanned {
            self.editor.composer.rescan_songs(&self.asset_base);
        }
        let default_path = self.asset_relative(&self.default_song_path());
        let song_path = self.song.as_ref().and_then(|song| song.path()).map(|path| self.asset_relative(path));
        let keyboard = ctx.input.keyboard();
        let enter_pressed = keyboard.is_key_just_pressed(KeyCode::Enter) || keyboard.is_key_just_pressed(KeyCode::NumpadEnter);
        let view = ComposerView {
            song: self.song.as_ref(),
            preview: self.composer_drag.as_ref().map(|drag| drag.preview()),
            read_only: self.composer_write_refusal().is_some(),
            drag_live: self.composer_drag.is_some(),
            modifiers: Modifiers::read(ctx.input),
            enter_pressed,
            default_path: &default_path,
            song_path: song_path.as_deref(),
            preview_audible: self.song_preview.is_audible(),
        };
        let output = render_composer(&mut self.editor, ctx.ui, bounds, &view);
        self.run_composer_actions(output.actions);
        self.update_composer_drag(output.pointer, ctx.ui.mouse_down());
    }

    /// Run what the composer's render asked for, in order.
    pub(super) fn run_composer_actions(&mut self, actions: Vec<ComposerAction>) {
        for action in actions {
            match action {
                ComposerAction::Focus(area) if self.composer_write_refusal().is_none() => {
                    self.editor.composer.focus_area(area);
                }
                ComposerAction::Focus(_) => {}
                ComposerAction::Select(notes) => {
                    if let Some(song) = self.song.as_mut() {
                        song.select(notes);
                    }
                }
                ComposerAction::BeginDrag { notes, point } => self.begin_composer_drag(notes, point),
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

/// The transport's control, the sidebar's control and its section; `false` when the
/// arrow was not theirs.
fn move_control_cursor(state: &mut ComposerState, direction: Direction) -> bool {
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
        _ => return false,
    }
    true
}

/// The roll's tick and key, a sixteenth and a semitone at a time, and the note list's
/// row, which selects the note it reaches.
fn move_note_cursor(state: &mut ComposerState, song: &mut SongDocument, direction: Direction) {
    let step = sixteenth_ticks(song);
    match (state.focus, direction) {
        (ComposerFocus::PianoRoll, Direction::Left) => state.cursor.tick = state.cursor.tick.saturating_sub(step),
        (ComposerFocus::PianoRoll, Direction::Right) => {
            state.cursor.tick = (state.cursor.tick + step).min(composer::LAST_WRITABLE_TICK - step);
        }
        (ComposerFocus::PianoRoll, Direction::Up) => state.cursor.key = (state.cursor.key + 1).min(composer::HIGHEST_KEY),
        (ComposerFocus::PianoRoll, Direction::Down) => state.cursor.key = state.cursor.key.saturating_sub(1),
        (ComposerFocus::NoteList, Direction::Up | Direction::Down) => {
            let notes = state.active_track.map(|track| composer::track_notes(song, track)).unwrap_or_default();
            state.note_list_row = match direction {
                Direction::Up => state.note_list_row.saturating_sub(1),
                _ => (state.note_list_row + 1).min(notes.len().saturating_sub(1)),
            };
            if let Some(image) = notes.get(state.note_list_row) {
                song.select([image.id]);
            }
            state.reveal_row = true;
        }
        _ => {}
    }
}
