//! Addressing for components in commands: typed registry kinds or dynamic names.

use ecs::{EntityId, World};

use super::{ComponentKind, StoredComponent};

/// A component addressed either through the typed registry overlay or by dynamic-tier name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentRef {
    Typed(ComponentKind),
    Dynamic(String),
}

impl ComponentRef {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Typed(kind) => kind.display_name(),
            Self::Dynamic(name) => name.as_str(),
        }
    }

    pub(crate) fn add_default(&self, world: &mut World, entity: EntityId) {
        match self {
            Self::Typed(kind) => kind.add_default(world, entity),
            Self::Dynamic(name) => {
                if let Err(e) = super::dynamic::add_dynamic_default(world, entity, name) {
                    log::error!("add dynamic component '{name}' failed: {e}");
                }
            }
        }
    }

    pub(crate) fn capture(&self, world: &World, entity: EntityId) -> Option<StoredComponent> {
        match self {
            Self::Typed(kind) => kind.capture(world, entity),
            Self::Dynamic(name) => {
                super::dynamic::capture_dynamic_by_name(world, entity, name).ok().flatten()
            }
        }
    }

    pub(crate) fn remove(&self, world: &mut World, entity: EntityId) {
        match self {
            Self::Typed(kind) => kind.remove(world, entity),
            Self::Dynamic(name) => {
                super::dynamic::remove_dynamic(world, entity, name);
            }
        }
    }

    /// Removing a RigidBody takes its Collider with it (a collider without a body is
    /// meaningless to the physics system). Nothing else cascades; dynamic never does.
    pub(crate) fn cascade(&self) -> Option<ComponentRef> {
        matches!(self, Self::Typed(ComponentKind::RigidBody))
            .then_some(Self::Typed(ComponentKind::Collider))
    }
}
