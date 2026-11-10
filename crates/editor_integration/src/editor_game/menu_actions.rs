//! Menu bar rendering and action dispatch.


use glam::Vec2;

use engine_core::contexts::GameContext;
use engine_core::Game;

use crate::entity_ops;

use super::EditorGame;
use super::{FileMutation, MutationPhase};
use super::scene_confirm::PendingSceneAction;
use crate::panel_renderer::asset_browser::{rename, remove};
use editor::context_menu::AssetAction;
use editor::{AssetKind, TileId, TileKind};
use std::path::{Path, PathBuf};

/// An open document a move touches: whether it is the song, its path, and each of its files
/// by the key it was recorded under and the path it moves to.
struct MovedDocument {
    song: bool,
    moved: Vec<(PathBuf, PathBuf)>,
}

/// A path's identity for comparing a document with a tile: canonical where the file exists,
/// the path as given where it does not.
pub(super) fn same_path_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

impl<G: Game> EditorGame<G> {
    pub(super) fn run_context_menu_action(&mut self, action: editor::context_menu::ContextMenuAction) {
        match action {
            editor::context_menu::ContextMenuAction::Open(request) => self.editor.open_requests.push(request),
            editor::context_menu::ContextMenuAction::Dock(action) => self.run_dock_action(action),
            editor::context_menu::ContextMenuAction::Asset(action) => self.begin_asset_action(action),
        }
    }

    fn begin_asset_action(&mut self, action: AssetAction) {
        if self.editor.in_play_session() { self.editor.status_bar.show_error("Stop Play before changing assets"); return; }
        match action {
            AssetAction::Rename(tile) => {
                if let Some(error) = self.asset_rename_refusal(&tile, &[]) { self.editor.status_bar.show_error(error); return; }
                let initial = Path::new(&tile.path).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
                self.editor.asset_creation = Some(AssetAction::Rename(tile));
                self.editor.asset_prompt.open("Rename asset", &initial, "Rename");
                self.editor.context_menu.close();
            }
            AssetAction::Delete(tile) => {
                let result = self.prepare_asset_delete(&tile, &[]);
                match result {
                    Ok(plan) => self.scene_confirm.pending_action = Some(PendingSceneAction::DeleteAsset(plan)),
                    Err(error) => self.editor.status_bar.show_error(error),
                }
                self.editor.context_menu.close();
            }
            action => crate::panel_renderer::asset_browser::mutate::begin_creation(&mut self.editor, &self.asset_base, action),
        }
    }

    /// The open scene and song the operation touches: the tile itself, everything under a
    /// folder, and every member of a prepared `family` — a recovered song open in the
    /// Conductor is moved or deleted by its primary's operation like any sibling.
    fn affected_documents(&self, tile: &TileId, family: &[String]) -> Vec<(bool, PathBuf)> {
        // Compared by one identity: the standalone editor's asset base may be relative while
        // a document opened through the API is absolute, and `..` may sit in either.
        let source = same_path_key(&self.asset_base.join(&tile.path));
        let affects = |path: &Path| {
            let path = self.resolve_asset_path(path);
            let key = same_path_key(&path);
            (key == source || (tile.kind == TileKind::Folder && key.starts_with(&source))
                || family.iter().any(|member| same_path_key(&self.asset_base.join(member)) == key)).then_some(path)
        };
        let mut documents = Vec::new();
        if let Some(path) = self.editor.scene_path().and_then(affects) { documents.push((false, path)); }
        // The song is two files; an orphan sidecar tile is the open song's map still.
        if let Some(song_path) = self.song.as_ref().and_then(|song| song.path()) {
            if affects(song_path).is_some() || affects(&editor::external_tools::midi_sidecar_path(song_path)).is_some() {
                documents.push((true, self.resolve_asset_path(song_path)));
            }
        }
        documents
    }

    fn asset_rename_refusal(&self, tile: &TileId, family: &[String]) -> Option<String> {
        if self.editor.in_play_session() { return Some("Stop Play before changing assets".into()); }
        let affected = self.affected_documents(tile, family);
        if !affected.is_empty() && (self.scene_confirm.pending_action.is_some()
            || self.hot_reload.waiting_scene.is_some() || self.hot_reload.waiting_song_map.is_some()) {
            return Some("Finish the open document's pending confirm or reload before renaming it".into());
        }
        // The rename's removal note silences the watcher for the old path, so a change it has
        // not reported yet would be lost under the next Save: the document's own stamp says.
        // A dirty song goes through Save and rename instead, whose save meets the change on
        // disk with its own Overwrite prompt; the scene's save has no such prompt, so a dirty
        // scene is held here like a clean one.
        for (song, path) in &affected {
            if *song && self.song.as_ref().is_some_and(editor::SongDocument::is_dirty) { continue; }
            let files = if *song { vec![path.clone(), editor::external_tools::midi_sidecar_path(path)] } else { vec![path.clone()] };
            if let Some(changed) = files.iter().find(|file| self.document_changed_on_disk(file)) {
                return Some(format!("{} changed on disk since it was loaded; reload it or save it before renaming", self.document_label(changed)));
            }
        }
        None
    }

    pub(super) fn prepare_asset_delete(&self, tile: &TileId, live: &[rename::LiveText]) -> Result<remove::DeletePlan, String> {
        if self.editor.in_play_session() { return Err("Stop Play before changing assets".into()); }
        let plan = remove::prepare(&self.asset_base, tile, live)?;
        if let Some((song, path)) = self.affected_documents(tile, &plan.files).first() {
            return Err(format!("Cannot delete {}: the {} {} is open", tile.path, if *song { "song" } else { "scene" }, self.document_label(path)));
        }
        Ok(plan)
    }

    /// A project-relative file as the watcher registered it: the asset base joined and
    /// resolved as `watched_paths` resolves, never canonicalised, so a note names the same
    /// string the registered set holds.
    pub(super) fn watched_spelling(&self, relative: &str) -> PathBuf {
        self.resolve_asset_path(&self.asset_base.join(relative))
    }

    /// A document named as the browser names assets — relative to the asset base, the
    /// absolute path only for one outside it.
    fn document_label(&self, path: &Path) -> String {
        rename::relative_path(&self.asset_base, path).unwrap_or_else(|_| path.display().to_string())
    }

    /// What the Save-and-rename prompt names: the first open document the tile affects.
    pub(super) fn affected_document_label(&self, tile: &TileId, family: &[String]) -> String {
        let affected = self.affected_documents(tile, family);
        let dirty = affected.iter().find(|(song, _)| {
            if *song { self.song.as_ref().is_some_and(editor::SongDocument::is_dirty) } else { self.command_history.is_dirty() }
        });
        dirty.or(affected.first()).map_or_else(|| tile.path.clone(), |(_, path)| self.document_label(path))
    }

    /// The unsaved documents as they would be written, for the reference scan: the dirty
    /// scene's RON and the dirty song's map. A clean document is on disk and scanned there.
    /// A document that cannot be rendered is `None`, which the scan counts as a reference.
    pub(super) fn unsaved_document_texts(&mut self, world: &ecs::World, texture_path_fn: &dyn Fn(u32) -> String) -> Vec<rename::LiveText> {
        let mut texts = Vec::new();
        if self.command_history.is_dirty() {
            texts.push(match self.scene_snapshot(world, texture_path_fn) {
                Ok(snapshot) => ("the open scene (unsaved)".to_string(), Some(snapshot.ron)),
                Err(error) => (format!("the open scene (unsaved, not scanned: {error})"), None),
            });
        }
        if let Some(song) = self.song.as_ref().filter(|song| song.is_dirty()) {
            texts.push(match song.rendered_files() {
                Ok((_, sidecar)) => ("the open song's map (unsaved)".to_string(), Some(sidecar)),
                Err(error) => (format!("the open song's map (unsaved, not scanned: {error})"), None),
            });
        }
        texts
    }

    /// A parked delete plan rescanned with the unsaved documents, once: the menu's entry
    /// points have no world, so the confirm's first frame supplies them. A plan that no
    /// longer prepares is dropped with its error.
    pub(super) fn scan_delete_plan_live(&mut self, plan: remove::DeletePlan, live: &[rename::LiveText]) -> Option<remove::DeletePlan> {
        if plan.live_scanned { return Some(plan); }
        match self.prepare_asset_delete(&plan.tile, live) {
            Ok(mut rescanned) => { rescanned.live_scanned = true; Some(rescanned) }
            Err(error) => { self.editor.status_bar.show_error(error); None }
        }
    }

    pub(super) fn confirm_asset_rename(&mut self, tile: TileId, name: String, live: Vec<rename::LiveText>) {
        let kind = if tile.kind == TileKind::Folder { AssetKind::Folder } else {
            let Some(entry) = self.editor.asset_browser.entries.iter().find(|entry| entry.relative_path == tile.path) else {
                self.editor.asset_prompt.show_error("The selected asset is no longer in the browser"); return;
            };
            entry.kind
        };
        self.request_asset_rename(rename::RenameRequest { tile, name, kind }, false, live);
    }

    pub(super) fn request_asset_rename(&mut self, request: rename::RenameRequest, references_approved: bool, live: Vec<rename::LiveText>) {
        let prepared = self.asset_rename_refusal(&request.tile, &[]).map_or_else(
            || rename::prepare(&self.asset_base, &request), Err);
        let plan = match prepared { Ok(plan) => plan, Err(error) => { self.rename_error(&request, error); return; } };
        let family: Vec<String> = plan.files.iter().map(|(old, _)| old.clone()).collect();
        // The family is known now: a recovered sibling open with a reload or a confirm parked
        // refuses as the primary would.
        if let Some(error) = self.asset_rename_refusal(&request.tile, &family) { self.rename_error(&request, error); return; }
        let dirty = self.affected_documents(&request.tile, &family).iter().any(|(song, _)| {
            if *song { self.song.as_ref().is_some_and(editor::SongDocument::is_dirty) } else { self.command_history.is_dirty() }
        });
        if dirty {
            self.editor.asset_prompt.close();
            self.scene_confirm.pending_action = Some(PendingSceneAction::SaveAndRenameAsset(request, family));
            return;
        }
        if !references_approved {
            let olds: Vec<&str> = if request.tile.kind == TileKind::Folder { vec![request.tile.path.as_str()] } else { family.iter().map(String::as_str).collect() };
            let scope = if request.tile.kind == TileKind::Folder { rename::ReferenceScope::RenamedFolder } else { rename::ReferenceScope::Files };
            match rename::reference_hits(&self.asset_base, &olds, scope, &live) {
                Ok(hits) if !hits.is_empty() => {
                    self.editor.asset_prompt.close();
                    self.scene_confirm.pending_action = Some(PendingSceneAction::RenameAsset(request, hits));
                    return;
                }
                Err(error) => { self.rename_error(&request, error); return; }
                _ => {}
            }
        }
        self.perform_asset_rename(&request, plan);
    }

    fn rename_error(&mut self, request: &rename::RenameRequest, error: String) {
        if !self.editor.asset_prompt.is_open() { self.editor.asset_prompt.open("Rename asset", &request.name, "Rename"); }
        self.editor.asset_creation = Some(AssetAction::Rename(request.tile.clone()));
        self.editor.asset_prompt.show_error(error.clone());
        self.editor.status_bar.show_error(error);
    }

    pub(super) fn perform_asset_rename(&mut self, request: &rename::RenameRequest, plan: rename::RenamePlan) {
        let family: Vec<String> = plan.files.iter().map(|(old, _)| old.clone()).collect();
        let documents = self.affected_documents(&plan.source, &family);
        // Checked again at the move itself, before the host's guard is asked, so a refusal
        // here never leaves the page's save block waiting for an outcome that never comes: the guard's own save and the confirm's frames lie
        // between the first check and this one, and an edit landing there would be taken
        // for the editor's own bytes once the removal note silences the watcher.
        for (song, path) in &documents {
            let files = if *song { vec![path.clone(), editor::external_tools::midi_sidecar_path(path)] } else { vec![path.clone()] };
            if let Some(changed) = files.iter().find(|file| self.document_changed_on_disk(file)) {
                let label = self.document_label(changed);
                self.rename_error(request, format!("{label} changed on disk since it was loaded; reload it or save it before renaming"));
                return;
            }
        }
        // The watcher registered each path as `resolve_asset_path` spells it; the notes must
        // carry that spelling, and only the matching of documents to pairs is canonical.
        let noted: Vec<PathBuf> = plan.files.iter().map(|(old, _)| self.watched_spelling(old)).collect();
        // Stamped before the page is asked, so a survivor of a failed move is compared with
        // what the editor saw before anything outside it — the page's own save included —
        // could touch the files.
        let stamped = self.stamp_unrecorded(&noted);
        let mut mutation = FileMutation { phase: MutationPhase::Prepare, removed: Vec::new(), renamed: plan.files.clone() };
        if let Some(guard) = self.file_mutation_guard {
            if let Err(error) = guard(&mutation) { self.forget_stamps(&stamped); self.rename_error(request, error); return; }
        }
        // Every path the move touches, in one identity, taken while the files still exist:
        // the asset base may be relative, the document absolute, and a path that is gone
        // no longer canonicalises to the key it was recorded under.
        let source_key = same_path_key(&self.asset_base.join(&plan.source.path));
        let pairs: Vec<(PathBuf, PathBuf)> = plan.files.iter().map(|(old, new)| (same_path_key(&self.asset_base.join(old)), self.resolve_asset_path(Path::new(new)))).collect();
        let documents: Vec<MovedDocument> = documents.into_iter().map(|(song, path)| {
            let key = same_path_key(&path);
            let new_path = pairs.iter().find(|(old_key, _)| *old_key == key).map(|(_, new)| new.clone())
                .or_else(|| key.strip_prefix(&source_key).ok().map(|suffix| self.resolve_asset_path(Path::new(&plan.destination.path)).join(suffix)));
            let moved = new_path.map_or_else(Vec::new, |new_path| if song {
                vec![(path.clone(), new_path.clone()), (editor::external_tools::midi_sidecar_path(&path), editor::external_tools::midi_sidecar_path(&new_path))]
            } else { vec![(path.clone(), new_path)] });
            let moved = moved.into_iter().map(|(old_file, new_file)| (same_path_key(&old_file), new_file)).collect();
            MovedDocument { song, moved }
        }).collect();
        for path in &noted { self.note_removed(path); }
        let result = rename::execute(&self.asset_base, &plan);
        mutation.phase = if result.is_ok() { MutationPhase::Done } else { MutationPhase::Failed };
        if let Some(guard) = self.file_mutation_guard { if let Err(error) = guard(&mutation) { self.report(editor::console::Severity::Error, format!("The page could not follow the file change: {error} — reload the page before saving a script"), None); } }
        match result {
            Ok(warning) => {
                let mut changed_during_move: Vec<(bool, PathBuf)> = Vec::new();
                for MovedDocument { song, moved } in documents {
                    let Some((_, new_path)) = moved.first().cloned() else { continue; };
                    // The baseline follows the document — the stamp recorded before the
                    // move, never the moved file's own: a rename keeps a file's length and
                    // time, so a moved file that reads differently carried an edit made
                    // between the check and the move, and that edit is the watcher's to
                    // report, not a baseline to adopt.
                    for (old_key, new_file) in moved {
                        let recorded = self.hot_reload.document_stamps.remove(&old_key).flatten();
                        let now = common::vfs::change_stamp(&new_file).ok().flatten();
                        if recorded.is_some() && now != recorded {
                            self.report(editor::console::Severity::Info, format!("{} changed on disk during the rename", self.document_label(&new_file)), None);
                            changed_during_move.push((song, new_file.clone()));
                        }
                        self.hot_reload.document_stamps.insert(same_path_key(&new_file), recorded.or(now));
                    }
                    if song { if let Some(song) = &mut self.song { song.follow_rename(new_path); } }
                    else { self.editor.set_scene_path(Some(new_path)); }
                }
                // Once the documents stand at their new paths, each change the move carried
                // takes the watcher's own route: the scene's reload waits for its gesture, a
                // score is adopted, a map reloads or waits as the song's dirtiness says.
                for (song, new_file) in changed_during_move {
                    if !song {
                        self.hot_reload.waiting_scene = Some(new_file);
                    } else if editor::external_tools::is_midi_path(&new_file) {
                        match common::vfs::read(&new_file) {
                            Ok(bytes) => self.adopt_score_bytes(&new_file, bytes),
                            Err(error) => self.report(editor::console::Severity::Error, format!("{} could not be read after the rename: {error}", self.document_label(&new_file)), None),
                        }
                    } else if let Some(midi) = editor::external_tools::sidecar_midi_path(&new_file) {
                        self.hot_reload.waiting_song_map = Some(midi);
                    }
                }
                self.rescan_asset_mutation();
                self.editor.asset_browser.enter(Path::new(&plan.destination.path).parent().and_then(Path::to_str).unwrap_or(""));
                self.editor.asset_browser.selected = Some(plan.destination.clone());
                self.editor.asset_browser.reveal_selected = true;
                self.editor.asset_prompt.close();
                self.editor.asset_creation = None;
                if let Some(warning) = warning { self.editor.status_bar.show_error(warning); }
                else {
                    let companions: Vec<&str> = plan.files.iter().map(|(old, _)| old.as_str()).filter(|old| *old != plan.source.path).collect();
                    let with = if companions.is_empty() { String::new() }
                        else if plan.source.kind == TileKind::Folder { format!(" ({} files)", companions.len()) }
                        else { format!(" (with {})", companions.join(", ")) };
                    self.editor.status_bar.show_message(format!("Renamed {} → {}{with}", plan.source.path, plan.destination.path));
                }
            }
            Err(error) => {
                if error.rescan { self.rescan_asset_mutation(); }
                // A survivor is the editor's baseline again only while it still reads as the
                // editor last saw it; one that changed meanwhile is left for the watcher to
                // report as the external change it is.
                for path in &noted {
                    if !self.document_changed_on_disk(path) { self.note_written(path); }
                }
                self.rename_error(request, error.message);
            }
        }
        self.forget_stamps(&stamped);
    }

    pub(super) fn perform_asset_delete(&mut self, plan: remove::DeletePlan, live: &[rename::LiveText]) {
        let prepared = self.prepare_asset_delete(&plan.tile, live);
        let mut current = match prepared { Ok(current) => current, Err(error) => { self.editor.status_bar.show_error(error); return; } };
        current.live_scanned = plan.live_scanned;
        if current != plan {
            self.editor.status_bar.show_notice("The asset's files changed; confirm the new file count");
            self.scene_confirm.pending_action = Some(PendingSceneAction::DeleteAsset(current));
            return;
        }
        let noted: Vec<PathBuf> = plan.files.iter().map(|file| self.watched_spelling(file)).collect();
        let stamped = self.stamp_unrecorded(&noted);
        let mut mutation = FileMutation { phase: MutationPhase::Prepare, removed: plan.files.clone(), renamed: Vec::new() };
        if let Some(guard) = self.file_mutation_guard {
            if let Err(error) = guard(&mutation) { self.forget_stamps(&stamped); self.editor.status_bar.show_error(error); return; }
        }
        for path in &noted { self.note_removed(path); }
        let result = remove::execute(&self.asset_base, &plan);
        mutation.phase = if result.is_ok() { MutationPhase::Done } else { MutationPhase::Failed };
        if let Some(guard) = self.file_mutation_guard { if let Err(error) = guard(&mutation) { self.report(editor::console::Severity::Error, format!("The page could not follow the file change: {error} — reload the page before saving a script"), None); } }
        if result.is_err() {
            // The survivors were noted absent before the attempt; a watcher left believing
            // that would report their next reading as an external change. One that changed
            // during the attempt is left for it to report as the external change it is.
            for path in &noted {
                if common::vfs::exists(path) && !self.document_changed_on_disk(path) { self.note_written(path); }
            }
        }
        self.forget_stamps(&stamped);
        // Either way the browser rescans: a family half removed must show what is left.
        self.rescan_asset_mutation();
        match result {
            Ok(Some(warning)) => self.editor.status_bar.show_error(warning),
            Ok(None) => self.editor.status_bar.show_message(format!("Deleted {} files", plan.files.len())),
            Err(error) => self.editor.status_bar.show_error(error),
        }
    }

    /// The browser rescans now, and the next draw's `ensure_scanned` rebuilds the script
    /// catalog over the moved or deleted files: this host has no script registry in hand.
    fn rescan_asset_mutation(&mut self) {
        self.editor.asset_browser.apply_scan(editor::scan_assets(&self.asset_base, &self.editor.external_tools.daw_project_extensions));
        self.editor.asset_browser.scanned = false;
        self.editor.conductor.songs_scanned = false;
    }

    pub(super) fn save_before_asset_rename(&mut self, request: rename::RenameRequest, world: &mut ecs::World, assets: &engine_core::assets::AssetManager, texture_path_fn: &dyn Fn(u32) -> String) {
        // The family is the plan's; a recovered song open beside the primary is saved too, or
        // the rename would park behind this prompt again for it.
        let family: Vec<String> = match rename::prepare(&self.asset_base, &request) {
            Ok(plan) => plan.files.iter().map(|(old, _)| old.clone()).collect(),
            Err(error) => { self.rename_error(&request, error); return; }
        };
        for (song, _) in self.affected_documents(&request.tile, &family) {
            let saved = if song { self.save_song(None).map(|path| path.is_some()) }
            else { self.save_scene(world, assets).map(|()| true).map_err(|error| error.to_string()) };
            match saved {
                Ok(true) => {}
                Ok(false) => { self.editor.asset_creation = None; self.editor.status_bar.show_notice("the save is waiting on Overwrite — rename after it"); return; }
                Err(error) => { self.rename_error(&request, format!("Save before rename failed: {error}")); return; }
            }
        }
        let live = self.unsaved_document_texts(world, texture_path_fn);
        self.request_asset_rename(request, false, live);
    }

    fn run_dock_action(&mut self, action: editor::DockAction) {
        match action {
            editor::DockAction::MoveTo(id, edge) => {
                if !self.editor.dock_area.can_move_panel_to_edge(id, edge) {
                    self.editor.status_bar.show_message("That edge has no room for another group, even collapsed");
                    return;
                }
                self.editor.dock_area.move_panel_to_edge(id, edge);
                self.editor.dock_area.show_panel(id);
            }
            editor::DockAction::GroupWith(id, other) => self.editor.dock_area.group_panel_with(id, other),
            editor::DockAction::Show(id) => self.editor.dock_area.show_panel(id),
            editor::DockAction::Hide(id) => self.editor.dock_area.set_panel_visible(id, false),
        }
        self.release_inactive_panel_focus();
        self.sync_view_menu_checks();
        self.editor.menu_bar.sync_window(&self.editor.dock_area);
    }

    /// Sync View-menu check indicators with the current editor state.
    fn sync_view_menu_checks(&mut self) {
        for label in ["Inspector", "Hierarchy", "Asset Browser", "Conductor", "Console"] {
            if let Some(id) = editor::panel_id_for_menu_label(label) {
                let visible = self
                    .editor
                    .dock_area
                    .get_panel(id)
                    .is_some_and(|p| p.visible);
                self.editor.menu_bar.set_checked("View", label, visible);
            }
        }
        self.editor.view.sync_menu(&mut self.editor.menu_bar);
    }

    /// Name Edit → Open Project after the code tool — the category's choice, as a project
    /// request resolves, never a file type's override — which the settings panel may have
    /// changed since the last frame, and enable it where it can open anything.
    fn sync_open_project_item(&mut self) {
        use editor::external_tools::{ToolCategory, ToolName};

        let tool = ToolName::of(self.editor.external_tools.category_choice(ToolCategory::Code));
        let (label, enabled) = editor::open_project_item(tool, cfg!(not(target_arch = "wasm32")));
        self.editor.menu_bar.set_action_by_prefix("Edit", editor::OPEN_PROJECT_PREFIX, label, enabled);
    }

    /// Render the menu bar and dispatch the chosen label as an editor
    /// action through the one dispatcher; while Playing only the actions
    /// that allow it run, the rest are dropped silently.
    pub(super) fn handle_menu_bar(&mut self, ctx: &mut GameContext, window_size: Vec2) {
        self.editor.menu_bar.sync_window(&self.editor.dock_area);
        self.editor.menu_bar.sync_ui_scale(self.ui_scale, ctx.display_scale);
        self.sync_view_menu_checks();
        self.sync_open_project_item();
        if let Some(label) = self.editor.menu_bar.render(ctx.ui, window_size.x, &self.editor.theme) {
            self.dispatch_menu_label(&label, ctx);
        }
        self.release_inactive_panel_focus();
    }

    pub(super) fn dispatch_menu_label(&mut self, label: &str, ctx: &mut GameContext) {
        if let Some(action) = self.editor.menu_bar.take_dock_action() {
            self.run_dock_action(action);
            return;
        }
        log::info!("Menu action: {}", label);
        match editor::action_for_menu_label(label) {
            Some(action) if !self.editor.is_playing() || action.allowed_while_playing() => {
                self.dispatch_editor_action(action, false, ctx);
            }
            Some(_) => {}
            None => log::info!("Unhandled action: {}", label),
        }
    }

    /// Toggle snap-to-grid and report the new state on the status bar
    /// (shared by the View-menu item and the bare `S` shortcut).
    pub(super) fn toggle_snap_with_feedback(&mut self) {
        let is_snap = self.editor.view.toggle(editor::ViewToggle::Snap);
        let message = if is_snap {
            format!("Snap to grid: on ({}px)", self.editor.grid_size())
        } else {
            "Snap to grid: off".to_string()
        };
        self.editor.status_bar.show_message(message);
    }

    /// Delete all selected entities as a single undoable action.
    pub(super) fn delete_selected_entities(&mut self, world: &mut ecs::World) {
        let selected: Vec<ecs::EntityId> = self.editor.selection.selected().collect();
        if selected.is_empty() {
            return;
        }
        let commands: Vec<Box<dyn editor::EditorCommand>> = selected
            .iter()
            .map(|&entity| Box::new(editor::commands::DeleteEntityCommand::new(entity)) as Box<dyn editor::EditorCommand>)
            .collect();
        self.command_history.execute_as_one("Delete Entities", commands, world);
        self.editor.selection.clear();
    }

    /// Duplicate the primary selected entity (and its subtree), recording
    /// undo via `SpawnTreeCommand` — its undo removes the WHOLE duplicated
    /// subtree (the old per-root `CreateEntityCommand` orphaned children).
    pub(super) fn duplicate_selected_entities(&mut self, world: &mut ecs::World) {
        use ecs::WorldHierarchyExt;
        let Some(primary) = self.editor.selection.primary() else {
            return;
        };
        let parent = world.get_parent(primary);
        let tree = editor::capture_entity_tree(world, primary);
        let mut cmd =
            editor::SpawnTreeCommand::duplicate(tree, parent, crate::constants::DUPLICATE_OFFSET);
        editor::EditorCommand::execute(&mut cmd, world);
        if let Some(root) = cmd.spawned_root() {
            self.editor.selection.select(root);
        }
        self.command_history.push_already_executed(Box::new(cmd));
    }

    /// Copy the selection roots to the entity clipboard (no world change).
    /// Unregistered component types can't be captured — warn, never block.
    pub(super) fn copy_selection(&mut self, world: &mut ecs::World) {
        let roots = entity_ops::selection_roots(world, &self.editor.selection);
        if roots.is_empty() {
            return;
        }
        let mut lost: Vec<&'static str> = Vec::new();
        self.clipboard = roots
            .iter()
            .map(|&root| {
                for name in editor::uncaptured_component_names(world, root) {
                    if !lost.contains(&name) {
                        lost.push(name);
                    }
                }
                editor::capture_entity_tree(world, root)
            })
            .collect();
        if lost.is_empty() {
            self.editor
                .status_bar
                .show_message(format!("Copied {} entities", self.clipboard.len()));
        } else {
            self.editor.status_bar.show_message(format!(
                "Copied {} entities — unregistered component(s) NOT captured: {}",
                self.clipboard.len(),
                lost.join(", ")
            ));
        }
    }

    /// Paste the entity clipboard as new root entities (one undo entry),
    /// offset like a duplicate, and select the pasted roots.
    pub(super) fn paste_clipboard(&mut self, world: &mut ecs::World) {
        if self.clipboard.is_empty() {
            return;
        }
        let mut commands: Vec<Box<dyn editor::EditorCommand>> = Vec::new();
        let mut new_roots: Vec<ecs::EntityId> = Vec::new();
        for tree in self.clipboard.clone() {
            let mut cmd =
                editor::SpawnTreeCommand::paste(tree, None, crate::constants::DUPLICATE_OFFSET);
            editor::EditorCommand::execute(&mut cmd, world);
            if let Some(root) = cmd.spawned_root() {
                new_roots.push(root);
            }
            commands.push(Box::new(cmd));
        }
        let count = commands.len();
        self.command_history.push_as_one("Paste", commands);
        self.editor.selection.select_multiple(new_roots);
        self.editor
            .status_bar
            .show_message(format!("Pasted {count} entities"));
    }

    /// Cut = copy + undoable WHOLE-subtree removal per selection root.
    /// Delete's reparent-the-children semantics would be wrong here — the
    /// clipboard holds the full subtree, so leaving promoted children
    /// behind would duplicate them on paste.
    pub(super) fn cut_selection(&mut self, world: &mut ecs::World) {
        let roots = entity_ops::selection_roots(world, &self.editor.selection);
        if roots.is_empty() {
            return;
        }
        self.copy_selection(world);
        let mut commands: Vec<Box<dyn editor::EditorCommand>> = Vec::new();
        for &root in &roots {
            let mut cmd = editor::DeleteTreeCommand::new(world, root);
            editor::EditorCommand::execute(&mut cmd, world);
            commands.push(Box::new(cmd));
        }
        let count = commands.len();
        self.command_history.push_as_one("Cut", commands);
        self.editor.selection.clear();
        self.editor
            .status_bar
            .show_message(format!("Cut {count} entities"));
    }
}
