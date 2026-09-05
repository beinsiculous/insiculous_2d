//! Panel content rendering for editor dock panels.
//!
//! Extracted from editor_demo.rs — renders the content inside each dock panel
//! (scene view, hierarchy tree, inspector, asset browser).

use glam::Vec2;

use editor::{layout, CommandHistory, EditorContext, HierarchyPanel, PanelId};
use engine_core::contexts::GameContext;

/// Render the content of a specific dock panel.
pub fn render_panel_content(
    editor: &mut EditorContext,
    ctx: &mut GameContext,
    panel_id: PanelId,
    bounds: common::Rect,
    command_history: &mut CommandHistory,
    pickables: &[editor::PickableEntity],
) {
    let padding = layout::PADDING;
    let content_x = bounds.x + padding;
    let y = bounds.y + padding;

    match panel_id {
        PanelId::SCENE_VIEW => render_scene_view(editor, ctx, bounds, pickables),
        PanelId::HIERARCHY => render_hierarchy(editor, ctx, bounds, command_history),
        PanelId::INSPECTOR => {
            let texture_path = |handle: u32| ctx.assets.texture_path(handle).map(str::to_string);
            render_inspector(
                editor,
                ctx.ui,
                ctx.world,
                &texture_path,
                bounds,
                command_history,
            )
        }
        PanelId::ASSET_BROWSER => {
            asset_browser::render_asset_browser(editor, ctx, bounds, command_history)
        }
        _ => render_default(ctx, content_x, y),
    }
}

pub(crate) use asset_browser::render_drag_ghost;

/// Scene view — grid info, viewport origin crosshair, and play-state border.
fn render_scene_view(
    editor: &EditorContext,
    ctx: &mut GameContext,
    bounds: common::Rect,
    pickables: &[editor::PickableEntity],
) {
    let theme = &editor.theme;
    let padding = layout::PADDING;
    let content_x = bounds.x + padding;
    let y = bounds.y + padding;

    // Authoring grid — a square, zoom-adaptive ruler drawn under everything
    // else in the panel. The size label comes after so lines never strike
    // through the text.
    if editor.is_grid_visible() {
        editor::render_grid_overlay(
            ctx.ui,
            &editor.grid,
            &editor.viewport,
            &editor.theme.grid_colors(),
            ui::Rect::new(bounds.x, bounds.y, bounds.width, bounds.height),
        );
        ctx.ui.label_styled(
            &format!("Grid: {}px", editor.grid_size()),
            Vec2::new(content_x, y),
            theme.text_muted,
            theme.fonts.small,
        );
    }

    // Draw the world-origin crosshair where (0,0) actually is under the
    // current pan/zoom (the panel clip rect trims any overshoot).
    let center = editor.world_to_screen(Vec2::ZERO);
    ctx.ui.circle(center, 5.0, theme.border_subtle);
    ctx.ui.line(
        Vec2::new(center.x - 20.0, center.y),
        Vec2::new(center.x + 20.0, center.y),
        theme.separator,
        1.0,
    );
    ctx.ui.line(
        Vec2::new(center.x, center.y - 20.0),
        Vec2::new(center.x, center.y + 20.0),
        theme.separator,
        1.0,
    );

    // Collider outlines — drawn over the rendered sprites so physics shapes
    // can be compared against the visuals and tuned until they line up.
    if editor.is_colliders_visible() {
        editor::render_collider_overlay(
            ctx.ui,
            ctx.world,
            &editor.viewport,
            &editor.selection,
            &editor.theme.collider_overlay_colors(),
            bounds,
        );
    }

    // Selection + hover outlines — an editing affordance, so hidden while
    // Playing (picking and gizmos are disabled then too). Built from the
    // same pickable list picking uses, so the outline always matches what a
    // click selects; hover reads the same input-frame mouse state picking
    // will read one step later, so hint and click agree.
    if !editor.is_playing() {
        let mouse = ctx.ui.mouse_pos();
        let hover_allowed = editor.viewport.contains_screen_point(mouse)
            && !crate::editor_game::chrome_owns_mouse(ctx.ui)
            && !editor.drag_drop.suppresses_click()
            && !editor.gizmo_has_priority();
        let hovered = if hover_allowed {
            editor::hover_entity_at(mouse, &editor.viewport, pickables)
        } else {
            None
        };
        editor::render_selection_outline(
            ctx.ui,
            &editor.viewport,
            &editor.selection,
            hovered,
            pickables,
            &editor.theme.selection_outline_colors(),
            ui::Rect::new(bounds.x, bounds.y, bounds.width, bounds.height),
        );
    }

    // Play-state border tint
    let border_color = theme.play_state_border(editor.play_state());
    let outline_width = if editor.in_play_session() { 3.0 } else { 1.0 };

    ctx.ui.rect_border(bounds, border_color, outline_width, 0.0);
}

/// Hierarchy — tree view with click-to-select, Ctrl toggle, Shift range
/// select, and F2 inline rename (committed renames are undo-recorded
/// here).
fn render_hierarchy(
    editor: &mut EditorContext,
    ctx: &mut GameContext,
    bounds: common::Rect,
    command_history: &mut CommandHistory,
) {
    let response = editor.hierarchy.render(
        ctx.ui,
        ctx.world,
        &mut editor.selection,
        bounds,
        &editor.theme,
        &mut editor.drag_drop,
    );

    if let Some((entity, raw)) = response.rename_committed {
        apply_hierarchy_rename(editor, ctx, command_history, entity, &raw);
    }

    if let Some((entity, path)) = response.script_dropped {
        apply_script_drop(editor, ctx.world, command_history, entity, &path);
    }

    let clicked = response.clicked;
    if !clicked.is_empty() {
        editor.close_add_component_popup();
    }
    // Same modifier read AND precedence as the marquee: either Ctrl /
    // either Shift, Ctrl wins a chord.
    let modifiers = editor::Modifiers::read(ctx.input);
    let mode = hierarchy_click_mode(modifiers.ctrl, modifiers.shift);
    for click in clicked {
        match click {
            editor::HierarchyClick::Entity(entity_id) => {
                match mode {
                    HierarchyClickMode::Toggle => editor.selection.toggle(entity_id),
                    // Range from the anchor row through the clicked one (anchor stays
                    // primary); with no selected row visible, Shift ADDS like the
                    // Shift+marquee does — never a silent collapse to one row.
                    HierarchyClickMode::Range => {
                        match editor.hierarchy.shift_click_range(&editor.selection, entity_id) {
                            Some(range) => editor.selection.select_multiple(range),
                            None => editor.selection.add(entity_id),
                        }
                    }
                    HierarchyClickMode::Select => editor.selection.select(entity_id),
                }
                log::info!(
                    "Selected entity: {} ({})",
                    HierarchyPanel::entity_display_name(ctx.world, entity_id),
                    entity_id.value()
                );
            }
            editor::HierarchyClick::Script { entity, .. } => {
                editor.selection.select(entity);
                editor.inspector_scroll_request = Some("Scripts");
            }
        }
    }
}


/// Apply a committed inline rename as one undoable command. An empty or
/// unchanged commit is a no-op (an entity is never stranded with a blank
/// Name); a name now shared by several entities gets a status-bar
/// warning because it stops being a usable command-API address.
fn apply_hierarchy_rename(
    editor: &mut EditorContext,
    ctx: &mut GameContext,
    command_history: &mut CommandHistory,
    entity: ecs::EntityId,
    raw: &str,
) {
    let current = ctx
        .world
        .get::<ecs::Name>(entity)
        .map(|n| n.as_str().to_string());
    let Some(new_name) = editor::normalized_rename(current.as_deref(), raw) else {
        return;
    };
    let cmd = editor::commands::RenameEntityCommand::new(
        ctx.world,
        entity,
        ecs::Name::new(new_name.clone()),
    );
    command_history.execute(Box::new(cmd), ctx.world);
    warn_if_name_ambiguous(editor, ctx.world, &new_name);
}

/// What a hierarchy row click does under the held modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HierarchyClickMode {
    /// Ctrl (with or without Shift): toggle the row — the marquee's precedence.
    Toggle,
    /// Shift alone: range-select from the anchor row.
    Range,
    /// No modifier: replace the selection.
    Select,
}

/// Modifier precedence shared with the marquee (`apply_marquee_selection`):
/// Ctrl beats Shift, so `Ctrl+Shift+click` still toggles.
pub(super) fn hierarchy_click_mode(ctrl_held: bool, shift_held: bool) -> HierarchyClickMode {
    if ctrl_held {
        HierarchyClickMode::Toggle
    } else if shift_held {
        HierarchyClickMode::Range
    } else {
        HierarchyClickMode::Select
    }
}

/// The warning text when a just-committed name is shared by several
/// entities — it stops being a usable command-API address. Both rename
/// paths raise it: the hierarchy F2 commit (via [`warn_if_name_ambiguous`])
/// and the inspector Name field, which folds it into the frame's field
/// warnings so neither overwrites the other.
pub(super) fn name_ambiguity_warning(world: &ecs::World, name: &str) -> Option<String> {
    match HierarchyPanel::resolve_by_name(world, name) {
        editor::NameResolution::Ambiguous(matches) => Some(format!(
            "{} entities are now named \"{}\" — the name is ambiguous for API addressing",
            matches.len(),
            name
        )),
        _ => None,
    }
}

/// Show [`name_ambiguity_warning`] on the status bar, if any.
pub(super) fn warn_if_name_ambiguous(
    editor: &mut EditorContext,
    world: &ecs::World,
    name: &str,
) {
    if let Some(warning) = name_ambiguity_warning(world, name) {
        editor.status_bar.show_message(format!("Warning: {warning}"));
    }
}

/// Attach a script to an entity via drag-drop.
///
/// If the entity already carries a `Scripts` component, appends the new
/// script reference via `SetScriptsCommand`. Otherwise, adds a default
/// `Scripts` component and sets its scripts as a single macro command so
/// one undo reverts the entire attachment.
pub(super) fn apply_script_drop(
    editor: &mut EditorContext,
    world: &mut ecs::World,
    command_history: &mut CommandHistory,
    entity: ecs::EntityId,
    path: &str,
) {
    let stem = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("script");
    let script_ref = ecs::script::ScriptRef {
        script_id: stem.to_string(),
        source_path: path.to_string(),
        params: std::collections::BTreeMap::new(),
    };

    let display_name = HierarchyPanel::entity_display_name(world, entity);

    // Attaching is idempotent, like assigning a texture: the same file
    // dropped twice does not bind the script twice.
    let already_attached = world
        .get::<ecs::script::Scripts>(entity)
        .is_some_and(|scripts| scripts.0.iter().any(|existing| existing.source_path == path));
    if already_attached {
        editor
            .status_bar
            .show_message(format!("{stem} is already attached to {display_name}"));
        return;
    }

    if let Some(old) = world.get::<ecs::script::Scripts>(entity).cloned() {
        let mut new = old.clone();
        new.0.push(script_ref);
        let cmd = editor::commands::SetScriptsCommand::new(entity, old, new, "script_drop");
        command_history.execute(Box::new(cmd), world);
    } else {
        let old = ecs::script::Scripts::default();
        let new = ecs::script::Scripts(vec![script_ref]);
        let add_cmd = Box::new(editor::commands::AddComponentCommand::new(
            entity,
            editor::ComponentKind::Scripts,
        ));
        let set_cmd = Box::new(editor::commands::SetScriptsCommand::new(
            entity,
            old,
            new,
            "script_drop",
        ));
        let macro_cmd = editor::commands::MacroCommand::new("Attach script", vec![add_cmd, set_cmd]);
        command_history.execute(Box::new(macro_cmd), world);
    }

    editor
        .status_bar
        .show_message(format!("Attached {stem} to {display_name}"));
}

/// Fallback for unknown panels.
fn render_default(ctx: &mut GameContext, content_x: f32, y: f32) {
    ctx.ui.label("Panel", Vec2::new(content_x, y));
}

mod add_component_popup;
mod asset_browser;
mod inspector;
use inspector::render_inspector;

#[cfg(test)]
mod click_mode_tests {
    use super::{hierarchy_click_mode, HierarchyClickMode};
    use editor::{CommandHistory, EditorContext};

    #[test]
    fn test_ctrl_beats_shift_like_the_marquee() {
        assert_eq!(hierarchy_click_mode(true, true), HierarchyClickMode::Toggle);
        assert_eq!(hierarchy_click_mode(true, false), HierarchyClickMode::Toggle);
        assert_eq!(hierarchy_click_mode(false, true), HierarchyClickMode::Range);
        assert_eq!(hierarchy_click_mode(false, false), HierarchyClickMode::Select);
    }

    #[test]
    fn test_script_drop_on_entity_with_scripts_appends_ref_and_undo_removes_it() {
        let mut world = ecs::World::new();
        let entity = world.create_entity();
        world.add_component(&entity, ecs::Name::new("Player")).ok();
        world
            .add_component(
                &entity,
                ecs::script::Scripts(vec![ecs::script::ScriptRef::new("existing")]),
            )
            .ok();

        let mut editor = EditorContext::new();
        let mut history = CommandHistory::new();

        super::apply_script_drop(
            &mut editor,
            &mut world,
            &mut history,
            entity,
            "assets/scripts/mover.rhai",
        );

        let scripts = world
            .get::<ecs::script::Scripts>(entity)
            .expect("scripts should exist");
        assert_eq!(scripts.0.len(), 2);
        assert_eq!(scripts.0[1].script_id, "mover");
        assert_eq!(scripts.0[1].source_path, "assets/scripts/mover.rhai");
        assert_eq!(
            editor.status_bar.message(),
            Some("Attached mover to Player")
        );

        history.undo(&mut world);
        let scripts_after_undo = world
            .get::<ecs::script::Scripts>(entity)
            .expect("scripts should still exist");
        assert_eq!(scripts_after_undo.0.len(), 1);
        assert_eq!(scripts_after_undo.0[0].script_id, "existing");
    }

    #[test]
    fn test_script_drop_on_entity_without_scripts_adds_component_and_one_undo_removes_it() {
        let mut world = ecs::World::new();
        let entity = world.create_entity();
        world.add_component(&entity, ecs::Name::new("Enemy")).ok();

        let mut editor = EditorContext::new();
        let mut history = CommandHistory::new();

        super::apply_script_drop(
            &mut editor,
            &mut world,
            &mut history,
            entity,
            "scripts/ai.rhai",
        );

        let scripts = world
            .get::<ecs::script::Scripts>(entity)
            .expect("scripts component should be added");
        assert_eq!(scripts.0.len(), 1);
        assert_eq!(scripts.0[0].script_id, "ai");
        assert_eq!(scripts.0[0].source_path, "scripts/ai.rhai");
        assert_eq!(editor.status_bar.message(), Some("Attached ai to Enemy"));

        history.undo(&mut world);
        assert!(world.get::<ecs::script::Scripts>(entity).is_none());
    }

    #[test]
    fn test_script_drop_of_an_attached_script_records_nothing() {
        let mut world = ecs::World::new();
        let entity = world.create_entity();
        world.add_component(&entity, ecs::Name::new("Enemy")).ok();
        let mut editor = EditorContext::new();
        let mut history = CommandHistory::new();
        super::apply_script_drop(&mut editor, &mut world, &mut history, entity, "scripts/ai.rhai");

        super::apply_script_drop(&mut editor, &mut world, &mut history, entity, "scripts/ai.rhai");

        let scripts = world.get::<ecs::script::Scripts>(entity).expect("scripts component exists");
        assert_eq!(scripts.0.len(), 1, "the second drop binds nothing");
        assert_eq!(editor.status_bar.message(), Some("ai is already attached to Enemy"));
        assert!(history.undo(&mut world), "the first attach is the only entry");
        assert!(!history.undo(&mut world), "the second drop recorded no entry");
    }
}
