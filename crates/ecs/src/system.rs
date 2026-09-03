//! System management for the ECS.

use std::any::Any;

use crate::world::World;

/// A trait for systems in the ECS
pub trait System: Any + Send + Sync {
    /// Initialize the system (called once when world is initialized)
    fn initialize(&mut self, _world: &mut World) -> Result<(), String> {
        Ok(())
    }

    /// Start the system (called when world starts running)
    fn start(&mut self, _world: &mut World) -> Result<(), String> {
        Ok(())
    }

    /// Update the system
    fn update(&mut self, world: &mut World, delta_time: f32);

    /// Stop the system (called when world stops running)
    fn stop(&mut self, _world: &mut World) -> Result<(), String> {
        Ok(())
    }

    /// Shutdown the system (called when world is shutting down)
    fn shutdown(&mut self, _world: &mut World) -> Result<(), String> {
        Ok(())
    }

    /// Get the name of the system
    fn name(&self) -> &str;
}

/// A simple system that can be created from a closure
pub(crate) struct SimpleSystem<F>
where
    F: FnMut(&mut World, f32) + Send + Sync + 'static,
{
    /// The name of the system
    name: String,
    /// The update function
    update_fn: F,
}

impl<F> SimpleSystem<F>
where
    F: FnMut(&mut World, f32) + Send + Sync + 'static,
{
    /// Create a new simple system
    pub(crate) fn new(name: impl Into<String>, update_fn: F) -> Self {
        Self {
            name: name.into(),
            update_fn,
        }
    }
}

impl<F> System for SimpleSystem<F>
where
    F: FnMut(&mut World, f32) + Send + Sync + 'static,
{
    fn update(&mut self, world: &mut World, delta_time: f32) {
        (self.update_fn)(world, delta_time);
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// A registry for systems with lifecycle management
#[derive(Default)]
pub struct SystemRegistry {
    /// The systems
    systems: Vec<Box<dyn System>>,
    /// Whether the systems are initialized
    initialized: bool,
    /// Whether the systems are running
    running: bool,
}

impl SystemRegistry {
    /// Create a new system registry
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            initialized: false,
            running: false,
        }
    }

    /// Initialize all systems, invoking each system's `initialize` hook.
    ///
    /// Propagates the first failure: a system that cannot initialize should
    /// abort world startup.
    pub fn initialize(&mut self, world: &mut World) -> Result<(), String> {
        if self.initialized {
            return Err("Systems already initialized".to_string());
        }

        log::info!("Initializing {} systems", self.systems.len());
        for system in &mut self.systems {
            system
                .initialize(world)
                .map_err(|e| format!("System '{}' failed to initialize: {e}", system.name()))?;
        }

        self.initialized = true;
        log::info!("All systems initialized successfully");
        Ok(())
    }

    /// Start all systems, invoking each system's `start` hook.
    ///
    /// Propagates the first failure.
    pub fn start(&mut self, world: &mut World) -> Result<(), String> {
        if !self.initialized {
            return Err("Systems not initialized".to_string());
        }
        if self.running {
            return Err("Systems already running".to_string());
        }

        log::info!("Starting {} systems", self.systems.len());
        for system in &mut self.systems {
            system
                .start(world)
                .map_err(|e| format!("System '{}' failed to start: {e}", system.name()))?;
        }

        self.running = true;
        log::info!("All systems started successfully");
        Ok(())
    }

    /// Stop all systems, invoking each system's `stop` hook.
    ///
    /// Hook failures are logged and skipped so every system gets a chance
    /// to stop (teardown should be best-effort).
    pub fn stop(&mut self, world: &mut World) -> Result<(), String> {
        if !self.running {
            return Err("Systems not running".to_string());
        }

        log::info!("Stopping {} systems", self.systems.len());
        for system in &mut self.systems {
            if let Err(e) = system.stop(world) {
                log::error!("System '{}' failed to stop: {e}", system.name());
            }
        }

        self.running = false;
        log::info!("All systems stopped");
        Ok(())
    }

    /// Shutdown all systems, invoking each system's `shutdown` hook.
    ///
    /// Hook failures are logged and skipped (teardown is best-effort).
    pub fn shutdown(&mut self, world: &mut World) -> Result<(), String> {
        if self.running {
            self.stop(world)?;
        }

        if !self.initialized {
            return Ok(()); // Nothing to shutdown
        }

        log::info!("Shutting down {} systems", self.systems.len());
        for system in &mut self.systems {
            if let Err(e) = system.shutdown(world) {
                log::error!("System '{}' failed to shut down: {e}", system.name());
            }
        }

        self.initialized = false;
        log::info!("All systems shut down");
        Ok(())
    }

    /// Add a system
    pub fn add<S: System + 'static>(&mut self, system: S) {
        self.systems.push(Box::new(system));
    }

    /// Add a simple system from a closure
    pub fn add_simple<F>(&mut self, name: impl Into<String>, update_fn: F)
    where
        F: FnMut(&mut World, f32) + Send + Sync + 'static,
    {
        self.add(SimpleSystem::new(name, update_fn));
    }

    /// Update all systems - safe version with proper error handling
    pub fn update_all(&mut self, world: &mut World, delta_time: f32) {
        if !self.running {
            log::warn!("Attempted to update systems while not running");
            return;
        }

        // Create a temporary vector to hold systems during update
        // This prevents borrowing issues and provides better error isolation
        let mut temp_systems = Vec::new();
        std::mem::swap(&mut self.systems, &mut temp_systems);
        
        // Update all systems with individual error handling
        for system in &mut temp_systems {
            let system_name = system.name().to_string();
            
            // Use catch_unwind to prevent panics from corrupting the system registry
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                system.update(world, delta_time);
            }));
            
            if let Err(_panic) = result {
                log::error!("System '{}' panicked during update", system_name);
                // Continue with other systems instead of crashing the whole engine
            }
        }
        
        // Move systems back
        self.systems = temp_systems;
    }

    /// Get the number of systems
    pub fn len(&self) -> usize {
        self.systems.len()
    }

    /// Check if there are no systems
    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }
}