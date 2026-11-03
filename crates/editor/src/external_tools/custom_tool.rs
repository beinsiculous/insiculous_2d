//! A tool the user writes the command line for: how it opens a file, and how it opens
//! the project, when it can.

use serde::{Deserialize, Serialize};

use super::template::CommandTemplate;

/// A custom tool's two commands. Written as the struct; read from the struct or from a
/// bare string, which an older preferences file wrote for the file's command alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "CustomToolShape")]
pub struct CustomTool {
    /// How a file opens; it names `{file}`
    pub file: CommandTemplate,
    /// How the project opens (Edit → Open Project); `None` leaves the action refused
    pub project: Option<CommandTemplate>,
}

impl CustomTool {
    /// A tool with a file command and no project command.
    pub fn file_only(file: CommandTemplate) -> Self {
        Self { file, project: None }
    }
}

/// Either shape a preferences file holds a custom tool in.
#[derive(Deserialize)]
#[serde(untagged)]
enum CustomToolShape {
    FileOnly(CommandTemplate),
    Both {
        file: CommandTemplate,
        #[serde(default)]
        project: Option<CommandTemplate>,
    },
}

impl From<CustomToolShape> for CustomTool {
    fn from(shape: CustomToolShape) -> Self {
        match shape {
            CustomToolShape::FileOnly(file) => CustomTool::file_only(file),
            CustomToolShape::Both { file, project } => CustomTool { file, project },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::config::{ExternalToolsConfig, ToolChoice};
    use super::*;

    #[test]
    fn test_a_bare_custom_string_from_an_older_prefs_file_loads_as_a_tool_without_a_project_command() -> Result<(), serde_json::Error> {
        let older: ExternalToolsConfig = serde_json::from_str(r#"{"code": {"Custom": "code --wait {file}"}}"#)?;
        assert_eq!(older.code, ToolChoice::Custom(CustomTool::file_only(CommandTemplate("code --wait {file}".to_string()))));

        let current: ExternalToolsConfig =
            serde_json::from_str(r#"{"code": {"Custom": {"file": "code --goto {file}", "project": "code {project}"}}}"#)?;
        let both = CustomTool {
            file: CommandTemplate("code --goto {file}".to_string()),
            project: Some(CommandTemplate("code {project}".to_string())),
        };
        assert_eq!(current.code, ToolChoice::Custom(both.clone()));

        let written = serde_json::to_string(&ToolChoice::Custom(both.clone()))?;
        assert_eq!(serde_json::from_str::<ToolChoice>(&written)?, ToolChoice::Custom(both), "the struct is what is written, and it reads back");
        Ok(())
    }
}
