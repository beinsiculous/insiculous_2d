//! One entry of an entity's `overrides`: the component as written, plus the
//! top-level fields its source text named.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::scene_data::ComponentData;

/// A prefab override.
///
/// `fields: None` replaces the prefab's component whole — always so for the
/// newtype variants (`Behavior`, `Scripts`), and for a runtime caller's
/// whole component. `Some(keys)` replaces only those top-level fields and
/// keeps the rest of the prefab's. Presence comes from the source text, never
/// from comparing against defaults: an explicit `scale: (1.0, 1.0)` is
/// honoured over a prefab's `(0.8, 0.4)`.
///
/// Serializing writes the component alone, so a round trip turns a field
/// override into a whole one; the scene serializer flattens prefabs and
/// writes no overrides.
#[derive(Debug, Clone)]
pub struct OverrideData {
    /// The component, every field it omits filled with its serde default.
    pub component: ComponentData,
    /// The top-level fields the override names, or `None` for the whole component.
    pub fields: Option<BTreeSet<String>>,
}

impl OverrideData {
    /// An override of only `fields` of `component` — the runtime counterpart
    /// of a scene file's override naming them. A newtype variant has no
    /// fields and replaces whole whatever `fields` names. A `Dynamic`
    /// component's field set is its `data` object: every key it carries
    /// merges and `fields` is not consulted, so put only the keys you mean
    /// in `data`. A component the prefab lacks is appended whole, `fields`
    /// or not — there is nothing to merge it over.
    pub fn fields(component: ComponentData, fields: &[&str]) -> Self {
        let fields = (!replaces_whole(&component))
            .then(|| fields.iter().map(|field| field.to_string()).collect());
        Self { component, fields }
    }
}

impl From<ComponentData> for OverrideData {
    fn from(component: ComponentData) -> Self {
        Self { component, fields: None }
    }
}

impl Serialize for OverrideData {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.component.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OverrideData {
    /// Only RON can carry an override: the field set is read back out of the
    /// entry's own text, which no other format hands over.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Box::<ron::value::RawValue>::deserialize(deserializer)?;
        let component: ComponentData = raw.into_rust().map_err(serde::de::Error::custom)?;
        let fields = if replaces_whole(&component) {
            None
        } else {
            Some(named_fields(raw.get_ron()).map_err(serde::de::Error::custom)?)
        };
        Ok(Self { component, fields })
    }
}

fn replaces_whole(component: &ComponentData) -> bool {
    matches!(component, ComponentData::Behavior(_) | ComponentData::Scripts(_))
}

/// The field names a struct variant's text spells out. ron visits a named
/// struct (`Transform2D(position: …)`) as a map keyed by its field names.
fn named_fields(text: &str) -> Result<BTreeSet<String>, String> {
    let value: ron::Value = ron::from_str(text).map_err(|error| error.to_string())?;
    let map = match value {
        ron::Value::Map(map) => map,
        // `Transform2D()` reads as an empty sequence: an override that names
        // nothing changes nothing.
        ron::Value::Seq(empty) if empty.is_empty() => return Ok(BTreeSet::new()),
        _ => return Err(format!("an override's fields could not be read from `{text}`")),
    };
    map.iter()
        .map(|(key, _)| match key {
            ron::Value::String(name) => Ok(name.clone()),
            other => Err(format!("an override's field name is not a name: {other:?} in `{text}`")),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_overrides(text: &str) -> Vec<OverrideData> {
        ron::from_str(text).expect("the override list parses")
    }

    #[test]
    fn ron_visits_a_named_struct_variant_as_a_map_of_its_written_fields() {
        let value: ron::Value =
            ron::from_str("Transform2D(position: (1.0, 2.0))").expect("parses as a value");

        let ron::Value::Map(map) = value else {
            panic!("a named struct variant must read as a map, got {value:?}");
        };
        let keys: Vec<&ron::Value> = map.keys().collect();
        assert_eq!(keys, vec![&ron::Value::String("position".to_string())]);
    }

    #[test]
    fn an_override_records_exactly_the_fields_its_text_names() {
        let overrides = parse_overrides(
            r#"[
                Transform2D(position: (1.0, 2.0), scale: (1.0, 1.0)),
                Dynamic(type: "Health", data: {"value": 5.0}),
            ]"#,
        );

        let transform_fields: Vec<&str> =
            overrides[0].fields.iter().flatten().map(String::as_str).collect();
        assert_eq!(transform_fields, ["position", "scale"], "an explicit default is still named");
        assert!(
            matches!(overrides[0].component, ComponentData::Transform2D { rotation, .. } if rotation == 0.0),
            "an omitted field is filled with its serde default"
        );
        assert!(overrides[1].fields.is_some(), "Dynamic is a struct variant and merges");
    }

    #[test]
    fn a_bare_unit_variant_value_still_reads_as_a_named_field() {
        // `body_type: Static` has no parentheses; ron must still visit the
        // struct as a map with that key, or a physics override could not load.
        let overrides = parse_overrides("[RigidBody(body_type: Static, can_rotate: false)]");

        let fields: Vec<&str> = overrides[0].fields.iter().flatten().map(String::as_str).collect();
        assert_eq!(fields, ["body_type", "can_rotate"]);
        assert!(matches!(
            overrides[0].component,
            ComponentData::RigidBody { body_type: crate::scene_data::RigidBodyTypeData::Static, .. }
        ));
    }

    #[test]
    fn an_override_that_names_no_fields_loads_and_changes_nothing() {
        let overrides = parse_overrides("[Transform2D(), Sprite()]");

        for layer in &overrides {
            assert_eq!(layer.fields.as_ref().map(BTreeSet::len), Some(0), "{layer:?}");
        }
    }

    #[test]
    fn a_newtype_variant_override_replaces_whole() {
        let overrides = parse_overrides(
            r#"[Behavior(Collectible(score_value: 5)), Scripts([])]"#,
        );
        assert!(overrides.iter().all(|layer| layer.fields.is_none()), "{overrides:?}");

        let runtime = OverrideData::fields(ComponentData::Scripts(Vec::new()), &["anything"]);
        assert!(runtime.fields.is_none(), "a runtime newtype override replaces whole too");
    }
}
