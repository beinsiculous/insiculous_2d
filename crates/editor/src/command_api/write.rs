//! Stage B write execution: every [`PureWrite`] runs here against a mutable
//! [`WriteCtx`], ALWAYS through `CommandHistory` — an API write that is not
//! undoable in the GUI is a trap. Batches collect commands
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
    /// The selection when `batch begin` ran — the macro's before-image.
    /// Frame-start notes overwrite the history's pending selection while a
    /// batch spans frames, so the snapshot lives HERE.
    pub selection_before: Vec<ecs::EntityId>,
}

/// Everything a pure write may touch, borrowed for one request.
pub struct WriteCtx<'a> {
    pub world: &'a mut World,
    pub history: &'a mut CommandHistory,
    pub selection: &'a mut Selection,
    pub play_state: EditorPlayState,
    pub batch: &'a mut Option<ApiBatch>,
    /// Whether the session's texture resolver ever issued this handle.
    /// `set`/`add` refuse a handle nothing can resolve on save — the
    /// editor crate cannot see the resolver, so the host answers by closure.
    pub texture_known: &'a dyn Fn(u32) -> bool,
}

/// Record a command that has ALREADY been executed: append to the open
/// batch, or push straight onto the history.
pub fn record_executed(
    history: &mut CommandHistory,
    batch: &mut Option<ApiBatch>,
    cmd: Box<dyn EditorCommand>,
) {
    match batch.as_mut() {
        Some(batch) => batch.commands.push(cmd),
        None => history.push_already_executed(cmd),
    }
}

impl WriteCtx<'_> {
    fn record(&mut self, cmd: Box<dyn EditorCommand>) {
        record_executed(self.history, self.batch, cmd);
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

/// Build a sanitized, texture-validated `SetComponentValueCommand` from an existing
/// component and a JSON patch.
fn build_set_command(
    world: &World,
    entity: ecs::EntityId,
    component: &str,
    old: StoredComponent,
    patch: &Value,
    texture_known: &dyn Fn(u32) -> bool,
) -> Result<SetComponentValueCommand, ApiError> {
    let current = current_value(world, entity, component);
    let merged = merge_patch(current, patch.clone(), component)?;
    let new = stored_component_from_json(component, merged).map_err(ApiError::Invalid)?;
    let new = sanitize(new);
    validate_texture_handles(&new, texture_known)?;
    Ok(SetComponentValueCommand::new(entity, old, new))
}

/// Build the follow-up `set` for `add <component> {patch}`: capture the
/// just-attached default as `old`, merge the patch over it, validate and
/// sanitize. Pure with respect to the world — the caller executes (or, on
/// error, rolls back the add).
fn build_add_patch_set(
    world: &World,
    entity: ecs::EntityId,
    component: &str,
    patch: &Value,
    texture_known: &dyn Fn(u32) -> bool,
) -> Result<SetComponentValueCommand, ApiError> {
    let old = capture_component_by_name(world, entity, component)
        .map_err(ApiError::Invalid)?
        .ok_or_else(|| ApiError::Invalid("add failed".to_string()))?;
    build_set_command(world, entity, component, old, patch, texture_known)
}

/// Reject a texture handle the session never issued: it would save as
/// `#texture_N` and fail loud only on the next load. `Sprite.texture_handle`
/// and `Tilemap.tileset` share one id space.
fn validate_texture_handles(
    stored: &StoredComponent,
    texture_known: &dyn Fn(u32) -> bool,
) -> Result<(), ApiError> {
    let handle = match stored {
        StoredComponent::Sprite(sprite) => sprite.texture_handle,
        StoredComponent::Tilemap(tilemap) => tilemap.tileset,
        _ => return Ok(()),
    };
    if texture_known(handle) {
        return Ok(());
    }
    Err(ApiError::Invalid(format!(
        "texture handle {handle} was never issued by this session's assets — \
         load a texture first (handle 0 is always #white)"
    )))
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
/// switched.
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
    match stored {
        StoredComponent::Transform2D(mut t) => {
            crate::physical_floors::clamp_transform(&mut t);
            StoredComponent::Transform2D(t)
        }
        StoredComponent::Sprite(mut sp) => {
            crate::physical_floors::clamp_sprite(&mut sp);
            StoredComponent::Sprite(sp)
        }
        StoredComponent::Collider(mut c) => {
            crate::physical_floors::clamp_collider(&mut c);
            StoredComponent::Collider(c)
        }
        StoredComponent::AudioSource(mut a) => {
            crate::physical_floors::clamp_audio_source(&mut a);
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
    // Per-line before-image for undo's selection restore. While a
    // batch is open the note is skipped: the eventual MacroCommand must
    // carry the PRE-BATCH selection (noted at BatchBegin).
    if ctx.batch.is_none() {
        ctx.history.note_selection(ctx.selection);
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
            let mut cmd = build_set_command(ctx.world, entity, component, old, patch, ctx.texture_known)?;
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
                .find(|k| k.display_name() == component);
            // Typed miss falls through to the dynamic tier:
            // game-registered components are addable by name too.
            let dynamic = kind.is_none();
            if dynamic && !crate::stored_component::dynamic::is_dynamic_component(component) {
                let mut known: Vec<String> = ComponentKind::ALL
                    .iter()
                    .map(|k| k.display_name().to_string())
                    .collect();
                known.extend(crate::stored_component::dynamic::dynamic_component_names());
                return Err(ApiError::Invalid(format!(
                    "\"{component}\" is not an addable component — known: {}",
                    known.join(", ")
                )));
            }
            let present = match kind {
                Some(kind) => kind.is_present(ctx.world, entity),
                None => crate::stored_component::dynamic::has_dynamic(ctx.world, entity, component),
            };
            if present {
                return Err(ApiError::Invalid(format!(
                    "entity already has {component} — use `set`"
                )));
            }
            let mut recorded: Box<dyn EditorCommand> = match kind {
                Some(kind) => {
                    let mut add = crate::commands::AddComponentCommand::new(entity, kind);
                    add.execute(ctx.world);
                    Box::new(add)
                }
                None => {
                    let mut add = crate::commands::AddComponentCommand::dynamic(
                        entity,
                        component.to_string(),
                    );
                    add.execute(ctx.world);
                    Box::new(add)
                }
            };
            if let Some(patch) = value {
                // An error after the attach must not leave an unrecorded
                // component behind — roll the add back so the world matches
                // the error response exactly.
                match build_add_patch_set(ctx.world, entity, component, patch, ctx.texture_known) {
                    Ok(mut set) => {
                        set.execute(ctx.world);
                        recorded = Box::new(MacroCommand::new(
                            format!("Add {component} (API)"),
                            vec![recorded, Box::new(set)],
                        ));
                    }
                    Err(e) => {
                        recorded.undo(ctx.world);
                        return Err(e);
                    }
                }
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
                .find(|k| k.display_name() == component);
            // Typed miss falls through to the dynamic tier.
            if kind.is_none()
                && !crate::stored_component::dynamic::is_dynamic_component(component)
            {
                return Err(ApiError::Invalid(format!(
                    "\"{component}\" is not a removable component"
                )));
            }
            let recorded: Box<dyn EditorCommand> = match kind {
                Some(kind) => {
                    if !kind.is_present(ctx.world, entity) {
                        return Err(ApiError::Invalid(format!("entity has no {component}")));
                    }
                    let mut cmd = RemoveComponentCommand::new(entity, kind);
                    cmd.execute(ctx.world);
                    Box::new(cmd)
                }
                None => {
                    if !crate::stored_component::dynamic::has_dynamic(ctx.world, entity, component)
                    {
                        return Err(ApiError::Invalid(format!("entity has no {component}")));
                    }
                    let mut cmd = crate::commands::RemoveComponentCommand::dynamic(
                        entity,
                        component.to_string(),
                    );
                    cmd.execute(ctx.world);
                    Box::new(cmd)
                }
            };
            ctx.record(recorded);
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
            if let Some(ids) = ctx.history.take_selection_restore() {
                ctx.selection.clear();
                ctx.selection.select_multiple(ids);
            }
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
            if let Some(ids) = ctx.history.take_selection_restore() {
                ctx.selection.clear();
                ctx.selection.select_multiple(ids);
            }
            Ok(serde_json::json!({ "redid": if redid { name } else { None } }))
        }
        PureWrite::BatchBegin { name } => {
            if ctx.batch.is_some() {
                return Err(ApiError::Refused(
                    "a batch is already open — `batch end` or `batch abort` it first".to_string(),
                ));
            }
            let name = name.clone().unwrap_or_else(|| "API Batch".to_string());
            *ctx.batch = Some(ApiBatch {
                name: name.clone(),
                commands: Vec::new(),
                selection_before: ctx.selection.selected().collect(),
            });
            Ok(serde_json::json!({ "batch": name }))
        }
        PureWrite::BatchEnd => {
            let Some(batch) = ctx.batch.take() else {
                return Err(ApiError::Refused("no batch is open".to_string()));
            };
            let count = batch.commands.len();
            if count > 0 {
                ctx.history.push_already_executed_with_before(
                    Box::new(MacroCommand::new(batch.name, batch.commands)),
                    batch.selection_before,
                );
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
