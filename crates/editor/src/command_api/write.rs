//! Stage B write execution: every [`PureWrite`] runs here against a mutable
//! [`WriteCtx`], ALWAYS through `CommandHistory` — an API write that is not
//! undoable in the GUI is a trap (audit §9.7(1)). Batches collect commands
//! after executing them and land as one `MacroCommand`; they are NOT
//! transactions (a mid-batch error leaves earlier effects applied — `batch
//! abort` is the recovery).

use ecs::World;
use serde_json::Value;

use crate::commands::{
    CommandHistory, DeleteEntityCommand, EditorCommand, MacroCommand, RemoveComponentCommand,
    RenameEntityCommand, SetComponentValueCommand,
};
use crate::play_state::EditorPlayState;
use crate::selection::Selection;
use crate::stored_component::{
    capture_all_values, capture_component_by_name, stored_component_from_json, ComponentKind,
    StoredComponent,
};

use super::{entity_record, ApiError, PureWrite};

/// An open API batch: commands already executed against the world, waiting
/// to land as one `MacroCommand` on `batch end` (or to be reverse-undone on
/// `batch abort`).
pub struct ApiBatch {
    pub name: String,
    pub commands: Vec<Box<dyn EditorCommand>>,
}

/// Everything a pure write may touch, borrowed for one request.
pub struct WriteCtx<'a> {
    pub world: &'a mut World,
    pub history: &'a mut CommandHistory,
    pub selection: &'a mut Selection,
    pub play_state: EditorPlayState,
    pub batch: &'a mut Option<ApiBatch>,
}

impl WriteCtx<'_> {
    /// Record a command that has ALREADY been executed: append to the open
    /// batch, or push straight onto the history.
    fn record(&mut self, cmd: Box<dyn EditorCommand>) {
        match self.batch.as_mut() {
            Some(batch) => batch.commands.push(cmd),
            None => self.history.push_already_executed(cmd),
        }
    }
}

/// Reject any non-finite number anywhere in a JSON patch — NaN/inf poison
/// physics and rendering math silently.
fn reject_non_finite(value: &Value) -> Result<(), ApiError> {
    match value {
        Value::Number(n) => {
            if n.as_f64().is_some_and(|f| !f.is_finite()) {
                return Err(ApiError::Invalid("non-finite number in value".to_string()));
            }
            Ok(())
        }
        Value::Array(items) => items.iter().try_for_each(reject_non_finite),
        Value::Object(map) => map.values().try_for_each(reject_non_finite),
        _ => Ok(()),
    }
}

/// The component's current serde value (the same generic read `describe`
/// uses), `Null` when absent or unserializable.
fn current_value(world: &World, entity: ecs::EntityId, component: &str) -> Value {
    capture_all_values(world, entity)
        .into_iter()
        .find(|(name, _)| *name == component)
        .map(|(_, v)| v)
        .unwrap_or(Value::Null)
}

/// Whether a serialized component value has the externally-tagged enum
/// shape (`{"VariantName": {...}}`): a single PascalCase key. Struct fields
/// in this codebase are snake_case, so the shapes never collide.
fn is_externally_tagged_enum(value: &serde_json::Map<String, Value>) -> bool {
    value.len() == 1
        && value
            .keys()
            .next()
            .and_then(|k| k.chars().next())
            .is_some_and(|c| c.is_uppercase())
}

/// Shallow-merge `patch` into `current` when both are struct-shaped
/// objects, validating that every patch key exists on the component. An
/// externally-tagged enum (like `Behavior`) and any non-object
/// serialization are a whole-value replace — that is how a variant is
/// switched (kimi F1).
fn merge_patch(current: Value, patch: Value, component: &str) -> Result<Value, ApiError> {
    match (current, patch) {
        (Value::Object(base), Value::Object(overlay)) if is_externally_tagged_enum(&base) => {
            Ok(Value::Object(overlay))
        }
        (Value::Object(mut base), Value::Object(overlay)) => {
            for (key, value) in overlay {
                if !base.contains_key(&key) {
                    let known: Vec<&String> = base.keys().collect();
                    return Err(ApiError::Invalid(format!(
                        "{component} has no field \"{key}\" — fields: {}",
                        known.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(", ")
                    )));
                }
                base.insert(key, value);
            }
            Ok(Value::Object(base))
        }
        (_, replacement) => Ok(replacement),
    }
}

/// Mirror the GUI's hard physical floors so an API write can't feed rapier
/// a zero-extent collider or break rendering/audio math (plan-review F2).
fn sanitize(stored: StoredComponent) -> StoredComponent {
    use glam::Vec2;
    use physics::components::ColliderShape;
    match stored {
        StoredComponent::Transform2D(mut t) => {
            t.scale = t.scale.max(Vec2::splat(0.01));
            StoredComponent::Transform2D(t)
        }
        StoredComponent::Sprite(mut sp) => {
            sp.scale = sp.scale.max(Vec2::splat(0.01));
            StoredComponent::Sprite(sp)
        }
        StoredComponent::Collider(mut c) => {
            c.shape = match c.shape {
                ColliderShape::Box { half_extents } => {
                    ColliderShape::Box { half_extents: half_extents.max(Vec2::splat(0.5)) }
                }
                ColliderShape::Circle { radius } => {
                    ColliderShape::Circle { radius: radius.max(0.5) }
                }
                ColliderShape::CapsuleY { half_height, radius } => ColliderShape::CapsuleY {
                    half_height: half_height.max(0.0),
                    radius: radius.max(0.5),
                },
                ColliderShape::CapsuleX { half_height, radius } => ColliderShape::CapsuleX {
                    half_height: half_height.max(0.0),
                    radius: radius.max(0.5),
                },
            };
            StoredComponent::Collider(c)
        }
        StoredComponent::AudioSource(mut a) => {
            a.volume = a.volume.clamp(0.0, 1.0);
            a.pitch = a.pitch.max(0.1);
            StoredComponent::AudioSource(a)
        }
        other => other,
    }
}

/// Execute one pure write. Every mutation lands in the history (or the open
/// batch); responses reuse the query payload shapes.
pub fn run(write: &PureWrite, ctx: &mut WriteCtx<'_>) -> Result<Value, ApiError> {
    if ctx.play_state == EditorPlayState::Playing {
        return Err(ApiError::Refused(
            "writes are refused while Playing — pause or stop first".to_string(),
        ));
    }

    match write {
        PureWrite::Set { entity, component, patch } => {
            if component == "Name" {
                return Err(ApiError::Invalid(
                    "Name is set through `rename`, which also validates it".to_string(),
                ));
            }
            reject_non_finite(patch)?;
            let entity = entity.resolve(ctx.world)?;
            let old = capture_component_by_name(ctx.world, entity, component)
                .map_err(ApiError::Invalid)?
                .ok_or_else(|| {
                    ApiError::Invalid(format!(
                        "entity has no {component} — `add` it first"
                    ))
                })?;
            let current = current_value(ctx.world, entity, component);
            let merged = merge_patch(current, patch.clone(), component)?;
            let new = stored_component_from_json(component, merged).map_err(ApiError::Invalid)?;
            let new = sanitize(new);
            let mut cmd = SetComponentValueCommand::new(entity, old, new);
            cmd.execute(ctx.world);
            ctx.record(Box::new(cmd));
            Ok(serde_json::json!({
                "command": format!("set {component}"),
                "entity": entity_record(ctx.world, entity),
            }))
        }
        PureWrite::Add { entity, component, value } => {
            if let Some(v) = value {
                reject_non_finite(v)?;
            }
            let entity = entity.resolve(ctx.world)?;
            let kind = ComponentKind::ALL
                .iter()
                .copied()
                .find(|k| k.display_name() == component)
                .ok_or_else(|| {
                    ApiError::Invalid(format!(
                        "\"{component}\" is not an addable component — known: {}",
                        ComponentKind::ALL
                            .iter()
                            .map(|k| k.display_name())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                })?;
            if kind.is_present(ctx.world, entity) {
                return Err(ApiError::Invalid(format!(
                    "entity already has {component} — use `set`"
                )));
            }
            let mut add = crate::commands::AddComponentCommand::new(entity, kind);
            add.execute(ctx.world);
            let mut recorded: Box<dyn EditorCommand> = Box::new(add);
            if let Some(patch) = value {
                let old = capture_component_by_name(ctx.world, entity, component)
                    .map_err(ApiError::Invalid)?
                    .ok_or_else(|| ApiError::Invalid("add failed".to_string()))?;
                let current = current_value(ctx.world, entity, component);
                let merged = merge_patch(current, patch.clone(), component)?;
                let new =
                    stored_component_from_json(component, merged).map_err(ApiError::Invalid)?;
                let new = sanitize(new);
                let mut set = SetComponentValueCommand::new(entity, old, new);
                set.execute(ctx.world);
                recorded = Box::new(MacroCommand::new(
                    format!("Add {component} (API)"),
                    vec![recorded, Box::new(set)],
                ));
            }
            ctx.record(recorded);
            Ok(serde_json::json!({
                "command": format!("add {component}"),
                "entity": entity_record(ctx.world, entity),
            }))
        }
        PureWrite::Remove { entity, component } => {
            let entity = entity.resolve(ctx.world)?;
            let kind = ComponentKind::ALL
                .iter()
                .copied()
                .find(|k| k.display_name() == component)
                .ok_or_else(|| {
                    ApiError::Invalid(format!("\"{component}\" is not a removable component"))
                })?;
            if !kind.is_present(ctx.world, entity) {
                return Err(ApiError::Invalid(format!("entity has no {component}")));
            }
            let mut cmd = RemoveComponentCommand::new(entity, kind);
            cmd.execute(ctx.world);
            ctx.record(Box::new(cmd));
            Ok(serde_json::json!({
                "command": format!("remove {component}"),
                "entity": entity_record(ctx.world, entity),
            }))
        }
        PureWrite::Rename { entity, name } => {
            let entity = entity.resolve(ctx.world)?;
            let current = ctx
                .world
                .get::<ecs::Name>(entity)
                .map(|n| n.as_str().to_string());
            let Some(new_name) = crate::hierarchy::normalized_rename(current.as_deref(), name)
            else {
                return Err(ApiError::Invalid(
                    "rename needs a non-empty name that differs from the current one".to_string(),
                ));
            };
            let mut cmd = RenameEntityCommand::new(ctx.world, entity, ecs::Name::new(new_name));
            cmd.execute(ctx.world);
            ctx.record(Box::new(cmd));
            Ok(serde_json::json!({
                "command": "rename",
                "entity": entity_record(ctx.world, entity),
            }))
        }
        PureWrite::Delete { entity } => {
            let entity = entity.resolve(ctx.world)?;
            let record = entity_record(ctx.world, entity);
            let mut cmd = DeleteEntityCommand::new(entity);
            cmd.execute(ctx.world);
            ctx.selection.remove(entity);
            ctx.record(Box::new(cmd));
            Ok(serde_json::json!({ "command": "delete", "entity": record }))
        }
        PureWrite::Select { entity } => match entity {
            Some(entity_ref) => {
                let entity = entity_ref.resolve(ctx.world)?;
                ctx.selection.select(entity);
                Ok(serde_json::json!({
                    "command": "select",
                    "entity": entity_record(ctx.world, entity),
                }))
            }
            None => {
                ctx.selection.clear();
                Ok(serde_json::json!({ "command": "select", "entity": Value::Null }))
            }
        },
        PureWrite::Undo => {
            if ctx.batch.is_some() {
                return Err(ApiError::Refused(
                    "undo inside an open batch would desync it — `batch end` or `batch abort` first"
                        .to_string(),
                ));
            }
            let name = ctx.history.undo_name().map(str::to_string);
            let undid = ctx.history.undo(ctx.world);
            Ok(serde_json::json!({ "undid": if undid { name } else { None } }))
        }
        PureWrite::Redo => {
            if ctx.batch.is_some() {
                return Err(ApiError::Refused(
                    "redo inside an open batch would desync it — `batch end` or `batch abort` first"
                        .to_string(),
                ));
            }
            let name = ctx.history.redo_name().map(str::to_string);
            let redid = ctx.history.redo(ctx.world);
            Ok(serde_json::json!({ "redid": if redid { name } else { None } }))
        }
        PureWrite::BatchBegin { name } => {
            if ctx.batch.is_some() {
                return Err(ApiError::Refused(
                    "a batch is already open — `batch end` or `batch abort` it first".to_string(),
                ));
            }
            let name = name.clone().unwrap_or_else(|| "API Batch".to_string());
            *ctx.batch = Some(ApiBatch { name: name.clone(), commands: Vec::new() });
            Ok(serde_json::json!({ "batch": name }))
        }
        PureWrite::BatchEnd => {
            let Some(batch) = ctx.batch.take() else {
                return Err(ApiError::Refused("no batch is open".to_string()));
            };
            let count = batch.commands.len();
            if count > 0 {
                ctx.history.push_already_executed(Box::new(MacroCommand::new(
                    batch.name,
                    batch.commands,
                )));
            }
            Ok(serde_json::json!({ "commands": count }))
        }
        PureWrite::BatchAbort => {
            let Some(batch) = ctx.batch.take() else {
                return Err(ApiError::Refused("no batch is open".to_string()));
            };
            let count = batch.commands.len();
            for mut cmd in batch.commands.into_iter().rev() {
                cmd.undo(ctx.world);
            }
            Ok(serde_json::json!({ "aborted": count }))
        }
    }
}
