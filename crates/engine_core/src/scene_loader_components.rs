//! Scene-component construction: `ComponentData` -> ECS components.
//!
//! Split from `scene_loader.rs` (file size); this is the loader half of the
//! scene schema. The inverse (World -> `ComponentData`) is the extractor table
//! in `scene_serializer/components.rs` — a new component type needs an arm
//! here, an extractor and a table row there, and the drift test proves the row.

use glam::Vec2;

use ecs::sprite_components::{AnimationClip, Camera, Sprite, SpriteAnimation, Transform2D};
use ecs::{EntityId, World};

use crate::scene_data::{ClipData, ComponentData, GridData, SceneLoadError};
#[cfg(feature = "physics")]
use crate::scene_data::{ColliderShapeData, RigidBodyTypeData};
use crate::texture_ref::TextureResolver;

use crate::scene_loader::SceneLoader;

/// Near clipping plane distance covering the entire 2D depth range a scene may use.
const CAMERA_NEAR: f32 = -1000.0;

/// Far clipping plane distance covering the entire 2D depth range a scene may use.
const CAMERA_FAR: f32 = 1000.0;

impl SceneLoader {
    /// Get a simple type name for component matching.
    ///
    /// This is an exhaustive match over every [`ComponentData`] variant; when adding
    /// a new variant here, the serializer table in `scene_serializer/components.rs`
    /// is the next required edit.
    pub(crate) fn component_type_name(component: &ComponentData) -> &str {
        match component {
            ComponentData::Transform2D { .. } => "Transform2D",
            ComponentData::Sprite { .. } => "Sprite",
            ComponentData::Camera2D { .. } => "Camera2D",
            ComponentData::Tilemap { .. } => "Tilemap",
            ComponentData::GridBackdrop { .. } => "GridBackdrop",
            ComponentData::SpriteAnimation { .. } => "SpriteAnimation",
            ComponentData::RigidBody { .. } => "RigidBody",
            ComponentData::Collider { .. } => "Collider",
            ComponentData::UiLabel { .. } => "UiLabel",
            ComponentData::UiPanel { .. } => "UiPanel",
            ComponentData::UiButton { .. } => "UiButton",
            ComponentData::Behavior(_) => "Behavior",
            ComponentData::Scripts(_) => "Scripts",
            ComponentData::EntityTag { .. } => "EntityTag",
            ComponentData::Dynamic { component_type, .. } => component_type.as_str(),
        }
    }

    /// Add a component to an entity based on ComponentData
    pub(crate) fn add_component_to_entity(
        entity_id: EntityId,
        component: &ComponentData,
        world: &mut World,
        assets: &mut impl TextureResolver,
    ) -> Result<(), SceneLoadError> {
        match component {
            ComponentData::Transform2D {
                position,
                rotation,
                scale,
            } => {
                let transform = Transform2D {
                    position: Vec2::new(position.0, position.1),
                    rotation: *rotation,
                    scale: Vec2::new(scale.0, scale.1),
                };
                Self::add_component_logged(world, entity_id, transform);
            }

            ComponentData::Sprite {
                texture,
                offset,
                rotation,
                scale,
                color,
                depth,
                emissive,
                tex_region,
                visible,
            } => {
                let texture_handle = assets.resolve_texture(texture)?;
                let sprite = Sprite {
                    texture_handle: texture_handle.id,
                    offset: Vec2::new(offset.0, offset.1),
                    rotation: *rotation,
                    scale: Vec2::new(scale.0, scale.1),
                    color: (*color).into(),
                    depth: *depth,
                    visible: *visible,
                    emissive: *emissive,
                    tex_region: [tex_region.0, tex_region.1, tex_region.2, tex_region.3],
                };
                Self::add_component_logged(world, entity_id, sprite);
            }

            ComponentData::Camera2D {
                position,
                rotation,
                zoom,
                viewport_size,
                is_main_camera,
            } => {
                let camera = Camera {
                    position: Vec2::new(position.0, position.1),
                    rotation: *rotation,
                    zoom: *zoom,
                    viewport_size: Vec2::new(viewport_size.0, viewport_size.1),
                    is_main_camera: *is_main_camera,
                    near: CAMERA_NEAR,
                    far: CAMERA_FAR,
                };
                Self::add_component_logged(world, entity_id, camera);
            }

            ComponentData::Tilemap {
                tileset,
                width,
                height,
                tile_size,
                tiles,
                tile_uv_size,
                depth,
            } => {
                let texture_handle = assets.resolve_texture(tileset)?;
                let tilemap = ecs::Tilemap {
                    width: *width,
                    height: *height,
                    tile_size: *tile_size,
                    tileset: texture_handle.id,
                    tiles: tiles.clone(),
                    tile_uv_size: Vec2::new(tile_uv_size.0, tile_uv_size.1),
                    depth: *depth,
                };
                Self::add_component_logged(world, entity_id, tilemap);
            }

            ComponentData::GridBackdrop {
                topology,
                cols,
                rows,
                spacing,
                color,
                emissive,
                visible,
                stiffness,
                damping,
                rest_pull,
                rest_alpha_fraction,
                activity_attack,
                activity_release,
                activity_displacement_ref,
                activity_velocity_ref,
            } => {
                let backdrop = ecs::GridBackdrop {
                    topology: *topology,
                    cols: *cols,
                    rows: *rows,
                    spacing: *spacing,
                    color: (*color).into(),
                    emissive: *emissive,
                    visible: *visible,
                    stiffness: *stiffness,
                    damping: *damping,
                    rest_pull: *rest_pull,
                    rest_alpha_fraction: *rest_alpha_fraction,
                    activity_attack: *activity_attack,
                    activity_release: *activity_release,
                    activity_displacement_ref: *activity_displacement_ref,
                    activity_velocity_ref: *activity_velocity_ref,
                };
                Self::add_component_logged(world, entity_id, backdrop);
            }

            ComponentData::SpriteAnimation {
                sheet,
                grid,
                clips,
                autoplay,
            } => {
                let animation =
                    Self::build_sprite_animation(sheet.as_deref(), *grid, clips, autoplay.as_deref(), assets);
                warn_if_inert(&animation, entity_id);
                Self::add_component_logged(world, entity_id, animation);
            }

            #[cfg(feature = "physics")]
            ComponentData::RigidBody {
                body_type,
                velocity,
                angular_velocity,
                gravity_scale,
                linear_damping,
                angular_damping,
                can_rotate,
                ccd_enabled,
            } => {
                let mut rigid_body = rigid_body_of_type(*body_type);
                rigid_body.velocity = Vec2::new(velocity.0, velocity.1);
                rigid_body.angular_velocity = *angular_velocity;
                rigid_body.gravity_scale = *gravity_scale;
                rigid_body.linear_damping = *linear_damping;
                rigid_body.angular_damping = *angular_damping;
                rigid_body.can_rotate = *can_rotate;
                rigid_body.ccd_enabled = *ccd_enabled;

                Self::add_component_logged(world, entity_id, rigid_body);
            }

            #[cfg(not(feature = "physics"))]
            ComponentData::RigidBody { .. } => {
                log::warn!("RigidBody component in scene but physics feature is disabled");
            }

            #[cfg(feature = "physics")]
            ComponentData::Collider {
                shape,
                offset,
                is_sensor,
                friction,
                restitution,
            } => {
                use physics::components::Collider;

                let collider_shape = collider_shape_from_data(shape);
                let mut collider = Collider::new(collider_shape);
                collider.offset = Vec2::new(offset.0, offset.1);
                collider.is_sensor = *is_sensor;
                collider.friction = *friction;
                collider.restitution = *restitution;

                Self::add_component_logged(world, entity_id, collider);
            }

            #[cfg(not(feature = "physics"))]
            ComponentData::Collider { .. } => {
                log::warn!("Collider component in scene but physics feature is disabled");
            }

            ComponentData::UiLabel { text, anchor, offset, font_size, color, visible } => {
                let label = ecs::UiLabel {
                    text: text.clone(),
                    anchor: *anchor,
                    offset: Vec2::new(offset.0, offset.1),
                    font_size: *font_size,
                    color: (*color).into(),
                    visible: *visible,
                };
                Self::add_component_logged(world, entity_id, label);
            }

            ComponentData::UiPanel {
                anchor,
                offset,
                size,
                background,
                border,
                border_width,
                visible,
            } => {
                let panel = ecs::UiPanel {
                    anchor: *anchor,
                    offset: Vec2::new(offset.0, offset.1),
                    size: Vec2::new(size.0, size.1),
                    background: (*background).into(),
                    border: (*border).into(),
                    border_width: *border_width,
                    visible: *visible,
                };
                Self::add_component_logged(world, entity_id, panel);
            }

            ComponentData::UiButton { text, id, anchor, offset, size, visible } => {
                let button = ecs::UiButton {
                    text: text.clone(),
                    id: id.clone(),
                    anchor: *anchor,
                    offset: Vec2::new(offset.0, offset.1),
                    size: Vec2::new(size.0, size.1),
                    visible: *visible,
                };
                Self::add_component_logged(world, entity_id, button);
            }

            ComponentData::Behavior(behavior) => {
                Self::add_component_logged(world, entity_id, behavior.clone());
            }

            ComponentData::EntityTag { tag } => {
                Self::add_component_logged(world, entity_id, ecs::behavior::EntityTag::new(tag.clone()));
            }

            ComponentData::Scripts(refs) => {
                let scripts = build_scripts(refs, entity_id, world);
                Self::add_component_logged(world, entity_id, scripts);
            }

            ComponentData::Dynamic { component_type, data } => {
                insert_dynamic_component(world, entity_id, component_type, data)?;
            }
        }

        Ok(())
    }

    /// Build a [`SpriteAnimation`] from its scene data, preferring the sheet's
    /// `.sheet.ron` sidecar over the values baked into the scene.
    ///
    /// The sidecar is the source of truth: re-reading it on load means an
    /// artist re-cutting a sheet or renaming a clip propagates to every scene
    /// that references it, with no scene re-save. When the sidecar is missing
    /// or unusable the resolver has already warned, and the baked snapshot —
    /// which is right there and was correct when the scene was written —
    /// carries the load.
    pub(crate) fn build_sprite_animation(
        sheet: Option<&str>,
        grid: GridData,
        clips: &[(String, ClipData)],
        autoplay: Option<&str>,
        assets: &mut impl TextureResolver,
    ) -> SpriteAnimation {
        let sidecar = sheet.and_then(|path| assets.sheet_for(path));
        let (grid, clips) = match sidecar {
            Some(data) => (data.grid, data.clips),
            None => (
                grid.into(),
                clips
                    .iter()
                    .map(|(name, clip)| (name.clone(), AnimationClip::from(clip.clone())))
                    .collect(),
            ),
        };

        let mut animation = SpriteAnimation {
            grid,
            clips,
            sheet: sheet.map(str::to_string),
            ..SpriteAnimation::default()
        };

        if let Some(name) = autoplay {
            if animation.has_clip(name) {
                let _ = animation.play(name);
            } else {
                log::warn!(
                    "Scene load: autoplay clip '{}' does not exist for sheet {:?} \
                     (check its .sheet.ron sidecar); leaving the animation stopped",
                    name,
                    sheet.unwrap_or("<none>")
                );
            }
        }

        animation
    }
}

/// Convert scene collider shape data into an engine physics `ColliderShape`.
#[cfg(feature = "physics")]
fn collider_shape_from_data(shape: &ColliderShapeData) -> physics::components::ColliderShape {
    use physics::components::ColliderShape;

    match shape {
        ColliderShapeData::Box { half_extents } => ColliderShape::Box {
            half_extents: Vec2::new(half_extents.0, half_extents.1),
        },
        ColliderShapeData::Circle { radius } => ColliderShape::Circle { radius: *radius },
        ColliderShapeData::CapsuleY { half_height, radius } => ColliderShape::CapsuleY {
            half_height: *half_height,
            radius: *radius,
        },
        ColliderShapeData::CapsuleX { half_height, radius } => ColliderShape::CapsuleX {
            half_height: *half_height,
            radius: *radius,
        },
    }
}

/// Create a new physics `RigidBody` matching the serialized body type.
#[cfg(feature = "physics")]
fn rigid_body_of_type(body_type: RigidBodyTypeData) -> physics::components::RigidBody {
    use physics::components::RigidBody;

    match body_type {
        RigidBodyTypeData::Dynamic => RigidBody::new_dynamic(),
        RigidBodyTypeData::Static => RigidBody::new_static(),
        RigidBodyTypeData::Kinematic => RigidBody::new_kinematic(),
    }
}

/// Build an [`ecs::Scripts`] component from serialized script references.
///
/// Entity params defer to a post-instantiate pass because target entities may
/// not exist yet. The pending target list is stored in a scene-load-scoped
/// World resource that `resolve_pending_script_targets` drains.
fn build_scripts(
    refs: &[crate::script_data::ScriptRefData],
    entity_id: EntityId,
    world: &mut World,
) -> ecs::Scripts {
    let mut pending = world
        .remove_resource::<crate::script_data::PendingScriptTargets>()
        .unwrap_or_default();
    let scripts = ecs::Scripts(
        refs.iter()
            .enumerate()
            .map(|(index, data)| {
                crate::script_data::script_ref_from_data(data, entity_id, index, &mut pending)
            })
            .collect(),
    );
    world.insert_resource(pending);
    scripts
}

/// Deserialize and attach a dynamic component through the global ECS component registry.
///
/// An unregistered component name is a hard load error: fail loud rather than
/// silently dropping authored data. Game-specific components require the game's
/// own binary (which registers them at startup); the standalone editor refuses
/// such scenes instead of corrupting them on re-save.
fn insert_dynamic_component(
    world: &mut World,
    entity_id: EntityId,
    component_type: &str,
    data: &serde_json::Value,
) -> Result<(), SceneLoadError> {
    ecs::with_global_registry(|registry| {
        if !registry.is_registered(component_type) {
            return Err(SceneLoadError::ComponentError(format!(
                "Unknown dynamic component '{}' — not registered. Game components \
                 are only editable from the game's own binary (which registers \
                 them at startup).",
                component_type
            )));
        }
        registry
            .insert_component(world, entity_id, component_type, data.clone())
            .map_err(|error| {
                SceneLoadError::ComponentError(format!(
                    "Failed to load dynamic component '{}': {}",
                    component_type, error
                ))
            })
    })
}

/// Warn if a loaded [`SpriteAnimation`] has neither a sheet nor clips.
///
/// A component with no sheet and no clips can never animate. Old-format scene
/// data (the pre-named-clip schema) parses successfully into this inert shape
/// because fields default — warn instead of silently loading a no-op component.
fn warn_if_inert(animation: &SpriteAnimation, entity_id: EntityId) {
    if animation.sheet.is_none() && animation.clips.is_empty() {
        log::warn!(
            "Scene load: SpriteAnimation on entity {entity_id:?} has no sheet and no \
             clips — it will never animate. Old-format scene data parses to this \
             inert default; re-author the component with clips or a sheet reference."
        );
    }
}
