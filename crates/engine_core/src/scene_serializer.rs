//! Scene serializer — converts a live World into SceneData for RON serialization.
//!
//! Pairs with `scene_loader.rs` (the loader half of the scene schema): a new
//! component type needs a builder-or-arm in `scene_loader_components.rs`, an
//! extractor and a table row in `scene_serializer/components.rs`, and the drift
//! test proves the row.

use std::path::Path;

use ecs::sprite_components::Name;
use ecs::{EntityId, World, WorldHierarchyExt};

use crate::scene_data::{EntityData, PhysicsSettings, SceneData};

mod components;
use components::extract_components;

/// Convert a World into SceneData suitable for RON serialization.
///
/// The `texture_path_fn` closure maps texture handle IDs back to their
/// original path strings (e.g., `"#white"`, `"player.png"`). Callers with
/// access to an `AssetManager` can use `assets.texture_path(handle)`;
/// tests can provide a simple default.
/// NOTE: `Scripts` Entity params persist by target NAME — a caller
/// whose world may hold scripts referencing UNNAMED entities should run
/// `script_data::ensure_script_target_names(world)` first, or those params
/// are dropped with a warning (the editor's save choke point does this,
/// undoably, for you).
pub fn world_to_scene_data(
    world: &World,
    scene_name: &str,
    physics_settings: Option<PhysicsSettings>,
    texture_path_fn: &dyn Fn(u32) -> String,
) -> SceneData {
    let mut roots = world.get_root_entities();
    roots.sort_by_key(|entity| entity.value());

    let entities: Vec<EntityData> = roots
        .iter()
        .map(|&root| entity_to_entity_data(world, root, texture_path_fn))
        .collect();

    SceneData {
        name: scene_name.to_string(),
        physics: physics_settings,
        editor: None,
        prefabs: std::collections::HashMap::new(),
        entities,
    }
}

/// Convert a single entity to EntityData, recursively including children.
fn entity_to_entity_data(
    world: &World,
    entity: EntityId,
    texture_path_fn: &dyn Fn(u32) -> String,
) -> EntityData {
    let name = world.get::<Name>(entity).map(|name| name.as_str().to_string());
    let components = extract_components(world, entity, texture_path_fn);

    let children_ids = world.get_children(entity).unwrap_or(&[]);
    let children: Vec<EntityData> = children_ids
        .iter()
        .map(|&child| entity_to_entity_data(world, child, texture_path_fn))
        .collect();

    EntityData {
        name,
        prefab: None,
        parent: None,
        overrides: Vec::new(),
        components,
        children,
    }
}

/// Serialize SceneData to a pretty-printed RON string.
pub fn serialize_to_ron(scene: &SceneData) -> Result<String, String> {
    ron::ser::to_string_pretty(scene, ron::ser::PrettyConfig::default())
        .map_err(|error| format!("RON serialization error: {error}"))
}

/// Write SceneData to a file as RON.
pub fn save_scene_to_file(scene: &SceneData, path: &Path) -> Result<(), String> {
    let ron_string = serialize_to_ron(scene)?;
    common::vfs::write_string(path, &ron_string)
        .map_err(|error| format!("Failed to write scene file: {error}"))
}

#[cfg(test)]
mod dynamic_and_scripts_tests;
#[cfg(test)]
mod tests;
