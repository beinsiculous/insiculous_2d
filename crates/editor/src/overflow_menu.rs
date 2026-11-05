//! Bounds and window clamps in this module are UI points.
//! Overflow menu for the toolbar strip: renders shed tools and/or shed view toggles
//! when the strip is too narrow to display them directly.

use ui::{Rect, UIContext, WidgetState};

use crate::icons::{draw_icon, Icon};
use crate::toolbar::{EditorTool, Toolbar, ICON_BOX_SIZE, ICON_STROKE_WEIGHT, TOOL_BUTTON_SIZE};
use crate::layout::ROW_HEIGHT;
use crate::toolbar_strip::StripLayout;
use crate::view_toggles::{ViewToggle, ViewToggles};
use crate::EditorTheme;

/// Width of the overflow menu: wide enough for a check mark and the item's
/// icon in a lane of [`TOOL_BUTTON_SIZE`] at the left, then its label and
/// shortcut.
pub const OVERFLOW_MENU_WIDTH: f32 = 140.0 + TOOL_BUTTON_SIZE;

/// Side of the check square drawn on an on-toggle's row.
const CHECK_SIZE: f32 = 6.0;

/// Distance from a row's left edge to its check square.
const CHECK_INSET: f32 = 2.0;

/// Where the menu hangs: under its button, and never past the window's
/// bottom or its right edge — a short embedded canvas would otherwise put
/// the last rows (Reset Layout, at the end) out of reach, and a narrow one
/// the labels and the shortcut hints, since the button sits past the tools
/// that fit and the menu is wider than the strip's room to its right.
/// Shifted rather than flipped: up, and no higher than `floor` — the
/// strip's top: the strip's buttons are drawn after the menu and its
/// blocking rect reaches them, but the menu bar above the strip is drawn
/// before it and would not be protected; left, and no further than the
/// window's left edge. In a canvas too short or too narrow for even that,
/// the far rows or columns fall off.
pub fn overflow_menu_bounds(button: Rect, total_height: f32, window: glam::Vec2, floor: f32) -> Rect {
    let below = button.bottom() + 2.0;
    let y = below.min(window.y - total_height).max(floor);
    let x = button.x.min(window.x - OVERFLOW_MENU_WIDTH).max(0.0);
    Rect::new(x, y, OVERFLOW_MENU_WIDTH, total_height)
}

/// A row's button: the stock button's frame and fills from the ui theme,
/// with no text of its own. `ui.button` draws a placeholder bar for an
/// empty label, and the row draws its label beside the icon instead.
/// Returns true on the click's release frame.
fn row_button(ui: &mut UIContext, id: &str, row: Rect) -> bool {
    let result = ui.interact(id, row, true);
    let style = &ui.theme().button;
    let background = match result.state {
        WidgetState::Normal => style.background,
        WidgetState::Hovered => style.background_hovered,
        WidgetState::Active => style.background_pressed,
        WidgetState::Disabled => style.background_disabled,
    };
    let (border, border_width, corner_radius) = (style.border, style.border_width, style.corner_radius);
    ui.rect_rounded(row, background, corner_radius);
    if border_width > 0.0 {
        ui.rect_border(row, border, border_width, corner_radius);
    }
    result.clicked
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

impl MenuItemKind {
    /// The icon the item's button carries on the strip, so a shed item is
    /// recognised by the same shape in the menu.
    fn icon(&self) -> Icon {
        match self {
            MenuItemKind::Tool(tool) => tool.icon(),
            MenuItemKind::Toggle(toggle) => toggle.icon(),
            MenuItemKind::ResetLayout => Icon::ResetLayout,
        }
    }
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

    ui.begin_overlay_in(ui::UiLayer::Modal, menu);
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
        let clicked = row_button(ui, &id, row);
        let label_bounds = Rect::new(row.x + TOOL_BUTTON_SIZE, row.y, row.width - TOOL_BUTTON_SIZE, row.height);
        ui.label_in_bounds_styled(
            item.label,
            label_bounds,
            ui::TextAlign::Center,
            theme.text_primary,
            theme.fonts.body,
            0.0,
        );
        let icon_bounds = Rect::new(
            row.x + TOOL_BUTTON_SIZE - ICON_BOX_SIZE,
            row.center().y - ICON_BOX_SIZE / 2.0,
            ICON_BOX_SIZE,
            ICON_BOX_SIZE,
        );
        draw_icon(ui, item.kind.icon(), icon_bounds, theme.icon_stroke, ICON_STROKE_WEIGHT);
        if clicked {
            picked = match item.kind {
                MenuItemKind::Tool(tool) => Some(OverflowPick::Tool(tool)),
                MenuItemKind::Toggle(toggle) => Some(OverflowPick::Toggle(toggle)),
                MenuItemKind::ResetLayout => Some(OverflowPick::ResetLayout),
            };
        }

        // The same check the View menu draws: a small accent square on the
        // row's left edge, before the icon — a primitive, so no
        // font-coverage risk.
        if item.checked == Some(true) {
            ui.rect(
                Rect::new(row.x + CHECK_INSET, row.center().y - CHECK_SIZE / 2.0, CHECK_SIZE, CHECK_SIZE),
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
        let button = Rect::new(8.0, 48.0, TOOL_BUTTON_SIZE, TOOL_BUTTON_SIZE);
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

    /// A narrow canvas that fits every tool puts the overflow button far to
    /// the right, where the menu under it would run past the window; the
    /// menu slides left to the edge instead, so through the real layout at
    /// 250 px every row, its label and its shortcut hint stay inside the
    /// window and the rows remain clickable where they are drawn.
    #[test]
    fn test_the_menu_slides_left_to_stay_inside_a_narrow_window() {
        let content = Rect::new(0.0, 48.0, 250.0, 560.0);
        let window = Vec2::new(content.width, content.bottom());
        let strip = toolbar_strip::split(content).0;
        let theme = EditorTheme::default();
        let mut toolbar = Toolbar::new();
        let toggles = ViewToggles::default();
        let layout = toolbar_strip::layout(strip, &toolbar, &PlayControls::new(), EditorPlayState::Editing);
        let button = layout.overflow_button.expect("the shed view group needs a menu to live in");
        assert!(
            button.x + OVERFLOW_MENU_WIDTH > window.x,
            "the premise: under its button the menu would leave the window"
        );
        toolbar.toggle_overflow();
        let mut ui = UIContext::new();
        let input = input::InputHandler::new();

        ui.begin_frame(&input, window);
        render_overflow_menu(&mut toolbar, &mut ui, &theme, &layout, &toggles);
        ui.end_frame();

        let menu = overflow_menu_bounds(button, 5.0 * ROW_HEIGHT + 8.0, window, strip.y);
        assert!(menu.x >= 0.0 && menu.right() <= window.x, "the menu {menu:?} leaves a {} px window", window.x);
        let button_centre_x = button.center().x;
        assert!(
            button_centre_x >= menu.x && button_centre_x <= menu.right(),
            "the menu {menu:?} still hangs under its button {button:?}"
        );
        let texts: Vec<(String, Vec2)> = ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                ui::DrawCommand::TextPlaceholder { text, position, .. } => Some((text.clone(), *position)),
                _ => None,
            })
            .collect();
        for expected in ["Grid", "Colliders", "Game Frame", "Snap", "Reset Layout", "G", "C", "S"] {
            let Some((_, position)) = texts.iter().find(|(text, _)| text == expected) else {
                panic!("{expected} is not drawn: {texts:?}");
            };
            assert!(
                position.x >= menu.x && position.x < menu.right(),
                "{expected} at {position:?} is outside the menu {menu:?}"
            );
        }
    }

    /// A row carries its item's icon in a lane at its left and its label to
    /// the right of that lane, and draws no text of its own beyond the label
    /// and the shortcut — an empty button label drew a placeholder bar across
    /// every row.
    #[test]
    fn test_every_row_labels_right_of_its_icon_lane_and_draws_no_empty_text() {
        use crate::test_support::frame;

        let strip = toolbar_strip::split(Rect::new(0.0, 48.0, TOOLBAR_STRIP_MIN_WIDTH - 1.0, 560.0)).0;
        let mut toolbar = Toolbar::new();
        let toggles = ViewToggles::default();
        let theme = EditorTheme::default();
        let layout = toolbar_strip::layout(strip, &toolbar, &PlayControls::new(), EditorPlayState::Editing);
        let button = layout.overflow_button.expect("the shed group needs a menu to live in");
        toolbar.toggle_overflow();
        let mut ui = UIContext::new();
        let input = input::InputHandler::new();

        frame(&mut ui, &input, |ui| render_overflow_menu(&mut toolbar, ui, &theme, &layout, &toggles));

        let lane_right = button.x + 4.0 + TOOL_BUTTON_SIZE;
        let texts: Vec<(String, Vec2)> = ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                ui::DrawCommand::TextPlaceholder { text, position, .. } => Some((text.clone(), *position)),
                _ => None,
            })
            .collect();
        assert!(texts.iter().all(|(text, _)| !text.is_empty()), "no empty text: {texts:?}");
        for label in ["Grid", "Colliders", "Game Frame", "Snap", "Reset Layout"] {
            let Some((_, position)) = texts.iter().find(|(text, _)| text == label) else {
                panic!("{label} has no row: {texts:?}");
            };
            assert!(position.x >= lane_right, "{label} starts at {} inside the icon lane", position.x);
        }
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
