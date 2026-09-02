//! The scripting seam, Stage 1 (issue #44, audit §6.3–§6.5): scripts as
//! INERT DATA. `Scripts` is one component holding N string-keyed
//! [`ScriptRef`]s — attach, edit, save, reload, undo, duplicate — and
//! nothing here executes anything. The runtime registry/runner and
//! `ParamSpec`-driven catalogs are later stages (engine_core).
//!
//! String ids on purpose: every closed enum (`Behavior`, `ComponentData`,
//! `ComponentKind`) lives upstream of the game crates and cannot be extended
//! downstream. The seam is string → map at the component, the wire format,
//! and the editor — that is what composes.

use std::collections::BTreeMap;

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::component_registry::ComponentMeta;
use crate::entity::EntityId;
use crate::DeriveComponentMeta;

/// One tunable script parameter value.
///
/// `Entity` stores a live [`EntityId`] at RUNTIME only — the scene wire
/// format persists entity references by NAME (`ScriptValueData::Entity` in
/// engine_core) because raw ids are not stable across save/load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScriptValue {
    F32(f32),
    I32(i32),
    Bool(bool),
    Str(String),
    Vec2(Vec2),
    Entity(EntityId),
    Color([f32; 4]),
}

impl ScriptValue {
    /// Variant names, cycle-selector order (the editor's add-param UI).
    pub const VARIANT_NAMES: &'static [&'static str] =
        &["F32", "I32", "Bool", "Str", "Vec2", "Entity", "Color"];

    /// The "no target chosen yet" Entity placeholder (kimi #44 F3): a fresh
    /// Entity param must NEVER alias a real entity (id 0 exists in most
    /// worlds), so the default is this sentinel — the editor shows it as
    /// unset and save drops it silently.
    pub fn unset_entity() -> EntityId {
        EntityId::with_generation(u64::MAX, 0)
    }

    /// Whether this is an Entity param still holding the unset sentinel.
    pub fn is_unset_entity(&self) -> bool {
        matches!(self, ScriptValue::Entity(id) if *id == Self::unset_entity())
    }

    /// The default value of the variant at `index` (wraps around).
    pub fn default_for_variant(index: usize) -> ScriptValue {
        match index % Self::VARIANT_NAMES.len() {
            0 => ScriptValue::F32(0.0),
            1 => ScriptValue::I32(0),
            2 => ScriptValue::Bool(false),
            3 => ScriptValue::Str(String::new()),
            4 => ScriptValue::Vec2(Vec2::ZERO),
            5 => ScriptValue::Entity(Self::unset_entity()),
            _ => ScriptValue::Color([1.0, 1.0, 1.0, 1.0]),
        }
    }

    /// This value's index in [`VARIANT_NAMES`](Self::VARIANT_NAMES).
    pub fn variant_index(&self) -> usize {
        match self {
            ScriptValue::F32(_) => 0,
            ScriptValue::I32(_) => 1,
            ScriptValue::Bool(_) => 2,
            ScriptValue::Str(_) => 3,
            ScriptValue::Vec2(_) => 4,
            ScriptValue::Entity(_) => 5,
            ScriptValue::Color(_) => 6,
        }
    }
}

/// A named script binding with its tunable parameters.
///
/// `script_id` is the string key a future runtime registry resolves
/// (Stage 3); `source_path` points at the script's source file for the
/// editor's Open-in-IDE affordance (Stage 2). In Stage 1 both are plain
/// editable data.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScriptRef {
    pub script_id: String,
    pub source_path: String,
    pub params: BTreeMap<String, ScriptValue>,
}

impl ScriptRef {
    /// A script binding with the given id and no parameters.
    pub fn new(script_id: impl Into<String>) -> Self {
        Self {
            script_id: script_id.into(),
            source_path: String::new(),
            params: BTreeMap::new(),
        }
    }
}

/// The scripts attached to an entity — ONE component, N scripts (the ECS is
/// HashMap-per-type; N script component types would recreate the
/// closed-world problem this seam exists to end).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, DeriveComponentMeta)]
pub struct Scripts(pub Vec<ScriptRef>);

impl Scripts {
    /// Whether no scripts are attached.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scripts_serde_round_trips_every_value_variant() -> Result<(), Box<dyn std::error::Error>> {
        let mut params = BTreeMap::new();
        params.insert("speed".to_string(), ScriptValue::F32(240.0));
        params.insert("lives".to_string(), ScriptValue::I32(-3));
        params.insert("armed".to_string(), ScriptValue::Bool(true));
        params.insert("greeting".to_string(), ScriptValue::Str("hi".to_string()));
        params.insert("dir".to_string(), ScriptValue::Vec2(Vec2::new(1.0, -0.5)));
        params.insert("target".to_string(), ScriptValue::Entity(EntityId::with_generation(7, 1)));
        params.insert("tint".to_string(), ScriptValue::Color([0.1, 0.2, 0.3, 1.0]));
        let scripts = Scripts(vec![ScriptRef {
            script_id: "patrol".to_string(),
            source_path: "src/scripts/patrol.rs".to_string(),
            params,
        }]);

        // JSON (inspector / command API path).
        let json = serde_json::to_value(&scripts)?;
        let from_json: Scripts = serde_json::from_value(json)?;
        assert_eq!(from_json, scripts);

        // RON (scene path: the Stage 1 wire maps entities by name, but the
        // component itself must round-trip too for snapshots).
        let ron = ron::to_string(&scripts)?;
        let from_ron: Scripts = ron::from_str(&ron)?;
        assert_eq!(from_ron, scripts);
        Ok(())
    }
}
