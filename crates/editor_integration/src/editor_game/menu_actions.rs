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

    /// Render the menu bar and dispatch the chosen label as an editor
    /// action through the one dispatcher; while Playing only the actions
    /// that allow it run, the rest are dropped silently.
    pub(super) fn handle_menu_bar(&mut self, ctx: &mut GameContext, window_size: Vec2) {
        self.sync_view_menu_checks();
        let Some(label) = self.editor.menu_bar.render(ctx.ui, window_size.x, &self.editor.theme) else {
            return;
        };
        log::info!("Menu action: {}", label);

        match editor::action_for_menu_label(&label) {
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
        self.editor.toggle_snap_to_grid();
        let message = if self.editor.is_snap_to_grid() {
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
