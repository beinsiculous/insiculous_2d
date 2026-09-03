//! Entity generation tracking for detecting stale entity references.

use std::sync::atomic::{AtomicU64, Ordering};

/// Represents the generation of an entity for detecting stale references
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityGeneration {
    /// The generation number
    generation: u64,
    /// Whether this entity is currently alive
    is_alive: bool,
}

impl EntityGeneration {
    /// Create a new entity generation
    pub fn new() -> Self {
        Self {
            generation: 1,
            is_alive: true,
        }
    }

    /// Create a new generation with a specific number
    pub fn with_generation(generation: u64) -> Self {
        Self {
            generation,
            is_alive: true,
        }
    }

    /// Get the generation number
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Check if this entity is alive
    pub fn is_alive(&self) -> bool {
        self.is_alive
    }

    /// Mark this entity as dead
    pub fn mark_dead(&mut self) {
        self.is_alive = false;
    }

    /// Increment the generation (for entity reuse)
    pub fn increment(&mut self) {
        self.generation += 1;
        self.is_alive = true;
    }

    /// Check if this generation is valid (not stale)
    pub fn is_valid(&self, current_generation: u64) -> bool {
        self.is_alive && self.generation == current_generation
    }
}

impl Default for EntityGeneration {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages entity ID generation to prevent ID reuse conflicts
#[derive(Debug)]
pub struct EntityIdGenerator {
    /// Counter for generating entity IDs
    counter: AtomicU64,
}

impl EntityIdGenerator {
    /// Create a new entity ID generator
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(1),
        }
    }

    /// Generate a new entity ID
    pub fn generate_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::Relaxed)
    }
}

impl Default for EntityIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Error types for entity generation validation
#[derive(Debug, thiserror::Error)]
pub enum GenerationError {
    #[error("Entity {0} is stale (reference generation {1}, current generation {2})")]
    StaleEntity(u64, u64, u64),
    
    #[error("Entity {0} is not alive")]
    EntityNotAlive(u64),
    
    #[error("Entity {0} has invalid generation (expected {1}, got {2})")]
    InvalidGeneration(u64, u64, u64),
}