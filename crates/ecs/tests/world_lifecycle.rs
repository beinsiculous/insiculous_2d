//! Public-API contract of the world lifecycle flags: they move only in order,
//! and every out-of-order call is refused with its own error and changes nothing.

use ecs::prelude::*;

#[test]
fn test_world_lifecycle_flags_follow_initialize_start_stop_shutdown_and_refuse_out_of_order_calls() -> Result<(), EcsError> {
    let mut world = World::new();
    assert!(!world.is_initialized());
    assert!(!world.is_running());

    assert!(matches!(world.start(), Err(EcsError::NotInitialized)));
    assert!(matches!(world.stop(), Err(EcsError::NotRunning)));
    assert!(!world.is_initialized() && !world.is_running(), "a refusal changes nothing");

    world.initialize()?;
    assert!(world.is_initialized() && !world.is_running());
    assert!(matches!(world.initialize(), Err(EcsError::AlreadyInitialized)));
    assert!(matches!(world.stop(), Err(EcsError::NotRunning)), "initialized is not running");

    world.start()?;
    assert!(world.is_running());
    assert!(matches!(world.start(), Err(EcsError::AlreadyRunning)));

    world.stop()?;
    assert!(world.is_initialized() && !world.is_running(), "stopping keeps the world initialized");
    assert!(matches!(world.stop(), Err(EcsError::NotRunning)));

    world.start()?;
    world.shutdown()?;
    assert!(!world.is_running(), "shutting down a running world stops it first");
    assert!(!world.is_initialized());
    assert!(matches!(world.start(), Err(EcsError::NotInitialized)), "a shut-down world must be initialized again");

    world.initialize()?;
    assert!(world.is_initialized(), "a shut-down world initializes again");
    Ok(())
}
