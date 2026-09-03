//! Inspector panel: editable component fields with undo-recorded writeback,
//! read-only view during play, remove buttons, and the add-component popup.

use glam::Vec2;

use editor::{
    edit_all_components, inspect_all_components, layout, CommandHistory,
    EditorContext, InspectorFrame, InspectorStyle,
};
use engine_core::contexts::GameContext;

use super::add_component_popup;

/// Inspector — component inspection for the selected entity.
///
/// During Editing/Paused: renders editable fields with live writeback.
/// During Playing: renders read-only view via `inspect_component()`.
pub(super) fn render_inspector(
    editor: &mut EditorContext,
    ctx: &mut GameContext,
    bounds: common::Rect,
    command_history: &mut CommandHistory,
) {
    let line_height = layout::LINE_HEIGHT;
    let padding = layout::PADDING;
    let content_x = bounds.x + padding;

    let entity_id = match editor.selection.primary() {
        Some(id) => id,
        None => {
            ctx.ui.label("No selection", Vec2::new(content_x, bounds.y + padding));
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
        ctx.ui.scroll_delta()
    };
    let offset = editor.inspector_scroll.begin_frame(
        bounds,
        ctx.ui.mouse_pos(),
        wheel,
        bounds.height,
    );
    let top = bounds.y + padding - offset;
    let mut y = top;

    // "Entity: 7  (1 of 5 selected)" in a multi-selection — the
    // inspector shows the primary, and says so.
    let heading = editor
        .selection
        .inspector_heading()
        .unwrap_or_else(|| format!("Entity: {}", entity_id.value()));
    match editor.fonts.bold {
        Some(bold) => ctx.ui.label_with_font(
            &heading,
            Vec2::new(content_x, y),
            bold,
            editor.theme.fonts.heading,
        ),
        None => ctx.ui.label(&heading, Vec2::new(content_x, y)),
    }
    y += line_height;

    let content_width = bounds.width - 2.0 * padding;
    let final_y = if editor.is_playing() {
        render_inspector_readonly(ctx, entity_id, content_x, y, &editor.theme.inspector_style())
    } else {
        render_inspector_editable(
            editor, ctx, entity_id, content_x, content_width, y, command_history,
        )
    };
    editor
        .inspector_scroll
        .end_frame(final_y - top + padding, bounds.height);
}

/// Read-only inspector using the editor's component registry (used during
/// Playing). Returns the next Y (for scroll content measurement).
fn render_inspector_readonly(
    ctx: &mut GameContext,
    entity_id: ecs::EntityId,
    content_x: f32,
    y: f32,
    style: &InspectorStyle,
) -> f32 {
    let line_height = layout::LINE_HEIGHT;
    inspect_all_components(
        ctx.ui, ctx.world, entity_id, content_x, y, style, line_height * 0.5,
    )
}

/// Editable inspector with live writeback (used during Editing/Paused).
/// Returns the next Y (for scroll content measurement).
fn render_inspector_editable(
    editor: &mut EditorContext,
    ctx: &mut GameContext,
    entity_id: ecs::EntityId,
    content_x: f32,
    content_width: f32,
    mut y: f32,
    command_history: &mut CommandHistory,
) -> f32 {
    let line_height = layout::LINE_HEIGHT;
    let inspect_style = editor.theme.inspector_style();
    // Numeric inputs render in the crate-shipped monospace face.
    let field_style = editor.theme.editable_field_style().with_numeric_font(editor.fonts.mono);

    // Resolve the sprite texture's display path up front (the editor crate
    // cannot see AssetManager) and hand the drag-drop coordinator to the
    // registry-generated inspector so the Texture slot can accept drops.
    let texture_display = ctx
        .world
        .get::<ecs::sprite_components::Sprite>(entity_id)
        .and_then(|s| ctx.assets.texture_path(s.texture_handle).map(str::to_string));
    let mut extras = editor::InspectorExtras {
        drag_drop: &mut editor.drag_drop,
        texture_display,
        warnings: Vec::new(),
    };

    // A Name edit landing this frame must trigger the same ambiguity
    // warning as a hierarchy F2 rename — snapshot before,
    // compare after.
    let name_before = ctx
        .world
        .get::<ecs::Name>(entity_id)
        .map(|n| n.as_str().to_string());

    // Every per-component block (field editors, undo-recorded writeback,
    // remove buttons, read-only fallbacks) is generated from the editor's
    // component registry — adding a component to the registry is all it
    // takes to appear here.
    let mut frame = InspectorFrame {
        ui: ctx.ui,
        inspect_style: &inspect_style,
        field_style: &field_style,
        x: content_x,
        width: content_width,
        section_gap: line_height * 0.5,
    };
    let (next_y, component_index) = edit_all_components(
        &mut frame,
        ctx.world,
        entity_id,
        command_history,
        y,
        &mut extras,
    );
    y = next_y;
    let mut warnings = std::mem::take(&mut extras.warnings);

    // Gesture boundary: an edit committed this frame (typed commit or scrub
    // release) seals the top undo entry, so the NEXT gesture on the same
    // field becomes its own undo step instead of merging forever.
    if ctx.ui.take_edit_commit() {
        command_history.break_merge();
    }

    let name_after = ctx
        .world
        .get::<ecs::Name>(entity_id)
        .map(|n| n.as_str().to_string());
    if let Some(new_name) = name_after.filter(|after| Some(after) != name_before.as_ref()) {
        warnings.extend(super::name_ambiguity_warning(ctx.world, &new_name));
    }
    // Soft-range warnings: typed values beyond a field's usual range
    // are accepted by design. Every warning raised this frame — name
    // ambiguity included — lands in ONE transient status message, so none
    // overwrites another.
    if !warnings.is_empty() {
        editor.status_bar.show_message(format!("Warning: {}", warnings.join(" · ")));
    }

    add_component_popup::render_add_component_section(
        editor,
        ctx,
        entity_id,
        command_history,
        content_x,
        y,
        component_index,
    )
}
