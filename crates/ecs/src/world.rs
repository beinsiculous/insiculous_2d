//! World management for the ECS.

use std::collections::HashMap;

use crate::component::{Component, ComponentRegistry};
use crate::entity::{Entity, EntityId};
use crate::hierarchy::{Children, Parent};
use crate::event::EventBus;
use crate::generation::EntityGeneration;
use crate::resource::ResourceStorage;
use crate::system::SystemRegistry;
use crate::query::QueryTypes;
use crate::EcsError;

/// Configuration for the ECS world
#[derive(Debug, Clone)]
pub struct WorldConfig {
    /// Initial capacity for entity storage
    pub entity_capacity: usize,
    /// Initial capacity for component storage
    pub component_capacity: usize,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            entity_capacity: 1024,
            component_capacity: 4096,
        }
    }
}

/// The main world struct for the ECS
pub struct World {
    /// The entities in the world
    entities: HashMap<EntityId, Entity>,
    /// Entity generation tracking for detecting stale references
    entity_generations: HashMap<EntityId, EntityGeneration>,
    /// The component registry
    components: ComponentRegistry,
    /// The system registry
    systems: SystemRegistry,
    /// Typed singleton resources for cross-system state
    resources: ResourceStorage,
    /// Typed event bus for loose-coupled system communication
    events: EventBus,
    /// Whether the world is initialized
    initialized: bool,
    /// Whether the world is running
    running: bool,
    /// World configuration
    config: WorldConfig,
}

impl World {
    /// Create a new world with default configuration
    pub fn new() -> Self {
        Self::with_config(WorldConfig::default())
    }

    /// Create a new world with custom configuration
    pub fn with_config(config: WorldConfig) -> Self {
        Self {
            entities: HashMap::with_capacity(config.entity_capacity),
            entity_generations: HashMap::with_capacity(config.entity_capacity),
            components: ComponentRegistry::new(),
            systems: SystemRegistry::new(),
            resources: ResourceStorage::new(),
            events: EventBus::new(),
            initialized: false,
            running: false,
            config,
        }
    }

    /// Run a system-registry operation with `self` handed to the systems.
    ///
    /// Temporarily swaps the registry out of `self` so systems can receive
    /// `&mut World` without a double borrow (same pattern as `update`).
    fn with_systems<R>(&mut self, f: impl FnOnce(&mut SystemRegistry, &mut World) -> R) -> R {
        let mut temp_systems = SystemRegistry::new();
        std::mem::swap(&mut self.systems, &mut temp_systems);
        let result = f(&mut temp_systems, self);
        std::mem::swap(&mut self.systems, &mut temp_systems);
        result
    }

    /// Initialize the world, invoking every system's `initialize` hook
    pub fn initialize(&mut self) -> Result<(), EcsError> {
        if self.initialized {
            return Err(EcsError::AlreadyInitialized);
        }

        self.with_systems(|systems, world| systems.initialize(world))?;

        self.initialized = true;
        log::info!("World initialized with {} entities and {} systems",
                  self.entities.len(), self.systems.len());
        Ok(())
    }

    /// Start the world (begin running systems), invoking `start` hooks
    pub fn start(&mut self) -> Result<(), EcsError> {
        if !self.initialized {
            return Err(EcsError::NotInitialized);
        }
        if self.running {
            return Err(EcsError::AlreadyRunning);
        }

        self.running = true;
        self.with_systems(|systems, world| systems.start(world))?;
        log::info!("World started with {} active systems", self.systems.len());
        Ok(())
    }

    /// Stop the world (pause systems), invoking `stop` hooks
    pub fn stop(&mut self) -> Result<(), EcsError> {
        if !self.running {
            return Err(EcsError::NotRunning);
        }

        self.running = false;
        self.with_systems(|systems, world| systems.stop(world))?;
        log::info!("World stopped");
        Ok(())
    }

    /// Shutdown the world, invoking every system's `shutdown` hook
    pub fn shutdown(&mut self) -> Result<(), EcsError> {
        if self.running {
            self.stop()?;
        }

        if self.initialized {
            self.with_systems(|systems, world| systems.shutdown(world))?;
            self.initialized = false;
        }

        log::info!("World shut down");
        Ok(())
    }

    /// Check if the world is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Check if the world is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Create a new entity with generation tracking
    /// Create an entity and return a builder for adding components.
    ///
    /// # Example
    /// ```
    /// # use ecs::{World, Transform2D, Sprite};
    /// # use glam::Vec2;
    /// # let mut world = World::new();
    /// # let (pos, tex) = (Vec2::new(10.0, 20.0), 0);
    /// let entity = world.spawn()
    ///     .with(Transform2D::new(pos))
    ///     .with(Sprite::new(tex))
    ///     .id();
    /// # assert!(world.has_component::<Transform2D>(&entity).unwrap());
    /// # assert!(world.has_component::<Sprite>(&entity).unwrap());
    /// ```
    pub fn spawn(&mut self) -> crate::entity_builder::EntityBuilder<'_> {
        crate::entity_builder::EntityBuilder::new(self)
    }

    /// Create a new entity and return its ID
    pub fn create_entity(&mut self) -> EntityId {
        let entity = Entity::new();
        let id = entity.id();

        // Track the entity generation
        let generation = EntityGeneration::with_generation(id.generation());
        self.entity_generations.insert(id, generation);
        self.entities.insert(id, entity);

        log::trace!("Created entity {} with generation {}", id.value(), id.generation());
        id
    }

    /// Remove an entity (with generation tracking).
    ///
    /// Hierarchy links are cleaned up automatically: the entity is unlinked
    /// from its parent's `Children` list, and its own children become root
    /// entities (their `Parent` component is removed). To delete a whole
    /// subtree instead, use `WorldHierarchyExt::remove_entity_hierarchy`.
    ///
    /// The dead generation entry is retained on purpose so later accesses
    /// with the stale ID report "not alive" rather than "not found".
    pub fn remove_entity(&mut self, entity_id: &EntityId) -> Result<(), EcsError> {
        // Validate entity generation first
        if let Some(generation) = self.entity_generations.get_mut(entity_id) {
            entity_id.validate(generation)?;
            generation.mark_dead();
        } else {
            return Err(EcsError::EntityNotFound(*entity_id));
        }

        if self.entities.remove(entity_id).is_none() {
            return Err(EcsError::EntityNotFound(*entity_id));
        }

        self.detach_from_hierarchy(entity_id);

        // Remove components for this entity
        self.components.remove_all(entity_id);

        log::trace!("Removed entity {} with generation {}", entity_id.value(), entity_id.generation());
        Ok(())
    }

    /// Unlink an entity from the hierarchy prior to its removal.
    ///
    /// Uses component storage directly: the entity is already marked dead at
    /// this point, so the validated accessors would refuse to touch it.
    fn detach_from_hierarchy(&mut self, entity_id: &EntityId) {
        // Remove the entity from its parent's Children list
        let parent_id = self.components.get_typed::<Parent>(entity_id).map(|p| p.entity());
        if let Some(parent_id) = parent_id {
            if let Some(children) = self.components.get_typed_mut::<Children>(&parent_id) {
                children.remove(entity_id);
            }
        }

        // Orphan the entity's children to root
        let child_ids: Vec<EntityId> = self
            .components
            .get_typed::<Children>(entity_id)
            .map(|c| c.entities().to_vec())
            .unwrap_or_default();
        for child in &child_ids {
            self.components.remove::<Parent>(child);
        }
    }

    /// Get a reference to an entity
    pub fn get_entity(&self, entity_id: &EntityId) -> Result<&Entity, EcsError> {
        self.entities
            .get(entity_id)
            .ok_or(EcsError::EntityNotFound(*entity_id))
    }

    /// Get a mutable reference to an entity
    pub fn get_entity_mut(&mut self, entity_id: &EntityId) -> Result<&mut Entity, EcsError> {
        self.entities
            .get_mut(entity_id)
            .ok_or(EcsError::EntityNotFound(*entity_id))
    }

    /// Add a component to an entity
    pub fn add_component<T: Component>(
        &mut self,
        entity_id: &EntityId,
        component: T,
    ) -> Result<(), EcsError> {
        self.validate_entity(entity_id)?;

        self.components.add(*entity_id, component);
        Ok(())
    }

    /// Remove a component from an entity
    pub fn remove_component<T: Component>(&mut self, entity_id: &EntityId) -> Result<(), EcsError> {
        self.validate_entity(entity_id)?;

        if self.components.remove::<T>(entity_id).is_none() {
            return Err(EcsError::ComponentNotFound(*entity_id));
        }

        Ok(())
    }

    /// Get a typed reference to a component for an entity.
    ///
    /// Returns `None` if the entity is dead, stale, or lacks the component.
    pub fn get<T: Component>(&self, entity_id: EntityId) -> Option<&T> {
        if self.validate_entity(&entity_id).is_err() {
            return None;
        }
        self.components.get_typed::<T>(&entity_id)
    }

    /// Get a typed mutable reference to a component for an entity.
    ///
    /// Returns `None` if the entity is dead, stale, or lacks the component.
    pub fn get_mut<T: Component>(&mut self, entity_id: EntityId) -> Option<&mut T> {
        if self.validate_entity(&entity_id).is_err() {
            return None;
        }
        self.components.get_typed_mut::<T>(&entity_id)
    }

    /// Check if an entity has a component
    pub fn has_component<T: Component>(&self, entity_id: &EntityId) -> Result<bool, EcsError> {
        self.validate_entity(entity_id)?;

        Ok(self.components.has::<T>(entity_id))
    }

    /// Get the `(TypeId, type name)` of every component currently attached
    /// to an entity, in unspecified order.
    ///
    /// Returns an empty `Vec` for a dead or stale entity. This is the only
    /// type-erased view of an entity's components; snapshot/duplication code
    /// uses it to detect component types it doesn't know how to clone.
    pub fn component_types(&self, entity_id: EntityId) -> Vec<(std::any::TypeId, &'static str)> {
        if self.validate_entity(&entity_id).is_err() {
            return Vec::new();
        }
        self.components.component_types(&entity_id)
    }

    /// Add a system.
    ///
    /// Systems added after `initialize()`/`start()` get the missed
    /// `initialize`/`start` hooks invoked immediately (failures are logged,
    /// the system is added regardless).
    pub fn add_system<S: crate::system::System + 'static>(&mut self, mut system: S) {
        if self.initialized {
            if let Err(e) = system.initialize(self) {
                log::error!("System '{}' failed late initialization: {e}", system.name());
            }
        }
        if self.running {
            if let Err(e) = system.start(self) {
                log::error!("System '{}' failed late start: {e}", system.name());
            }
        }
        self.systems.add(system);
    }

    /// Update all systems - safe version with proper error handling
    pub fn update(&mut self, delta_time: f32) -> Result<(), EcsError> {
        if !self.initialized {
            return Err(EcsError::NotInitialized);
        }
        if !self.running {
            return Err(EcsError::NotRunning);
        }

        self.with_systems(|systems, world| systems.update_all(world, delta_time));

        Ok(())
    }

    /// Get the number of entities
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Get a snapshot of all entity IDs as an owned Vec.
    ///
    /// Useful when you need to mutate the world while iterating. For
    /// read-only iteration prefer `entity_ids()`, which does not allocate.
    pub fn entities(&self) -> Vec<EntityId> {
        self.entities.keys().copied().collect()
    }

    /// Get a non-allocating iterator over all entity IDs.
    pub fn entity_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.keys().copied()
    }

    /// Get the number of systems
    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    /// Validate that an entity exists and is alive
    pub fn validate_entity(&self, entity_id: &EntityId) -> Result<(), EcsError> {
        if let Some(generation) = self.entity_generations.get(entity_id) {
            entity_id.validate(generation)?;
        } else {
            return Err(EcsError::EntityNotFound(*entity_id));
        }
        Ok(())
    }

    /// Get the current generation for an entity
    pub fn get_entity_generation(&self, entity_id: &EntityId) -> Option<EntityGeneration> {
        self.entity_generations.get(entity_id).copied()
    }

    /// Query for entities with specific component types.
    ///
    /// Returns a vector of entity IDs that have all required component types.
    ///
    /// # Example
    /// ```
    /// use ecs::{World, EntityId, Single, Pair};
    /// use ecs::sprite_components::{Transform2D, Sprite};
    ///
    /// let mut world = World::new();
    /// # let e = world.create_entity();
    /// # world.add_component(&e, Transform2D::default()).ok();
    /// # world.add_component(&e, Sprite::new(0)).ok();
    /// // Get all entities with Transform2D
    /// let entities: Vec<EntityId> = world.query_entities::<Single<Transform2D>>();
    /// // Get all entities with both Transform2D and Sprite
    /// let both: Vec<EntityId> = world.query_entities::<Pair<Transform2D, Sprite>>();
    /// # assert_eq!(entities.len(), 1);
    /// # assert_eq!(both.len(), 1);
    /// ```
    pub fn query_entities<Q: QueryTypes>(&self) -> Vec<EntityId> {
        let required_types = Q::component_types();
        self.entities
            .keys()
            .filter(|entity| {
                required_types
                    .iter()
                    .all(|type_id| self.components.has_type(entity, *type_id))
            })
            .copied()
            .collect()
    }

    /// Remove all entities and components from the world.
    ///
    /// Clears entities, generations, and component storage. Does not
    /// affect systems, initialization state, or configuration.
    pub fn clear(&mut self) {
        self.entities.clear();
        self.entity_generations.clear();
        self.components.shutdown().ok();
    }

    /// Create an entity with a specific ID (for snapshot restoration only).
    ///
    /// Revives the ID's generation as alive: stale references carrying the
    /// same `(id, generation)` will validate again afterwards. That is
    /// intentional for play/stop snapshot restore — the entity legitimately
    /// exists again — and safe because entity IDs are globally monotonic and
    /// never recycled. Callers must `clear()` the world first or otherwise
    /// guarantee the ID is unoccupied; overwriting a live entity loses its
    /// components silently.
    pub fn create_entity_with_id(&mut self, id: EntityId) -> EntityId {
        debug_assert!(
            !self.entities.contains_key(&id),
            "create_entity_with_id overwrote live entity {}",
            id.value()
        );
        let entity = Entity::with_id(id);
        let generation = EntityGeneration::with_generation(id.generation());
        self.entity_generations.insert(id, generation);
        self.entities.insert(id, entity);
        id
    }

    // --- Resources (typed singleton state) ---

    /// Insert a resource, replacing any previous value of the same type.
    pub fn insert_resource<T: Send + Sync + 'static>(&mut self, resource: T) {
        self.resources.insert(resource);
    }

    /// Get an immutable reference to a resource by type.
    pub fn resource<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.resources.get::<T>()
    }

    /// Get a mutable reference to a resource by type.
    pub fn resource_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.resources.get_mut::<T>()
    }

    /// Remove a resource by type, returning it if it existed.
    pub fn remove_resource<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.resources.remove::<T>()
    }

    /// Check if a resource of the given type exists.
    pub fn has_resource<T: Send + Sync + 'static>(&self) -> bool {
        self.resources.contains::<T>()
    }

    // --- Events (typed per-frame messaging) ---

    /// Emit an event. Readable by any system until the next `flush_events()`.
    pub fn emit_event<E: Send + Sync + 'static>(&mut self, event: E) {
        self.events.emit(event);
    }

    /// Read all events of type `E` emitted since the last flush.
    pub fn read_events<E: Send + Sync + 'static>(&self) -> &[E] {
        self.events.read::<E>()
    }

    /// Clear all event queues. Call at the end of each frame.
    pub fn flush_events(&mut self) {
        self.events.flush();
    }

    /// Get world configuration
    pub fn config(&self) -> &WorldConfig {
        &self.config
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
