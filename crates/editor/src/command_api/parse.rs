//! Request-line parsing: whitespace-separated tokens, double quotes for
//! tokens containing spaces (no escape sequences — the protocol stays
//! boring). Stage B: `set`/`add` take the REST OF THE LINE after their
//! fixed tokens as raw JSON, so JSON containing spaces or quotes never
//! meets the tokenizer.

use super::{ApiError, EntityRef, HostedWrite, PureWrite, Query, Request, WriteCmd};
use crate::archetype::Archetype;

/// Spawnable archetypes for the `create` verb, derived from
/// [`Archetype::ALL`] at compile time so the two lists cannot drift.
pub const ARCHETYPES: [&str; ARCHETYPE_COUNT] = {
    let mut names = [""; ARCHETYPE_COUNT];
    let mut index = 0;
    while index < ARCHETYPE_COUNT {
        names[index] = Archetype::ALL[index].kebab();
        index += 1;
    }
    names
};

const ARCHETYPE_COUNT: usize = Archetype::ALL.len();

/// Every verb the parser accepts (queries + writes); `specs` mirrors this
/// list for `commands` discovery — a drift test keeps them equal.
pub(super) const VERBS: [&str; 16] = [
    "list", "describe", "selection", "scene", "commands",
    "set", "add", "remove", "rename", "create", "delete", "select",
    "undo", "redo", "save", "batch",
];

/// Split a line into tokens; `"quoted strings"` become one token.
fn tokens(line: &str) -> Result<Vec<String>, ApiError> {
    let mut out = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '"' {
            chars.next();
            let mut token = String::new();
            loop {
                match chars.next() {
                    Some('"') => break,
                    Some(ch) => token.push(ch),
                    None => return Err(ApiError::Parse("unterminated quote".to_string())),
                }
            }
            out.push(token);
        } else {
            let mut token = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_whitespace() {
                    break;
                }
                token.push(ch);
                chars.next();
            }
            out.push(token);
        }
    }
    Ok(out)
}

/// Split off the first `n` whitespace tokens (quote-aware) and return them
/// with the untouched remainder of the line — how `set`/`add` carry raw
/// JSON that must never meet the tokenizer.
fn split_fixed_tokens(line: &str, n: usize) -> Result<(Vec<String>, &str), ApiError> {
    let mut out = Vec::new();
    let mut rest = line;
    for _ in 0..n {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        if let Some(inner) = rest.strip_prefix('"') {
            let close = inner
                .find('"')
                .ok_or_else(|| ApiError::Parse("unterminated quote".to_string()))?;
            out.push(inner[..close].to_string());
            rest = &inner[close + 1..];
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            out.push(rest[..end].to_string());
            rest = &rest[end..];
        }
    }
    Ok((out, rest.trim()))
}

fn parse_json_rest(rest: &str, verb: &str) -> Result<serde_json::Value, ApiError> {
    serde_json::from_str(rest)
        .map_err(|e| ApiError::Parse(format!("{verb} needs valid JSON after the component: {e}")))
}

fn entity_ref(token: &str) -> Result<EntityRef, ApiError> {
    if let Some(digits) = token.strip_prefix('#') {
        let id = digits
            .parse::<u64>()
            .map_err(|_| ApiError::Parse(format!("\"{token}\" is not a valid #id")))?;
        Ok(EntityRef::Id(id))
    } else {
        Ok(EntityRef::Name(token.to_string()))
    }
}

fn parse_set(line: &str) -> Result<Request, ApiError> {
    let (fixed, rest) = split_fixed_tokens(line, 3)?;
    if fixed.len() < 3 || rest.is_empty() {
        return Err(ApiError::Parse(
            "usage: set <entity> <Component> <json>".to_string(),
        ));
    }
    Ok(Request::Write(WriteCmd::Pure(PureWrite::Set {
        entity: entity_ref(&fixed[1])?,
        component: fixed[2].clone(),
        patch: parse_json_rest(rest, "set")?,
    })))
}

fn parse_add(line: &str) -> Result<Request, ApiError> {
    let (fixed, rest) = split_fixed_tokens(line, 3)?;
    if fixed.len() < 3 {
        return Err(ApiError::Parse(
            "usage: add <entity> <Component> [json]".to_string(),
        ));
    }
    let value = if rest.is_empty() { None } else { Some(parse_json_rest(rest, "add")?) };
    Ok(Request::Write(WriteCmd::Pure(PureWrite::Add {
        entity: entity_ref(&fixed[1])?,
        component: fixed[2].clone(),
        value,
    })))
}

fn parse_describe(tokens: &mut impl Iterator<Item = String>) -> Result<Request, ApiError> {
    let target = tokens
        .next()
        .ok_or_else(|| ApiError::Parse("describe needs an entity (name or #id)".to_string()))?;
    Ok(Request::Query(Query::Describe { entity: entity_ref(&target)? }))
}

fn parse_remove(tokens: &mut impl Iterator<Item = String>) -> Result<Request, ApiError> {
    let (entity, component) = (tokens.next(), tokens.next());
    let (Some(entity), Some(component)) = (entity, component) else {
        return Err(ApiError::Parse("usage: remove <entity> <Component>".to_string()));
    };
    Ok(Request::Write(WriteCmd::Pure(PureWrite::Remove {
        entity: entity_ref(&entity)?,
        component,
    })))
}

fn parse_rename(tokens: &mut impl Iterator<Item = String>) -> Result<Request, ApiError> {
    let (entity, name) = (tokens.next(), tokens.next());
    let (Some(entity), Some(name)) = (entity, name) else {
        return Err(ApiError::Parse("usage: rename <entity> <name>".to_string()));
    };
    Ok(Request::Write(WriteCmd::Pure(PureWrite::Rename { entity: entity_ref(&entity)?, name })))
}

/// A trailing `x y` numeric pair is a position; anything before it (or a lone token) is the name.
fn split_trailing_position(remaining: &mut Vec<String>) -> Option<(f32, f32)> {
    if remaining.len() >= 2 {
        let maybe_y = remaining[remaining.len() - 1].parse::<f32>();
        let maybe_x = remaining[remaining.len() - 2].parse::<f32>();
        match (maybe_x, maybe_y) {
            (Ok(x), Ok(y)) if x.is_finite() && y.is_finite() => {
                remaining.truncate(remaining.len() - 2);
                Some((x, y))
            }
            _ => None,
        }
    } else {
        None
    }
}

fn parse_create(tokens: &mut impl Iterator<Item = String>) -> Result<Request, ApiError> {
    let archetype_token = tokens
        .next()
        .ok_or_else(|| ApiError::Parse("usage: create <archetype> [name] [x y]".to_string()))?;
    let archetype = Archetype::from_kebab(&archetype_token).ok_or_else(|| {
        ApiError::Invalid(format!(
            "unknown archetype \"{archetype_token}\" — known: {}",
            ARCHETYPES.join(", ")
        ))
    })?;
    let mut remaining: Vec<String> = tokens.collect();
    let position = split_trailing_position(&mut remaining);
    let name = match remaining.len() {
        0 => None,
        1 => Some(remaining.remove(0)),
        _ => {
            return Err(ApiError::Parse(
                "usage: create <archetype> [name] [x y] (quote a name with spaces)"
                    .to_string(),
            ))
        }
    };
    Ok(Request::Write(WriteCmd::Hosted(HostedWrite::Create {
        archetype,
        name,
        position,
    })))
}

fn parse_delete(tokens: &mut impl Iterator<Item = String>) -> Result<Request, ApiError> {
    let entity = tokens
        .next()
        .ok_or_else(|| ApiError::Parse("usage: delete <entity>".to_string()))?;
    Ok(Request::Write(WriteCmd::Pure(PureWrite::Delete { entity: entity_ref(&entity)? })))
}

fn parse_select(tokens: &mut impl Iterator<Item = String>) -> Result<Request, ApiError> {
    let target = tokens
        .next()
        .ok_or_else(|| ApiError::Parse("usage: select <entity>|none".to_string()))?;
    let entity = if target == "none" { None } else { Some(entity_ref(&target)?) };
    Ok(Request::Write(WriteCmd::Pure(PureWrite::Select { entity })))
}

fn parse_batch(tokens: &mut impl Iterator<Item = String>) -> Result<Request, ApiError> {
    let sub = tokens
        .next()
        .ok_or_else(|| ApiError::Parse("usage: batch begin [name] | end | abort".to_string()))?;
    match sub.as_str() {
        "begin" => Ok(Request::Write(WriteCmd::Pure(PureWrite::BatchBegin { name: tokens.next() }))),
        "end" => Ok(Request::Write(WriteCmd::Pure(PureWrite::BatchEnd))),
        "abort" => Ok(Request::Write(WriteCmd::Pure(PureWrite::BatchAbort))),
        other => {
            Err(ApiError::Parse(format!(
                "unknown batch action \"{other}\" — expected begin, end, or abort"
            )))
        }
    }
}

/// Parse one request line into a [`Request`].
pub fn parse_line(line: &str) -> Result<Request, ApiError> {
    // `set`/`add` carry rest-of-line JSON: peel their fixed tokens without
    // tokenizing the remainder.
    let first = line.split_whitespace().next().unwrap_or("");
    match first {
        "set" => return parse_set(line),
        "add" => return parse_add(line),
        _ => {}
    }

    let tokens = tokens(line)?;
    let mut tokens = tokens.into_iter();
    let verb = tokens
        .next()
        .ok_or_else(|| ApiError::Parse("empty request".to_string()))?;

    let request = match verb.as_str() {
        "list" => Request::Query(Query::ListEntities { filter: tokens.next() }),
        "describe" => parse_describe(&mut tokens)?,
        "selection" => Request::Query(Query::Selection),
        "scene" => Request::Query(Query::SceneInfo),
        "commands" => Request::Query(Query::ListCommands),
        "remove" => parse_remove(&mut tokens)?,
        "rename" => parse_rename(&mut tokens)?,
        "create" => return parse_create(&mut tokens),
        "delete" => parse_delete(&mut tokens)?,
        "select" => parse_select(&mut tokens)?,
        "undo" => Request::Write(WriteCmd::Pure(PureWrite::Undo)),
        "redo" => Request::Write(WriteCmd::Pure(PureWrite::Redo)),
        "save" => Request::Write(WriteCmd::Hosted(HostedWrite::Save { path: tokens.next() })),
        "batch" => parse_batch(&mut tokens)?,
        other => {
            return Err(ApiError::Parse(format!(
                "unknown verb \"{other}\" — expected one of: {}",
                VERBS.join(", ")
            )))
        }
    };

    if let Some(extra) = tokens.next() {
        return Err(ApiError::Parse(format!("unexpected trailing token \"{extra}\"")));
    }
    Ok(request)
}
