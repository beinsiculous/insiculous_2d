//! The settings panel's rows as data, pure over the config: which values a tool row
//! cycles over and what each is called, which one the config holds, what choosing one
//! writes, and what a typed template, program or extension list writes. The walk draws
//! these and nothing else decides them.

use crate::external_tools::{
    CommandTemplate, CustomTool, ExternalToolsConfig, FileType, PresetId, TemplateError, ToolCategory, ToolChoice,
    ToolName, PRESETS,
};

/// A tool row: a category's own choice, or a file type's override of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRow {
    Category(ToolCategory),
    Override(FileType),
}

impl ToolRow {
    /// The category whose presets the row offers and whose tool an override inherits.
    pub fn category(self) -> ToolCategory {
        match self {
            ToolRow::Category(category) => category,
            ToolRow::Override(file_type) => file_type.category(),
        }
    }
}

/// A template field: a row's command, or a code category's project command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateSlot {
    Command(ToolRow),
    ProjectCommand(ToolCategory),
}

impl TemplateSlot {
    /// The tool row the field sits under.
    pub fn row(self) -> ToolRow {
        match self {
            TemplateSlot::Command(row) => row,
            TemplateSlot::ProjectCommand(category) => ToolRow::Category(category),
        }
    }
}

/// Why a typed template was not written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DraftError {
    #[error("{0}")]
    Template(TemplateError),
    #[error("the command never names {{file}}, so the file would not reach the tool")]
    NamesNoFile,
}

/// One value a tool row cycles over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolOption {
    /// An override's: no entry, so the category's tool opens the type
    Inherit,
    SystemDefault,
    Preset(PresetId),
    Custom,
}

/// The values `row` cycles over, in order. A category: "System default", its presets in
/// the table's order, "Custom command". An override: "Inherit (…)", the presets, "Custom
/// command", "System default". A preset of another category the config holds (a hand
/// edit) is offered last, so the row names what is there rather than something else.
pub fn cycle_values(config: &ExternalToolsConfig, row: ToolRow) -> Vec<ToolOption> {
    let presets = PRESETS.iter().filter(|preset| preset.category == row.category()).map(|preset| ToolOption::Preset(preset.id));
    let mut values: Vec<ToolOption> = match row {
        ToolRow::Category(_) => std::iter::once(ToolOption::SystemDefault).chain(presets).chain([ToolOption::Custom]).collect(),
        ToolRow::Override(_) => std::iter::once(ToolOption::Inherit)
            .chain(presets)
            .chain([ToolOption::Custom, ToolOption::SystemDefault])
            .collect(),
    };
    let current = current_option(config, row);
    if !values.contains(&current) {
        values.push(current);
    }
    values
}

/// What the row shows for `option`; an override's Inherit names the category's tool.
pub fn option_name(config: &ExternalToolsConfig, row: ToolRow, option: ToolOption) -> String {
    match option {
        ToolOption::Inherit => format!("Inherit ({})", ToolName::of(config.category_choice(row.category())).subject()),
        ToolOption::SystemDefault => "System default".to_string(),
        ToolOption::Preset(id) => id.preset().name.to_string(),
        ToolOption::Custom => "Custom command".to_string(),
    }
}

/// The row's own choice: `None` for an override with no entry.
pub fn row_choice(config: &ExternalToolsConfig, row: ToolRow) -> Option<&ToolChoice> {
    match row {
        ToolRow::Category(category) => Some(config.category_choice(category)),
        ToolRow::Override(file_type) => config.overrides.get(&file_type),
    }
}

/// The value the config holds for `row`.
pub fn current_option(config: &ExternalToolsConfig, row: ToolRow) -> ToolOption {
    match row_choice(config, row) {
        None => ToolOption::Inherit,
        Some(ToolChoice::Preset(id)) => ToolOption::Preset(*id),
        Some(ToolChoice::Custom(_)) => ToolOption::Custom,
        Some(ToolChoice::OsDefault) => ToolOption::SystemDefault,
    }
}

/// Write `option` as `row`'s choice. Custom keeps a custom tool already chosen, its
/// commands with it, and otherwise starts one with an empty command; an override's
/// Inherit removes its entry, and every other value writes one, the system default
/// included.
pub fn choose(config: &mut ExternalToolsConfig, row: ToolRow, option: ToolOption) {
    let chosen = match option {
        ToolOption::Inherit => None,
        ToolOption::SystemDefault => Some(ToolChoice::OsDefault),
        ToolOption::Preset(id) => Some(ToolChoice::Preset(id)),
        ToolOption::Custom => match row_choice(config, row) {
            Some(custom @ ToolChoice::Custom(_)) => Some(custom.clone()),
            _ => Some(ToolChoice::Custom(CustomTool::file_only(CommandTemplate(String::new())))),
        },
    };
    match (row, chosen) {
        (ToolRow::Category(category), Some(choice)) => *category_choice_mut(config, category) = choice,
        (ToolRow::Category(_), None) => {}
        (ToolRow::Override(file_type), Some(choice)) => {
            config.overrides.insert(file_type, choice);
        }
        (ToolRow::Override(file_type), None) => {
            config.overrides.remove(&file_type);
        }
    }
}

/// [`choose`] with a memory: the custom tool a row cycles away from is kept in `kept`,
/// and Custom chosen again on that row brings it back, commands and all, instead of
/// starting one empty — stepping through the cycle to look is no edit of the command.
/// The memory is the session's; what the row holds when the session ends is what is saved.
pub fn choose_remembering(config: &mut ExternalToolsConfig, row: ToolRow, option: ToolOption, kept: &mut Vec<(ToolRow, CustomTool)>) {
    let was_custom = matches!(row_choice(config, row), Some(ToolChoice::Custom(_)));
    if was_custom && option != ToolOption::Custom {
        if let Some(ToolChoice::Custom(tool)) = row_choice(config, row) {
            let tool = tool.clone();
            kept.retain(|(kept_row, _)| *kept_row != row);
            kept.push((row, tool));
        }
    }
    choose(config, row, option);
    if !was_custom && option == ToolOption::Custom {
        if let Some(position) = kept.iter().position(|(kept_row, _)| *kept_row == row) {
            let (_, tool) = kept.remove(position);
            if let Some(custom) = custom_tool_mut(config, TemplateSlot::Command(row)) {
                *custom = tool;
            }
        }
    }
}

fn category_choice_mut(config: &mut ExternalToolsConfig, category: ToolCategory) -> &mut ToolChoice {
    match category {
        ToolCategory::Code => &mut config.code,
        ToolCategory::Music => &mut config.music,
        ToolCategory::Sample => &mut config.sample,
        ToolCategory::Art => &mut config.art,
    }
}

/// The custom tool `slot`'s field edits, while its row holds one.
fn custom_tool_mut(config: &mut ExternalToolsConfig, slot: TemplateSlot) -> Option<&mut CustomTool> {
    let choice = match slot.row() {
        ToolRow::Category(category) => Some(category_choice_mut(config, category)),
        ToolRow::Override(file_type) => config.overrides.get_mut(&file_type),
    };
    match choice {
        Some(ToolChoice::Custom(tool)) => Some(tool),
        _ => None,
    }
}

/// The saved text of `slot`'s field: empty for a project command not set, `None` while
/// the row holds no custom tool.
pub fn template_text(config: &ExternalToolsConfig, slot: TemplateSlot) -> Option<String> {
    let Some(ToolChoice::Custom(tool)) = row_choice(config, slot.row()) else {
        return None;
    };
    Some(match slot {
        TemplateSlot::Command(_) => tool.file.0.clone(),
        TemplateSlot::ProjectCommand(_) => tool.project.as_ref().map(|project| project.0.clone()).unwrap_or_default(),
    })
}

/// The template `text` writes into `slot`, or why it cannot: a command must parse and
/// name `{file}`; a project command must parse, and blank is no project command.
pub fn check_template(slot: TemplateSlot, text: &str) -> Result<Option<CommandTemplate>, DraftError> {
    let template = CommandTemplate(text.to_string());
    if matches!(slot, TemplateSlot::ProjectCommand(_)) && text.trim().is_empty() {
        return Ok(None);
    }
    template.validate().map_err(DraftError::Template)?;
    if matches!(slot, TemplateSlot::Command(_)) && !template.names_file() {
        return Err(DraftError::NamesNoFile);
    }
    Ok(Some(template))
}

/// Write `text` into `slot` when it passes [`check_template`]; a failing one leaves the
/// config as it was.
pub fn commit_template(config: &mut ExternalToolsConfig, slot: TemplateSlot, text: &str) -> Result<(), DraftError> {
    let template = check_template(slot, text)?;
    if let Some(tool) = custom_tool_mut(config, slot) {
        match (slot, template) {
            (TemplateSlot::Command(_), Some(template)) => tool.file = template,
            (TemplateSlot::Command(_), None) => {}
            (TemplateSlot::ProjectCommand(_), project) => tool.project = project,
        }
    }
    Ok(())
}

/// Set a preset's program; blank removes the entry, so the preset's own runs.
pub fn write_program(config: &mut ExternalToolsConfig, preset: PresetId, text: &str) {
    let program = text.trim();
    if program.is_empty() {
        config.programs.remove(&preset);
    } else {
        config.programs.insert(preset, program.to_string());
    }
}

/// The DAW project extensions as the field shows them.
pub fn extensions_text(config: &ExternalToolsConfig) -> String {
    config.daw_project_extensions.join(" ")
}

/// The extensions a typed list names: split on whitespace and commas, each kept as typed
/// (the dot is optional where the list is read); empty is an empty list.
pub fn parse_extensions(text: &str) -> Vec<String> {
    text.split(|character: char| character.is_whitespace() || character == ',')
        .filter(|extension| !extension.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(config: &ExternalToolsConfig, row: ToolRow) -> Vec<String> {
        cycle_values(config, row).into_iter().map(|option| option_name(config, row, option)).collect()
    }

    #[test]
    fn a_category_cycles_the_system_default_its_presets_and_custom_and_an_override_inherits_first() {
        let mut config = ExternalToolsConfig::default();
        assert_eq!(
            names(&config, ToolRow::Category(ToolCategory::Code)),
            ["System default", "RustRover", "VS Code", "VSCodium", "Zed", "Helix", "Custom command"]
        );
        assert_eq!(names(&config, ToolRow::Category(ToolCategory::Art)), ["System default", "Aseprite", "Custom command"]);
        config.music = ToolChoice::Preset(PresetId::MidiEvil);
        assert_eq!(
            names(&config, ToolRow::Override(FileType::SoundFont)),
            ["Inherit (MIDI Evil)", "MIDI Evil", "Custom command", "System default"]
        );
    }

    #[test]
    fn choosing_custom_keeps_a_chosen_custom_tools_commands_and_inherit_removes_an_override() {
        let mut config = ExternalToolsConfig::default();
        let code = ToolRow::Category(ToolCategory::Code);
        choose(&mut config, code, ToolOption::Custom);
        assert_eq!(config.code, ToolChoice::Custom(CustomTool::file_only(CommandTemplate(String::new()))));
        commit_template(&mut config, TemplateSlot::Command(code), "edit {file}").expect("a good template");
        choose(&mut config, code, ToolOption::Custom);
        assert_eq!(template_text(&config, TemplateSlot::Command(code)).as_deref(), Some("edit {file}"), "a chosen Custom keeps its text");
        choose(&mut config, code, ToolOption::Preset(PresetId::Zed));
        assert_eq!(config.code, ToolChoice::Preset(PresetId::Zed));

        let images = ToolRow::Override(FileType::Image);
        choose(&mut config, images, ToolOption::SystemDefault);
        assert_eq!(config.overrides.get(&FileType::Image), Some(&ToolChoice::OsDefault), "an explicit system default is an entry");
        assert_eq!(current_option(&config, images), ToolOption::SystemDefault);
        choose(&mut config, images, ToolOption::Inherit);
        assert!(config.overrides.is_empty(), "Inherit removes the entry");
    }

    #[test]
    fn cycling_away_from_custom_and_back_restores_its_commands_within_the_session() {
        let mut config = ExternalToolsConfig::default();
        let code = ToolRow::Category(ToolCategory::Code);
        let mut kept = Vec::new();
        choose_remembering(&mut config, code, ToolOption::Custom, &mut kept);
        commit_template(&mut config, TemplateSlot::Command(code), "edit {file}").expect("a good template");
        commit_template(&mut config, TemplateSlot::ProjectCommand(ToolCategory::Code), "edit {project}").expect("a project command");
        let written = config.code.clone();

        choose_remembering(&mut config, code, ToolOption::Preset(PresetId::Zed), &mut kept);
        assert_eq!(config.code, ToolChoice::Preset(PresetId::Zed));
        choose_remembering(&mut config, code, ToolOption::SystemDefault, &mut kept);
        choose_remembering(&mut config, code, ToolOption::Custom, &mut kept);
        assert_eq!(config.code, written, "both commands come back");
        assert!(kept.is_empty(), "the memory is spent once restored");

        let images = ToolRow::Override(FileType::Image);
        choose_remembering(&mut config, images, ToolOption::Custom, &mut kept);
        assert_eq!(template_text(&config, TemplateSlot::Command(images)).as_deref(), Some(""), "a row never custom starts empty");
    }

    #[test]
    fn a_failing_template_leaves_the_config_and_a_project_command_may_be_blank() {
        let mut config = ExternalToolsConfig::default();
        let code = ToolRow::Category(ToolCategory::Code);
        choose(&mut config, code, ToolOption::Custom);
        commit_template(&mut config, TemplateSlot::Command(code), "edit {file}").expect("a good template");
        let before = config.clone();
        assert_eq!(
            commit_template(&mut config, TemplateSlot::Command(code), "edit {path}"),
            Err(DraftError::Template(TemplateError::UnknownPlaceholder("path".to_string())))
        );
        assert_eq!(commit_template(&mut config, TemplateSlot::Command(code), "edit --wait"), Err(DraftError::NamesNoFile));
        assert_eq!(config, before, "neither failure wrote anything");

        let project = TemplateSlot::ProjectCommand(ToolCategory::Code);
        commit_template(&mut config, project, "edit {project}").expect("a project command");
        assert_eq!(template_text(&config, project).as_deref(), Some("edit {project}"));
        commit_template(&mut config, project, "  ").expect("blank is no project command");
        assert!(matches!(&config.code, ToolChoice::Custom(CustomTool { project: None, .. })));
    }

    #[test]
    fn a_blank_program_removes_the_entry_and_extensions_split_on_spaces_and_commas() {
        let mut config = ExternalToolsConfig::default();
        write_program(&mut config, PresetId::MidiEvil, " /opt/midi-evil ");
        assert_eq!(config.programs.get(&PresetId::MidiEvil).map(String::as_str), Some("/opt/midi-evil"));
        write_program(&mut config, PresetId::MidiEvil, "");
        assert!(config.programs.is_empty());

        assert_eq!(parse_extensions(".evil, prj  .NSP,,"), [".evil", "prj", ".NSP"]);
        assert!(parse_extensions(" , ").is_empty());
        assert_eq!(extensions_text(&ExternalToolsConfig::default()), ".evil .prj .nsp");
    }
}
