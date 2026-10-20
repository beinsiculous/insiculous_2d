//! Component names the engine once registered and has since retired.
//!
//! A saved scene may still carry one — a playground project or an older
//! editor save, written while the inspector could add it. Such an entry is
//! dropped with a warning and the scene loads; every other unregistered
//! name stays the hard error `ComponentData::Dynamic` documents.

use std::borrow::Cow;

use crate::scene_data::{ComponentData, EntityData, SceneData};

/// The retired names. A game that registers one of them again owns that
/// component, and its entries load as usual.
const RETIRED_COMPONENT_NAMES: [&str; 3] = ["AudioSource", "AudioListener", "PlaySoundEffect"];

/// `data` without any `Dynamic` entry of a retired, unregistered type — in a
/// prefab's components, an entity's overrides and its inline components,
/// children included — plus one warning per dropped entry naming its owner
/// and the component. Borrowed untouched when nothing is retired.
pub(crate) fn without_retired_components(data: &SceneData) -> (Cow<'_, SceneData>, Vec<String>) {
    let retired: Vec<&str> = RETIRED_COMPONENT_NAMES
        .into_iter()
        .filter(|name| !ecs::with_global_registry(|registry| registry.is_registered(name)))
        .collect();
    let mut warnings = Vec::new();
    if retired.is_empty() || !scene_mentions_any(data, &retired) {
        return (Cow::Borrowed(data), warnings);
    }

    let mut stripped = data.clone();
    // The prefab map is a HashMap; walking it in its own order would make the
    // first warning the status bar shows vary between runs.
    let mut prefab_names: Vec<String> = stripped.prefabs.keys().cloned().collect();
    prefab_names.sort();
    for prefab_name in prefab_names {
        if let Some(prefab) = stripped.prefabs.get_mut(&prefab_name) {
            strip_layer(&mut prefab.components, &format!("prefab '{prefab_name}'"), &retired, &mut warnings);
        }
    }
    for entity in &mut stripped.entities {
        strip_entity(entity, None, &retired, &mut warnings);
    }
    for warning in &warnings {
        log::warn!("Scene load: {warning}");
    }
    (Cow::Owned(stripped), warnings)
}

fn retired_name<'a>(component: &'a ComponentData, retired: &[&str]) -> Option<&'a str> {
    match component {
        ComponentData::Dynamic { component_type, .. } if retired.contains(&component_type.as_str()) => {
            Some(component_type.as_str())
        }
        _ => None,
    }
}

fn layer_mentions_any(layer: &[ComponentData], retired: &[&str]) -> bool {
    layer.iter().any(|component| retired_name(component, retired).is_some())
}

fn entity_mentions_any(entity: &EntityData, retired: &[&str]) -> bool {
    layer_mentions_any(&entity.overrides, retired)
        || layer_mentions_any(&entity.components, retired)
        || entity.children.iter().any(|child| entity_mentions_any(child, retired))
}

fn scene_mentions_any(data: &SceneData, retired: &[&str]) -> bool {
    data.prefabs.values().any(|prefab| layer_mentions_any(&prefab.components, retired))
        || data.entities.iter().any(|entity| entity_mentions_any(entity, retired))
}

/// `named_ancestor` is the nearest named entity above `entity`, so an unnamed
/// child's warning still says where in the tree it sits.
fn strip_entity(
    entity: &mut EntityData,
    named_ancestor: Option<&str>,
    retired: &[&str],
    warnings: &mut Vec<String>,
) {
    let owner = match (&entity.name, named_ancestor) {
        (Some(name), _) => format!("entity '{name}'"),
        (None, Some(ancestor)) => format!("an unnamed descendant of '{ancestor}'"),
        (None, None) => "an unnamed root entity".to_string(),
    };
    strip_layer(&mut entity.overrides, &owner, retired, warnings);
    strip_layer(&mut entity.components, &owner, retired, warnings);
    let nearest_named = entity.name.as_deref().or(named_ancestor);
    for child in &mut entity.children {
        strip_entity(child, nearest_named, retired, warnings);
    }
}

fn strip_layer(layer: &mut Vec<ComponentData>, owner: &str, retired: &[&str], warnings: &mut Vec<String>) {
    layer.retain(|component| match retired_name(component, retired) {
        Some(name) => {
            warnings.push(format!(
                "{owner} carried the retired component '{name}', which was dropped (play sounds from game code)"
            ));
            false
        }
        None => true,
    });
}
