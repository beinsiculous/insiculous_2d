//! Pure write implementations, one function per verb.

use serde_json::Value;

use crate::commands::{
    DeleteEntityCommand, EditorCommand, MacroCommand, RemoveComponentCommand, RenameEntityCommand,
};
use crate::stored_component::{capture_component_by_name, ComponentKind};

use super::super::{entity_record, ApiError, EntityRef};
use super::{build_add_patch_set, build_set_command, reject_non_finite, ApiBatch, WriteCtx};

/// Lookup a built-in component kind by its display name.
fn typed_kind(component: &str) -> Option<ComponentKind> {
    ComponentKind::ALL
        .iter()
        .copied()
        .find(|kind| kind.display_name() == component)
}

/// Restore selection from undo/redo history if recorded.
fn restore_selection(ctx: &mut WriteCtx<'_>) {
    if let Some(ids) = ctx.history.take_selection_restore() {
        ctx.selection.clear();
        ctx.selection.select_multiple(ids);
    }
}

pub(super) fn set(
    ctx: &mut WriteCtx<'_>,
    entity: &EntityRef,
    component: &str,
    patch: &Value,
) -> Result<Value, ApiError> {
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

pub(super) fn add(
    ctx: &mut WriteCtx<'_>,
    entity: &EntityRef,
    component: &str,
    value: Option<&Value>,
) -> Result<Value, ApiError> {
    if let Some(v) = value {
        reject_non_finite(v)?;
    }
    let entity = entity.resolve(ctx.world)?;
    let kind = typed_kind(component);
    // Typed miss falls through to the dynamic tier:
    // game-registered components are addable by name too.
    let dynamic = kind.is_none();
    if dynamic && !crate::stored_component::dynamic::is_dynamic_component(component) {
        let mut known: Vec<String> = ComponentKind::ALL
            .iter()
            .map(|kind| kind.display_name().to_string())
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
            Err(error) => {
                recorded.undo(ctx.world);
                return Err(error);
            }
        }
    }
    ctx.record(recorded);
    Ok(serde_json::json!({
        "command": format!("add {component}"),
        "entity": entity_record(ctx.world, entity),
    }))
}

pub(super) fn remove(
    ctx: &mut WriteCtx<'_>,
    entity: &EntityRef,
    component: &str,
) -> Result<Value, ApiError> {
    let entity = entity.resolve(ctx.world)?;
    let kind = typed_kind(component);
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

pub(super) fn rename(
    ctx: &mut WriteCtx<'_>,
    entity: &EntityRef,
    name: &str,
) -> Result<Value, ApiError> {
    let entity = entity.resolve(ctx.world)?;
    let current = ctx
        .world
        .get::<ecs::Name>(entity)
        .map(|name_component| name_component.as_str().to_string());
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

pub(super) fn delete(ctx: &mut WriteCtx<'_>, entity: &EntityRef) -> Result<Value, ApiError> {
    let entity = entity.resolve(ctx.world)?;
    let record = entity_record(ctx.world, entity);
    let mut cmd = DeleteEntityCommand::new(entity);
    cmd.execute(ctx.world);
    ctx.selection.remove(entity);
    ctx.record(Box::new(cmd));
    Ok(serde_json::json!({ "command": "delete", "entity": record }))
}

pub(super) fn select(
    ctx: &mut WriteCtx<'_>,
    entity: Option<&EntityRef>,
) -> Result<Value, ApiError> {
    match entity {
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
    }
}

pub(super) fn undo(ctx: &mut WriteCtx<'_>) -> Result<Value, ApiError> {
    if ctx.batch.is_some() {
        return Err(ApiError::Refused(
            "undo inside an open batch would desync it — `batch end` or `batch abort` first"
                .to_string(),
        ));
    }
    let name = ctx.history.undo_name().map(str::to_string);
    let undid = ctx.history.undo(ctx.world);
    restore_selection(ctx);
    Ok(serde_json::json!({ "undid": if undid { name } else { None } }))
}

pub(super) fn redo(ctx: &mut WriteCtx<'_>) -> Result<Value, ApiError> {
    if ctx.batch.is_some() {
        return Err(ApiError::Refused(
            "redo inside an open batch would desync it — `batch end` or `batch abort` first"
                .to_string(),
        ));
    }
    let name = ctx.history.redo_name().map(str::to_string);
    let redid = ctx.history.redo(ctx.world);
    restore_selection(ctx);
    Ok(serde_json::json!({ "redid": if redid { name } else { None } }))
}

pub(super) fn batch_begin(ctx: &mut WriteCtx<'_>, name: Option<&str>) -> Result<Value, ApiError> {
    if ctx.batch.is_some() {
        return Err(ApiError::Refused(
            "a batch is already open — `batch end` or `batch abort` it first".to_string(),
        ));
    }
    let name = name.unwrap_or("API Batch").to_string();
    *ctx.batch = Some(ApiBatch {
        name: name.clone(),
        commands: Vec::new(),
        selection_before: ctx.selection.selected().collect(),
    });
    Ok(serde_json::json!({ "batch": name }))
}

pub(super) fn batch_end(ctx: &mut WriteCtx<'_>) -> Result<Value, ApiError> {
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

pub(super) fn batch_abort(ctx: &mut WriteCtx<'_>) -> Result<Value, ApiError> {
    let Some(batch) = ctx.batch.take() else {
        return Err(ApiError::Refused("no batch is open".to_string()));
    };
    let count = batch.commands.len();
    for mut cmd in batch.commands.into_iter().rev() {
        cmd.undo(ctx.world);
    }
    Ok(serde_json::json!({ "aborted": count }))
}
