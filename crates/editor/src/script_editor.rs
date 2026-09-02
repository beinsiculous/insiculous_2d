//! Editable inspector for the `Scripts` component (issue #44, Stage 1).
//!
//! Widgets are inferred from the [`ScriptValue`] variant — no per-script
//! Rust anywhere in this crate, which is the entire point of the seam.
//! Everything returns a full-value [`ComponentEdit`] so undo/redo rides the
//! ordinary `SetScriptsCommand`; structural clicks (add/remove script or
//! param) merge by `field_hint` like behavior variant cycling — the
//! accepted Stage-1 precedent.
//!
//! Stage-1 limits (documented): `Entity` params display read-only (an
//! entity picker is a later stage), and a param's TYPE change resets its
//! value to that variant's default.

use ecs::script::{ScriptRef, ScriptValue, Scripts};

use crate::component_editors::ComponentEdit;
use crate::editable_inspector::EditableInspector;
use crate::field_style::EditResult;

/// Range for numeric script params: unconstrained scrub, soft by design —
/// the engine cannot know a game parameter's meaningful bounds.
const PARAM_RANGE: std::ops::RangeInclusive<f32> = -1_000_000.0..=1_000_000.0;

/// Edit a `Scripts` component. One change per frame wins (full-value edit).
pub fn edit_scripts(
    inspector: &mut EditableInspector<'_>,
    scripts: &Scripts,
    _extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<Scripts>> {
    inspector.header("Scripts");

    let mut edit: Option<ComponentEdit<Scripts>> = None;

    for (script_index, script) in scripts.0.iter().enumerate() {
        if let Some(e) = edit_one_script(inspector, scripts, script_index, script) {
            edit = edit.or(Some(e));
        }
    }

    if inspector.action_button("+ Add Script") && edit.is_none() {
        let mut new = scripts.clone();
        new.0.push(ScriptRef::new("new_script"));
        edit = Some(ComponentEdit {
            new_value: new,
            field_hint: "scripts_structure",
        });
    }

    edit
}

/// Rows for one script: id, source, params (name/type/value per param),
/// param add/remove, script remove.
fn edit_one_script(
    inspector: &mut EditableInspector<'_>,
    scripts: &Scripts,
    script_index: usize,
    script: &ScriptRef,
) -> Option<ComponentEdit<Scripts>> {
    if let EditResult::Changed(v) = inspector.string_edit("Script id", &script.script_id) {
        if v != script.script_id {
            let mut new = scripts.clone();
            new.0[script_index].script_id = v;
            return Some(ComponentEdit { new_value: new, field_hint: "scripts_id" });
        }
    }
    if let EditResult::Changed(v) = inspector.string_edit("Source", &script.source_path) {
        if v != script.source_path {
            let mut new = scripts.clone();
            new.0[script_index].source_path = v;
            return Some(ComponentEdit { new_value: new, field_hint: "scripts_source" });
        }
    }

    let keys: Vec<String> = script.params.keys().cloned().collect();
    for key in &keys {
        if let Some(e) = edit_one_param(inspector, scripts, script_index, script, key) {
            return Some(e);
        }
    }

    if inspector.action_button("+ Add param") {
        let mut new = scripts.clone();
        let name = unique_param_name(&new.0[script_index]);
        new.0[script_index]
            .params
            .insert(name, ScriptValue::F32(0.0));
        return Some(ComponentEdit { new_value: new, field_hint: "scripts_structure" });
    }
    if inspector.action_button(&format!("− Remove script '{}'", script.script_id)) {
        let mut new = scripts.clone();
        new.0.remove(script_index);
        return Some(ComponentEdit { new_value: new, field_hint: "scripts_structure" });
    }

    None
}

/// Rows for one param: rename (name row), type cycle, value widget by
/// variant, remove.
fn edit_one_param(
    inspector: &mut EditableInspector<'_>,
    scripts: &Scripts,
    script_index: usize,
    script: &ScriptRef,
    key: &str,
) -> Option<ComponentEdit<Scripts>> {
    let value = &script.params[key];

    // Rename: editing the KEY moves the value under the new name
    // (collisions with an existing param are ignored — no silent clobber).
    if let EditResult::Changed(new_key) = inspector.string_edit("Param", key) {
        let new_key = new_key.trim().to_string();
        if !new_key.is_empty() && new_key != key && !script.params.contains_key(&new_key) {
            let mut new = scripts.clone();
            let moved = new.0[script_index].params.remove(key);
            if let Some(moved) = moved {
                new.0[script_index].params.insert(new_key, moved);
            }
            return Some(ComponentEdit { new_value: new, field_hint: "scripts_structure" });
        }
    }

    // Type: cycling resets the value to the new variant's default.
    let index = value.variant_index();
    let count = ScriptValue::VARIANT_NAMES.len();
    if let EditResult::Changed(new_index) =
        inspector.cycle("  type", ScriptValue::VARIANT_NAMES[index], index, count)
    {
        if new_index != index {
            let mut new = scripts.clone();
            new.0[script_index]
                .params
                .insert(key.to_string(), ScriptValue::default_for_variant(new_index));
            return Some(ComponentEdit { new_value: new, field_hint: "scripts_structure" });
        }
    }

    // Value widget by variant.
    let changed: Option<ScriptValue> = match value {
        ScriptValue::F32(v) => match inspector.f32("  value", *v, PARAM_RANGE) {
            EditResult::Changed(nv) => Some(ScriptValue::F32(nv)),
            _ => None,
        },
        ScriptValue::I32(v) => match inspector.f32("  value", *v as f32, PARAM_RANGE) {
            // Local integer wrapper: round on commit (Stage 1 — no int field).
            EditResult::Changed(nv) => Some(ScriptValue::I32(nv.round() as i32)),
            _ => None,
        },
        ScriptValue::Bool(v) => match inspector.bool("  value", *v) {
            EditResult::Changed(nv) => Some(ScriptValue::Bool(nv)),
            _ => None,
        },
        ScriptValue::Str(v) => match inspector.string_edit("  value", v) {
            EditResult::Changed(nv) => Some(ScriptValue::Str(nv)),
            _ => None,
        },
        ScriptValue::Vec2(v) => match inspector.vec2("  value", *v, PARAM_RANGE) {
            EditResult::Changed(nv) => Some(ScriptValue::Vec2(nv)),
            _ => None,
        },
        ScriptValue::Color(c) => {
            let v4 = glam::Vec4::new(c[0], c[1], c[2], c[3]);
            match inspector.color("  value", v4) {
                EditResult::Changed(nv) => Some(ScriptValue::Color([nv.x, nv.y, nv.z, nv.w])),
                _ => None,
            }
        }
        ScriptValue::Entity(id) => {
            // Read-only in Stage 1 (no entity picker yet). The unset
            // sentinel never aliases a real entity (kimi #44 F3).
            if value.is_unset_entity() {
                inspector.string("  value", "Entity (unset)");
            } else {
                inspector.string("  value", &format!("Entity #{}", id.value()));
            }
            None
        }
    };
    if let Some(new_value) = changed {
        if new_value != *value {
            let mut new = scripts.clone();
            new.0[script_index].params.insert(key.to_string(), new_value);
            return Some(ComponentEdit { new_value: new, field_hint: "scripts_param" });
        }
    }

    if inspector.action_button(&format!("− Remove param '{key}'")) {
        let mut new = scripts.clone();
        new.0[script_index].params.remove(key);
        return Some(ComponentEdit { new_value: new, field_hint: "scripts_structure" });
    }

    None
}

/// First `param_N` not already taken on this script.
fn unique_param_name(script: &ScriptRef) -> String {
    let mut n = 1usize;
    loop {
        let candidate = format!("param_{n}");
        if !script.params.contains_key(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    const ORIGIN: Vec2 = Vec2::new(10.0, 10.0);
    const WIDTH: f32 = 300.0;

    /// Center of the action button on row `row_index` (0 = first row under
    /// the header), mirroring EditableInspector's layout math.
    fn action_button_center(row_index: usize) -> Vec2 {
        let style = crate::EditableFieldStyle::default();
        let row_y = ORIGIN.y + style.row_height + 4.0 + row_index as f32 * style.row_height;
        let pos_x = ORIGIN.x + style.indent;
        let control_x = pos_x + style.label_width;
        let right = ORIGIN.x + WIDTH;
        Vec2::new(
            control_x + (right - control_x).max(60.0) / 2.0,
            row_y + 2.0 + (style.row_height - 4.0) / 2.0,
        )
    }

    /// Press+release frames at `point`, running edit_scripts each frame
    /// against `scripts`; returns the release frame's edit.
    fn click_scripts(scripts: &Scripts, point: Vec2) -> Option<ComponentEdit<Scripts>> {
        use input::prelude::MouseButton;
        let mut ui = ui::UIContext::new();
        let mut input = input::InputHandler::new();
        let mut drag_drop = crate::DragDropState::new();

        let mut run = |ui: &mut ui::UIContext, input: &input::InputHandler| {
            ui.begin_frame(input, Vec2::new(800.0, 600.0));
            let mut extras =
                crate::InspectorExtras { drag_drop: &mut drag_drop, texture_display: None, warnings: Vec::new() };
            let mut inspector = EditableInspector::new(ui, ORIGIN.x, ORIGIN.y);
            edit_scripts(&mut inspector, scripts, &mut extras)
        };

        input.mouse_mut().update_position(point.x, point.y);
        input.mouse_mut().handle_button_press(MouseButton::Left);
        let _press = run(&mut ui, &input);
        ui.end_frame();
        input.mouse_mut().handle_button_release(MouseButton::Left);
        let release = run(&mut ui, &input);
        ui.end_frame();
        release
    }

    #[test]
    fn test_add_script_button_appends_a_default_script() {
        let scripts = Scripts::default();
        // Empty component: "+ Add Script" is the first row under the header.
        let edit = click_scripts(&scripts, action_button_center(0))
            .expect("clicking + Add Script emits an edit");
        assert_eq!(edit.field_hint, "scripts_structure");
        assert_eq!(edit.new_value.0.len(), 1);
        assert_eq!(edit.new_value.0[0].script_id, "new_script");
    }

    #[test]
    fn test_add_param_button_creates_a_unique_f32_param() {
        let mut scripts = Scripts(vec![ScriptRef::new("patrol")]);
        scripts.0[0]
            .params
            .insert("param_1".to_string(), ScriptValue::Bool(true));
        // Rows: Id(0), Source(1), param name(2), type(3), value(4),
        // remove-param(5), + Add param(6).
        let edit = click_scripts(&scripts, action_button_center(6))
            .expect("clicking + Add param emits an edit");
        assert_eq!(edit.field_hint, "scripts_structure");
        let params = &edit.new_value.0[0].params;
        assert_eq!(params.len(), 2);
        assert_eq!(params.get("param_2"), Some(&ScriptValue::F32(0.0)), "name collision skipped");
    }

    #[test]
    fn test_remove_script_button_deletes_the_script() {
        let scripts = Scripts(vec![ScriptRef::new("patrol")]);
        // Rows: Id(0), Source(1), + Add param(2), − Remove script(3).
        let edit = click_scripts(&scripts, action_button_center(3))
            .expect("clicking − Remove script emits an edit");
        assert_eq!(edit.field_hint, "scripts_structure");
        assert!(edit.new_value.0.is_empty());
    }

    #[test]
    fn test_no_input_emits_no_edit() {
        let scripts = Scripts(vec![ScriptRef::new("patrol")]);
        let mut ui = ui::UIContext::new();
        let input = input::InputHandler::new();
        let mut drag_drop = crate::DragDropState::new();
        ui.begin_frame(&input, Vec2::new(800.0, 600.0));
        let mut extras =
            crate::InspectorExtras { drag_drop: &mut drag_drop, texture_display: None, warnings: Vec::new() };
        let mut inspector = EditableInspector::new(&mut ui, ORIGIN.x, ORIGIN.y);
        assert!(edit_scripts(&mut inspector, &scripts, &mut extras).is_none());
    }
}
