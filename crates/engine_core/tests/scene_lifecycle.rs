//! A `Scene` through its public API: its lifecycle refuses out-of-order
//! transitions and drives its world's lifecycle.

use engine_core::{LifecycleState, Scene};

#[test]
fn test_scene_lifecycle_refuses_out_of_order_transitions_and_drives_its_world() -> Result<(), String> {
    let mut scene = Scene::new("TestScene");

    assert_eq!(scene.lifecycle_state(), LifecycleState::Created);
    assert!(!scene.is_operational());
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

    scene.stop()?;
    assert!(scene.is_operational() && !scene.is_running());
    assert!(!scene.world.is_running());
    assert!(scene.stop().is_err(), "stop twice");
    scene.shutdown()?;
    assert_eq!(scene.lifecycle_state(), LifecycleState::ShutDown);
    assert!(!scene.is_operational());
    assert!(!scene.world.is_initialized());
    Ok(())
}
