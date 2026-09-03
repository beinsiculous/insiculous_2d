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
/// NOTE (#44): `Scripts` Entity params persist by target NAME — a caller
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
    let name = world.get::<Name>(entity).map(|n| n.as_str().to_string());
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
/// `ComponentData::Dynamic` — name-sorted for stable scene diffs (issue #43;
/// this ended the AudioSource/AudioListener silent drop).
fn extract_components(
    world: &World,
    entity: EntityId,
    texture_path_fn: &dyn Fn(u32) -> String,
) -> Vec<ComponentData> {
    let mut components = Vec::new();

    // Transform2D
    if let Some(t) = world.get::<Transform2D>(entity) {
        components.push(ComponentData::Transform2D {
            position: (t.position.x, t.position.y),
            rotation: t.rotation,
            scale: (t.scale.x, t.scale.y),
        });
    }

    // Sprite
    if let Some(s) = world.get::<Sprite>(entity) {
        components.push(ComponentData::Sprite {
            texture: texture_path_fn(s.texture_handle),
            offset: (s.offset.x, s.offset.y),
            rotation: s.rotation,
            scale: (s.scale.x, s.scale.y),
            color: (s.color.x, s.color.y, s.color.z, s.color.w),
            depth: s.depth,
            emissive: s.emissive,
            tex_region: (s.tex_region[0], s.tex_region[1], s.tex_region[2], s.tex_region[3]),
            visible: s.visible,
        });
    }

    // Camera
    if let Some(c) = world.get::<Camera>(entity) {
        components.push(ComponentData::Camera2D {
            position: (c.position.x, c.position.y),
            rotation: c.rotation,
            zoom: c.zoom,
            viewport_size: (c.viewport_size.x, c.viewport_size.y),
            is_main_camera: c.is_main_camera,
        });
    }

    // Tilemap
    if let Some(tm) = world.get::<ecs::Tilemap>(entity) {
        components.push(ComponentData::Tilemap {
            tileset: texture_path_fn(tm.tileset),
            width: tm.width,
            height: tm.height,
            tile_size: tm.tile_size,
            tiles: tm.tiles.clone(),
            tile_uv_size: (tm.tile_uv_size.x, tm.tile_uv_size.y),
            depth: tm.depth,
        });
    }

    // GridBackdrop (#46)
    if let Some(g) = world.get::<ecs::GridBackdrop>(entity) {
        components.push(ComponentData::GridBackdrop {
            topology: g.topology,
            cols: g.cols,
            rows: g.rows,
            spacing: g.spacing,
            color: g.color.into(),
            emissive: g.emissive,
            visible: g.visible,
            stiffness: g.stiffness,
            damping: g.damping,
            rest_pull: g.rest_pull,
            rest_alpha_fraction: g.rest_alpha_fraction,
            activity_attack: g.activity_attack,
            activity_release: g.activity_release,
            activity_displacement_ref: g.activity_displacement_ref,
            activity_velocity_ref: g.activity_velocity_ref,
        });
    }

    // SpriteAnimation
    if let Some(a) = world.get::<SpriteAnimation>(entity) {
        components.push(ComponentData::SpriteAnimation {
            sheet: a.sheet.clone(),
            grid: a.grid.into(),
            clips: a
                .clips
                .iter()
                .map(|(name, clip)| (name.clone(), ClipData::from(clip.clone())))
                .collect(),
            // Only a running animation names an autoplay clip: a paused one
            // must not come back playing. Frame position is runtime state and
            // is never written, so playback always restarts from the top.
            autoplay: a.playing.then(|| a.current_clip.clone()).flatten(),
        });
    }

    // RigidBody (behind physics feature)
    #[cfg(feature = "physics")]
    if let Some(rb) = world.get::<physics::components::RigidBody>(entity) {
        let body_type = match rb.body_type {
            physics::components::RigidBodyType::Dynamic => RigidBodyTypeData::Dynamic,
            physics::components::RigidBodyType::Static => RigidBodyTypeData::Static,
            physics::components::RigidBodyType::Kinematic => RigidBodyTypeData::Kinematic,
        };
        components.push(ComponentData::RigidBody {
            body_type,
            velocity: (rb.velocity.x, rb.velocity.y),
            angular_velocity: rb.angular_velocity,
            gravity_scale: rb.gravity_scale,
            linear_damping: rb.linear_damping,
            angular_damping: rb.angular_damping,
            can_rotate: rb.can_rotate,
            ccd_enabled: rb.ccd_enabled,
        });
    }

    // Collider (behind physics feature)
    #[cfg(feature = "physics")]
    if let Some(col) = world.get::<physics::components::Collider>(entity) {
        let shape = match &col.shape {
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
            offset: (col.offset.x, col.offset.y),
            is_sensor: col.is_sensor,
            friction: col.friction,
            restitution: col.restitution,
        });
    }

    // UI elements (screen-space, anchor-placed)
    if let Some(l) = world.get::<ecs::UiLabel>(entity) {
        components.push(ComponentData::UiLabel {
            text: l.text.clone(),
            anchor: l.anchor,
            offset: (l.offset.x, l.offset.y),
            font_size: l.font_size,
            color: (l.color.x, l.color.y, l.color.z, l.color.w),
            visible: l.visible,
        });
    }
    if let Some(p) = world.get::<ecs::UiPanel>(entity) {
        components.push(ComponentData::UiPanel {
            anchor: p.anchor,
            offset: (p.offset.x, p.offset.y),
            size: (p.size.x, p.size.y),
            background: (p.background.x, p.background.y, p.background.z, p.background.w),
            border: (p.border.x, p.border.y, p.border.z, p.border.w),
            border_width: p.border_width,
            visible: p.visible,
        });
    }
    if let Some(b) = world.get::<ecs::UiButton>(entity) {
        components.push(ComponentData::UiButton {
            text: b.text.clone(),
            id: b.id.clone(),
            anchor: b.anchor,
            offset: (b.offset.x, b.offset.y),
            size: (b.size.x, b.size.y),
            visible: b.visible,
        });
    }

    // Behavior — conversion lives on `From<&Behavior> for BehaviorData` in scene_data.rs
    if let Some(b) = world.get::<ecs::behavior::Behavior>(entity) {
        components.push(ComponentData::Behavior(BehaviorData::from(b)));
    }

    // EntityTag
    if let Some(t) = world.get::<ecs::behavior::EntityTag>(entity) {
        components.push(ComponentData::EntityTag { tag: t.0.clone() });
    }

    // Scripts — Entity params persist by target Name (issue #44); the
    // editor's save choke point auto-names referenced unnamed targets first.
    if let Some(s) = world.get::<ecs::Scripts>(entity) {
        components.push(ComponentData::Scripts(crate::script_data::scripts_to_data(
            world, s,
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
