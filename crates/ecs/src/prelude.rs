//! Prelude module for the ecs crate.
//!
//! This module re-exports the most commonly used items from the crate
//! for ergonomic imports.

pub use crate::{
    component::Component,
    entity::{Entity, EntityId},
    entity_builder::EntityBuilder,
    event::EventBus,
    hierarchy::{Children, GlobalTransform2D, Parent},
    hierarchy_extension::WorldHierarchyExt,
    hierarchy_system::{request_transform_hierarchy_reset, TransformHierarchyReset, TransformHierarchySystem},
    resource::ResourceStorage,
    state_machine::StateMachine,
    system::System,
    world::World,
    EcsError,
};
