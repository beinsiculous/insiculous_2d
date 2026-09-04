//! Stage B write execution: every [`PureWrite`] runs here against a mutable
//! [`WriteCtx`], ALWAYS through `CommandHistory` — an API write that is not
//! undoable in the GUI is a trap. Batches collect commands
//! after executing them and land as one `MacroCommand`; they are NOT
//! transactions (a mid-batch error leaves earlier effects applied — `batch
//! abort` is the recovery).

mod verbs;

use ecs::World;
use serde_json::Value;

use crate::commands::{CommandHistory, EditorCommand, SetComponentValueCommand};
use crate::play_state::EditorPlayState;
use crate::selection::Selection;
use crate::stored_component::{
    capture_all_values, capture_component_by_name, stored_component_from_json, StoredComponent,
};

use super::{ApiError, PureWrite};

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
    pub(super) fn record(&mut self, cmd: Box<dyn EditorCommand>) {
        record_executed(self.history, self.batch, cmd);
    }
}

/// Reject any non-finite number anywhere in a JSON patch — NaN/inf poison
/// physics and rendering math silently.
pub(super) fn reject_non_finite(value: &Value) -> Result<(), ApiError> {
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
pub(super) fn build_set_command(
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
pub(super) fn build_add_patch_set(
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
        PureWrite::Set { entity, component, patch } => verbs::set(ctx, entity, component, patch),
        PureWrite::Add { entity, component, value } => verbs::add(ctx, entity, component, value.as_ref()),
        PureWrite::Remove { entity, component } => verbs::remove(ctx, entity, component),
        PureWrite::Rename { entity, name } => verbs::rename(ctx, entity, name),
        PureWrite::Delete { entity } => verbs::delete(ctx, entity),
        PureWrite::Select { entity } => verbs::select(ctx, entity.as_ref()),
        PureWrite::Undo => verbs::undo(ctx),
        PureWrite::Redo => verbs::redo(ctx),
        PureWrite::BatchBegin { name } => verbs::batch_begin(ctx, name.as_deref()),
        PureWrite::BatchEnd => verbs::batch_end(ctx),
        PureWrite::BatchAbort => verbs::batch_abort(ctx),
    }
}
