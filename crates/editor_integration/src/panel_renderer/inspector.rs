//! Inspector panel: editable component fields with undo-recorded writeback,
//! read-only view during play, remove buttons, and the add-component popup.

use glam::Vec2;

use ecs::World;
use editor::{
    edit_all_components, inspect_all_components, layout, CommandHistory,
    EditorContext, InspectorFrame, InspectorStyle,
};
use ui::UIContext;

use super::add_component_popup;

/// Inspector — component inspection for the selected entity.
///
/// During Editing/Paused: renders editable fields with live writeback.
/// During Playing: renders read-only view via `inspect_component()`.
pub(super) fn render_inspector(
    editor: &mut EditorContext,
    ui: &mut UIContext,
    world: &mut World,
    texture_path: &dyn Fn(u32) -> Option<String>,
    bounds: common::Rect,
    command_history: &mut CommandHistory,
) {
    let line_height = layout::LINE_HEIGHT;
    let padding = layout::PADDING;
    let content_x = bounds.x + padding;

    let entity_id = match editor.selection.primary() {
        Some(id) => id,
        None => {
            ui.label("No selection", Vec2::new(content_x, bounds.y + padding));
            return;
        }
    };

    // A different entity starts at the top — a leaked offset would open
    // entity B scrolled to wherever entity A was.
    if editor.inspector_scroll_entity != Some(entity_id) {
        editor.inspector_scroll = Default::default();
        editor.inspector_scroll_entity = Some(entity_id);
    }

    // Panel scroll: offset the whole walk; content height is
    // measured at the end (partial rows bleed one row past the panel edge;
    // clipping today is cull-only).
    // While the add-component popup is open the wheel is NOT ours: the
    // window-anchored popup would detach from its scrolling button.
    let wheel = if editor.is_add_component_popup_open() {
        0.0
    } else {
        ui.scroll_delta()
    };
    let offset = editor.inspector_scroll.begin_frame(
        bounds,
        ui.mouse_pos(),
        wheel,
        bounds.height,
    );
    let top = bounds.y + padding - offset;
    let mut y = top;

    // The name leads, the id follows underneath. Both are drawn in a rect
    // of their own line height rather than at a bare position: UI text y is
    // the BASELINE, so a heading placed at the content top loses its
    // ascenders off the panel edge.
    let display_name = editor::entity_display_name(world, entity_id);
    let (heading, detail) = editor
        .selection
        .inspector_heading(&display_name)
        .unwrap_or_else(|| (display_name.clone(), format!("Entity {}", entity_id.value())));
    let heading_size = editor.theme.fonts.heading;
    let heading_rect = common::Rect::new(content_x, y, bounds.width - 2.0 * padding, heading_size);
    match editor.fonts.bold {
        Some(bold) => ui.label_in_bounds_with_font(
            &heading,
            heading_rect,
            ui::TextAlign::Left,
            editor.theme.text_primary,
            bold,
            heading_size,
        ),
        None => ui.label_in_bounds_styled(
            &heading,
            heading_rect,
            ui::TextAlign::Left,
            editor.theme.text_primary,
            heading_size,
            0.0,
        ),
    }
    y += heading_size;
    let detail_size = editor.theme.fonts.small;
    ui.label_in_bounds_styled(
        &detail,
        common::Rect::new(content_x, y, bounds.width - 2.0 * padding, detail_size),
        ui::TextAlign::Left,
        editor.theme.text_muted,
        detail_size,
        0.0,
    );
    y += line_height;

    let content_width = bounds.width - 2.0 * padding;
    let final_y = if editor.is_playing() {
        editor.inspector_scroll_request = None;
        render_inspector_readonly(ui, world, entity_id, content_x, y, &editor.theme.inspector_style())
    } else {
        render_inspector_editable(
            editor,
            ui,
            world,
            texture_path,
            entity_id,
            InspectorLayout {
                x: content_x,
                width: content_width,
                y,
                bounds,
                scroll_offset: offset,
            },
            command_history,
        )
    };
    editor
        .inspector_scroll
        .end_frame(final_y - top + padding, bounds.height);
}

/// Read-only inspector using the editor's component registry (used during
/// Playing). Returns the next Y (for scroll content measurement).
fn render_inspector_readonly(
    ui: &mut UIContext,
    world: &World,
    entity_id: ecs::EntityId,
    content_x: f32,
    y: f32,
    style: &InspectorStyle,
) -> f32 {
    let line_height = layout::LINE_HEIGHT;
    inspect_all_components(
        ui, world, entity_id, content_x, y, style, line_height * 0.5,
    )
}

/// Build inspector extras: texture display path resolved up front and the
/// drag-drop coordinator handed to the inspector.
fn build_inspector_extras<'a>(
    editor: &'a mut EditorContext,
    world: &World,
    entity_id: ecs::EntityId,
    texture_path: &dyn Fn(u32) -> Option<String>,
) -> editor::InspectorExtras<'a> {
    let texture_display = world
        .get::<ecs::sprite_components::Sprite>(entity_id)
        .and_then(|sprite| texture_path(sprite.texture_handle));
    editor::InspectorExtras {
        drag_drop: &mut editor.drag_drop,
        texture_display,
        warnings: Vec::new(),
        scroll_target: editor.inspector_scroll_request,
        scroll_target_y: None,
        can_open_source: cfg!(not(target_arch = "wasm32")),
        open_source: None,
        script_catalog: &editor.script_catalog,
        script_picker_open: editor.script_picker_open,
    }
}

/// Seal undo merge on commit, check for name ambiguity, and publish warnings.
fn warn_after_edit(
    editor: &mut EditorContext,
    ui: &mut UIContext,
    world: &World,
    command_history: &mut CommandHistory,
    entity_id: ecs::EntityId,
    name_before: Option<String>,
    mut warnings: Vec<String>,
) {
    // Gesture boundary: an edit committed this frame (typed commit or scrub
    // release) seals the top undo entry, so the NEXT gesture on the same
    // field becomes its own undo step instead of merging forever.
    if ui.take_edit_commit() {
        command_history.break_merge();
    }

    let name_after = world
        .get::<ecs::Name>(entity_id)
        .map(|name| name.as_str().to_string());
    if let Some(new_name) = name_after.filter(|after| Some(after) != name_before.as_ref()) {
        warnings.extend(super::name_ambiguity_warning(world, &new_name));
    }
    // Soft-range warnings: typed values beyond a field's usual range
    // are accepted by design. Every warning raised this frame — name
    // ambiguity included — lands in ONE transient status message, so none
    // overwrites another.
    if !warnings.is_empty() {
        editor.status_bar.show_message(format!("Warning: {}", warnings.join(" · ")));
    }
}

/// Where the editable walk starts: the content column, its width, and the
/// first row's y after the heading.
struct InspectorLayout {
    x: f32,
    width: f32,
    y: f32,
    bounds: common::Rect,
    scroll_offset: f32,
}

/// Editable inspector with live writeback (used during Editing/Paused).
/// Returns the next Y (for scroll content measurement).
fn render_inspector_editable(
    editor: &mut EditorContext,
    ui: &mut UIContext,
    world: &mut World,
    texture_path: &dyn Fn(u32) -> Option<String>,
    entity_id: ecs::EntityId,
    layout: InspectorLayout,
    command_history: &mut CommandHistory,
) -> f32 {
    let line_height = layout::LINE_HEIGHT;
    let inspect_style = editor.theme.inspector_style();
    // Numeric inputs render in the crate-shipped monospace face.
    let field_style = editor.theme.editable_field_style().with_numeric_font(editor.fonts.mono);

    let mut extras = build_inspector_extras(editor, world, entity_id, texture_path);

    // A Name edit landing this frame must trigger the same ambiguity
    // warning as a hierarchy F2 rename — snapshot before,
    // compare after.
    let name_before = world
        .get::<ecs::Name>(entity_id)
        .map(|name| name.as_str().to_string());

    // Every per-component block (field editors, undo-recorded writeback,
    // remove buttons, read-only fallbacks) is generated from the editor's
    // component registry — adding a component to the registry is all it
    // takes to appear here.
    let mut frame = InspectorFrame {
        ui,
        inspect_style: &inspect_style,
        field_style: &field_style,
        x: layout.x,
        width: layout.width,
        section_gap: line_height * 0.5,
    };
    let (next_y, component_index) = edit_all_components(
        &mut frame,
        world,
        entity_id,
        command_history,
        layout.y,
        &mut extras,
    );
    let target_y = extras.scroll_target_y;
    let source_path = extras.open_source.take();
    let warnings = std::mem::take(&mut extras.warnings);
    let script_picker_open = extras.script_picker_open;
    drop(extras);
    editor.script_picker_open = script_picker_open;

    if let Some(target_y) = target_y {
        let target_offset =
            target_y + layout.scroll_offset - (layout.bounds.y + layout::PADDING);
        editor.inspector_scroll.scroll_to(target_offset);
    }
    editor.inspector_scroll_request = None;
    if let Some(source_path) = source_path {
        editor.pending_open_source = Some(source_path);
    }

    warn_after_edit(
        editor,
        ui,
        world,
        command_history,
        entity_id,
        name_before,
        warnings,
    );

    add_component_popup::render_add_component_section(
        editor,
        ui,
        world,
        command_history,
        entity_id,
        Vec2::new(layout.x, next_y),
        component_index,
    )
}

#[cfg(test)]
mod tests {
    use super::render_inspector;
    use ecs::World;
    use editor::{CommandHistory, EditorContext};
    use glam::Vec2;
    use ui::UIContext;

    #[test]
    fn test_inspector_offers_add_component_while_editing_and_not_while_playing() {
        let mut editor = EditorContext::new();
        let mut world = World::new();
        let mut command_history = CommandHistory::new();
        let entity = world.create_entity();
        world
            .add_component(&entity, common::Transform2D::new(Vec2::ZERO))
            .ok();
        editor.selection.select(entity);

        let bounds = common::Rect::new(0.0, 0.0, 300.0, 600.0);
        let window = Vec2::new(800.0, 600.0);
        let no_texture = |_| None;

        let has_add_component = |ui: &UIContext| {
            ui.draw_list().commands().iter().any(|command| match command {
                ui::DrawCommand::TextPlaceholder { text, .. } => text == "+ Add Component",
                ui::DrawCommand::Text { data, .. } => data.text == "+ Add Component",
                _ => false,
            })
        };

        // 1. While Editing: Add Component button is rendered.
        let mut ui = UIContext::new();
        let input = input::InputHandler::new();
        ui.begin_frame(&input, window);
        render_inspector(
            &mut editor,
            &mut ui,
            &mut world,
            &no_texture,
            bounds,
            &mut command_history,
        );
        ui.end_frame();
        assert!(
            has_add_component(&ui),
            "editing inspector must offer + Add Component"
        );

        // 2. While Playing: Add Component button is not rendered.
        editor.set_play_state(editor::EditorPlayState::Playing);
        let mut ui = UIContext::new();
        ui.begin_frame(&input, window);
        render_inspector(
            &mut editor,
            &mut ui,
            &mut world,
            &no_texture,
            bounds,
            &mut command_history,
        );
        ui.end_frame();
        assert!(
            !has_add_component(&ui),
            "playing inspector must not offer + Add Component"
        );
    }
}
