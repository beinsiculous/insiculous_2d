//! Serialization mirror of `ecs::script::{Scripts, ScriptRef, ScriptValue}`
//! for scene files (issue #44, Stage 1).
//!
//! Same pattern as `behavior_data.rs`: `ComponentData::Scripts` wraps
//! [`ScriptRefData`], and `scene_data.rs` re-exports these. The ONE
//! deliberate difference from the runtime type: **entity references persist
//! by NAME** (`ScriptValueData::Entity(String)`) — raw `EntityId`s are
//! meaningless across save/load. On save an id maps to its target's `Name`
//! (the editor's save choke point auto-assigns one to unnamed referenced
//! targets first); on load names resolve to fresh ids in a post-instantiate
//! pass ([`resolve_pending_script_targets`]).

use std::collections::BTreeMap;

use ecs::script::{ScriptRef, ScriptValue, Scripts};
use ecs::sprite_components::Name;
use ecs::{EntityId, World};
use serde::{Deserialize, Serialize};

/// Wire form of one script parameter value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScriptValueData {
    F32(f32),
    I32(i32),
    Bool(bool),
    Str(String),
    Vec2((f32, f32)),
    /// The target entity's `Name` — resolved back to an `EntityId` on load.
    Entity(String),
    Color((f32, f32, f32, f32)),
}

/// Wire form of one script binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptRefData {
    pub script_id: String,
    #[serde(default)]
    pub source_path: String,
    #[serde(default)]
    pub params: BTreeMap<String, ScriptValueData>,
}

/// Deferred name→id resolutions collected while loading `Scripts`
/// components (a scene-load-scoped World resource: an Entity param may
/// reference an entity that is created later in the same scene).
#[derive(Debug, Default)]
pub struct PendingScriptTargets(pub Vec<PendingScriptTarget>);

/// One deferred resolution: `owner`'s script at `script_index` wants its
/// `param` to point at the entity named `target_name`.
#[derive(Debug)]
pub struct PendingScriptTarget {
    pub owner: EntityId,
    pub script_index: usize,
    pub param: String,
    pub target_name: String,
}

/// Convert one wire script into the runtime type, queueing Entity params
/// for the post-instantiate resolve pass.
pub fn script_ref_from_data(
    data: &ScriptRefData,
    owner: EntityId,
    script_index: usize,
    pending: &mut PendingScriptTargets,
) -> ScriptRef {
    let mut script = ScriptRef {
        script_id: data.script_id.clone(),
        source_path: data.source_path.clone(),
        params: BTreeMap::new(),
    };
    for (key, value) in &data.params {
        match value {
            ScriptValueData::F32(v) => {
                script.params.insert(key.clone(), ScriptValue::F32(*v));
            }
            ScriptValueData::I32(v) => {
                script.params.insert(key.clone(), ScriptValue::I32(*v));
            }
            ScriptValueData::Bool(v) => {
                script.params.insert(key.clone(), ScriptValue::Bool(*v));
            }
            ScriptValueData::Str(v) => {
                script.params.insert(key.clone(), ScriptValue::Str(v.clone()));
            }
            ScriptValueData::Vec2((x, y)) => {
                script
                    .params
                    .insert(key.clone(), ScriptValue::Vec2(glam::Vec2::new(*x, *y)));
            }
            ScriptValueData::Color((r, g, b, a)) => {
                script
                    .params
                    .insert(key.clone(), ScriptValue::Color([*r, *g, *b, *a]));
            }
            ScriptValueData::Entity(target_name) => {
                pending.0.push(PendingScriptTarget {
                    owner,
                    script_index,
                    param: key.clone(),
                    target_name: target_name.clone(),
                });
            }
        }
    }
    script
}

/// Resolve every queued Entity param against the scene's name table.
/// Unresolved names leave the param absent (never a dangling id) and come
/// back as warnings the caller can surface (kimi #44 F5 — a typo in a
/// hand-edited scene must reach the status bar, not just a log line).
pub fn resolve_pending_script_targets(
    world: &mut World,
    named_entities: &std::collections::HashMap<String, EntityId>,
) -> Vec<String> {
    let mut warnings = Vec::new();
    let Some(PendingScriptTargets(pending)) = world.remove_resource::<PendingScriptTargets>()
    else {
        return warnings;
    };
    for entry in pending {
        let Some(&target) = named_entities.get(&entry.target_name) else {
            let warning = format!(
                "script param '{}' references entity '{}' which does not exist — dropped",
                entry.param, entry.target_name
            );
            log::warn!("Scene load: {warning}");
            warnings.push(warning);
            continue;
        };
        if let Some(scripts) = world.get_mut::<Scripts>(entry.owner) {
            if let Some(script) = scripts.0.get_mut(entry.script_index) {
                script.params.insert(entry.param, ScriptValue::Entity(target));
            }
        }
    }
    warnings
}

/// Convert an entity's runtime `Scripts` to wire form. Entity params map to
/// the target's `Name`; a dead or unnamed target warns and drops the param
/// (the editor's save choke point runs [`ensure_script_target_names`] first,
/// so unnamed targets only reach here on direct headless saves).
pub fn scripts_to_data(world: &World, scripts: &Scripts) -> Vec<ScriptRefData> {
    scripts
        .0
        .iter()
        .map(|script| {
            let mut data = ScriptRefData {
                script_id: script.script_id.clone(),
                source_path: script.source_path.clone(),
                params: BTreeMap::new(),
            };
            for (key, value) in &script.params {
                let wire = match value {
                    ScriptValue::F32(v) => ScriptValueData::F32(*v),
                    ScriptValue::I32(v) => ScriptValueData::I32(*v),
                    ScriptValue::Bool(v) => ScriptValueData::Bool(*v),
                    ScriptValue::Str(v) => ScriptValueData::Str(v.clone()),
                    ScriptValue::Vec2(v) => ScriptValueData::Vec2((v.x, v.y)),
                    ScriptValue::Color(c) => ScriptValueData::Color((c[0], c[1], c[2], c[3])),
                    ScriptValue::Entity(id) if *id == ScriptValue::unset_entity() => {
                        // Placeholder — never chosen; dropping it is not
                        // data loss (kimi #44 F3).
                        continue;
                    }
                    ScriptValue::Entity(id) => match world.get::<Name>(*id) {
                        Some(name) => ScriptValueData::Entity(name.0.clone()),
                        None => {
                            log::warn!(
                                "script '{}' param '{}' references a dead or unnamed entity — \
                                 dropped on save (the editor auto-names referenced targets)",
                                script.script_id,
                                key
                            );
                            continue;
                        }
                    },
                };
                data.params.insert(key.clone(), wire);
            }
            data
        })
        .collect()
}

/// Plan a `Name` for every LIVE, unnamed entity referenced by a script's
/// Entity param — save persists entity references by name, and silently
/// dropping a binding the user authored would be data loss (kimi plan
/// round-2 F4, decided: auto-name). PURE: mutates nothing; the editor
/// executes the plan through `CommandHistory` so the naming is undoable
/// and dirty-tracked (kimi #44 F1/F2), while headless callers apply it via
/// [`ensure_script_target_names`].
pub fn plan_script_target_names(world: &World) -> Vec<(EntityId, String)> {
    use std::collections::HashSet;

    // Collect referenced targets that exist but have no Name.
    let mut unnamed: Vec<EntityId> = Vec::new();
    let mut taken: HashSet<String> = HashSet::new();
    for entity in world.entities() {
        if let Some(name) = world.get::<Name>(entity) {
            taken.insert(name.0.clone());
        }
    }
    for entity in world.entities() {
        if let Some(scripts) = world.get::<Scripts>(entity) {
            for script in &scripts.0 {
                for value in script.params.values() {
                    if let ScriptValue::Entity(target) = value {
                        if world.get::<Name>(*target).is_none()
                            && world.entities().contains(target)
                            && !unnamed.contains(target)
                        {
                            unnamed.push(*target);
                        }
                    }
                }
            }
        }
    }

    let mut assigned = Vec::new();
    let mut counter = 1usize;
    for target in unnamed {
        let name = loop {
            let candidate = format!("script_target_{counter}");
            counter += 1;
            if !taken.contains(&candidate) {
                break candidate;
            }
        };
        taken.insert(name.clone());
        assigned.push((target, name));
    }
    assigned
}

/// Apply [`plan_script_target_names`] directly (headless/tooling path — the
/// editor routes the plan through `CommandHistory` instead). Callers of the
/// raw `world_to_scene_data` serializer should run this first when their
/// worlds may hold `Scripts` referencing unnamed entities (kimi #44 F4).
pub fn ensure_script_target_names(world: &mut World) -> Vec<(EntityId, String)> {
    let assigned = plan_script_target_names(world);
    for (target, name) in &assigned {
        world.add_component(target, Name::new(name.clone())).ok();
    }
    assigned
}
