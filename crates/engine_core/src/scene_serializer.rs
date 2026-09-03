//! Scene serializer — converts a live World into SceneData for RON serialization.
//!
//! This is the inverse of `scene_loader.rs`. It walks the ECS world, extracts
//! known component types from each entity, and produces a `SceneData` structure
//! that can be written to a `.scene.ron` file.

use std::path::Path;

use ecs::sprite_components::{Camera, Name, Sprite, SpriteAnimation, Transform2D};
use ecs::{EntityId, World, WorldHierarchyExt};

use crate::scene_data::*;

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
    let roots = world.get_root_entities();

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

/// Extract all serializable components from an entity as ComponentData variants.
///
/// Skips computed/internal components (GlobalTransform2D, Parent, Children).
/// The `Name` component is handled separately as `EntityData.name`. After
/// the concrete variants, every OTHER registry-persisted type present on the
/// entity (audio components, game-registered types) is emitted as
/// `ComponentData::Dynamic` — name-sorted for stable scene diffs (this
/// ended the AudioSource/AudioListener silent drop).
fn extract_components(
    world: &World,
    entity: EntityId,
    texture_path_fn: &dyn Fn(u32) -> String,
) -> Vec<ComponentData> {
    let mut components = Vec::new();

    // Transform2D
    if let Some(transform) = world.get::<Transform2D>(entity) {
        components.push(ComponentData::Transform2D {
            position: (transform.position.x, transform.position.y),
            rotation: transform.rotation,
            scale: (transform.scale.x, transform.scale.y),
        });
    }

    // Sprite
    if let Some(sprite) = world.get::<Sprite>(entity) {
        components.push(ComponentData::Sprite {
            texture: texture_path_fn(sprite.texture_handle),
            offset: (sprite.offset.x, sprite.offset.y),
            rotation: sprite.rotation,
            scale: (sprite.scale.x, sprite.scale.y),
            color: (sprite.color.x, sprite.color.y, sprite.color.z, sprite.color.w),
            depth: sprite.depth,
            emissive: sprite.emissive,
            tex_region: (sprite.tex_region[0], sprite.tex_region[1], sprite.tex_region[2], sprite.tex_region[3]),
            visible: sprite.visible,
        });
    }

    // Camera
    if let Some(camera) = world.get::<Camera>(entity) {
        components.push(ComponentData::Camera2D {
            position: (camera.position.x, camera.position.y),
            rotation: camera.rotation,
            zoom: camera.zoom,
            viewport_size: (camera.viewport_size.x, camera.viewport_size.y),
            is_main_camera: camera.is_main_camera,
        });
    }

    // Tilemap
    if let Some(tilemap) = world.get::<ecs::Tilemap>(entity) {
        components.push(ComponentData::Tilemap {
            tileset: texture_path_fn(tilemap.tileset),
            width: tilemap.width,
            height: tilemap.height,
            tile_size: tilemap.tile_size,
            tiles: tilemap.tiles.clone(),
            tile_uv_size: (tilemap.tile_uv_size.x, tilemap.tile_uv_size.y),
            depth: tilemap.depth,
        });
    }

    // GridBackdrop
    if let Some(grid) = world.get::<ecs::GridBackdrop>(entity) {
        components.push(ComponentData::GridBackdrop {
            topology: grid.topology,
            cols: grid.cols,
            rows: grid.rows,
            spacing: grid.spacing,
            color: grid.color.into(),
            emissive: grid.emissive,
            visible: grid.visible,
            stiffness: grid.stiffness,
            damping: grid.damping,
            rest_pull: grid.rest_pull,
            rest_alpha_fraction: grid.rest_alpha_fraction,
            activity_attack: grid.activity_attack,
            activity_release: grid.activity_release,
            activity_displacement_ref: grid.activity_displacement_ref,
            activity_velocity_ref: grid.activity_velocity_ref,
        });
    }

    // SpriteAnimation
    if let Some(animation) = world.get::<SpriteAnimation>(entity) {
        components.push(ComponentData::SpriteAnimation {
            sheet: animation.sheet.clone(),
            grid: animation.grid.into(),
            clips: animation
                .clips
                .iter()
                .map(|(name, clip)| (name.clone(), ClipData::from(clip.clone())))
                .collect(),
            // Only a running animation names an autoplay clip: a paused one
            // must not come back playing. Frame position is runtime state and
            // is never written, so playback always restarts from the top.
            autoplay: animation.playing.then(|| animation.current_clip.clone()).flatten(),
        });
    }

    // RigidBody (behind physics feature)
    #[cfg(feature = "physics")]
    if let Some(rigid_body) = world.get::<physics::components::RigidBody>(entity) {
        let body_type = match rigid_body.body_type {
            physics::components::RigidBodyType::Dynamic => RigidBodyTypeData::Dynamic,
            physics::components::RigidBodyType::Static => RigidBodyTypeData::Static,
            physics::components::RigidBodyType::Kinematic => RigidBodyTypeData::Kinematic,
        };
        components.push(ComponentData::RigidBody {
            body_type,
            velocity: (rigid_body.velocity.x, rigid_body.velocity.y),
            angular_velocity: rigid_body.angular_velocity,
            gravity_scale: rigid_body.gravity_scale,
            linear_damping: rigid_body.linear_damping,
            angular_damping: rigid_body.angular_damping,
            can_rotate: rigid_body.can_rotate,
            ccd_enabled: rigid_body.ccd_enabled,
        });
    }

    // Collider (behind physics feature)
    #[cfg(feature = "physics")]
    if let Some(collider) = world.get::<physics::components::Collider>(entity) {
        let shape = match &collider.shape {
            physics::components::ColliderShape::Box { half_extents } => {
                ColliderShapeData::Box {
                    half_extents: (half_extents.x, half_extents.y),
                }
            }
            physics::components::ColliderShape::Circle { radius } => {
                ColliderShapeData::Circle { radius: *radius }
            }
            physics::components::ColliderShape::CapsuleY { half_height, radius } => {
                ColliderShapeData::CapsuleY {
                    half_height: *half_height,
                    radius: *radius,
                }
            }
            physics::components::ColliderShape::CapsuleX { half_height, radius } => {
                ColliderShapeData::CapsuleX {
                    half_height: *half_height,
                    radius: *radius,
                }
            }
        };
        components.push(ComponentData::Collider {
            shape,
            offset: (collider.offset.x, collider.offset.y),
            is_sensor: collider.is_sensor,
            friction: collider.friction,
            restitution: collider.restitution,
        });
    }

    // UI elements (screen-space, anchor-placed)
    if let Some(label) = world.get::<ecs::UiLabel>(entity) {
        components.push(ComponentData::UiLabel {
            text: label.text.clone(),
            anchor: label.anchor,
            offset: (label.offset.x, label.offset.y),
            font_size: label.font_size,
            color: (label.color.x, label.color.y, label.color.z, label.color.w),
            visible: label.visible,
        });
    }
    if let Some(panel) = world.get::<ecs::UiPanel>(entity) {
        components.push(ComponentData::UiPanel {
            anchor: panel.anchor,
            offset: (panel.offset.x, panel.offset.y),
            size: (panel.size.x, panel.size.y),
            background: (panel.background.x, panel.background.y, panel.background.z, panel.background.w),
            border: (panel.border.x, panel.border.y, panel.border.z, panel.border.w),
            border_width: panel.border_width,
            visible: panel.visible,
        });
    }
    if let Some(button) = world.get::<ecs::UiButton>(entity) {
        components.push(ComponentData::UiButton {
            text: button.text.clone(),
            id: button.id.clone(),
            anchor: button.anchor,
            offset: (button.offset.x, button.offset.y),
            size: (button.size.x, button.size.y),
            visible: button.visible,
        });
    }

    // Behavior — conversion lives on `From<&Behavior> for BehaviorData` in scene_data.rs
    if let Some(behavior) = world.get::<ecs::behavior::Behavior>(entity) {
        components.push(ComponentData::Behavior(BehaviorData::from(behavior)));
    }

    // EntityTag
    if let Some(tag) = world.get::<ecs::behavior::EntityTag>(entity) {
        components.push(ComponentData::EntityTag { tag: tag.0.clone() });
    }

    // Scripts — Entity params persist by target Name; the
    // editor's save choke point auto-names referenced unnamed targets first.
    if let Some(scripts) = world.get::<ecs::Scripts>(entity) {
        components.push(ComponentData::Scripts(crate::script_data::scripts_to_data(
            world, scripts,
        )));
    }

    append_dynamic_components(world, entity, &mut components);

    components
}

/// Emit every registry-persisted component on `entity` that has no concrete
/// `ComponentData` variant, as `Dynamic { type, data }` — sorted by name so
/// repeated saves diff cleanly. Serialization failures skip the component
/// with a loud log (should not happen for serde-derived types).
fn append_dynamic_components(
    world: &World,
    entity: EntityId,
    components: &mut Vec<ComponentData>,
) {
    // Names already covered by concrete variants (or deliberately excluded:
    // Name → EntityData.name; GlobalTransform2D/hierarchy are computed).
    const CONCRETE_OR_EXCLUDED: &[&str] = &[
        "Transform2D",
        "Sprite",
        "Camera",
        "SpriteAnimation",
        "Tilemap",
        "GridBackdrop",
        "RigidBody",
        "Collider",
        "UiLabel",
        "UiPanel",
        "UiButton",
        "Behavior",
        "Scripts",
        "EntityTag",
        "Name",
        "GlobalTransform2D",
    ];
    ecs::with_global_registry(|registry| {
        for name in registry.persistent_names() {
            if CONCRETE_OR_EXCLUDED.contains(&name) {
                continue;
            }
            match registry.extract_component(world, entity, name) {
                Ok(Some(data)) => components.push(ComponentData::Dynamic {
                    component_type: name.to_string(),
                    data,
                }),
                Ok(None) => {}
                Err(e) => {
                    log::error!("skipping dynamic component '{name}' on save: {e}");
                }
            }
        }
    });
}

/// Serialize SceneData to a pretty-printed RON string.
pub fn serialize_to_ron(scene: &SceneData) -> Result<String, String> {
    ron::ser::to_string_pretty(scene, ron::ser::PrettyConfig::default())
        .map_err(|e| format!("RON serialization error: {}", e))
}

/// Write SceneData to a file as RON.
pub fn save_scene_to_file(scene: &SceneData, path: &Path) -> Result<(), String> {
    let ron_string = serialize_to_ron(scene)?;
    std::fs::write(path, ron_string).map_err(|e| format!("Failed to write scene file: {}", e))
}

#[cfg(test)]
mod dynamic_and_scripts_tests;
#[cfg(test)]
mod tests;
