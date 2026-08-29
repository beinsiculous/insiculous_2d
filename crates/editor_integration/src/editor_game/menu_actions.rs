//! Menu bar rendering and action dispatch.


use glam::Vec2;

use engine_core::contexts::GameContext;
use engine_core::Game;

use crate::entity_ops;

use super::EditorGame;

impl<G: Game> EditorGame<G> {
    /// Sync View-menu check indicators with the current editor state.
    fn sync_view_menu_checks(&mut self) {
        for label in ["Inspector", "Hierarchy", "Asset Browser"] {
            if let Some(id) = editor::panel_id_for_menu_label(label) {
                let visible = self
                    .editor
                    .dock_area
                    .get_panel(id)
                    .is_some_and(|p| p.visible);
                self.editor.menu_bar.set_checked("View", label, visible);
            }
        }
        let grid = self.editor.is_grid_visible();
        self.editor.menu_bar.set_checked("View", "Toggle Grid", grid);
        let colliders = self.editor.is_colliders_visible();
        self.editor.menu_bar.set_checked("View", "Toggle Colliders", colliders);
        let snap = self.editor.is_snap_to_grid();
        self.editor.menu_bar.set_checked("View", "Snap to Grid", snap);
    }

    /// Render the menu bar and dispatch any selected action.
    pub(super) fn handle_menu_bar(&mut self, ctx: &mut GameContext, window_size: Vec2) {
        self.sync_view_menu_checks();
        let Some(action) = self.editor.menu_bar.render(ctx.ui, window_size.x, &self.editor.theme) else {
            return;
        };
        log::info!("Menu action: {}", action);

        match action.as_str() {
            "Create Empty" | "Create Sprite" | "Create Camera"
            | "Create Static Body" | "Create Dynamic Body" | "Create Kinematic Body"
            | "Create UI Label" | "Create UI Panel" | "Create UI Button"
                if !self.editor.is_playing() =>
            {
                // Spawn at the center of the current view — a hardcoded
                // world origin lands off-screen whenever the camera has
                // panned (users read that as "the button does nothing").
                let spawn_pos = self.editor.viewport.camera_position();
                if let Some(entity) = entity_ops::handle_create_action(
                    &action,
                    ctx.world,
                    &mut self.editor.selection,
                    spawn_pos,
                    &mut self.entity_counter,
                ) {
                    let cmd = editor::commands::CreateEntityCommand::already_created(ctx.world, entity);
                    self.command_history.push_already_executed(Box::new(cmd));
                }
            }
            "Cut" if !self.editor.is_playing() => self.cut_selection(ctx),
            "Copy" if !self.editor.is_playing() => self.copy_selection(ctx),
            "Paste" if !self.editor.is_playing() => self.paste_clipboard(ctx),
            "Delete" if !self.editor.is_playing() => {
                self.delete_selected_entities(ctx);
            }
            "Duplicate" if !self.editor.is_playing() => {
                self.duplicate_selected_entities(ctx);
            }
            "Undo" if !self.editor.is_playing() => {
                if let Some(name) = self.command_history.undo_name() {
                    self.editor.status_bar.show_message(format!("Undo: {}", name));
                }
                if self.command_history.undo(ctx.world) {
                    self.apply_selection_restore();
                }
            }
            "Redo" if !self.editor.is_playing() => {
                if let Some(name) = self.command_history.redo_name() {
                    self.editor.status_bar.show_message(format!("Redo: {}", name));
                }
                if self.command_history.redo(ctx.world) {
                    self.apply_selection_restore();
                }
            }
            "New Scene" if !self.editor.is_playing() => {
                if self.request_scene_replace(super::scene_confirm::PendingSceneAction::NewScene) {
                    self.new_scene(ctx.world);
                }
            }
            "Open Scene..." if !self.editor.is_playing() => {
                let path = self.default_scene_path();
                let action = super::scene_confirm::PendingSceneAction::OpenScene(path);
                if self.request_scene_replace(action.clone()) {
                    self.perform_scene_action(ctx, action);
                }
            }
            "Save" => {
                if let Err(e) = self.save_scene(ctx.world, ctx.assets) {
                    self.editor.status_bar.show_error(format!("Save failed: {}", e));
                    log::error!("Failed to save: {}", e);
                }
            }
            "Save As..." => {
                let path = self.default_scene_path();
                if let Err(e) = self.save_scene_as(ctx.world, ctx.assets, path) {
                    self.editor.status_bar.show_error(format!("Save failed: {}", e));
                    log::error!("Failed to save: {}", e);
                }
            }
            // Clean shutdown (runs GameRunner::shutdown → on_exit → prefs save)
            "Exit" => ctx.exit_requested = true,
            "Toggle Grid" => self.editor.toggle_grid(),
            "Toggle Colliders" => self.editor.toggle_colliders(),
            "Snap to Grid" => self.toggle_snap_with_feedback(),
            "Inspector" | "Hierarchy" | "Asset Browser" => {
                if let Some(id) = editor::panel_id_for_menu_label(&action) {
                    self.editor.dock_area.toggle_panel_visible(id);
                }
            }
            "Reset Layout" => {
                self.editor.reset_layout();
                self.editor.status_bar.show_message("Layout reset to defaults");
            }
            "Cycle Game Locale" => {
                ctx.strings.cycle_locale();
                self.editor.status_bar.show_message(format!(
                    "Game locale: {}",
                    ctx.strings.current_display_name()
                ));
            }
            _ => log::info!("Unhandled action: {}", action),
        }
    }

    /// Toggle snap-to-grid and report the new state on the status bar
    /// (shared by the View-menu item and the bare `S` shortcut).
    pub(super) fn toggle_snap_with_feedback(&mut self) {
        self.editor.toggle_snap_to_grid();
        let message = if self.editor.is_snap_to_grid() {
            format!("Snap to grid: on ({}px)", self.editor.grid_size())
        } else {
            "Snap to grid: off".to_string()
        };
        self.editor.status_bar.show_message(message);
    }

    /// Delete all selected entities as a single undoable action.
    pub(super) fn delete_selected_entities(&mut self, ctx: &mut GameContext) {
        let selected: Vec<ecs::EntityId> = self.editor.selection.selected().collect();
        if selected.is_empty() {
            return;
        }
        if selected.len() == 1 {
            let cmd = editor::commands::DeleteEntityCommand::new(selected[0]);
            self.command_history.execute(Box::new(cmd), ctx.world);
        } else {
            let cmds: Vec<Box<dyn editor::EditorCommand>> = selected.iter()
                .map(|&e| Box::new(editor::commands::DeleteEntityCommand::new(e)) as Box<dyn editor::EditorCommand>)
                .collect();
            let cmd = editor::commands::MacroCommand::new("Delete Entities", cmds);
            self.command_history.execute(Box::new(cmd), ctx.world);
        }
        self.editor.selection.clear();
    }

    /// Duplicate the primary selected entity (and its subtree), recording
    /// undo via `SpawnTreeCommand` — its undo removes the WHOLE duplicated
    /// subtree (the old per-root `CreateEntityCommand` orphaned children).
    pub(super) fn duplicate_selected_entities(&mut self, ctx: &mut GameContext) {
        use ecs::WorldHierarchyExt;
        let Some(primary) = self.editor.selection.primary() else {
            return;
        };
        let parent = ctx.world.get_parent(primary);
        let tree = editor::capture_entity_tree(ctx.world, primary);
        let mut cmd =
            editor::SpawnTreeCommand::duplicate(tree, parent, crate::constants::DUPLICATE_OFFSET);
        editor::EditorCommand::execute(&mut cmd, ctx.world);
        if let Some(root) = cmd.spawned_root() {
            self.editor.selection.select(root);
        }
        self.command_history.push_already_executed(Box::new(cmd));
    }

    /// Copy the selection roots to the entity clipboard (no world change).
    /// Unregistered component types can't be captured — warn, never block.
    pub(super) fn copy_selection(&mut self, ctx: &mut GameContext) {
        let roots = entity_ops::selection_roots(ctx.world, &self.editor.selection);
        if roots.is_empty() {
            return;
        }
        let mut lost: Vec<&'static str> = Vec::new();
        self.clipboard = roots
            .iter()
            .map(|&root| {
                for name in editor::uncaptured_component_names(ctx.world, root) {
                    if !lost.contains(&name) {
                        lost.push(name);
                    }
                }
                editor::capture_entity_tree(ctx.world, root)
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
    pub(super) fn paste_clipboard(&mut self, ctx: &mut GameContext) {
        if self.clipboard.is_empty() {
            return;
        }
        let mut commands: Vec<Box<dyn editor::EditorCommand>> = Vec::new();
        let mut new_roots: Vec<ecs::EntityId> = Vec::new();
        for tree in self.clipboard.clone() {
            let mut cmd =
                editor::SpawnTreeCommand::paste(tree, None, crate::constants::DUPLICATE_OFFSET);
            editor::EditorCommand::execute(&mut cmd, ctx.world);
            if let Some(root) = cmd.spawned_root() {
                new_roots.push(root);
            }
            commands.push(Box::new(cmd));
        }
        let count = commands.len();
        match commands.len() {
            1 => {
                if let Some(cmd) = commands.pop() {
                    self.command_history.push_already_executed(cmd);
                }
            }
            _ => {
                self.command_history.push_already_executed(Box::new(
                    editor::commands::MacroCommand::new("Paste", commands),
                ));
            }
        }
        self.editor.selection.select_multiple(new_roots);
        self.editor
            .status_bar
            .show_message(format!("Pasted {count} entities"));
    }

    /// Cut = copy + undoable WHOLE-subtree removal per selection root.
    /// Delete's reparent-the-children semantics would be wrong here — the
    /// clipboard holds the full subtree, so leaving promoted children
    /// behind would duplicate them on paste.
    pub(super) fn cut_selection(&mut self, ctx: &mut GameContext) {
        let roots = entity_ops::selection_roots(ctx.world, &self.editor.selection);
        if roots.is_empty() {
            return;
        }
        self.copy_selection(ctx);
        let mut commands: Vec<Box<dyn editor::EditorCommand>> = Vec::new();
        for &root in &roots {
            let mut cmd = editor::DeleteTreeCommand::new(ctx.world, root);
            editor::EditorCommand::execute(&mut cmd, ctx.world);
            commands.push(Box::new(cmd));
        }
        let count = commands.len();
        match commands.len() {
            1 => {
                if let Some(cmd) = commands.pop() {
                    self.command_history.push_already_executed(cmd);
                }
            }
            _ => {
                self.command_history.push_already_executed(Box::new(
                    editor::commands::MacroCommand::new("Cut", commands),
                ));
            }
        }
        self.editor.selection.clear();
        self.editor
            .status_bar
            .show_message(format!("Cut {count} entities"));
    }
}
