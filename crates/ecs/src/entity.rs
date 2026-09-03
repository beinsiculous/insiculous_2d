//! Entity management for the ECS.

use crate::generation::{EntityGeneration, EntityIdGenerator, GenerationError};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// A unique identifier for an entity with generation tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId {
    /// The numeric ID of the entity
    id: u64,
    /// The generation for this entity ID
    generation: u64,
}

impl EntityId {
    /// Create a new entity ID with generation 1
    pub fn new() -> Self {
        static GENERATOR: LazyLock<EntityIdGenerator> = LazyLock::new(EntityIdGenerator::new);
        Self {
            id: GENERATOR.generate_id(),
            generation: 1,
        }
    }

    /// Create a new entity ID with a specific generation
    pub fn with_generation(id: u64, generation: u64) -> Self {
        Self { id, generation }
    }

    /// Get the numeric ID value
    pub fn value(&self) -> u64 {
        self.id
    }

    /// Get the generation
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Validate this entity ID against a generation
    pub fn validate(&self, generation: &EntityGeneration) -> Result<(), GenerationError> {
        if !generation.is_alive() {
            return Err(GenerationError::EntityNotAlive(self.id));
        }
        
        if !generation.is_valid(self.generation) {
            return Err(GenerationError::InvalidGeneration(self.id, self.generation, generation.generation()));
        }
        
        Ok(())
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Entity({}, gen: {})", self.id, self.generation)
    }
}

/// An entity in the ECS with generation tracking
#[derive(Debug)]
pub struct Entity {
    /// The unique identifier for this entity
    id: EntityId,
    /// Whether this entity is active
    active: bool,
}

impl Entity {
    /// Create a new entity
    pub fn new() -> Self {
        Self {
            id: EntityId::new(),
            active: true,
        }
    }

    /// Create a new entity with a specific ID (for entity reuse)
    pub fn with_id(id: EntityId) -> Self {
        Self {
            id,
            active: true,
        }
    }

    /// Get the entity's ID
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// Check if the entity is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Set whether the entity is active
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Deactivate the entity (mark as not alive)
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::new()
    }
}
