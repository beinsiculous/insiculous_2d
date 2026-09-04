//! Concrete component extraction table and dynamic component emission.
//!
//! Pairs with `scene_loader_components.rs`: each component variant has an arm in the
//! loader, an extractor function here, and a row in [`concrete_components`].

use ecs::sprite_components::{Camera, Sprite, SpriteAnimation, Transform2D};
use ecs::{EntityId, World};

use crate::scene_data::{ClipData, ComponentData};
#[cfg(feature = "physics")]
use crate::scene_data::{ColliderShapeData, RigidBodyTypeData};

pub(super) type Extractor = fn(&World, EntityId, &dyn Fn(u32) -> String) -> Option<ComponentData>;

/// One row per concrete `ComponentData` variant, in emission order.
/// `append_dynamic_components` skips every `registry_name` here, so a
/// component is never written twice. The wire name is not a field: the
/// extractor's output carries it (`SceneLoader::component_type_name`), and the
/// drift tests hold the two in step.
pub(super) struct ConcreteComponent {
    /// ECS registry name — what `ComponentRegistry::persistent_names` returns
    /// ("Camera", where the wire variant is `Camera2D`).
    pub registry_name: &'static str,
    pub extract: Extractor,
}

pub(super) fn concrete_components() -> Vec<ConcreteComponent> {
    let mut rows = vec![
        ConcreteComponent {
            registry_name: "Transform2D",
            extract: extract_transform_2d,
        },
        ConcreteComponent {
            registry_name: "Sprite",
            extract: extract_sprite,
        },
        ConcreteComponent {
            registry_name: "Camera",
            extract: extract_camera,
        },
        ConcreteComponent {
            registry_name: "Tilemap",
            extract: extract_tilemap,
        },
        ConcreteComponent {
            registry_name: "GridBackdrop",
            extract: extract_grid_backdrop,
        },
        ConcreteComponent {
            registry_name: "SpriteAnimation",
            extract: extract_sprite_animation,
        },
    ];

    #[cfg(feature = "physics")]
    rows.extend([
        ConcreteComponent {
            registry_name: "RigidBody",
            extract: extract_rigid_body,
        },
        ConcreteComponent {
            registry_name: "Collider",
            extract: extract_collider,
        },
    ]);

    rows.extend([
        ConcreteComponent {
            registry_name: "UiLabel",
            extract: extract_ui_label,
        },
        ConcreteComponent {
            registry_name: "UiPanel",
            extract: extract_ui_panel,
        },
        ConcreteComponent {
            registry_name: "UiButton",
            extract: extract_ui_button,
        },
        ConcreteComponent {
            registry_name: "Behavior",
            extract: extract_behavior,
        },
        ConcreteComponent {
            registry_name: "EntityTag",
            extract: extract_entity_tag,
        },
        ConcreteComponent {
            registry_name: "Scripts",
            extract: extract_scripts,
        },
    ]);

    rows
}

/// Registry names with no wire variant that are still never emitted as `Dynamic`:
/// Name lives on `EntityData.name`; GlobalTransform2D is computed.
const EXCLUDED_NON_WIRE: &[&str] = &["Name", "GlobalTransform2D"];

pub(super) fn extract_components(
    world: &World,
    entity: EntityId,
    texture_path_fn: &dyn Fn(u32) -> String,
) -> Vec<ComponentData> {
    let rows = concrete_components();
    let mut components: Vec<ComponentData> = rows
        .iter()
        .filter_map(|row| (row.extract)(world, entity, texture_path_fn))
        .collect();
    append_dynamic_components(world, entity, &rows, &mut components);
    components
}

fn extract_transform_2d(
    world: &World,
    entity: EntityId,
    _texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    world.get::<Transform2D>(entity).map(|transform| ComponentData::Transform2D {
        position: (transform.position.x, transform.position.y),
        rotation: transform.rotation,
        scale: (transform.scale.x, transform.scale.y),
    })
}

fn extract_sprite(
    world: &World,
    entity: EntityId,
    texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    world.get::<Sprite>(entity).map(|sprite| ComponentData::Sprite {
        texture: texture_path_fn(sprite.texture_handle),
        offset: (sprite.offset.x, sprite.offset.y),
        rotation: sprite.rotation,
        scale: (sprite.scale.x, sprite.scale.y),
        color: sprite.color.into(),
        depth: sprite.depth,
        emissive: sprite.emissive,
        tex_region: (
            sprite.tex_region[0],
            sprite.tex_region[1],
            sprite.tex_region[2],
            sprite.tex_region[3],
        ),
        visible: sprite.visible,
    })
}

fn extract_camera(
    world: &World,
    entity: EntityId,
    _texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    world.get::<Camera>(entity).map(|camera| ComponentData::Camera2D {
        position: (camera.position.x, camera.position.y),
        rotation: camera.rotation,
        zoom: camera.zoom,
        viewport_size: (camera.viewport_size.x, camera.viewport_size.y),
        is_main_camera: camera.is_main_camera,
    })
}

fn extract_tilemap(
    world: &World,
    entity: EntityId,
    texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    world.get::<ecs::Tilemap>(entity).map(|tilemap| ComponentData::Tilemap {
        tileset: texture_path_fn(tilemap.tileset),
        width: tilemap.width,
        height: tilemap.height,
        tile_size: tilemap.tile_size,
        tiles: tilemap.tiles.clone(),
        tile_uv_size: (tilemap.tile_uv_size.x, tilemap.tile_uv_size.y),
        depth: tilemap.depth,
    })
}

fn extract_grid_backdrop(
    world: &World,
    entity: EntityId,
    _texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    world.get::<ecs::GridBackdrop>(entity).map(|grid| ComponentData::GridBackdrop {
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
    })
}

fn extract_sprite_animation(
    world: &World,
    entity: EntityId,
    _texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    world.get::<SpriteAnimation>(entity).map(|animation| {
        ComponentData::SpriteAnimation {
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
        }
    })
}

#[cfg(feature = "physics")]
fn extract_rigid_body(
    world: &World,
    entity: EntityId,
    _texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    world.get::<physics::components::RigidBody>(entity).map(|rigid_body| {
        let body_type = match rigid_body.body_type {
            physics::components::RigidBodyType::Dynamic => RigidBodyTypeData::Dynamic,
            physics::components::RigidBodyType::Static => RigidBodyTypeData::Static,
            physics::components::RigidBodyType::Kinematic => RigidBodyTypeData::Kinematic,
        };
        ComponentData::RigidBody {
            body_type,
            velocity: (rigid_body.velocity.x, rigid_body.velocity.y),
            angular_velocity: rigid_body.angular_velocity,
            gravity_scale: rigid_body.gravity_scale,
            linear_damping: rigid_body.linear_damping,
            angular_damping: rigid_body.angular_damping,
            can_rotate: rigid_body.can_rotate,
            ccd_enabled: rigid_body.ccd_enabled,
        }
    })
}

#[cfg(feature = "physics")]
fn extract_collider(
    world: &World,
    entity: EntityId,
    _texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    world.get::<physics::components::Collider>(entity).map(|collider| {
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
        ComponentData::Collider {
            shape,
            offset: (collider.offset.x, collider.offset.y),
            is_sensor: collider.is_sensor,
            friction: collider.friction,
            restitution: collider.restitution,
        }
    })
}

fn extract_ui_label(
    world: &World,
    entity: EntityId,
    _texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    world.get::<ecs::UiLabel>(entity).map(|label| ComponentData::UiLabel {
        text: label.text.clone(),
        anchor: label.anchor,
        offset: (label.offset.x, label.offset.y),
        font_size: label.font_size,
        color: label.color.into(),
        visible: label.visible,
    })
}

fn extract_ui_panel(
    world: &World,
    entity: EntityId,
    _texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    world.get::<ecs::UiPanel>(entity).map(|panel| ComponentData::UiPanel {
        anchor: panel.anchor,
        offset: (panel.offset.x, panel.offset.y),
        size: (panel.size.x, panel.size.y),
        background: panel.background.into(),
        border: panel.border.into(),
        border_width: panel.border_width,
        visible: panel.visible,
    })
}

fn extract_ui_button(
    world: &World,
    entity: EntityId,
    _texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    world.get::<ecs::UiButton>(entity).map(|button| ComponentData::UiButton {
        text: button.text.clone(),
        id: button.id.clone(),
        anchor: button.anchor,
        offset: (button.offset.x, button.offset.y),
        size: (button.size.x, button.size.y),
        visible: button.visible,
    })
}

fn extract_behavior(
    world: &World,
    entity: EntityId,
    _texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    world
        .get::<ecs::behavior::Behavior>(entity)
        .map(|behavior| ComponentData::Behavior(behavior.clone()))
}

fn extract_entity_tag(
    world: &World,
    entity: EntityId,
    _texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    world
        .get::<ecs::behavior::EntityTag>(entity)
        .map(|tag| ComponentData::EntityTag { tag: tag.0.clone() })
}

fn extract_scripts(
    world: &World,
    entity: EntityId,
    _texture_path_fn: &dyn Fn(u32) -> String,
) -> Option<ComponentData> {
    // Scripts — Entity params persist by target Name; the
    // editor's save choke point auto-names referenced unnamed targets first.
    world.get::<ecs::Scripts>(entity).map(|scripts| {
        ComponentData::Scripts(crate::script_data::scripts_to_data(world, scripts))
    })
}

fn append_dynamic_components(
    world: &World,
    entity: EntityId,
    rows: &[ConcreteComponent],
    components: &mut Vec<ComponentData>,
) {
    ecs::with_global_registry(|registry| {
        for name in registry.persistent_names() {
            if rows.iter().any(|row| row.registry_name == name) || EXCLUDED_NON_WIRE.contains(&name) {
                continue;
            }
            match registry.extract_component(world, entity, name) {
                Ok(Some(data)) => components.push(ComponentData::Dynamic {
                    component_type: name.to_string(),
                    data,
                }),
                Ok(None) => {}
                Err(error) => {
                    log::error!("skipping dynamic component '{name}' on save: {error}");
                }
            }
        }
    });
}
