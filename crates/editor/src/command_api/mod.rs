//! The editor's command API — Stage A queries + Stage B writes (audit §9).
//!
//! Line-oriented text in, single-line JSON out. This module is pure
//! dispatch: no I/O, no threads, no `cfg` — a transport (today the
//! `--api` stdin/stdout loop in the editor binary, later a WebSocket for
//! the web editor) reads lines from somewhere and feeds them to the
//! dispatcher. Entities are addressed name-first ([`EntityRef`]);
//! ambiguity is an error, never a silent first match.
//!
//! Stage B: [`parse_line`] yields a [`Request`] — queries run against
//! [`QueryCtx`], pure writes run in [`write`] against a mutable
//! [`write::WriteCtx`] (always through `CommandHistory`), and
//! [`HostedWrite`]s (create/save) are returned to the integration layer,
//! which owns entity factories and the save choke point. Every path emits
//! the same envelope via [`ok_response`]/[`error_response`].
//!
//! The protocol contract lives in `docs/EDITOR_COMMAND_API.md`.

mod parse;
mod query;
pub mod specs;
pub mod write;

pub use parse::{parse_line, ARCHETYPES};

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
    /// Self-description: every verb with usage/example, plus the live
    /// component and archetype name lists.
    ListCommands,
}

/// One parsed request line.
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Query(Query),
    Write(WriteCmd),
}

/// A mutating request.
#[derive(Debug, Clone, PartialEq)]
pub enum WriteCmd {
    /// Executable inside the editor crate against a [`write::WriteCtx`].
    Pure(PureWrite),
    /// Needs integration-layer state (entity factories, the save choke
    /// point) — returned to the caller to perform, same envelope.
    Hosted(HostedWrite),
}

/// Writes the editor crate can perform itself (always through
/// `CommandHistory`).
#[derive(Debug, Clone, PartialEq)]
pub enum PureWrite {
    /// Shallow-patch (or, for non-object serializations, replace) one
    /// component with raw JSON.
    Set { entity: EntityRef, component: String, patch: serde_json::Value },
    /// Add a component (default-valued, or patched with raw JSON).
    Add { entity: EntityRef, component: String, value: Option<serde_json::Value> },
    /// Remove a component.
    Remove { entity: EntityRef, component: String },
    /// Assign or replace the entity's Name (works on unnamed entities).
    Rename { entity: EntityRef, name: String },
    /// Delete an entity (undoable; children reparent to the grandparent).
    Delete { entity: EntityRef },
    /// Replace the selection (`None` = clear). Not undoable — GUI parity.
    Select { entity: Option<EntityRef> },
    /// Undo / redo the top of the history.
    Undo,
    Redo,
    /// Open / close / abort an explicit batch (one MacroCommand).
    BatchBegin { name: Option<String> },
    BatchEnd,
    BatchAbort,
}

/// Writes only the integration layer can perform.
#[derive(Debug, Clone, PartialEq)]
pub enum HostedWrite {
    /// Spawn an archetype (see [`ARCHETYPES`]) at the viewport center or an
    /// explicit position, optionally named — one undo entry.
    Create { archetype: String, name: Option<String>, position: Option<(f32, f32)> },
    /// Save the scene (through the mandatory save choke point).
    Save { path: Option<String> },
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
    /// The request parsed but its arguments are unusable (unknown
    /// component/field, bad JSON, non-finite number, ...).
    Invalid(String),
    /// The editor's current state forbids the request (Playing, open
    /// batch rules, read-only transport).
    Refused(String),
}

impl ApiError {
    fn kind(&self) -> &'static str {
        match self {
            ApiError::Parse(_) => "parse",
            ApiError::NotFound(_) => "not_found",
            ApiError::AmbiguousName { .. } => "ambiguous_name",
            ApiError::Invalid(_) => "invalid",
            ApiError::Refused(_) => "refused",
        }
    }

    fn message(&self) -> String {
        match self {
            ApiError::Parse(msg)
            | ApiError::NotFound(msg)
            | ApiError::Invalid(msg)
            | ApiError::Refused(msg) => msg.clone(),
            ApiError::AmbiguousName { name, matches } => format!(
                "{} entities are named \"{name}\" — address one by #id",
                matches.len()
            ),
        }
    }
}

/// Wrap a successful payload in the response envelope (one line of JSON).
pub fn ok_response(data: serde_json::Value) -> String {
    serde_json::json!({ "ok": true, "data": data }).to_string()
}

/// Wrap an error in the response envelope (one line of JSON).
pub fn error_response(err: &ApiError) -> String {
    let mut error = serde_json::json!({
        "kind": err.kind(),
        "message": err.message(),
    });
    if let ApiError::AmbiguousName { matches, .. } = err {
        error["matches"] = serde_json::json!(matches);
    }
    serde_json::json!({ "ok": false, "error": error }).to_string()
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
    let result = parse::parse_line(line).and_then(|request| match request {
        Request::Query(q) => query::run(&q, ctx),
        Request::Write(_) => Err(ApiError::Refused(
            "write over read-only dispatch — use the editor's --api transport".to_string(),
        )),
    });
    Some(match result {
        Ok(data) => ok_response(data),
        Err(err) => error_response(&err),
    })
}

/// The `{id, generation, name, display}` record every entity-bearing
/// response uses.
pub fn entity_record(world: &World, entity: EntityId) -> serde_json::Value {
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
#[cfg(test)]
mod write_tests;
