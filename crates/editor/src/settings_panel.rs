//! The External Tools panel: which program opens which project file, edited in place.
//!
//! A heading per category with its Tool cycle — the system default, the category's
//! presets, a custom command — and under it the chosen tool's fields: a preset's Program
//! (blank keeps the preset's own), a custom tool's Command and, for Code, its Project
//! command; then a Test button. Then an override row per file type, with the same fields,
//! and the DAW project extensions. Every edit writes the config at once; the host's
//! preferences settle rule is what persists it.
//!
//! A command that fails its check is never written over the last good one: the text
//! stays shown in the field as the draft and a row under it says "Error: …" in words. A
//! failed Enter keeps the keyboard in the field, since the typist's next key is the fix;
//! a failed press away lets it go, since the person left — a re-focus there would hand
//! every press back to the field. A click back into the field edits the draft, and Escape
//! in it drops the draft and shows the saved value. A custom tool a row cycles away from
//! is remembered for the session, so cycling back restores it.
//!
//! The keyboard is on the model of the Conductor panel's sidebar: the walk counts every control,
//! the arrows move a cursor over them and the cursor's control wears the focus ring while the
//! panel holds the keyboard; Enter acts on it and Left and Right step a cycle row. A key
//! asks through [`SettingsRequest`], which the next render carries out, since only the
//! walk knows which control sits at the cursor. The host routes the keys
//! (`editor_integration`'s `editor_game/settings_keys.rs`).

mod rows;
mod walk;

use ui::{Rect, UIContext, WidgetId};

use crate::external_tools::{CustomTool, ExternalToolsConfig, FileType, ToolCategory, ToolChoice, DESKTOP_ONLY_HINT};
use crate::field_style::FieldId;
use crate::theme::EditorTheme;
use crate::ScrollState;

pub use rows::{
    check_template, choose, choose_remembering, commit_template, current_option, cycle_values, extensions_text, option_name,
    parse_extensions, row_choice, template_text, write_program, DraftError, TemplateSlot, ToolOption, ToolRow,
};
use walk::SettingsWalk;

/// The widget-id space the panel owns, past the inspector's component blocks and the
/// colour editor's.
const SETTINGS_COMPONENT_SLOT: usize = 920;
/// The DAW extensions field's block, after the four categories' and the eight overrides'.
const EXTENSIONS_BLOCK: usize = 12;
const CYCLE_FIELD: usize = 0;
const PROGRAM_FIELD: usize = 1;
const COMMAND_FIELD: usize = 2;
const PROJECT_FIELD: usize = 3;

/// What a key asked of the control at the cursor, carried out by the next render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsRequest {
    /// Enter: a button presses, a field takes the keyboard, a cycle steps forward
    Activate,
    /// Left or Right: a cycle row steps; a field or a button ignores it
    Cycle { forward: bool },
}

/// A command typed into a field that failed its check, kept until it passes or Escape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateDraft {
    pub slot: TemplateSlot,
    pub text: String,
    pub error: DraftError,
}

/// The panel's state (a field on `EditorContext`).
#[derive(Debug, Default)]
pub struct SettingsPanelState {
    pub holds_keyboard: bool,
    /// The control the arrows walk, by its index in the walk
    pub cursor: usize,
    /// How many controls the last render counted
    pub control_count: usize,
    pub scroll: ScrollState,
    /// Scroll the cursor's control wholly into view at the next render
    pub reveal: bool,
    pub request: Option<SettingsRequest>,
    pub draft: Option<TemplateDraft>,
    /// The custom tool each row cycled away from this session, restored when the row
    /// comes back to Custom
    pub kept_custom: Vec<(ToolRow, CustomTool)>,
    /// The categories whose Test was pressed; the host drains them with the open requests
    pub tool_tests: Vec<ToolCategory>,
}

impl SettingsPanelState {
    pub fn release_keyboard(&mut self) {
        self.holds_keyboard = false;
        self.request = None;
    }

    /// Move the cursor one control, held inside the panel, and reveal it.
    pub fn step_cursor(&mut self, up: bool) {
        let Some(last) = self.control_count.checked_sub(1) else {
            return;
        };
        self.cursor = if up { self.cursor.saturating_sub(1) } else { (self.cursor + 1).min(last) };
        self.reveal = true;
    }
}

/// The field id of `field` in a row's block of ids.
fn field_id(block: usize, field: usize) -> FieldId {
    FieldId::new(SETTINGS_COMPONENT_SLOT + block, field, 0)
}

fn row_block(row: ToolRow) -> usize {
    match row {
        ToolRow::Category(category) => ToolCategory::ALL.iter().position(|listed| *listed == category).unwrap_or(0),
        ToolRow::Override(file_type) => ToolCategory::ALL.len() + FileType::ALL.iter().position(|listed| *listed == file_type).unwrap_or(0),
    }
}

/// The widget id of a template field, which a host or a test focuses.
pub fn template_field_id(slot: TemplateSlot) -> FieldId {
    let field = match slot {
        TemplateSlot::Command(_) => COMMAND_FIELD,
        TemplateSlot::ProjectCommand(_) => PROJECT_FIELD,
    };
    field_id(row_block(slot.row()), field)
}

/// The Test button of `category`.
pub fn test_button_id(category: ToolCategory) -> WidgetId {
    WidgetId::from_str_index("settings_test", row_block(ToolRow::Category(category)))
}

/// Draw the panel into `bounds`, writing every edit into `config`. `desktop` is false on
/// the web, where Test draws disabled over the desktop-only hint. Returns whether the DAW
/// project extensions changed, which reclassifies the asset browser's entries.
pub fn render_settings_panel(
    ui: &mut UIContext,
    theme: &EditorTheme,
    state: &mut SettingsPanelState,
    config: &mut ExternalToolsConfig,
    bounds: Rect,
    desktop: bool,
) -> bool {
    let request = state.request.take();
    let offset = state.scroll.begin_frame(bounds, ui.mouse_pos(), ui.scroll_delta(), bounds.height);
    let mut walk = SettingsWalk::new(ui, theme, bounds, offset, state, request);
    for category in ToolCategory::ALL {
        walk.heading(category.name());
        tool_rows(&mut walk, state, config, ToolRow::Category(category));
        let hint = (!desktop).then_some(DESKTOP_ONLY_HINT);
        if walk.button(test_button_id(category), "Test", desktop, hint) {
            state.tool_tests.push(category);
        }
    }
    walk.heading("Overrides");
    for file_type in FileType::ALL {
        tool_rows(&mut walk, state, config, ToolRow::Override(file_type));
    }
    walk.heading("DAW project extensions");
    let extensions = extensions_text(config);
    let outcome = walk.field(field_id(EXTENSIONS_BLOCK, COMMAND_FIELD), "Extensions", &extensions, None, |_| true);
    let mut extensions_changed = false;
    if let Some(text) = outcome.committed {
        let parsed = parse_extensions(&text);
        extensions_changed = parsed != config.daw_project_extensions;
        config.daw_project_extensions = parsed;
    }
    let end = walk.finish();

    // The cursor's control with the note under it: a revealed field shows its error too.
    // A reveal the walk asked for is applied now, from the walk's own measure, and once
    // more at the next render from the drawn control: a draft dropped or taken on the
    // same frame moves every row under it, and the first measure is then stale.
    if let (true, Some(top), Some(bottom)) = (state.reveal || end.reveal, end.cursor_top, end.cursor_bottom) {
        let offset = state.scroll.offset();
        if top < offset {
            state.scroll.scroll_to(top);
        } else if bottom > offset + bounds.height {
            state.scroll.scroll_to(bottom - bounds.height);
        }
    }
    state.reveal = end.reveal;
    state.scroll.end_frame(end.content_height, bounds.height);
    state.control_count = end.control_count;
    state.cursor = end.cursor.min(end.control_count.saturating_sub(1));
    extensions_changed
}

/// A tool row's cycle, then the fields its choice has.
fn tool_rows(walk: &mut SettingsWalk, state: &mut SettingsPanelState, config: &mut ExternalToolsConfig, row: ToolRow) {
    let SettingsPanelState { draft, kept_custom, .. } = state;
    let block = row_block(row);
    let values = cycle_values(config, row);
    let names: Vec<String> = values.iter().map(|value| option_name(config, row, *value)).collect();
    let current = current_option(config, row);
    let index = values.iter().position(|value| *value == current).unwrap_or(0);
    let label = match row {
        ToolRow::Category(_) => "Tool",
        ToolRow::Override(file_type) => file_type.name(),
    };
    let chosen = walk.cycle(field_id(block, CYCLE_FIELD), label, &names, index);
    // The fields draw before the cycle's choice is applied: a press on the cycle is a
    // press away from a focused field, whose commit must land this frame — the next
    // frame draws no field for the new choice, and the ui keeps a focus until the field
    // is drawn. Applied first, the choice hid the field with its text unsaved and the
    // keyboard held by a widget that never came back.
    row_fields(walk, draft, config, row, block);
    if let Some(chosen) = chosen {
        if let Some(value) = values.get(chosen) {
            choose_remembering(config, row, *value, kept_custom);
        }
        if draft.as_ref().is_some_and(|held| held.slot.row() == row) {
            *draft = None;
        }
        let row_field_focused =
            [PROGRAM_FIELD, COMMAND_FIELD, PROJECT_FIELD].into_iter().any(|field| walk.ui.is_focused(field_id(block, field)));
        if row_field_focused {
            walk.ui.clear_text_focus();
        }
    }
}

/// The fields the row's choice has: a preset's Program, a custom tool's Command and, for
/// the Code category, its Project command; none for the system default or no entry.
fn row_fields(walk: &mut SettingsWalk, draft: &mut Option<TemplateDraft>, config: &mut ExternalToolsConfig, row: ToolRow, block: usize) {
    match row_choice(config, row).cloned() {
        Some(ToolChoice::Preset(preset)) => {
            let program = config.programs.get(&preset).cloned().unwrap_or_default();
            let outcome = walk.field(field_id(block, PROGRAM_FIELD), "Program", &program, None, |_| true);
            let own = preset.preset().file_template.split_whitespace().next().unwrap_or_default();
            walk.tooltip(&format!("A path or a program name; blank runs {}'s own, {own}", preset.preset().name));
            if let Some(text) = outcome.committed {
                write_program(config, preset, &text);
            }
        }
        Some(ToolChoice::Custom(_)) => {
            template_field(walk, draft, config, TemplateSlot::Command(row));
            if row == ToolRow::Category(ToolCategory::Code) {
                template_field(walk, draft, config, TemplateSlot::ProjectCommand(ToolCategory::Code));
            }
        }
        Some(ToolChoice::OsDefault) | None => {}
    }
}

/// A command field: its commit written when it passes the check, else kept as the draft
/// shown in the field with the error under it. A failed Enter re-focuses the field on
/// the draft, so the keys that fix it stay in the field instead of reaching the editor's
/// shortcuts; a failed press away does not — re-focused, every press away would commit,
/// fail and take the keyboard back, a trap that held the game's keys when the press was
/// on Play.
fn template_field(walk: &mut SettingsWalk, draft: &mut Option<TemplateDraft>, config: &mut ExternalToolsConfig, slot: TemplateSlot) {
    let id = template_field_id(slot);
    let saved = template_text(config, slot).unwrap_or_default();
    let drafted = draft.as_ref().filter(|held| held.slot == slot).map(|held| held.text.clone());
    let label = match slot {
        TemplateSlot::Command(_) => "Command",
        TemplateSlot::ProjectCommand(_) => "Project command",
    };
    let outcome = walk.field(id, label, &saved, drafted.as_deref(), |text| check_template(slot, text).is_ok());
    let holds_this = |draft: &Option<TemplateDraft>| draft.as_ref().is_some_and(|held| held.slot == slot);
    if let Some(text) = outcome.committed {
        match commit_template(config, slot, &text) {
            Ok(()) if holds_this(draft) => *draft = None,
            Ok(()) => {}
            Err(error) => {
                if outcome.committed_by_key {
                    walk.ui.focus_text_input(id, &text);
                    walk.reveal_control(outcome.control);
                }
                *draft = Some(TemplateDraft { slot, text, error });
            }
        }
    } else if outcome.cancelled && holds_this(draft) {
        *draft = None;
    }
    if let Some(held) = draft.as_ref().filter(|held| held.slot == slot) {
        walk.error(&format!("Error: {}", held.error));
    }
}

#[cfg(test)]
mod tests;
