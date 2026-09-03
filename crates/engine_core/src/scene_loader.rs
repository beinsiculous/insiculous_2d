//! Scene loader for RON scene files
//!
//! This module provides functionality to load scenes from RON files,
//! resolve prefabs, and instantiate entities in the ECS world.

use std::collections::HashMap;
use std::path::Path;

use ecs::sprite_components::{Name, Transform2D};
use ecs::{EntityId, World, WorldHierarchyExt};

use crate::texture_ref::TextureResolver;
use crate::scene_data::{
    ComponentData, EntityData, PhysicsSettings, PrefabData, SceneData, SceneLoadError,
};

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
    /// `overrides` replace matching component types from the prefab — the
    /// same semantics as scene-file `overrides`. Returns the new entity, or
    /// an error if the prefab doesn't exist or a component fails to build
    /// (in which case no half-built entity is left behind). The spawned
    /// entity is NOT added to `entities`/`named_entities` — the caller owns
    /// its lifecycle.
    pub fn spawn_prefab(
        &self,
        world: &mut World,
        assets: &mut impl TextureResolver,
        prefab_name: &str,
        overrides: &[ComponentData],
    ) -> Result<EntityId, SceneLoadError> {
        let prefab = self
            .prefabs
            .get(prefab_name)
            .ok_or_else(|| SceneLoadError::PrefabNotFound(prefab_name.to_string()))?;

        let entity_id = world.create_entity();
        let merged = SceneLoader::merge_components(&prefab.components, overrides, &[]);
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

                        // Add GlobalTransform2D component if the entity has Transform2D
                        if world.get::<Transform2D>(entity_id).is_some() {
                            Self::add_component_logged(
                                world,
                                entity_id,
                                ecs::hierarchy::GlobalTransform2D::default(),
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
        let load_warnings =
            crate::script_data::resolve_pending_script_targets(world, &named_entities);

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

            // Add GlobalTransform2D component for hierarchical entities
            if world.get::<Transform2D>(entity_id).is_some() {
                Self::add_component_logged(world, entity_id, ecs::hierarchy::GlobalTransform2D::default());
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
        let entity_id = world.create_entity();

        // Get base components from prefab (if any)
        let base_components = if let Some(prefab_name) = &entity_data.prefab {
            let prefab = prefabs
                .get(prefab_name)
                .ok_or_else(|| SceneLoadError::PrefabNotFound(prefab_name.clone()))?;
            prefab.components.clone()
        } else {
            Vec::new()
        };

        // Merge base components with overrides and inline components
        let merged_components =
            Self::merge_components(&base_components, &entity_data.overrides, &entity_data.components);

        // Apply all components to the entity
        for component in &merged_components {
            Self::add_component_to_entity(entity_id, component, world, assets)?;
        }

        Ok(entity_id)
    }

    /// Merge prefab components with overrides and inline components
    fn merge_components(
        base: &[ComponentData],
        overrides: &[ComponentData],
        inline: &[ComponentData],
    ) -> Vec<ComponentData> {
        let mut result: Vec<ComponentData> = base.to_vec();
        // Later layers win: overrides replace base, inline replaces both.
        Self::apply_component_layer(&mut result, overrides);
        Self::apply_component_layer(&mut result, inline);
        result
    }

    /// Replace-or-append each component of `layer` into `result`, matching
    /// by component type name.
    fn apply_component_layer(result: &mut Vec<ComponentData>, layer: &[ComponentData]) {
        for comp in layer {
            let component_type = Self::component_type_name(comp);
            if let Some(pos) = result
                .iter()
                .position(|c| Self::component_type_name(c) == component_type)
            {
                result[pos] = comp.clone();
            } else {
                result.push(comp.clone());
            }
        }
    }

}

// Parse-level tests (public API) live in `tests/scene_loader_parse.rs`;
// only tests needing private methods stay inline.
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
    fn later_layers_replace_earlier_ones_by_component_type_and_append_the_rest() {
        // prefab base < entity overrides < inline components — the same
        // type is replaced in place, a type only one layer carries is kept.
        let base = vec![transform_at(0.0), ComponentData::EntityTag { tag: "ball".to_string() }];
        let overrides = vec![transform_at(100.0)];
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

        let base_and_overrides = SceneLoader::merge_components(&base, &overrides, &[]);
        let all_three = SceneLoader::merge_components(&base, &overrides, &inline);

        assert_eq!(base_and_overrides.len(), 2, "override replaces, tag kept: {base_and_overrides:?}");
        assert_eq!(position_of(&base_and_overrides[0]), (100.0, 0.0));
        assert_eq!(all_three.len(), 3, "inline replaces the transform and appends the sprite: {all_three:?}");
        assert_eq!(position_of(&all_three[0]), (500.0, 0.0));
        assert!(matches!(&all_three[1], ComponentData::EntityTag { tag } if tag == "ball"));
        assert!(matches!(&all_three[2], ComponentData::Sprite { .. }));
    }
}
