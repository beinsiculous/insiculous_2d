//! Public-API contracts of the world lifecycle: hooks fire in order and
//! against the real world, late systems catch up, and one panicking system
//! never takes the others down.

use ecs::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

struct InitMarker;
struct StartMarker;

/// Records every lifecycle hook it receives; the shared handles let a test
/// keep watching after the world has taken ownership of the system.
#[derive(Default, Clone)]
struct HookLog {
    initialized: Arc<AtomicBool>,
    started: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
    shut_down: Arc<AtomicBool>,
    updates: Arc<AtomicU32>,
}

impl HookLog {
    fn updates(&self) -> u32 {
        self.updates.load(Ordering::SeqCst)
    }
}

struct LoggingSystem(HookLog);

impl System for LoggingSystem {
    fn initialize(&mut self, world: &mut World) -> Result<(), String> {
        world.insert_resource(InitMarker);
        self.0.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn start(&mut self, world: &mut World) -> Result<(), String> {
        world.insert_resource(StartMarker);
        self.0.started.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn update(&mut self, _world: &mut World, _delta_time: f32) {
        self.0.updates.fetch_add(1, Ordering::SeqCst);
    }

    fn stop(&mut self, _world: &mut World) -> Result<(), String> {
        self.0.stopped.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn shutdown(&mut self, _world: &mut World) -> Result<(), String> {
        self.0.shut_down.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn name(&self) -> &str {
        "LoggingSystem"
    }
}

#[test]
fn test_world_lifecycle_runs_systems_only_while_running_and_refuses_out_of_order_calls() -> Result<(), EcsError> {
    let log = HookLog::default();
    let mut world = World::new();
    world.add_system(LoggingSystem(log.clone()));
    assert!(!world.is_initialized());
    assert!(!world.is_running());

    assert!(matches!(world.update(0.016), Err(EcsError::NotInitialized)));
    assert!(matches!(world.start(), Err(EcsError::NotInitialized)));

    world.initialize()?;
    assert!(world.is_initialized());
    assert!(log.initialized.load(Ordering::SeqCst));
    assert!(matches!(world.initialize(), Err(EcsError::AlreadyInitialized)));
    assert!(matches!(world.update(0.016), Err(EcsError::NotRunning)));
    assert_eq!(log.updates(), 0, "nothing updates before start");

    world.start()?;
    assert!(world.is_running());
    assert!(log.started.load(Ordering::SeqCst));
    assert!(matches!(world.start(), Err(EcsError::AlreadyRunning)));
    world.update(0.016)?;
    world.update(0.016)?;
    assert_eq!(log.updates(), 2, "every update reaches the system while running");

    world.stop()?;
    assert!(!world.is_running());
    assert!(log.stopped.load(Ordering::SeqCst));
    assert!(matches!(world.stop(), Err(EcsError::NotRunning)));
    assert!(matches!(world.update(0.016), Err(EcsError::NotRunning)));
    assert_eq!(log.updates(), 2, "a stopped world updates nothing");

    world.shutdown()?;
    assert!(!world.is_initialized());
    assert!(log.shut_down.load(Ordering::SeqCst));
    Ok(())
}

#[test]
fn test_late_added_system_gets_missed_hooks() -> Result<(), EcsError> {
    let mut world = World::new();
    world.initialize()?;
    world.start()?;
    let log = HookLog::default();

    world.add_system(LoggingSystem(log.clone()));

    assert!(log.initialized.load(Ordering::SeqCst), "late-added system is initialized immediately");
    assert!(log.started.load(Ordering::SeqCst), "late-added system is started immediately");
    assert!(world.has_resource::<InitMarker>(), "the initialize hook received the real world");
    assert!(world.has_resource::<StartMarker>(), "the start hook received the real world");
    world.update(0.016)?;
    assert_eq!(log.updates(), 1, "and it takes part in the next update");
    Ok(())
}

#[test]
fn test_a_panicking_system_does_not_stop_later_systems_from_updating() -> Result<(), EcsError> {
    struct PanicSystem;

    impl System for PanicSystem {
        fn update(&mut self, _world: &mut World, _delta_time: f32) {
            panic!("Test panic in system");
        }

        fn name(&self) -> &str {
            "PanicSystem"
        }
    }

    let log = HookLog::default();
    let mut world = World::new();
    world.add_system(PanicSystem);
    world.add_system(LoggingSystem(log.clone()));
    world.initialize()?;
    world.start()?;

    // The registry catches the panic, so the frame completes and the
    // next system still runs, this frame and every frame after.
    world.update(0.016)?;
    world.update(0.016)?;

    assert_eq!(log.updates(), 2);
    assert!(world.is_running());
    Ok(())
}
