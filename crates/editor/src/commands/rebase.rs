//! Replaying a paused edit onto the world Stop restored.
//!
//! A command recorded while the play session was paused holds SIMULATED
//! values: its before-image is where the simulation had moved the entity,
//! and its after-image carries every field the simulation moved alongside
//! the one field the user actually edited. Replaying either verbatim onto
//! the authored world writes simulation state into the scene.
//!
//! The rule this module implements: keep the fields the user CHANGED, at
//! their edited values, and take the authored value for everything else.

use ecs::script::{ScriptValue, Scripts};
use ecs::{EntityId, World};
use serde_json::Value;

/// Whether a rebased command still makes sense on the restored world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rebase {
    /// Re-execute the command as part of Keep.
    Apply,
    /// Re-execute what remains of a macro whose `dropped` children could
    /// not apply, so the count reaches the status line.
    Partial { dropped: usize },
    /// The command's precondition no longer holds — leave it out of the
    /// retained history entirely. Keeping it would let its later undo
    /// resurrect a phantom entity or strip an authored component.
    /// `entries` is how many edits went with it: one for a plain command,
    /// every child for a macro that lost all of them.
    Drop { entries: usize },
}

impl Rebase {
    /// One command that cannot apply.
    pub const DROP: Rebase = Rebase::Drop { entries: 1 };
}

/// `authored` with every leaf that differs between `before` and `after`
/// replaced by the `after` leaf.
///
/// Objects recurse by key and EQUAL-LENGTH arrays recurse by index: a
/// `glam::Vec2` serializes as `[x, y]`, so a paused edit of X alone must
/// not drag the simulated Y along with it. Unequal-length arrays and
/// scalars are leaves — a whole-vector rewrite is one change.
pub fn patch_changed_leaves(authored: &Value, before: &Value, after: &Value) -> Value {
    if before == after {
        return authored.clone();
    }
    match (authored, before, after) {
        (Value::Object(authored_map), Value::Object(before_map), Value::Object(after_map)) => {
            // An externally tagged enum is a one-key object whose key IS
            // the value: when that key changed, the edit is the whole
            // value, whatever variant the authored scene or the
            // simulation held.
            if before_map.len() == 1 && after_map.len() == 1 && before_map.keys().ne(after_map.keys()) {
                return after.clone();
            }
            let mut patched = authored_map.clone();
            for (key, after_field) in after_map {
                let Some(before_field) = before_map.get(key) else {
                    patched.insert(key.clone(), after_field.clone());
                    continue;
                };
                match authored_map.get(key) {
                    Some(authored_field) => {
                        patched.insert(
                            key.clone(),
                            patch_changed_leaves(authored_field, before_field, after_field),
                        );
                    }
                    None => {
                        patched.insert(key.clone(), after_field.clone());
                    }
                }
            }
            // A key present before and absent after was removed — an
            // externally tagged enum changes variant this way, and leaving
            // the old variant beside the new one deserializes to nothing.
            for key in before_map.keys() {
                if !after_map.contains_key(key) {
                    patched.remove(key);
                }
            }
            Value::Object(patched)
        }
        (Value::Array(authored_items), Value::Array(before_items), Value::Array(after_items))
            if authored_items.len() == before_items.len()
                && before_items.len() == after_items.len() =>
        {
            Value::Array(
                authored_items
                    .iter()
                    .zip(before_items)
                    .zip(after_items)
                    .map(|((authored_item, before_item), after_item)| {
                        patch_changed_leaves(authored_item, before_item, after_item)
                    })
                    .collect(),
            )
        }
        _ => after.clone(),
    }
}

/// Whether every `ScriptValue::Entity` parameter in `value` points at an
/// entity that exists in `world` (the unset sentinel counts as fine).
///
/// A binding to an entity that only existed during Play cannot survive
/// Stop: `scripts_to_data` would drop it silently on preview and export,
/// so the whole edit is dropped instead and counted for the user.
pub fn script_references_resolve<T: 'static>(value: &T, world: &World) -> bool {
    let Some(scripts) = (value as &dyn std::any::Any).downcast_ref::<Scripts>() else {
        return true;
    };
    let live = world.entities();
    let unset = ScriptValue::unset_entity();
    scripts.0.iter().all(|script| {
        script.params.values().all(|param| match param {
            ScriptValue::Entity(target) => *target == unset || live.contains(target),
            _ => true,
        })
    })
}

/// Whether `entity` exists in `world`.
pub fn entity_is_alive(world: &World, entity: EntityId) -> bool {
    world.entities().contains(&entity)
}

#[cfg(test)]
mod tests {
    use super::patch_changed_leaves;
    use serde_json::json;

    #[test]
    fn test_patch_removes_a_key_absent_after_so_an_enum_changes_variant() {
        // An externally tagged enum is a one-key object; switching a
        // collider from Box to Circle must not leave both variants behind.
        let authored = json!({ "shape": { "Box": { "w": 1.0, "h": 1.0 } }, "friction": 0.5 });
        let before = json!({ "shape": { "Box": { "w": 1.0, "h": 1.0 } }, "friction": 0.9 });
        let after = json!({ "shape": { "Circle": { "r": 2.0 } }, "friction": 0.9 });
        let patched = patch_changed_leaves(&authored, &before, &after);
        assert_eq!(patched, json!({ "shape": { "Circle": { "r": 2.0 } }, "friction": 0.5 }));
    }

    #[test]
    fn test_patch_replaces_an_enum_whose_authored_before_and_after_variants_all_differ() {
        let authored = json!({ "shape": { "Box": { "w": 1.0, "h": 1.0 } } });
        let before = json!({ "shape": { "Circle": { "r": 2.0 } } });
        let after = json!({ "shape": { "CapsuleY": { "r": 1.0, "half": 3.0 } } });
        assert_eq!(patch_changed_leaves(&authored, &before, &after), after);
    }

    #[test]
    fn test_patch_recurses_equal_length_arrays_by_index() {
        let authored = json!({ "position": [0.0, 0.0] });
        let before = json!({ "position": [100.0, 50.0] });
        let after = json!({ "position": [120.0, 50.0] });
        assert_eq!(
            patch_changed_leaves(&authored, &before, &after),
            json!({ "position": [120.0, 0.0] })
        );
    }
}
