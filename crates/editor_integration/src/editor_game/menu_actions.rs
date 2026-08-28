//! Menu bar rendering and action dispatch.

use std::path::PathBuf;

use glam::Vec2;

use engine_core::contexts::GameContext;
use engine_core::Game;

use crate::constants::DEFAULT_SCENE_PATH;
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
                self.command_history.undo(ctx.world);
            }
            "Redo" if !self.editor.is_playing() => {
                if let Some(name) = self.command_history.redo_name() {
                    self.editor.status_bar.show_message(format!("Redo: {}", name));
                }
                self.command_history.redo(ctx.world);
            }
            "New Scene" if !self.editor.is_playing() => {
                self.new_scene(ctx.world);
            }
            "Open Scene..." if !self.editor.is_playing() => {
                let path = PathBuf::from(DEFAULT_SCENE_PATH);
                self.load_scene_with_feedback(ctx.world, ctx.assets, &path);
            }
            "Save" => {
                if let Err(e) = self.save_scene(ctx.world, ctx.assets) {
                    self.editor.status_bar.show_error(format!("Save failed: {}", e));
                    log::error!("Failed to save: {}", e);
                }
            }
            "Save As..." => {
                let path = PathBuf::from(DEFAULT_SCENE_PATH);
                if let Err(e) = self.save_scene_as(ctx.world, ctx.assets, path) {
                    self.editor.status_bar.show_error(format!("Save failed: {}", e));
                    log::error!("Failed to save: {}", e);
                }
            }
            // Clean shutdown (runs GameRunner::shutdown → on_exit → prefs save)
            "Exit" => ctx.exit_requested = true,
            "Toggle Grid" => self.editor.toggle_grid(),
            "Toggle Colliders" => self.editor.toggle_colliders(),
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

    /// Duplicate the primary selected entity (and its subtree), recording undo.
    pub(super) fn duplicate_selected_entities(&mut self, ctx: &mut GameContext) {
        let Some(primary) = self.editor.selection.primary() else {
            return;
        };
        entity_ops::duplicate_selected_entities(
            ctx.world,
            &mut self.editor.selection,
            &mut self.entity_counter,
        );
        if let Some(new_entity) = self.editor.selection.primary() {
            if new_entity != primary {
                let cmd = editor::commands::CreateEntityCommand::already_created(ctx.world, new_entity);
                self.command_history.push_already_executed(Box::new(cmd));
            }
        }
    }
}
