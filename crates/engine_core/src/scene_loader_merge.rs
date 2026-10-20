//! The layering every scene entity loads through: its prefab's components,
//! then its overrides field by field, then its inline components whole.

use std::collections::{BTreeSet, HashMap};

use serde_json::{Map, Value};

use crate::scene_data::{ComponentData, EntityData, PrefabData, SceneData, SceneLoadError};
use crate::scene_loader::SceneLoader;
use crate::scene_override::OverrideData;

impl SceneLoader {
    /// The components `entity` loads with, its inline children aside: the
    /// prefab's, each override merged over the prefab's component of the same
    /// type, then the inline `components`, each replacing its type whole.
    ///
    /// An override names only the fields it changes; everything else stays
    /// the prefab's. An override whose component the prefab lacks is appended
    /// with serde defaults for what it omits. Scene loading, runtime
    /// [`spawn_prefab`](crate::scene_loader::SceneInstance::spawn_prefab) and
    /// any tool reading a scene's entities share this one merge.
    ///
    /// # Errors
    /// The prefab does not exist; an override names a field its component
    /// lacks; a typed component and a `Dynamic` one share a type name; a
    /// `Dynamic` payload is not an object.
    pub fn effective_components(
        scene: &SceneData,
        entity: &EntityData,
    ) -> Result<Vec<ComponentData>, SceneLoadError> {
        Self::layered_components(&scene.prefabs, entity)
    }

    pub(crate) fn layered_components(
        prefabs: &HashMap<String, PrefabData>,
        entity: &EntityData,
    ) -> Result<Vec<ComponentData>, SceneLoadError> {
        let base: &[ComponentData] = match &entity.prefab {
            Some(prefab_name) => &prefabs
                .get(prefab_name)
                .ok_or_else(|| SceneLoadError::PrefabNotFound(prefab_name.clone()))?
                .components,
            None => &[],
        };
        Self::merge_components(base, &entity.overrides, &entity.components, &entity_label(entity))
    }

    /// `base` with each override merged in, then each inline component
    /// replacing or appending by type name. `entity` names the owner in errors.
    pub(crate) fn merge_components(
        base: &[ComponentData],
        overrides: &[OverrideData],
        inline: &[ComponentData],
        entity: &str,
    ) -> Result<Vec<ComponentData>, SceneLoadError> {
        let mut result = base.to_vec();
        for layer in overrides {
            match Self::index_of_type(&result, &layer.component) {
                Some(index) => result[index] = merge_override(&result[index], layer, entity)?,
                None => result.push(layer.component.clone()),
            }
        }
        for component in inline {
            match Self::index_of_type(&result, component) {
                Some(index) => result[index] = component.clone(),
                None => result.push(component.clone()),
            }
        }
        Ok(result)
    }

    fn index_of_type(components: &[ComponentData], component: &ComponentData) -> Option<usize> {
        let component_type = Self::component_type_name(component);
        components
            .iter()
            .position(|candidate| Self::component_type_name(candidate) == component_type)
    }
}

fn entity_label(entity: &EntityData) -> String {
    match (&entity.name, &entity.prefab) {
        (Some(name), _) => format!("entity '{name}'"),
        (None, Some(prefab)) => format!("an unnamed instance of prefab '{prefab}'"),
        (None, None) => "an unnamed entity".to_string(),
    }
}

fn component_error(entity: &str, component: &str, problem: impl std::fmt::Display) -> SceneLoadError {
    SceneLoadError::ComponentError(format!("{entity}, component '{component}': {problem}"))
}

/// `layer` merged over `base`, two components of the same type name.
fn merge_override(
    base: &ComponentData,
    layer: &OverrideData,
    entity: &str,
) -> Result<ComponentData, SceneLoadError> {
    let Some(fields) = &layer.fields else {
        return Ok(layer.component.clone());
    };
    let component = SceneLoader::component_type_name(base);
    match (base, &layer.component) {
        (
            ComponentData::Dynamic { data: base_data, .. },
            ComponentData::Dynamic { component_type, data: override_data },
        ) => merge_dynamic(component_type, base_data, override_data, entity),
        (ComponentData::Dynamic { .. }, _) | (_, ComponentData::Dynamic { .. }) => Err(component_error(
            entity,
            component,
            "written once as a typed component and once as Dynamic; write it one way",
        )),
        _ => merge_struct_fields(base, &layer.component, fields, entity),
    }
}

/// Copy each of `fields` from `layer` onto `base`, two struct variants of the
/// same type. serde_json writes both externally tagged, `{"Transform2D": {…}}`.
fn merge_struct_fields(
    base: &ComponentData,
    layer: &ComponentData,
    fields: &BTreeSet<String>,
    entity: &str,
) -> Result<ComponentData, SceneLoadError> {
    let component = SceneLoader::component_type_name(base);
    let mut merged = serde_json::to_value(base).map_err(|error| component_error(entity, component, error))?;
    let layer_value = serde_json::to_value(layer).map_err(|error| component_error(entity, component, error))?;
    let (Some(base_fields), Some(layer_fields)) = (
        merged.get_mut(component).and_then(Value::as_object_mut),
        layer_value.get(component).and_then(Value::as_object),
    ) else {
        return Err(component_error(entity, component, "has no fields to merge"));
    };
    for field in fields {
        let value = layer_fields.get(field).filter(|_| base_fields.contains_key(field));
        let Some(value) = value else {
            return Err(component_error(entity, component, format!("has no field '{field}'")));
        };
        base_fields.insert(field.clone(), value.clone());
    }
    serde_json::from_value(merged).map_err(|error| component_error(entity, component, error))
}

/// `override_data`'s keys copied onto `base_data`, each one a field of the
/// registered type's default value. Either payload may omit fields.
fn merge_dynamic(
    component_type: &str,
    base_data: &Value,
    override_data: &Value,
    entity: &str,
) -> Result<ComponentData, SceneLoadError> {
    let (Some(base_object), Some(override_object)) = (base_data.as_object(), override_data.as_object())
    else {
        return Err(component_error(
            entity,
            component_type,
            "Dynamic data must be an object on both the prefab and the override",
        ));
    };
    let known_fields: Map<String, Value> =
        ecs::with_global_registry(|registry| registry.default_value(component_type))
            .and_then(|default| default.as_object().cloned())
            .ok_or_else(|| component_error(entity, component_type, "is not a registered component with fields"))?;

    let mut merged = base_object.clone();
    for (field, value) in override_object {
        if !known_fields.contains_key(field) {
            return Err(component_error(entity, component_type, format!("has no field '{field}'")));
        }
        merged.insert(field.clone(), value.clone());
    }
    Ok(ComponentData::Dynamic { component_type: component_type.to_string(), data: Value::Object(merged) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transform_at(x: f32) -> ComponentData {
        ComponentData::Transform2D { position: (x, 0.0), rotation: 0.0, scale: (1.0, 1.0) }
    }

    fn position_of(component: &ComponentData) -> (f32, f32) {
        match component {
            ComponentData::Transform2D { position, .. } => *position,
            other => panic!("expected Transform2D, got {other:?}"),
        }
    }

    #[test]
    fn later_layers_replace_earlier_ones_by_component_type_and_append_the_rest() -> Result<(), SceneLoadError> {
        // prefab base < entity overrides < inline components — the same
        // type is replaced in place, a type only one layer carries is kept.
        let base = vec![transform_at(0.0), ComponentData::EntityTag { tag: "ball".to_string() }];
        let overrides = vec![OverrideData::from(transform_at(100.0))];
        let inline = vec![transform_at(500.0), ComponentData::Sprite {
            texture: "#white".to_string(),
            offset: (0.0, 0.0),
            rotation: 0.0,
            scale: (1.0, 1.0),
            color: (1.0, 1.0, 1.0, 1.0),
            depth: 0.0,
            emissive: 0.0,
            tex_region: (0.0, 0.0, 1.0, 1.0),
            visible: true,
        }];

        let base_and_overrides = SceneLoader::merge_components(&base, &overrides, &[], "test")?;
        let all_three = SceneLoader::merge_components(&base, &overrides, &inline, "test")?;

        assert_eq!(base_and_overrides.len(), 2, "override replaces, tag kept: {base_and_overrides:?}");
        assert_eq!(position_of(&base_and_overrides[0]), (100.0, 0.0));
        assert_eq!(all_three.len(), 3, "inline replaces the transform and appends the sprite: {all_three:?}");
        assert_eq!(position_of(&all_three[0]), (500.0, 0.0));
        assert!(matches!(&all_three[1], ComponentData::EntityTag { tag } if tag == "ball"));
        assert!(matches!(&all_three[2], ComponentData::Sprite { .. }));
        Ok(())
    }
}
