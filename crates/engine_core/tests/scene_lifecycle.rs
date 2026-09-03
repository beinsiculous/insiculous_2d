//! A `Scene` through its public API: its lifecycle gates `update`, drives
//! its world's lifecycle, and a started scene actually runs its schedule.

use ecs::system::SystemRegistry;
use engine_core::{LifecycleState, Scene};
use std::sync::{Arc, Mutex};

#[test]
fn test_scene_with_schedule() -> Result<(), String> {
    let mut scene = Scene::new("TestScene");
    let mut schedule = SystemRegistry::new();
    let update_count = Arc::new(Mutex::new(0));
    let counter = Arc::clone(&update_count);
    schedule.add_simple("TestSystem", move |_world, _dt| {
        *counter.lock().expect("counter lock") += 1;
    });

    // Not operational yet: updates are refused, and so is starting.
    assert_eq!(scene.lifecycle_state(), LifecycleState::Created);
    assert!(scene.update(0.016).is_err());
    assert!(scene.start().is_err(), "start before initialize");
    assert_eq!(scene.lifecycle_state(), LifecycleState::Created, "a refusal changes nothing");

    // The scene's lifecycle propagates to its world.
    scene.initialize()?;
    assert!(scene.is_initialized() && !scene.is_running());
    assert!(scene.world.is_initialized());
    assert!(scene.initialize().is_err(), "initialize twice");
    scene.start()?;
    assert!(scene.is_running());
    assert!(scene.world.is_running());
    assert!(scene.start().is_err(), "start twice");

    // Lifecycle hooks need a world to act on.
    let mut hook_world = ecs::World::new();
    schedule.initialize(&mut hook_world)?;
    schedule.start(&mut hook_world)?;

    scene.update_with_schedule(&mut schedule, 0.016)?;
    scene.update_with_schedule(&mut schedule, 0.016)?;

    assert_eq!(*update_count.lock().expect("counter lock"), 2, "a started scene runs its schedule");

    schedule.stop(&mut hook_world)?;
    schedule.shutdown(&mut hook_world)?;
    scene.stop()?;
    assert!(!scene.world.is_running());
    assert!(scene.update(0.016).is_err(), "a stopped scene no longer updates");
    assert!(scene.stop().is_err(), "stop twice");
    scene.shutdown()?;
    assert_eq!(scene.lifecycle_state(), LifecycleState::ShutDown);
    assert!(!scene.world.is_initialized());
    Ok(())
}
