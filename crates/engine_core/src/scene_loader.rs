//! Scene loader for RON scene files
//!
//! This module provides functionality to load scenes from RON files,
//! resolve prefabs, and instantiate entities in the ECS world.

use std::collections::HashMap;
use std::path::Path;

use ecs::sprite_components::Name;
use ecs::{EntityId, World, WorldHierarchyExt};

use crate::texture_ref::TextureResolver;
use crate::scene_data::{EntityData, PhysicsSettings, PrefabData, SceneData, SceneLoadError};
use crate::scene_override::OverrideData;

/// Result of loading a scene
#[derive(Debug)]
pub struct SceneInstance {
    /// Scene name
    pub name: String,
    /// Physics settings (if any)
    pub physics: Option<PhysicsSettings>,
    /// Mapping from entity names to EntityIds
    pub named_entities: HashMap<String, EntityId>,
    /// All created entity IDs
    pub entities: Vec<EntityId>,
    /// Number of entities created
    pub entity_count: usize,
    /// The scene's prefab table, retained for runtime spawning via
    /// [`spawn_prefab`](Self::spawn_prefab).
    pub prefabs: HashMap<String, PrefabData>,
    /// Non-fatal load diagnostics worth showing the user (e.g. a script
    /// Entity param naming a missing entity). Empty on a
    /// clean load.
    pub load_warnings: Vec<String>,
}

impl SceneInstance {
    /// Get an entity by name
    pub fn get_entity(&self, name: &str) -> Option<EntityId> {
        self.named_entities.get(name).copied()
    }

    /// Whether the scene defines a prefab with this name.
    pub fn has_prefab(&self, name: &str) -> bool {
        self.prefabs.contains_key(name)
    }

    /// Spawn a new entity from a named prefab at runtime (Prototype pattern).
    ///
    /// Each override names only the fields it changes, the same merge as
    /// scene-file `overrides` ([`SceneLoader::effective_components`]). Returns
    /// the new entity, or an error if the prefab doesn't exist, an override
    /// does not fit it, or a component fails to build (in which case no
    /// half-built entity is left behind). The spawned
    /// entity is NOT added to `entities`/`named_entities` — the caller owns
    /// its lifecycle.
    pub fn spawn_prefab(
        &self,
        world: &mut World,
        assets: &mut impl TextureResolver,
        prefab_name: &str,
        overrides: &[OverrideData],
    ) -> Result<EntityId, SceneLoadError> {
        let prefab = self
            .prefabs
            .get(prefab_name)
            .ok_or_else(|| SceneLoadError::PrefabNotFound(prefab_name.to_string()))?;

        let merged = SceneLoader::merge_components(
            &prefab.components,
            overrides,
            &[],
            &format!("a spawned '{prefab_name}'"),
        )?;
        let entity_id = world.create_entity();
        for component in &merged {
            if let Err(e) = SceneLoader::add_component_to_entity(entity_id, component, world, assets) {
                world.remove_entity(&entity_id).ok();
                return Err(e);
            }
        }
        Ok(entity_id)
    }
}

/// Scene loader for parsing and instantiating RON scene files
pub struct SceneLoader;

impl SceneLoader {
    /// Load scene data from a RON file
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<SceneData, SceneLoadError> {
        let content = common::vfs::read_to_string(path.as_ref())?;
        Self::parse(&content)
    }

    /// First `.ron` scene in a directory, in sorted order, or `None` when the
    /// directory holds no scene or cannot be read. Reads through the VFS, so a
    /// browser build finds the scenes its bundle preloaded.
    ///
    /// A game that names no scene file loads whatever this returns, which is
    /// what lets a project exported from the playground drop onto it and run.
    pub fn first_scene_in(scenes_dir: &Path) -> Option<std::path::PathBuf> {
        common::vfs::list_dir_files(scenes_dir, "ron").into_iter().next()
    }

    /// Parse scene data from a RON string
    pub fn parse(content: &str) -> Result<SceneData, SceneLoadError> {
        ron::from_str(content).map_err(SceneLoadError::RonError)
    }

    /// Instantiate a scene in the given world
    ///
    /// This creates all entities defined in the scene, resolving prefabs
    /// and applying overrides as specified. Parent-child relationships are
    /// established based on the `parent` field or inline `children`.
    pub fn instantiate(
        data: &SceneData,
        world: &mut World,
        assets: &mut impl TextureResolver,
    ) -> Result<SceneInstance, SceneLoadError> {
        // Engine components (physics) must be in the dynamic registry before
        // any ComponentData::Dynamic loads — idempotent, covers headless
        // scene tests that never call run_game.
        crate::component_registration::register_engine_components();
        // Sidecar reads are cached for the duration of a load; start each one
        // from disk so an artist's `.sheet.ron` edit — including fixing a file
        // that was malformed a moment ago — takes effect on reload.
        assets.clear_sidecar_cache();
        let (stripped, retired_warnings) =
            crate::scene_retired_components::without_retired_components(data);
        let data: &SceneData = &stripped;

        let mut named_entities = HashMap::new();
        let mut entities = Vec::new();

        // First pass: create all entities and collect their IDs
        for entity_data in &data.entities {
            Self::create_entity_recursive(
                entity_data,
                &data.prefabs,
                world,
                assets,
                &mut named_entities,
                &mut entities,
                None, // No parent for top-level entities initially
            )?;
        }

        // Second pass: establish parent relationships for entities using `parent` field
        for entity_data in &data.entities {
            if let Some(parent_name) = &entity_data.parent {
                if let Some(entity_name) = &entity_data.name {
                    if let (Some(&entity_id), Some(&parent_id)) = (
                        named_entities.get(entity_name),
                        named_entities.get(parent_name),
                    ) {
                        if let Err(e) = world.set_parent(entity_id, parent_id) {
                            log::warn!(
                                "Scene load: failed to parent '{}' under '{}': {}",
                                entity_name, parent_name, e
                            );
                        }
                    } else {
                        log::warn!(
                            "Scene load: entity '{}' references parent '{}' but one of them was not found by name",
                            entity_name, parent_name
                        );
                    }
                }
            }
        }

        // Script Entity params were deferred until every entity existed —
        // resolve them against the finished name table.
        let mut load_warnings = retired_warnings;
        load_warnings.extend(crate::script_data::resolve_pending_script_targets(world, &named_entities));

        let entity_count = entities.len();

        Ok(SceneInstance {
            name: data.name.clone(),
            physics: data.physics.clone(),
            named_entities,
            entities,
            entity_count,
            prefabs: data.prefabs.clone(),
            load_warnings,
        })
    }

    /// Add a component during scene instantiation, logging failures (e.g.
    /// duplicate components in a malformed scene file) instead of silently
    /// dropping them and loading a half-formed entity.
    pub(crate) fn add_component_logged<T: ecs::Component>(
        world: &mut World,
        entity_id: EntityId,
        component: T,
    ) {
        if let Err(e) = world.add_component(&entity_id, component) {
            log::warn!(
                "Scene load: failed to add {} to entity {:?}: {}",
                std::any::type_name::<T>(),
                entity_id,
                e
            );
        }
    }

    /// Recursively create an entity and its inline children
    fn create_entity_recursive(
        entity_data: &EntityData,
        prefabs: &HashMap<String, PrefabData>,
        world: &mut World,
        assets: &mut impl TextureResolver,
        named_entities: &mut HashMap<String, EntityId>,
        entities: &mut Vec<EntityId>,
        parent_id: Option<EntityId>,
    ) -> Result<EntityId, SceneLoadError> {
        let entity_id = Self::create_entity(entity_data, prefabs, world, assets)?;
        entities.push(entity_id);

        if let Some(name) = &entity_data.name {
            named_entities.insert(name.clone(), entity_id);
            // Also attach a Name component so the name survives a save
            // round-trip (the serializer reads Name into EntityData.name)
            // and shows up in the editor hierarchy.
            Self::add_component_logged(world, entity_id, Name::new(name.clone()));
        }

        // Set up parent relationship if specified
        if let Some(parent) = parent_id {
            if let Err(e) = world.set_parent(entity_id, parent) {
                log::warn!(
                    "Scene load: failed to parent entity {:?} under {:?}: {}",
                    entity_id, parent, e
                );
            }
        }

        // Create inline children recursively
        for child_data in &entity_data.children {
            Self::create_entity_recursive(
                child_data,
                prefabs,
                world,
                assets,
                named_entities,
                entities,
                Some(entity_id),
            )?;
        }

        Ok(entity_id)
    }

    /// Load a scene from file and instantiate it
    pub fn load_and_instantiate(
        path: impl AsRef<Path>,
        world: &mut World,
        assets: &mut impl TextureResolver,
    ) -> Result<SceneInstance, SceneLoadError> {
        let data = Self::load_from_file(path)?;
        Self::instantiate(&data, world, assets)
    }

    /// Create a single entity from EntityData
    fn create_entity(
        entity_data: &EntityData,
        prefabs: &HashMap<String, PrefabData>,
        world: &mut World,
        assets: &mut impl TextureResolver,
    ) -> Result<EntityId, SceneLoadError> {
        let merged_components = Self::layered_components(prefabs, entity_data)?;
        let entity_id = world.create_entity();

        // Apply all components to the entity
        for component in &merged_components {
            Self::add_component_to_entity(entity_id, component, world, assets)?;
        }

        Ok(entity_id)
    }
}

// Parse-level tests (public API) live in `tests/scene_loader_parse.rs`;
// only tests needing private methods stay inline.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_scene_in_is_the_sorted_first_ron_whatever_the_dir_order() -> std::io::Result<()> {
        // Which scene a game opens must not depend on `read_dir`'s order.
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("zeta.ron"), "x")?;
        std::fs::write(dir.path().join("alpha.RON"), "x")?;
        std::fs::write(dir.path().join("aaa.txt"), "x")?;

        let first = SceneLoader::first_scene_in(dir.path()).expect("a scene exists");
        assert_eq!(first.file_name().and_then(|n| n.to_str()), Some("alpha.RON"));

        let empty = tempfile::tempdir()?;
        assert_eq!(SceneLoader::first_scene_in(empty.path()), None, "no .ron file, no scene");
        assert_eq!(
            SceneLoader::first_scene_in(&dir.path().join("missing")),
            None,
            "unreadable dir, no scene"
        );
        Ok(())
    }
}
