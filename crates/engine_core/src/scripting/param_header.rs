//! Script parameter header parser and validation.

use std::collections::BTreeMap;

use glam::Vec2;

use ecs::script::ScriptValue;

use super::rhai_bindings::create_rhai_engine;
use super::runner::{ScriptError, ScriptErrorKind};

/// Parsed header block with owned default values.
pub fn parse_param_header(source: &str) -> Result<BTreeMap<String, ScriptValue>, ScriptError> {
    let mut params = BTreeMap::new();

    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with("//") {
            continue;
        }
        let comment = trimmed[2..].trim();
        if !comment.starts_with("@param") {
            continue;
        }
        let rest = comment["@param".len()..].trim();
        let colon_pos = rest.find(':').ok_or_else(|| ScriptError {
            file: "header".to_string(),
            line: line_index + 1,
            kind: ScriptErrorKind::Header,
            message: "Missing ':' in @param declaration".to_string(),
        })?;
        let name = rest[..colon_pos].trim();
        let after_colon = rest[colon_pos + 1..].trim();

        let eq_pos = after_colon.find('=').ok_or_else(|| ScriptError {
            file: "header".to_string(),
            line: line_index + 1,
            kind: ScriptErrorKind::Header,
            message: "Missing '=' in @param declaration".to_string(),
        })?;
        let param_type = after_colon[..eq_pos].trim().to_ascii_lowercase();
        let default_str = after_colon[eq_pos + 1..].trim();

        let value = match param_type.as_str() {
            "f32" => {
                let parsed: f32 = default_str.parse().map_err(|e| ScriptError {
                    file: "header".to_string(),
                    line: line_index + 1,
                    kind: ScriptErrorKind::Header,
                    message: format!("Invalid f32 literal '{default_str}': {e}"),
                })?;
                ScriptValue::F32(parsed)
            }
            "i32" => {
                let parsed: i32 = default_str.parse().map_err(|e| ScriptError {
                    file: "header".to_string(),
                    line: line_index + 1,
                    kind: ScriptErrorKind::Header,
                    message: format!("Invalid i32 literal '{default_str}': {e}"),
                })?;
                ScriptValue::I32(parsed)
            }
            "bool" => {
                let parsed: bool = default_str.parse().map_err(|e| ScriptError {
                    file: "header".to_string(),
                    line: line_index + 1,
                    kind: ScriptErrorKind::Header,
                    message: format!("Invalid bool literal '{default_str}': {e}"),
                })?;
                ScriptValue::Bool(parsed)
            }
            "str" => {
                let unquoted = default_str.trim_matches('"').trim_matches('\'');
                ScriptValue::Str(unquoted.to_string())
            }
            "vec2" => {
                let stripped = default_str
                    .trim_start_matches("vec2(")
                    .trim_start_matches('(')
                    .trim_end_matches(')');
                let parts: Vec<&str> = stripped.split(',').map(|s| s.trim()).collect();
                if parts.len() != 2 {
                    return Err(ScriptError {
                        file: "header".to_string(),
                        line: line_index + 1,
                        kind: ScriptErrorKind::Header,
                        message: format!("Invalid vec2 format '{default_str}'"),
                    });
                }
                let x: f32 = parts[0].parse().map_err(|e| ScriptError {
                    file: "header".to_string(),
                    line: line_index + 1,
                    kind: ScriptErrorKind::Header,
                    message: format!("Invalid vec2 x '{}': {e}", parts[0]),
                })?;
                let y: f32 = parts[1].parse().map_err(|e| ScriptError {
                    file: "header".to_string(),
                    line: line_index + 1,
                    kind: ScriptErrorKind::Header,
                    message: format!("Invalid vec2 y '{}': {e}", parts[1]),
                })?;
                ScriptValue::Vec2(Vec2::new(x, y))
            }
            "entity" => {
                let unquoted = default_str.trim_matches('"').trim_matches('\'');
                ScriptValue::Str(unquoted.to_string())
            }
            "color" => {
                let stripped = default_str
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .trim_start_matches('(')
                    .trim_end_matches(')');
                let parts: Vec<&str> = stripped.split(',').map(|s| s.trim()).collect();
                if parts.len() != 4 {
                    return Err(ScriptError {
                        file: "header".to_string(),
                        line: line_index + 1,
                        kind: ScriptErrorKind::Header,
                        message: format!("Invalid color format '{default_str}'"),
                    });
                }
                let mut c = [0.0f32; 4];
                for (i, p) in parts.iter().enumerate() {
                    c[i] = p.parse().map_err(|e| ScriptError {
                        file: "header".to_string(),
                        line: line_index + 1,
                        kind: ScriptErrorKind::Header,
                        message: format!("Invalid color channel '{p}': {e}"),
                    })?;
                }
                ScriptValue::Color(c)
            }
            other => {
                return Err(ScriptError {
                    file: "header".to_string(),
                    line: line_index + 1,
                    kind: ScriptErrorKind::Header,
                    message: format!("Unknown param type '{other}'"),
                });
            }
        };

        params.insert(name.to_string(), value);
    }

    Ok(params)
}

/// Pure syntax and header check used by the web playground on save.
pub fn check_source(text: &str) -> Result<(), ScriptError> {
    parse_param_header(text)?;
    let engine = create_rhai_engine();
    engine.compile(text).map_err(|e| {
        let line = e.position().line().unwrap_or(0);
        ScriptError {
            file: "source".to_string(),
            line,
            kind: ScriptErrorKind::Syntax,
            message: e.to_string(),
        }
    })?;
    Ok(())
}
