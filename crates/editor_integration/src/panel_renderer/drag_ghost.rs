//! The drag ghost: the thumbnail or script name that follows the cursor
//! while an asset drag is in flight, on the DragGhost band above every
//! other overlay. The drag itself is armed in the asset browser and taken
//! by the viewport and the hierarchy; this file only draws it.

use editor::{DragPayload, EditorContext};
use engine_core::contexts::GameContext;

/// Draw the drag ghost following the cursor while a drag is in flight.
/// The overlay's blocking rect also makes widgets and viewport picking
/// under the cursor inert for the frame.
pub(crate) fn render_drag_ghost(editor: &mut EditorContext, ctx: &mut GameContext) {
    match editor.drag_drop.dragging_payload() {
        Some(DragPayload::Texture { handle, .. }) => {
            let handle = *handle;
            let mouse = ctx.ui.mouse_pos();
            let ghost = ui::Rect::new(mouse.x - 24.0, mouse.y - 24.0, 48.0, 48.0);
            // DragGhost band: the ghost rides above even an open dropdown.
            ctx.ui.begin_overlay_in(ui::UiLayer::DragGhost, ghost);
            ctx.ui.image(ghost, handle, ui::Color::new(1.0, 1.0, 1.0, 0.8));
            ctx.ui.end_overlay();
        }
        Some(DragPayload::Script { path }) => {
            let file_name = std::path::Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(path.as_str());
            let mouse = ctx.ui.mouse_pos();
            let text_width =
                (ctx.ui.measure_text_styled(file_name, editor.theme.fonts.small).x + 16.0).max(48.0);
            let ghost = ui::Rect::new(mouse.x - text_width / 2.0, mouse.y - 12.0, text_width, 24.0);
            ctx.ui.begin_overlay_in(ui::UiLayer::DragGhost, ghost);
            ctx.ui.rect_rounded(ghost, editor.theme.surface_3, 4.0);
            ctx.ui.rect_border(ghost, editor.theme.accent_blue, 1.0, 4.0);
            ctx.ui.label_in_bounds_styled(
                file_name,
                ghost,
                ui::TextAlign::Center,
                editor.theme.text_primary,
                editor.theme.fonts.small,
                0.0,
            );
            ctx.ui.end_overlay();
        }
        None => {}
    }
}
