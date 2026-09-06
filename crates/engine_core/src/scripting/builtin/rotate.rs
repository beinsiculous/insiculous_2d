//! Engine built-in rotation behavior.

use std::collections::BTreeMap;

use ecs::script::ScriptValue;

use crate::scripting::commands::ScriptCommands;
use crate::scripting::registry::{ParamSpec, ScriptBehavior, ScriptDescriptor};
use crate::scripting::view::{ScriptView, SelfView};

/// Static descriptor for `engine::rotate`.
pub const ROTATE_DESCRIPTOR: ScriptDescriptor = ScriptDescriptor {
    id: "engine::rotate",
    display_name: "Rotate",
    category: "Motion",
    params: &[ParamSpec {
        name: "degrees_per_second",
        default: ScriptValue::F32(90.0),
    }],
    make: || Box::new(RotateBehavior),
};

/// Continuous rotation behavior updating `Transform2D.rotation`.
pub struct RotateBehavior;

impl ScriptBehavior for RotateBehavior {
    fn update(
        &mut self,
        me: &SelfView,
        view: &ScriptView,
        params: &BTreeMap<String, ScriptValue>,
        out: &mut ScriptCommands,
    ) {
        let degrees_per_second = match params.get("degrees_per_second") {
            Some(ScriptValue::F32(d)) => *d,
            _ => 90.0,
        };
        let delta_radians = degrees_per_second.to_radians() * view.delta_time;
        out.set_rotation(me, me.transform.rotation + delta_radians);
    }
}
