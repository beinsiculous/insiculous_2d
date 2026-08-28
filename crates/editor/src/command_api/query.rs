//! Query execution against a [`QueryCtx`]. Read-only by construction —
//! everything here takes `&World`.

use ecs::{EntityId, World};
use serde_json::{json, Value};

use crate::hierarchy::{HierarchyPanel, NameResolution};
use crate::play_state::EditorPlayState;
use crate::stored_component::capture_all_values;

use super::{entity_record, ApiError, EntityRef, Query, QueryCtx};

impl EntityRef {
    /// Resolve to a live entity: names via the `Name` component (ambiguity
    /// is an error), `#id` by scanning for a matching `EntityId::value()`.
    pub fn resolve(&self, world: &World) -> Result<EntityId, ApiError> {
        match self {
            EntityRef::Name(name) => match HierarchyPanel::resolve_by_name(world, name) {
                NameResolution::One(entity) => Ok(entity),
                NameResolution::None => Err(ApiError::NotFound(format!(
                    "no entity named \"{name}\" (names are exact and case-sensitive; \
                     `list` to see them)"
                ))),
                NameResolution::Ambiguous(entities) => Err(ApiError::AmbiguousName {
                    name: name.clone(),
                    matches: entities.iter().map(|e| e.value()).collect(),
                }),
            },
            EntityRef::Id(id) => world
                .entities()
                .into_iter()
                .find(|e| e.value() == *id)
                .ok_or_else(|| ApiError::NotFound(format!("no entity with id #{id}"))),
        }
    }
}

pub(super) fn run(query: &Query, ctx: &QueryCtx<'_>) -> Result<Value, ApiError> {
    match query {
        Query::ListEntities { filter } => Ok(list_entities(ctx.world, filter.as_deref())),
        Query::Describe { entity } => describe(ctx.world, entity),
        Query::Selection => Ok(selection(ctx)),
        Query::SceneInfo => Ok(scene_info(ctx)),
    }
}

fn list_entities(world: &World, filter: Option<&str>) -> Value {
    let filter_lower = filter.map(str::to_lowercase);
    let mut entities: Vec<EntityId> = world.entities();
    entities.sort_by_key(|e| e.value());
    let records: Vec<Value> = entities
        .into_iter()
        .filter(|e| match &filter_lower {
            Some(f) => HierarchyPanel::entity_display_name(world, *e)
                .to_lowercase()
                .contains(f),
            None => true,
        })
        .map(|e| entity_record(world, e))
        .collect();
    json!({ "entities": records })
}

fn describe(world: &World, entity_ref: &EntityRef) -> Result<Value, ApiError> {
    let entity = entity_ref.resolve(world)?;
    let mut components = serde_json::Map::new();
    for (type_name, value) in capture_all_values(world, entity) {
        // Name is already the record's top-level `name` field (it is the
        // API's entity address); repeating it as a component would be
        // duplicate, diverging state.
        if type_name == "Name" {
            continue;
        }
        components.insert(type_name.to_string(), value);
    }
    let mut record = entity_record(world, entity);
    record["components"] = Value::Object(components);
    Ok(record)
}

fn selection(ctx: &QueryCtx<'_>) -> Value {
    let primary = ctx.selection.primary().map(|e| entity_record(ctx.world, e));
    let all: Vec<Value> = ctx
        .selection
        .selected()
        .map(|e| entity_record(ctx.world, e))
        .collect();
    json!({ "primary": primary, "all": all })
}

fn scene_info(ctx: &QueryCtx<'_>) -> Value {
    let play_state = match ctx.play_state {
        EditorPlayState::Editing => "editing",
        EditorPlayState::Playing => "playing",
        EditorPlayState::Paused => "paused",
    };
    json!({
        "path": ctx.scene_path.map(|p| p.display().to_string()),
        "dirty": ctx.dirty,
        "entity_count": ctx.world.entities().len(),
        "play_state": play_state,
    })
}
