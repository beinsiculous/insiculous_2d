//! The editor's command API — Stage A: read-only queries (audit §9).
//!
//! Line-oriented text in, single-line JSON out. This module is pure
//! dispatch: no I/O, no threads, no `cfg` — a transport (today the
//! `--api` stdin/stdout loop in the editor binary, later a WebSocket for
//! the web editor) reads lines from somewhere and feeds them to
//! [`dispatch_line`]. Entities are addressed name-first ([`EntityRef`]);
//! ambiguity is an error, never a silent first match.
//!
//! The protocol contract lives in `docs/EDITOR_COMMAND_API.md`.

mod parse;
mod query;

use ecs::{EntityId, World};

use crate::play_state::EditorPlayState;
use crate::selection::Selection;

/// Everything a query may read. Borrowed for one dispatch; queries never
/// mutate (writes are Stage B and will route through `CommandHistory`).
pub struct QueryCtx<'a> {
    pub world: &'a World,
    pub selection: &'a Selection,
    pub scene_path: Option<&'a std::path::Path>,
    /// From `CommandHistory::is_dirty()` — the dirty source of truth.
    pub dirty: bool,
    pub play_state: EditorPlayState,
}

/// How a request names an entity: by `Name` component (the stable,
/// human-meaningful address) or by the session-local numeric id shown in
/// the hierarchy/inspector (`#7` — `EntityId::value()`, NOT stable across
/// sessions or Play/Stop).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityRef {
    Name(String),
    Id(u64),
}

/// Read-only questions. Never touches `CommandHistory`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    /// Entities whose display name contains the filter (case-insensitive);
    /// no filter = all.
    ListEntities { filter: Option<String> },
    /// Every registry component of one entity, as serde values.
    Describe { entity: EntityRef },
    /// Current selection (primary + all, insertion order).
    Selection,
    /// Scene path, dirty state, entity count, play state.
    SceneInfo,
}

/// A request that could not be answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// The request line did not parse.
    Parse(String),
    /// The referenced entity does not exist.
    NotFound(String),
    /// Multiple entities share the requested name.
    AmbiguousName { name: String, matches: Vec<u64> },
}

impl ApiError {
    fn kind(&self) -> &'static str {
        match self {
            ApiError::Parse(_) => "parse",
            ApiError::NotFound(_) => "not_found",
            ApiError::AmbiguousName { .. } => "ambiguous_name",
        }
    }

    fn message(&self) -> String {
        match self {
            ApiError::Parse(msg) | ApiError::NotFound(msg) => msg.clone(),
            ApiError::AmbiguousName { name, matches } => format!(
                "{} entities are named \"{name}\" — address one by #id",
                matches.len()
            ),
        }
    }
}

/// Answer one request line.
///
/// Returns `None` for blank input (no response owed); otherwise always
/// exactly one line of JSON — errors included, so a caller reading
/// response-per-request never desyncs.
pub fn dispatch_line(line: &str, ctx: &QueryCtx<'_>) -> Option<String> {
    if line.trim().is_empty() {
        return None;
    }
    let response = match parse::parse_request(line).and_then(|q| query::run(&q, ctx)) {
        Ok(data) => serde_json::json!({ "ok": true, "data": data }),
        Err(err) => {
            let mut error = serde_json::json!({
                "kind": err.kind(),
                "message": err.message(),
            });
            if let ApiError::AmbiguousName { matches, .. } = &err {
                error["matches"] = serde_json::json!(matches);
            }
            serde_json::json!({ "ok": false, "error": error })
        }
    };
    Some(response.to_string())
}

/// The `{id, generation, name, display}` record every entity-bearing
/// response uses.
fn entity_record(world: &World, entity: EntityId) -> serde_json::Value {
    let name = world
        .get::<ecs::sprite_components::Name>(entity)
        .map(|n| n.as_str().to_string());
    serde_json::json!({
        "id": entity.value(),
        "generation": entity.generation(),
        "name": name,
        "display": crate::hierarchy::HierarchyPanel::entity_display_name(world, entity),
    })
}

#[cfg(test)]
mod tests;
