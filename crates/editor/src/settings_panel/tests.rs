//! The panel through the ui: a command that fails its check never reaches the config and
//! stays as the draft with its error in words, a fixed one is written, Escape drops the
//! draft, a focused field scrolled out of the panel still commits, a changed extension
//! list is reported, and the cursor's ring shows only while the panel holds the keyboard.

use glam::Vec2;
use input::prelude::{KeyCode, MouseButton};

use super::*;
use crate::external_tools::{CommandTemplate, CustomTool, TemplateError};
use crate::test_support::{drawn_text, frame, next_frame, press_button, release_button};

/// Tall enough that every row is inside, so none is skipped as scrolled out.
const PANEL: Rect = Rect { x: 0.0, y: 0.0, width: 320.0, height: 1400.0 };
const CODE_COMMAND: TemplateSlot = TemplateSlot::Command(ToolRow::Category(ToolCategory::Code));

struct Panel {
    ui: UIContext,
    input: input::InputHandler,
    theme: EditorTheme,
    state: SettingsPanelState,
    config: ExternalToolsConfig,
    bounds: Rect,
}

impl Panel {
    fn with_code(choice: ToolChoice) -> Self {
        let mut panel = Self {
            ui: UIContext::new(),
            input: input::InputHandler::new(),
            theme: EditorTheme::default(),
            state: SettingsPanelState::default(),
            config: ExternalToolsConfig { code: choice, ..Default::default() },
            bounds: PANEL,
        };
        panel.render();
        panel
    }

    /// One frame; whether the extension list changed in it.
    fn render(&mut self) -> bool {
        let Self { ui, input, theme, state, config, bounds } = self;
        frame(ui, input, |ui| render_settings_panel(ui, theme, state, config, *bounds, true))
    }

    /// One frame with `key` just pressed, then one with it released; whether the
    /// extension list changed in the first.
    fn key(&mut self, key: KeyCode) -> bool {
        next_frame(&mut self.input);
        self.input.keyboard_mut().handle_key_press(key);
        let changed = self.render();
        next_frame(&mut self.input);
        self.input.keyboard_mut().handle_key_release(key);
        self.render();
        changed
    }

    /// Type `text` into the Code command's field as a whole and press Enter.
    fn commit_code_command(&mut self, text: &str) {
        self.ui.focus_text_input(template_field_id(CODE_COMMAND), text);
        self.key(KeyCode::Enter);
    }

    fn focus_rings(&self) -> usize {
        let ring = self.theme.focus_ring;
        self.ui
            .draw_list()
            .commands()
            .iter()
            .filter(|command| matches!(command, ui::DrawCommand::RectBorder { color, .. } if *color == ring))
            .count()
    }
}

fn custom(file: &str) -> ToolChoice {
    ToolChoice::Custom(CustomTool::file_only(CommandTemplate(file.to_string())))
}

#[test]
fn a_command_with_an_unknown_placeholder_leaves_the_config_and_stays_as_the_draft_with_its_error() {
    let mut panel = Panel::with_code(custom("edit {file}"));
    panel.commit_code_command("edit {path}");

    assert_eq!(panel.config.code, custom("edit {file}"), "the last good command stands");
    let draft = panel.state.draft.clone().expect("the failing text is kept");
    assert_eq!(draft.text, "edit {path}");
    assert_eq!(draft.error, DraftError::Template(TemplateError::UnknownPlaceholder("path".to_string())));
    assert!(panel.ui.is_focused(template_field_id(CODE_COMMAND)), "a failed Enter keeps the keyboard: the next key is the fix");
    assert!(drawn_text(&panel.ui).contains("Error: unknown placeholder {path}"), "{}", drawn_text(&panel.ui));
}

#[test]
fn a_failing_press_away_keeps_the_draft_shown_and_lets_the_keyboard_go() {
    let mut panel = Panel::with_code(custom("edit {file}"));
    panel.ui.focus_text_input(template_field_id(CODE_COMMAND), "edit {path}");
    panel.render();
    press_button(&mut panel.input, MouseButton::Left, Vec2::new(20.0, PANEL.height - 20.0));
    panel.render();

    assert_eq!(panel.config.code, custom("edit {file}"), "the last good command stands");
    assert_eq!(panel.state.draft.as_ref().map(|draft| draft.text.as_str()), Some("edit {path}"));
    assert!(!panel.ui.is_focused(template_field_id(CODE_COMMAND)), "the person left: a re-focus would hand every press away back to the field");
    let text = drawn_text(&panel.ui);
    assert!(text.contains("edit {path}") && text.contains("Error: unknown placeholder {path}"), "the draft and its error stay shown: {text}");
}

#[test]
fn a_focused_field_scrolled_out_of_the_panel_still_commits_on_enter() {
    let mut panel = Panel::with_code(custom("edit {file}"));
    panel.bounds = Rect { height: 100.0, ..PANEL };
    panel.render();
    panel.state.scroll.scroll_to(600.0);
    panel.render();
    assert!(panel.state.scroll.offset() > 200.0, "fixture: the Code command row is scrolled out of the panel");
    assert!(!drawn_text(&panel.ui).contains("edit {file}"), "fixture: the field is not drawn");

    panel.commit_code_command("edit --wait {file}");
    assert_eq!(panel.config.code, custom("edit --wait {file}"), "Enter reaches the field the ui still holds focused");
    assert!(!panel.ui.is_focused(template_field_id(CODE_COMMAND)), "and the commit lets it go");

    panel.state.scroll.scroll_to(600.0);
    panel.render();
    panel.commit_code_command("edit {path}");
    panel.render();
    let text = drawn_text(&panel.ui);
    assert!(panel.ui.is_focused(template_field_id(CODE_COMMAND)), "a failed Enter keeps the keyboard");
    assert!(
        text.contains("edit {path}") && text.contains("Error: unknown placeholder {path}"),
        "and scrolls the field and its error into view, so the keyboard is not held by a field nobody sees: {text}"
    );

    // A field below the panel's bottom reveals with its error row, not its bottom edge alone.
    let project = TemplateSlot::ProjectCommand(ToolCategory::Code);
    panel.state.scroll.scroll_to(0.0);
    panel.render();
    assert!(!drawn_text(&panel.ui).contains("Project comm"), "fixture: the Project command row is below the panel");
    panel.ui.focus_text_input(template_field_id(project), "edit {nope}");
    panel.key(KeyCode::Enter);
    panel.render();
    let text = drawn_text(&panel.ui);
    assert!(
        text.contains("edit {nope}") && text.contains("Error: unknown placeholder {nope}"),
        "the field and the error under it are both inside: {text}"
    );
}

#[test]
fn a_changed_extension_list_is_reported_and_an_unchanged_commit_is_not() {
    let mut panel = Panel::with_code(ToolChoice::OsDefault);
    let extensions = field_id(EXTENSIONS_BLOCK, COMMAND_FIELD);
    panel.ui.focus_text_input(extensions, ".evil .prj .nsp");
    assert!(!panel.key(KeyCode::Enter), "the same list is no change");

    panel.ui.focus_text_input(extensions, ".evil, .als");
    assert!(panel.key(KeyCode::Enter), "a new list is reported for the browser's rescan");
    assert_eq!(panel.config.daw_project_extensions, [".evil", ".als"]);
}

#[test]
fn cycling_the_tool_away_from_a_focused_field_commits_its_text_first_and_frees_the_keyboard() {
    let mut panel = Panel::with_code(custom("edit {file}"));
    panel.ui.focus_text_input(template_field_id(CODE_COMMAND), "edit --wait {file}");
    panel.render();
    assert!(panel.ui.wants_keyboard(), "fixture: the Command field holds the keyboard");

    // Code's Tool cycle is the first row under the first heading; its ">" sits at the
    // value span's end, 120 px past the "<" at the control column.
    let style = panel.theme.editable_field_style();
    let cycle_row_y = style.row_height + 16.0;
    let next_arrow = Vec2::new(6.0 + style.label_width + 2.0 * (style.row_height - 6.0) + 120.0 - 4.0, cycle_row_y + style.row_height / 2.0);
    press_button(&mut panel.input, MouseButton::Left, next_arrow);
    panel.render();
    assert_eq!(panel.config.code, custom("edit --wait {file}"), "the press away commits the field on its own frame");
    release_button(&mut panel.input, MouseButton::Left);
    panel.render();

    assert_eq!(panel.config.code, ToolChoice::OsDefault, "fixture: the click stepped Custom (the last value) round to System default");
    assert_eq!(
        panel.state.kept_custom,
        [(ToolRow::Category(ToolCategory::Code), CustomTool::file_only(CommandTemplate("edit --wait {file}".to_string())))],
        "the text typed before the press was committed, then kept"
    );
    assert!(!panel.ui.wants_keyboard(), "no focus is left on the field the choice hid");
    panel.render();
    assert!(!panel.ui.wants_keyboard());
}

#[test]
fn a_press_into_an_earlier_field_commits_the_focused_field_below_the_fold_first() {
    let mut panel = Panel::with_code(custom("edit {file}"));
    panel.bounds = Rect { height: 100.0, ..PANEL };
    panel.render();
    let project = TemplateSlot::ProjectCommand(ToolCategory::Code);
    assert!(!drawn_text(&panel.ui).contains("Project comm"), "fixture: the Project command row is below the panel");
    panel.ui.focus_text_input(template_field_id(project), "edit {project}");
    panel.render();

    let style = panel.theme.editable_field_style();
    let command_row_y = style.row_height + 16.0 + style.row_height + 2.0;
    press_button(&mut panel.input, MouseButton::Left, Vec2::new(200.0, command_row_y + style.row_height / 2.0));
    panel.render();
    release_button(&mut panel.input, MouseButton::Left);
    panel.render();

    assert!(panel.ui.is_focused(template_field_id(CODE_COMMAND)), "fixture: the click landed in the Command field");
    assert!(
        matches!(&panel.config.code, ToolChoice::Custom(CustomTool { project: Some(CommandTemplate(text)), .. }) if text == "edit {project}"),
        "the field the press left, drawn later in the walk, still committed on it: {:?}",
        panel.config.code
    );
}

#[test]
fn enter_with_a_press_on_the_same_frame_is_still_a_keyboard_commit() {
    let mut panel = Panel::with_code(custom("edit {file}"));
    panel.ui.focus_text_input(template_field_id(CODE_COMMAND), "edit {path}");
    panel.render();
    press_button(&mut panel.input, MouseButton::Left, Vec2::new(20.0, PANEL.height - 20.0));
    panel.input.keyboard_mut().handle_key_press(KeyCode::Enter);
    panel.render();

    assert_eq!(panel.state.draft.as_ref().map(|draft| draft.text.as_str()), Some("edit {path}"));
    assert!(panel.ui.is_focused(template_field_id(CODE_COMMAND)), "Enter keeps the field whatever the mouse did that frame");
}

#[test]
fn a_fixed_command_is_written_and_clears_the_draft() {
    let mut panel = Panel::with_code(custom("edit {file}"));
    panel.commit_code_command("edit --wait");
    assert_eq!(panel.state.draft.as_ref().map(|draft| draft.error.clone()), Some(DraftError::NamesNoFile));

    panel.commit_code_command("edit --wait {file}");
    assert_eq!(panel.config.code, custom("edit --wait {file}"));
    assert_eq!(panel.state.draft, None);
    assert!(!drawn_text(&panel.ui).contains("Error:"));
}

#[test]
fn escape_in_the_field_drops_the_draft_and_shows_the_saved_command() {
    let mut panel = Panel::with_code(custom("edit {file}"));
    panel.commit_code_command("edit {path}");
    assert!(panel.state.draft.is_some());

    panel.key(KeyCode::Escape);
    assert_eq!(panel.state.draft, None);
    assert_eq!(panel.config.code, custom("edit {file}"));
    let text = drawn_text(&panel.ui);
    assert!(text.contains("edit {file}") && !text.contains("Error:"), "{text}");
}

#[test]
fn the_cursors_control_wears_the_ring_only_while_the_panel_holds_the_keyboard() {
    let mut panel = Panel::with_code(ToolChoice::OsDefault);
    assert_eq!(panel.focus_rings(), 0);
    panel.state.holds_keyboard = true;
    panel.render();
    assert_eq!(panel.focus_rings(), 1);
    assert!(panel.state.control_count > ToolCategory::ALL.len() * 2 + FileType::ALL.len(), "every row is counted");
}
