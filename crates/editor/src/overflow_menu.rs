//! Overflow menu for the toolbar strip: renders shed tools and/or shed view toggles
//! when the strip is too narrow to display them directly.

use ui::{Rect, UIContext};

use crate::toolbar::{EditorTool, Toolbar};
use crate::layout::ROW_HEIGHT;
use crate::toolbar_strip::StripLayout;
use crate::view_toggles::{ViewToggle, ViewToggles};
use crate::EditorTheme;

/// Width of the overflow menu: wide enough for labels, shortcuts and checkmarks.
pub const OVERFLOW_MENU_WIDTH: f32 = 140.0;

/// Side of the check square drawn on an on-toggle's row.
const CHECK_SIZE: f32 = 6.0;

/// Where the menu hangs: under its button, and never past the window's
/// bottom — a short embedded canvas would otherwise put the last rows
/// (Reset Layout, at the end) out of reach. Shifted up rather than flipped,
/// and no higher than `floor` — the strip's top: the strip's buttons are
/// drawn after the menu and its blocking rect reaches them, but the menu
/// bar above the strip is drawn before it and would not be protected. In a
/// canvas too short for even that, the last rows fall off the bottom.
pub fn overflow_menu_bounds(button: Rect, total_height: f32, window: glam::Vec2, floor: f32) -> Rect {
    let below = button.bottom() + 2.0;
    let y = below.min(window.y - total_height).max(floor);
    Rect::new(button.x, y, OVERFLOW_MENU_WIDTH, total_height)
}

/// An item selected from the overflow menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowPick {
    Tool(EditorTool),
    Toggle(ViewToggle),
    ResetLayout,
}

enum MenuItemKind {
    Tool(EditorTool),
    Toggle(ViewToggle),
    ResetLayout,
}

struct OverflowRowItem {
    label: &'static str,
    hint: Option<&'static str>,
    checked: Option<bool>,
    kind: MenuItemKind,
}

/// Render the open overflow menu: shed tools, an optional separator, and shed view toggles.
/// A click on an entry picks that item and closes the menu; a press outside closes it.
///
/// Call BEFORE the strip's scope opens: a short window shifts the menu up
/// over the strip's own buttons, and a blocking rect only protects widgets
/// whose interact call comes after it. (Overlay scopes cannot nest, so the
/// menu's scope is closed again before the strip's opens.)
pub fn render_overflow_menu(
    toolbar: &mut Toolbar,
    ui: &mut UIContext,
    theme: &EditorTheme,
    layout: &StripLayout,
    toggles: &ViewToggles,
) -> Option<OverflowPick> {
    if !toolbar.is_overflow_open() {
        return None;
    }
    let Some(button) = layout.overflow_button else {
        toolbar.close_overflow();
        return None;
    };

    let shed_tools: Vec<EditorTool> =
        EditorTool::all().iter().skip(layout.visible_tools).copied().collect();
    let view_group_shed = layout.view_group.is_none();

    if shed_tools.is_empty() && !view_group_shed {
        toolbar.close_overflow();
        return None;
    }

    let mut rows: Vec<OverflowRowItem> = Vec::new();
    for tool in shed_tools {
        rows.push(OverflowRowItem {
            label: tool.name(),
            hint: Some(tool.shortcut()),
            checked: None,
            kind: MenuItemKind::Tool(tool),
        });
    }

    let has_separator = !rows.is_empty() && view_group_shed;

    if view_group_shed {
        for toggle in ViewToggle::ALL {
            rows.push(OverflowRowItem {
                label: toggle.name(),
                hint: toggle.shortcut(),
                checked: Some(toggles.is_on(toggle)),
                kind: MenuItemKind::Toggle(toggle),
            });
        }
        rows.push(OverflowRowItem {
            label: "Reset Layout",
            hint: None,
            checked: None,
            kind: MenuItemKind::ResetLayout,
        });
    }

    let separator_height = if has_separator { 6.0 } else { 0.0 };
    let total_height = rows.len() as f32 * ROW_HEIGHT + separator_height + 8.0;
    let menu = overflow_menu_bounds(button, total_height, ui.window_size(), layout.strip.y);

    // A press outside both the menu and the button that opened it closes the menu.
    if ui.mouse_just_pressed()
        && !menu.contains(ui.mouse_pos())
        && !button.contains(ui.mouse_pos())
    {
        toolbar.close_overflow();
        return None;
    }

    ui.begin_overlay_in(ui::UiLayer::Floating, menu);
    ui.panel_styled(menu, theme.surface_4, theme.popup_border, 1.0);

    let mut picked = None;
    let mut current_y = menu.y + 4.0;
    let separator_index = if has_separator {
        EditorTool::all().len() - layout.visible_tools
    } else {
        usize::MAX
    };

    for (index, item) in rows.into_iter().enumerate() {
        if index == separator_index {
            // Draw separator rule
            ui.line(
                glam::Vec2::new(menu.x + 4.0, current_y + 2.0),
                glam::Vec2::new(menu.right() - 4.0, current_y + 2.0),
                theme.border_subtle,
                1.0,
            );
            current_y += 6.0;
        }

        let row = Rect::new(
            menu.x + 4.0,
            current_y,
            menu.width - 8.0,
            ROW_HEIGHT,
        );
        let id = format!("toolbar_overflow_{}", item.label);
        if ui.button(id.as_str(), item.label, row) {
            picked = match item.kind {
                MenuItemKind::Tool(tool) => Some(OverflowPick::Tool(tool)),
                MenuItemKind::Toggle(toggle) => Some(OverflowPick::Toggle(toggle)),
                MenuItemKind::ResetLayout => Some(OverflowPick::ResetLayout),
            };
        }

        // The same check the View menu draws: a small accent square on the
        // row's left edge — a primitive, so no font-coverage risk.
        if item.checked == Some(true) {
            ui.rect(
                Rect::new(row.x + 4.0, row.center().y - CHECK_SIZE / 2.0, CHECK_SIZE, CHECK_SIZE),
                theme.accent_cyan,
            );
        }

        if let Some(hint) = item.hint {
            ui.label_in_bounds_styled(
                hint,
                row,
                ui::TextAlign::Right,
                theme.shortcut_hint,
                theme.fonts.small,
                8.0,
            );
        }

        current_y += ROW_HEIGHT;
    }

    ui.end_overlay();

    if let Some(pick) = picked {
        if let OverflowPick::Tool(tool) = pick {
            toolbar.set_tool(tool);
        }
        toolbar.close_overflow();
    }

    picked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play_controls::PlayControls;
    use crate::play_state::EditorPlayState;
    use crate::test_support::{press_at, release};
    use crate::toolbar_strip::{self, TOOLBAR_STRIP_MIN_WIDTH};
    use crate::view_toggles::ViewToggle;
    use glam::Vec2;

    /// The menu never hangs past the window's bottom: in a canvas too short
    /// to hold every row below the button, it shifts up so the last row
    /// (Reset Layout) stays clickable.
    #[test]
    fn test_the_menu_shifts_up_to_stay_inside_a_short_window() {
        let button = Rect::new(8.0, 48.0, 34.0, 30.0);
        let seven_rows = 7.0 * ROW_HEIGHT + 6.0 + 8.0;

        let strip_top = 43.0;

        let tall = overflow_menu_bounds(button, seven_rows, Vec2::new(390.0, 700.0), strip_top);
        assert_eq!(tall.y, button.bottom() + 2.0, "with room below, the menu hangs under its button");

        let short = overflow_menu_bounds(button, seven_rows, Vec2::new(390.0, 240.0), strip_top);
        assert!(short.bottom() <= 240.0, "the menu's bottom {} is past the window", short.bottom());
        assert!(short.y >= strip_top);

        // Shorter than the strip plus the menu: the shift stops at the strip's
        // top rather than covering the menu bar drawn before it.
        let tiny = overflow_menu_bounds(button, seven_rows, Vec2::new(390.0, 150.0), strip_top);
        assert_eq!(tiny.y, strip_top, "the menu never climbs above the strip");
    }

    /// One pixel under the minimum the view group is shed, and through the
    /// real open path — the menu open, a click on a toggle's row — every
    /// toggle and Reset Layout is reachable.
    #[test]
    fn test_a_shed_view_group_is_reachable_through_the_open_menu() {
        let strip = toolbar_strip::split(Rect::new(0.0, 48.0, TOOLBAR_STRIP_MIN_WIDTH - 1.0, 560.0)).0;
        let mut toolbar = Toolbar::new();
        let controls = PlayControls::new();
        let toggles = ViewToggles::default();
        let theme = EditorTheme::default();
        let layout = toolbar_strip::layout(strip, &toolbar, &controls, EditorPlayState::Editing);
        assert_eq!(layout.view_group, None, "one pixel under the minimum sheds the group");
        let button = layout.overflow_button.expect("the shed group needs a menu to live in");
        toolbar.toggle_overflow();

        let shed_tools = EditorTool::all().len() - layout.visible_tools;
        let separator = if shed_tools > 0 { 6.0 } else { 0.0 };
        let row_of = |index: usize| {
            Vec2::new(
                button.x + OVERFLOW_MENU_WIDTH / 2.0,
                button.bottom() + 2.0 + 4.0 + separator + (index as f32 + 0.5) * ROW_HEIGHT,
            )
        };
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();

        for (offset, expected) in [
            (0, OverflowPick::Toggle(ViewToggle::Grid)),
            (3, OverflowPick::Toggle(ViewToggle::Snap)),
            (4, OverflowPick::ResetLayout),
        ] {
            let point = row_of(shed_tools + offset);
            press_at(&mut ui, &mut input, point, |ui| {
                render_overflow_menu(&mut toolbar, ui, &theme, &layout, &toggles)
            });
            let picked = release(&mut ui, &mut input, |ui| {
                render_overflow_menu(&mut toolbar, ui, &theme, &layout, &toggles)
            });
            assert_eq!(picked, Some(expected), "row {offset} after the shed tools");
            toolbar.toggle_overflow();
        }
    }

    /// In a window too short to hang the menu below the strip, it shifts up
    /// over the strip's own buttons. Drawn before the strip's scope, its
    /// blocking rect reaches them: a click on a row that covers the overflow
    /// button picks the row, and the button beneath neither toggles the menu
    /// nor selects anything.
    #[test]
    fn test_a_menu_shifted_over_the_strip_takes_the_click_from_the_button_beneath() {
        use crate::test_support::{press_mouse, release_mouse};

        let content = Rect::new(0.0, 48.0, TOOLBAR_STRIP_MIN_WIDTH - 1.0, 400.0);
        let strip = toolbar_strip::split(content).0;
        let theme = EditorTheme::default();
        let mut toolbar = Toolbar::new();
        let mut controls = PlayControls::new();
        let toggles = ViewToggles::default();
        let layout = toolbar_strip::layout(strip, &toolbar, &controls, EditorPlayState::Editing);
        controls.position = layout.play_controls_origin;
        let button = layout.overflow_button.expect("a shed group needs a menu");
        toolbar.toggle_overflow();

        let shed_tools = EditorTool::all().len() - layout.visible_tools;
        let separator = if shed_tools > 0 { 6.0 } else { 0.0 };
        let height = (shed_tools + 5) as f32 * ROW_HEIGHT + separator + 8.0;
        // A window exactly short enough to put the menu's top at the button's top.
        let window = Vec2::new(content.width, button.y + height);
        let menu = overflow_menu_bounds(button, height, window, strip.y);
        assert_eq!(menu.y, button.y, "the short window shifts the menu over the strip");
        let first_row = Vec2::new(button.x + 8.0, menu.y + 4.0 + ROW_HEIGHT / 2.0);
        assert!(button.contains(first_row), "the first row covers the overflow button");
        let first_item = if shed_tools > 0 {
            OverflowPick::Tool(EditorTool::all()[layout.visible_tools])
        } else {
            OverflowPick::Toggle(ViewToggle::Grid)
        };

        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();
        let mut frame = |input: &input::InputHandler, toolbar: &mut Toolbar, controls: &mut PlayControls| {
            ui.begin_frame(input, window);
            let pick = render_overflow_menu(toolbar, &mut ui, &theme, &layout, &toggles);
            toolbar_strip::begin(&mut ui, strip, &theme);
            let tool = toolbar.render(&mut ui, &theme, &layout);
            let action = controls.render(&mut ui, EditorPlayState::Editing, false, &theme);
            toolbar_strip::end(&mut ui);
            ui.end_frame();
            (pick, tool, action)
        };
        press_mouse(&mut input, first_row);
        frame(&input, &mut toolbar, &mut controls);
        release_mouse(&mut input);
        let (pick, tool, action) = frame(&input, &mut toolbar, &mut controls);

        assert_eq!(pick, Some(first_item), "the row over the button takes the click");
        assert_eq!(tool, None, "the strip's buttons beneath the menu are inert");
        assert_eq!(action, None);
        assert!(!toolbar.is_overflow_open(), "the pick closed the menu, not the button's toggle");
    }
}
