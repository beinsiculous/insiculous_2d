//! Simple entity-component-system for the insiculous_2d game engine.
//!
//! This crate provides a minimal ECS implementation with HashMap-based
//! per-type component storage.
//!
//! # Module Visibility Strategy
//!
//! This crate uses two visibility patterns intentionally:
//!
//! - **Private modules** (`mod` + `pub use *`): Core infrastructure types like
//!   [`EntityId`], [`Component`], [`World`], and query types. These are re-exported
//!   at the crate root for convenient access while keeping implementation details hidden.
//!
//! - **Public modules** (`pub mod` + `pub use *`): Domain-specific modules like
//!   [`behavior`], [`hierarchy`], [`sprite_components`], etc. These are publicly
//!   visible for documentation discoverability while also re-exported at the crate root.
//!
//! All public types are accessible from the crate root: `use ecs::EntityId;`

// Core infrastructure - private modules, re-exported at crate root
mod component;
mod entity;
mod entity_builder;
mod query;
mod world;

// Domain modules - public for documentation, also re-exported at crate root
pub mod audio_components;
pub mod behavior;
pub mod lifetime;
pub mod component_registry;
pub mod event;
pub mod generation;
pub mod grid_backdrop;
pub mod hierarchy;
pub mod hierarchy_extension;
pub mod hierarchy_system;
pub mod resource;
pub mod script;
pub mod sprite_components;
pub mod sprite_system;
pub mod state_machine;
pub mod system;
pub mod tilemap;
pub mod ui_components;

pub mod prelude;

// Re-export all public items at crate root for convenient access
pub use query::*;
pub use audio_components::*;
pub use behavior::*;
pub use lifetime::{Lifetime, LifetimeSystem};
pub use component::*;
// NOTE: `component_registry::ComponentRegistry` (the dynamic name-keyed
// tier) is deliberately NOT re-exported at the crate root — the glob above
// already exports `component::ComponentRegistry` (the per-type storage map).
// Reach the dynamic registry via `ecs::component_registry::ComponentRegistry`.
pub use component_registry::{register_components, with_global_registry, ComponentMeta};
pub use ecs_macros::ComponentMeta as DeriveComponentMeta;
pub use entity::*;
pub use entity_builder::*;
pub use event::EventBus;
pub use generation::*;
pub use grid_backdrop::{GridBackdrop, GridTopology};
pub use hierarchy::*;
pub use hierarchy_extension::*;
pub use hierarchy_system::*;
pub use resource::ResourceStorage;
pub use script::{ScriptRef, ScriptValue, Scripts};
pub use sprite_components::*;
pub use sprite_system::*;
pub use state_machine::{HierarchicalStateMachine, StateMachine};
pub use system::*;
pub use tilemap::{TileInstance, Tilemap};
pub use ui_components::{resolve_anchored_pos, UiAnchor, UiButton, UiLabel, UiPanel};
pub use world::*;

/// Initialize the ECS
pub fn init() -> Result<World, EcsError> {
    log::info!("ECS initialized");
    Ok(World::new())
}

/// Errors that can occur in the ECS
#[derive(Debug, thiserror::Error)]
pub enum EcsError {
    #[error("Entity not found: {0}")]
    EntityNotFound(EntityId),

    #[error("Component not found for entity {0}")]
    ComponentNotFound(EntityId),

    #[error("System error: {0}")]
    SystemError(String),

    #[error("Entity generation error: {0}")]
    GenerationError(#[from] GenerationError),

    #[error("World not initialized")]
    NotInitialized,

    #[error("World already initialized")]
    AlreadyInitialized,

    #[error("World not running")]
    NotRunning,

    #[error("World already running")]
    AlreadyRunning,
}

impl From<String> for EcsError {
    fn from(error: String) -> Self {
        EcsError::SystemError(error)
    }
}