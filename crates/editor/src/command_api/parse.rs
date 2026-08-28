//! Request-line parsing: whitespace-separated tokens, double quotes for
//! tokens containing spaces (no escape sequences — Stage A keeps the
//! protocol boring).

use super::{ApiError, EntityRef, Query};

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

/// Parse one request line into a [`Query`].
pub(super) fn parse_request(line: &str) -> Result<Query, ApiError> {
    let tokens = tokens(line)?;
    let mut tokens = tokens.into_iter();
    let verb = tokens
        .next()
        .ok_or_else(|| ApiError::Parse("empty request".to_string()))?;

    let query = match verb.as_str() {
        "list" => Query::ListEntities { filter: tokens.next() },
        "describe" => {
            let target = tokens
                .next()
                .ok_or_else(|| ApiError::Parse("describe needs an entity (name or #id)".to_string()))?;
            Query::Describe { entity: entity_ref(&target)? }
        }
        "selection" => Query::Selection,
        "scene" => Query::SceneInfo,
        other => {
            return Err(ApiError::Parse(format!(
                "unknown query \"{other}\" — expected list, describe, selection, or scene"
            )))
        }
    };

    if let Some(extra) = tokens.next() {
        return Err(ApiError::Parse(format!("unexpected trailing token \"{extra}\"")));
    }
    Ok(query)
}
