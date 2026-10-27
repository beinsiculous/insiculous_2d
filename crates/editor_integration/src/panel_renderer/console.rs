//! The console panel: a header with its Clear button over one row per entry, scrolled
//! with the shared `ScrollState`. An error reads "Error: " plus its text in the status
//! bar's error colour, so it is told by more than its colour; a repeat count shows beside
//! the text from 2 on; an entry naming a file is underlined, asks for the pointer while
//! hovered, and a click on it files its request. Every click goes through the ui's
//! interaction, so an open menu or a dialog over the panel blocks it. While the view stands
//! at the bottom it follows each new line in; scrolled up, it keeps the lines it shows in
//! place, the log at its capacity dropping the oldest included. While the console
//! holds the keyboard the focused entry wears the focus ring; what each key does is the
//! host's (`editor_game/console.rs`).

use glam::Vec2;

use editor::console::{ConsoleEntry, Severity};
use editor::layout::PADDING;
use editor::{EditorContext, EditorTheme};

use super::asset_browser::keyboard::reveal_offset;

/// Height of the panel's own header row, which holds the Clear button.
const HEADER_HEIGHT: f32 = 26.0;
const CLEAR_BUTTON_SIZE: Vec2 = Vec2::new(70.0, 20.0);
const ROW_HEIGHT: f32 = 20.0;
/// Space between an entry's text and its repeat count.
const COUNT_GAP: f32 = 8.0;

/// Render the console into the panel's content `bounds`.
pub(super) fn render_console(editor: &mut EditorContext, ui: &mut ui::UIContext, bounds: common::Rect) {
    let clear_bounds = ui::Rect::new(bounds.x + PADDING, bounds.y + 2.0, CLEAR_BUTTON_SIZE.x, CLEAR_BUTTON_SIZE.y);
    if ui.button("console_clear", "Clear", clear_bounds) {
        editor.console.clear();
    }

    let list = common::Rect::new(bounds.x, bounds.y + HEADER_HEIGHT, bounds.width, (bounds.height - HEADER_HEIGHT).max(0.0));
    let console = &mut editor.console;
    let content_height = console.log.len() as f32 * ROW_HEIGHT;
    let dropped_height = std::mem::take(&mut console.rows_dropped) as f32 * ROW_HEIGHT;
    console.scroll.end_frame(content_height, list.height);
    if console.scrolled_up {
        console.scroll.scroll_to((console.scroll.offset() - dropped_height).max(0.0));
    } else {
        console.scroll.scroll_to((content_height - list.height).max(0.0));
    }
    if std::mem::take(&mut console.reveal_focused) {
        if let Some(index) = console.focused {
            let top = index as f32 * ROW_HEIGHT;
            console.scroll.scroll_to(reveal_offset(top, top + ROW_HEIGHT, console.scroll.offset(), list.height));
        }
    }
    let scroll = console.scroll.begin_frame(list, ui.mouse_pos(), ui.scroll_delta(), list.height);
    console.scrolled_up = scroll + list.height < content_height - 0.5;

    ui.push_clip_rect(list);
    let mut clicked = None;
    for (index, entry) in editor.console.log.entries().iter().enumerate() {
        let row = common::Rect::new(list.x, list.y + index as f32 * ROW_HEIGHT - scroll, list.width, ROW_HEIGHT);
        if row.bottom() < list.y || row.y > list.bottom() {
            continue;
        }
        draw_entry(ui, &editor.theme, entry, row);
        // Clipped to the list, so a row scrolled half under the header never takes the
        // Clear button's press.
        let top = row.y.max(list.y);
        let hit = common::Rect::new(row.x, top, row.width, (row.bottom().min(list.bottom()) - top).max(0.0));
        let result = ui.interact(ui::WidgetId::from_str_index("console_entry", entry.id), hit, true);
        let hovered = matches!(result.state, ui::WidgetState::Hovered | ui::WidgetState::Active);
        if hovered && entry.location.is_some() {
            ui.request_cursor(ui::CursorIcon::Pointer);
        }
        if result.clicked {
            clicked = Some(index);
        }
        if editor.console.holds_keyboard && editor.console.focused == Some(index) {
            ui.rect_border(row.shrink(1.0), editor.theme.focus_ring, 2.0, 2.0);
        }
    }
    ui.pop_clip_rect();

    if let Some(index) = clicked {
        editor.console.focused = Some(index);
        if let Some(request) = editor.console.focused_request() {
            editor.open_requests.push(request);
        }
    }
}

/// One entry's row: its text, the underline of a linked entry, and its repeat count.
fn draw_entry(ui: &mut ui::UIContext, theme: &EditorTheme, entry: &ConsoleEntry, row: common::Rect) {
    let font_size = theme.fonts.small;
    let (text, color) = match entry.severity {
        Severity::Error => (format!("Error: {}", entry.text), theme.status_error),
        Severity::Info => (entry.text.clone(), theme.text_secondary),
    };
    let count = (entry.count > 1).then(|| format!("×{}", entry.count));
    let measure = |text: &str| ui.measure_text_styled(text, font_size).x;
    let count_width = count.as_deref().map_or(0.0, |count| measure(count) + COUNT_GAP);
    let shown = editor::ellipsize(&text, (row.width - 2.0 * PADDING - count_width).max(0.0), measure);
    let shown_width = measure(&shown);

    ui.label_in_bounds_styled(&shown, row, ui::TextAlign::Left, color, font_size, PADDING);
    if entry.location.is_some() {
        let underline_y = row.center().y + font_size * 0.5;
        ui.line(Vec2::new(row.x + PADDING, underline_y), Vec2::new(row.x + PADDING + shown_width, underline_y), color, 1.0);
    }
    if let Some(count) = count {
        let count_bounds = common::Rect::new(row.x + PADDING + shown_width + COUNT_GAP, row.y, count_width, row.height);
        ui.label_in_bounds_styled(&count, count_bounds, ui::TextAlign::Left, theme.text_secondary, font_size, 0.0);
    }
    if shown != text {
        ui.tooltip(row, &text);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use editor::external_tools::{Location, OpenRequest};

    use super::*;

    const WINDOW: Vec2 = Vec2::new(1280.0, 720.0);
    const PANEL: common::Rect = common::Rect::new(0.0, 400.0, 800.0, 200.0);

    fn paddle_at(line: usize) -> Option<OpenRequest> {
        Some(OpenRequest { path: PathBuf::from("scripts/paddle.rhai"), location: Location::Line { line, column: None } })
    }

    /// The centre of entry `index`'s row, unscrolled.
    fn row_centre(index: usize) -> Vec2 {
        Vec2::new(PANEL.x + 100.0, PANEL.y + HEADER_HEIGHT + (index as f32 + 0.5) * ROW_HEIGHT)
    }

    /// One frame of the panel with `input` as it stands.
    fn frame(editor: &mut EditorContext, ui: &mut ui::UIContext, input: &input::InputHandler) {
        ui.begin_frame(input, WINDOW);
        render_console(editor, ui, PANEL);
        ui.end_frame();
    }

    /// A press then a release at `point`, each frame rendering the panel.
    fn click(editor: &mut EditorContext, point: Vec2) {
        let mut input = input::InputHandler::new();
        let mut ui = ui::UIContext::new();
        input.mouse_mut().update_position(point.x, point.y);
        input.mouse_mut().handle_button_press(input::prelude::MouseButton::Left);
        frame(editor, &mut ui, &input);
        input.update();
        input.mouse_mut().handle_button_release(input::prelude::MouseButton::Left);
        frame(editor, &mut ui, &input);
    }

    #[test]
    fn an_error_row_is_drawn_prefixed_in_the_status_error_colour_with_its_count_and_underline() {
        let mut editor = EditorContext::new();
        editor.console.push(Severity::Error, "scripts/paddle.rhai:12: [Syntax] frame 1", paddle_at(12));
        editor.console.push(Severity::Error, "scripts/paddle.rhai:12: [Syntax] frame 2", paddle_at(12));
        let mut ui = ui::UIContext::new();
        frame(&mut editor, &mut ui, &input::InputHandler::new());

        let theme = &editor.theme;
        let commands = ui.draw_list().commands();
        let texts: Vec<(&str, ui::Color)> = commands
            .iter()
            .filter_map(|command| match command {
                ui::DrawCommand::TextPlaceholder { text, color, .. } => Some((text.as_str(), *color)),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&("Error: scripts/paddle.rhai:12: [Syntax] frame 2", theme.status_error)), "{texts:?}");
        assert!(texts.iter().any(|(text, _)| *text == "×2"), "the repeat count is drawn beside it");
        let underlined = commands.iter().any(|command| matches!(command, ui::DrawCommand::Line { color, .. } if *color == theme.status_error));
        assert!(underlined, "a linked entry is underlined");
    }

    #[test]
    fn a_click_on_a_linked_entry_files_exactly_its_request_and_on_an_unlinked_one_nothing() {
        let mut editor = EditorContext::new();
        editor.console.push(Severity::Info, "Opened paddle.rhai in RustRover", None);
        editor.console.push(Severity::Error, "scripts/paddle.rhai:12: [Syntax] expected ';'", paddle_at(12));
        editor.console.push(Severity::Error, "scripts/paddle.rhai:30: [Runtime] division by zero", paddle_at(30));

        click(&mut editor, row_centre(0));
        assert!(editor.open_requests.is_empty(), "an unlinked entry files nothing");
        click(&mut editor, row_centre(1));
        assert_eq!(editor.open_requests, vec![paddle_at(12).expect("a request")]);
        assert_eq!(editor.console.focused, Some(1), "the click focuses the entry");
    }

    #[test]
    fn a_click_under_an_open_overlay_files_nothing() {
        let mut editor = EditorContext::new();
        editor.console.push(Severity::Error, "scripts/paddle.rhai:12: [Syntax] expected ';'", paddle_at(12));
        let mut input = input::InputHandler::new();
        let mut ui = ui::UIContext::new();
        let point = row_centre(0);
        for press in [true, false] {
            if press {
                input.mouse_mut().update_position(point.x, point.y);
                input.mouse_mut().handle_button_press(input::prelude::MouseButton::Left);
            } else {
                input.update();
                input.mouse_mut().handle_button_release(input::prelude::MouseButton::Left);
            }
            ui.begin_frame(&input, WINDOW);
            ui.begin_overlay_in(ui::UiLayer::Modal, ui::Rect::new(0.0, 0.0, WINDOW.x, WINDOW.y));
            ui.end_overlay();
            render_console(&mut editor, &mut ui, PANEL);
            ui.end_frame();
        }
        assert!(editor.open_requests.is_empty(), "a dialog's scrim over the panel blocks the click");
    }

    #[test]
    fn clear_empties_the_log() {
        let mut editor = EditorContext::new();
        editor.console.push(Severity::Error, "scripts/paddle.rhai:12: [Syntax] expected ';'", paddle_at(12));
        click(&mut editor, Vec2::new(PANEL.x + PADDING + 10.0, PANEL.y + 12.0));
        assert!(editor.console.log.is_empty());
        assert!(editor.open_requests.is_empty());
    }

    /// The text of every entry row drawn this frame.
    fn drawn_texts(ui: &ui::UIContext) -> Vec<String> {
        ui.draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                ui::DrawCommand::TextPlaceholder { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_view_follows_each_new_line_from_the_bottom_and_stays_put_once_scrolled_up() {
        let mut editor = EditorContext::new();
        let rows_shown = ((PANEL.height - HEADER_HEIGHT) / ROW_HEIGHT) as usize;
        for line in 1..=rows_shown + 4 {
            editor.console.push(Severity::Error, format!("error {line}"), paddle_at(line));
        }
        let mut ui = ui::UIContext::new();
        let input = input::InputHandler::new();
        frame(&mut editor, &mut ui, &input);
        let newest = format!("Error: error {}", rows_shown + 4);
        assert!(drawn_texts(&ui).contains(&newest), "the newest line is in view");

        editor.console.focused = Some(0);
        editor.console.reveal_focused = true;
        frame(&mut editor, &mut ui, &input);
        editor.console.push(Severity::Error, "the one after", paddle_at(99));
        frame(&mut editor, &mut ui, &input);
        let texts = drawn_texts(&ui);
        assert!(texts.contains(&"Error: error 1".to_string()), "scrolled up to the oldest, the view stays there");
        assert!(!texts.contains(&"Error: the one after".to_string()));
    }

    #[test]
    fn at_capacity_a_scrolled_up_view_keeps_its_lines_in_place_as_new_ones_drop_the_oldest() {
        let mut editor = EditorContext::new();
        for line in 1..=editor::console::CONSOLE_CAPACITY {
            editor.console.push(Severity::Error, format!("error {line}"), paddle_at(line));
        }
        let mut ui = ui::UIContext::new();
        let input = input::InputHandler::new();
        editor.console.focused = Some(250);
        editor.console.reveal_focused = true;
        frame(&mut editor, &mut ui, &input);
        let reading = drawn_texts(&ui);
        assert!(reading.contains(&"Error: error 251".to_string()), "{reading:?}");

        for extra in 0..3 {
            editor.console.push(Severity::Error, format!("new {extra}"), paddle_at(1000 + extra));
        }
        frame(&mut editor, &mut ui, &input);

        assert_eq!(drawn_texts(&ui), reading, "three lines in, three dropped, and the reader's lines stay put");
    }

    #[test]
    fn a_press_on_an_entry_the_log_moved_before_the_release_opens_nothing() {
        let mut editor = EditorContext::new();
        for line in 1..=editor::console::CONSOLE_CAPACITY {
            editor.console.push(Severity::Error, format!("error {line}"), paddle_at(line));
        }
        editor.console.scrolled_up = true;
        let mut input = input::InputHandler::new();
        let mut ui = ui::UIContext::new();
        let point = row_centre(1);
        input.mouse_mut().update_position(point.x, point.y);
        input.mouse_mut().handle_button_press(input::prelude::MouseButton::Left);
        frame(&mut editor, &mut ui, &input);

        editor.console.push(Severity::Error, "one more, which drops the oldest", paddle_at(9999));
        input.update();
        input.mouse_mut().handle_button_release(input::prelude::MouseButton::Left);
        frame(&mut editor, &mut ui, &input);

        assert!(editor.open_requests.is_empty(), "the pressed entry moved up a row; the one now under the pointer was never pressed");
    }

    #[test]
    fn the_focused_entry_wears_the_ring_only_while_the_console_holds_the_keyboard() {
        let mut editor = EditorContext::new();
        editor.console.push(Severity::Error, "one", paddle_at(1));
        editor.console.focused = Some(0);
        let rings = |editor: &mut EditorContext| {
            let mut ui = ui::UIContext::new();
            frame(editor, &mut ui, &input::InputHandler::new());
            let ring = editor.theme.focus_ring;
            ui.draw_list()
                .commands()
                .iter()
                .filter(|command| matches!(command, ui::DrawCommand::RectBorder { color, .. } if *color == ring))
                .count()
        };
        assert_eq!(rings(&mut editor), 0);
        editor.console.holds_keyboard = true;
        assert_eq!(rings(&mut editor), 1);
    }
}
