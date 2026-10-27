//! Command templates: the text a tool row or a user writes to say how a program is
//! called, expanded into an argv without ever passing through a shell.
//!
//! The grammar is group-first. A template is a sequence of spans — bare or quoted
//! text, a `{placeholder}`, or an optional group `[ … ]` holding spans of its own,
//! nesting allowed. A group whose own placeholders all resolve expands to its spans;
//! one with any unresolved placeholder expands to nothing. Whitespace outside quotes
//! separates arguments, so `[--line {line}]` yields two and `{file}[:{line}]` one.
//! A placeholder's value is substituted whole and never re-split, so a path with
//! spaces needs no quoting.

use serde::{Deserialize, Serialize};

/// A template as written, kept verbatim so a bad one round-trips through the
/// preferences file unchanged and is refused, with its reason, when used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandTemplate(pub String);

/// The values a template may name. `file` is always present; the others are absent
/// when the open request does not carry them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaceholderValues {
    pub file: String,
    pub project: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub track: Option<String>,
    pub track_index: Option<usize>,
}

/// Why a template cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TemplateError {
    #[error("unknown placeholder {{{0}}} (the placeholders are {{file}}, {{project}}, {{line}}, {{column}}, {{track}} and {{track_index}})")]
    UnknownPlaceholder(String),
    #[error("a {{ is never closed")]
    UnclosedPlaceholder,
    #[error("a [ is never closed")]
    UnclosedGroup,
    #[error("a ] closes no group")]
    UnopenedGroup,
    #[error("a quote is never closed")]
    UnclosedQuote,
    #[error("the command names no program")]
    NoProgram,
    #[error("{{{0}}} is outside any [ … ] group and this open has no value for it")]
    Unresolved(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Placeholder {
    File,
    Project,
    Line,
    Column,
    Track,
    TrackIndex,
}

impl Placeholder {
    fn named(name: &str) -> Option<Placeholder> {
        match name {
            "file" => Some(Placeholder::File),
            "project" => Some(Placeholder::Project),
            "line" => Some(Placeholder::Line),
            "column" => Some(Placeholder::Column),
            "track" => Some(Placeholder::Track),
            "track_index" => Some(Placeholder::TrackIndex),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Placeholder::File => "file",
            Placeholder::Project => "project",
            Placeholder::Line => "line",
            Placeholder::Column => "column",
            Placeholder::Track => "track",
            Placeholder::TrackIndex => "track_index",
        }
    }

    fn value(self, values: &PlaceholderValues) -> Option<String> {
        match self {
            Placeholder::File => Some(values.file.clone()),
            Placeholder::Project => values.project.clone(),
            Placeholder::Line => values.line.map(|line| line.to_string()),
            Placeholder::Column => values.column.map(|column| column.to_string()),
            Placeholder::Track => values.track.clone(),
            Placeholder::TrackIndex => values.track_index.map(|index| index.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Span {
    /// Literal text, quotes already removed. An empty `Text` from `""` still makes an argument.
    Text(String),
    /// Unquoted whitespace: an argument boundary
    Break,
    Placeholder(Placeholder),
    Group(Vec<Span>),
}

impl CommandTemplate {
    /// Check the template the way the launcher will read it, so a settings panel can
    /// show the error before anything is launched.
    pub fn validate(&self) -> Result<(), TemplateError> {
        parse(&self.0).map(|_| ())
    }

    /// Whether the template names `{file}` anywhere, groups included.
    pub fn names_file(&self) -> bool {
        parse(&self.0).is_ok_and(|spans| names_placeholder(&spans, Placeholder::File))
    }

    /// The argv this template expands to; the first element is the program.
    pub fn expand(&self, values: &PlaceholderValues) -> Result<Vec<String>, TemplateError> {
        let spans = parse(&self.0)?;
        let mut builder = ArgvBuilder::default();
        expand_spans(&spans, values, &mut builder)?;
        let argv = builder.finish();
        if argv.first().is_none_or(|program| program.is_empty()) {
            return Err(TemplateError::NoProgram);
        }
        Ok(argv)
    }
}

fn names_placeholder(spans: &[Span], wanted: Placeholder) -> bool {
    spans.iter().any(|span| match span {
        Span::Placeholder(placeholder) => *placeholder == wanted,
        Span::Group(inner) => names_placeholder(inner, wanted),
        Span::Text(_) | Span::Break => false,
    })
}

fn parse(text: &str) -> Result<Vec<Span>, TemplateError> {
    let mut characters = text.chars().peekable();
    let spans = parse_spans(&mut characters, false)?;
    Ok(spans)
}

type Characters<'a> = std::iter::Peekable<std::str::Chars<'a>>;

fn parse_spans(characters: &mut Characters, inside_group: bool) -> Result<Vec<Span>, TemplateError> {
    let mut spans = Vec::new();
    while let Some(character) = characters.next() {
        match character {
            '[' => spans.push(Span::Group(parse_spans(characters, true)?)),
            ']' if inside_group => return Ok(spans),
            ']' => return Err(TemplateError::UnopenedGroup),
            '{' => spans.push(Span::Placeholder(parse_placeholder(characters)?)),
            '"' => parse_double_quoted(characters, &mut spans)?,
            '\'' => spans.push(Span::Text(parse_single_quoted(characters)?)),
            '\\' => push_text(&mut spans, characters.next().unwrap_or('\\')),
            character if character.is_whitespace() => {
                if spans.last() != Some(&Span::Break) {
                    spans.push(Span::Break);
                }
            }
            character => push_text(&mut spans, character),
        }
    }
    if inside_group {
        Err(TemplateError::UnclosedGroup)
    } else {
        Ok(spans)
    }
}

fn push_text(spans: &mut Vec<Span>, character: char) {
    if let Some(Span::Text(text)) = spans.last_mut() {
        text.push(character);
    } else {
        spans.push(Span::Text(character.to_string()));
    }
}

fn parse_placeholder(characters: &mut Characters) -> Result<Placeholder, TemplateError> {
    let mut name = String::new();
    for character in characters.by_ref() {
        if character == '}' {
            return Placeholder::named(&name).ok_or(TemplateError::UnknownPlaceholder(name));
        }
        name.push(character);
    }
    Err(TemplateError::UnclosedPlaceholder)
}

/// A double-quoted run keeps whitespace and brackets literal; a placeholder inside
/// it still expands, so `"--file={file}"` works as it would in a shell.
fn parse_double_quoted(characters: &mut Characters, spans: &mut Vec<Span>) -> Result<(), TemplateError> {
    spans.push(Span::Text(String::new()));
    while let Some(character) = characters.next() {
        match character {
            '"' => return Ok(()),
            '\\' => push_text(spans, characters.next().unwrap_or('\\')),
            '{' => spans.push(Span::Placeholder(parse_placeholder(characters)?)),
            character => push_text(spans, character),
        }
    }
    Err(TemplateError::UnclosedQuote)
}

fn parse_single_quoted(characters: &mut Characters) -> Result<String, TemplateError> {
    let mut text = String::new();
    for character in characters.by_ref() {
        if character == '\'' {
            return Ok(text);
        }
        text.push(character);
    }
    Err(TemplateError::UnclosedQuote)
}

#[derive(Default)]
struct ArgvBuilder {
    argv: Vec<String>,
    current: Option<String>,
}

impl ArgvBuilder {
    fn push_text(&mut self, text: &str) {
        self.current.get_or_insert_with(String::new).push_str(text);
    }

    fn break_argument(&mut self) {
        if let Some(argument) = self.current.take() {
            self.argv.push(argument);
        }
    }

    fn finish(mut self) -> Vec<String> {
        self.break_argument();
        self.argv
    }
}

fn expand_spans(spans: &[Span], values: &PlaceholderValues, builder: &mut ArgvBuilder) -> Result<(), TemplateError> {
    for span in spans {
        match span {
            Span::Text(text) => builder.push_text(text),
            Span::Break => builder.break_argument(),
            Span::Placeholder(placeholder) => {
                let value = placeholder.value(values).ok_or(TemplateError::Unresolved(placeholder.name()))?;
                builder.push_text(&value);
            }
            Span::Group(inner) => {
                if group_resolves(inner, values) {
                    expand_spans(inner, values, builder)?;
                }
            }
        }
    }
    Ok(())
}

/// A group counts only its own placeholders: a nested group drops on its own, so
/// `[:{line}[:{column}]]` keeps the line when only the column is missing.
fn group_resolves(spans: &[Span], values: &PlaceholderValues) -> bool {
    spans.iter().all(|span| match span {
        Span::Placeholder(placeholder) => placeholder.value(values).is_some(),
        Span::Text(_) | Span::Break | Span::Group(_) => true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(text: &str) -> CommandTemplate {
        CommandTemplate(text.to_string())
    }

    fn file_only(file: &str) -> PlaceholderValues {
        PlaceholderValues { file: file.to_string(), ..Default::default() }
    }

    fn at_line(file: &str, line: usize, column: Option<usize>) -> PlaceholderValues {
        PlaceholderValues { line: Some(line), column, ..file_only(file) }
    }

    #[test]
    fn test_a_path_with_spaces_survives_as_one_argument() {
        let argv = template("code --goto {file}").expand(&file_only("/home/me/My Game/main.rs"));
        assert_eq!(argv, Ok(vec!["code".to_string(), "--goto".to_string(), "/home/me/My Game/main.rs".to_string()]));
    }

    #[test]
    fn test_rustrovers_template_with_no_line_drops_both_groups() {
        let argv = template("rustrover [--line {line}] [--column {column}] {file}").expand(&file_only("/p/main.rs"));
        assert_eq!(argv, Ok(vec!["rustrover".to_string(), "/p/main.rs".to_string()]));
    }

    #[test]
    fn test_a_bare_group_with_its_value_becomes_two_arguments() {
        let argv = template("rustrover [--line {line}] [--column {column}] {file}").expand(&at_line("/p/main.rs", 42, None));
        assert_eq!(
            argv,
            Ok(vec!["rustrover".to_string(), "--line".to_string(), "42".to_string(), "/p/main.rs".to_string()])
        );
    }

    #[test]
    fn test_a_group_against_a_word_joins_that_word_and_nests_independently() {
        let goto = template("zed {file}[:{line}[:{column}]]");
        assert_eq!(goto.expand(&at_line("/p/main.rs", 42, None)), Ok(vec!["zed".to_string(), "/p/main.rs:42".to_string()]));
        assert_eq!(
            goto.expand(&at_line("/p/main.rs", 42, Some(10))),
            Ok(vec!["zed".to_string(), "/p/main.rs:42:10".to_string()])
        );
        assert_eq!(
            template("{file}[:{line}]").expand(&file_only("/p/main.rs")),
            Ok(vec!["/p/main.rs".to_string()]),
            "no line yields the bare file"
        );
    }

    #[test]
    fn test_a_quoted_program_path_is_one_program() {
        let argv = template(r#""/opt/My Tools/bin/editor" --new-window {file}"#).expand(&file_only("/p/a.rhai"));
        assert_eq!(
            argv,
            Ok(vec!["/opt/My Tools/bin/editor".to_string(), "--new-window".to_string(), "/p/a.rhai".to_string()])
        );
        let single = template("'C:\\Program Files\\Tool\\tool.exe' {file}").expand(&file_only("a.png"));
        assert_eq!(single, Ok(vec!["C:\\Program Files\\Tool\\tool.exe".to_string(), "a.png".to_string()]));
    }

    #[test]
    fn test_an_unknown_placeholder_is_refused_before_anything_expands() {
        assert_eq!(
            template("tool {filename}").validate(),
            Err(TemplateError::UnknownPlaceholder("filename".to_string()))
        );
        assert_eq!(template("tool [--line {line} {file}").validate(), Err(TemplateError::UnclosedGroup));
        assert_eq!(template("tool \"{file}").validate(), Err(TemplateError::UnclosedQuote));
        assert_eq!(template("tool ]").validate(), Err(TemplateError::UnopenedGroup));
    }

    #[test]
    fn test_a_placeholder_outside_a_group_without_a_value_is_an_error_not_a_blank() {
        assert_eq!(template("tool --line {line} {file}").expand(&file_only("a.rs")), Err(TemplateError::Unresolved("line")));
        assert_eq!(template("[{file}]").expand(&file_only("")), Err(TemplateError::NoProgram));
    }

    #[test]
    fn test_track_placeholders_expand_name_and_index() {
        let values = PlaceholderValues {
            project: Some("/game/assets".to_string()),
            track: Some("lead synth".to_string()),
            track_index: Some(3),
            ..file_only("/game/assets/music/theme.mid")
        };
        let argv = template("midi-evil [--project {project}] [--track {track}] [--track-index {track_index}] {file}")
            .expand(&values);
        assert_eq!(
            argv,
            Ok(vec![
                "midi-evil".to_string(),
                "--project".to_string(),
                "/game/assets".to_string(),
                "--track".to_string(),
                "lead synth".to_string(),
                "--track-index".to_string(),
                "3".to_string(),
                "/game/assets/music/theme.mid".to_string(),
            ])
        );
    }

    #[test]
    fn test_names_file_finds_a_file_placeholder_inside_a_group() {
        assert!(template("tool [--open {file}]").names_file());
        assert!(!template("code --wait").names_file());
    }
}
