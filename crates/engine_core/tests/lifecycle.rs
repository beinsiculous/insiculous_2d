//! The engine lifecycle FSM through its public API: every transition either
//! advances the state or is refused and leaves it untouched.

use engine_core::lifecycle::{LifecycleManager, LifecycleState};

#[test]
fn test_lifecycle_state_transitions() {
    let lifecycle = LifecycleManager::new();
    assert_eq!(lifecycle.current_state(), LifecycleState::Created);
    assert!(lifecycle.can_initialize());
    assert!(!lifecycle.is_operational());

    // Refused from Created: nothing runs before initialization.
    assert!(lifecycle.start().is_err());
    assert!(lifecycle.stop().is_err());
    assert!(lifecycle.begin_shutdown().is_err());
    assert_eq!(lifecycle.current_state(), LifecycleState::Created, "a refusal changes nothing");

    lifecycle.begin_initialization().expect("Created → Initializing");
    assert_eq!(lifecycle.current_state(), LifecycleState::Initializing);
    assert!(lifecycle.begin_initialization().is_err(), "already initializing");
    lifecycle.complete_initialization().expect("Initializing → Initialized");
    assert_eq!(lifecycle.current_state(), LifecycleState::Initialized);
    assert!(lifecycle.is_operational());
    assert!(lifecycle.can_start());

    // Refused from Initialized.
    assert!(lifecycle.begin_initialization().is_err());
    assert!(lifecycle.stop().is_err());

    lifecycle.start().expect("Initialized → Running");
    assert_eq!(lifecycle.current_state(), LifecycleState::Running);
    assert!(lifecycle.is_operational());

    // Refused from Running.
    assert!(lifecycle.begin_initialization().is_err());
    assert!(lifecycle.start().is_err());

    lifecycle.stop().expect("Running → Initialized");
    assert_eq!(lifecycle.current_state(), LifecycleState::Initialized);
    assert!(lifecycle.is_operational(), "a stopped scene is still initialized");

    lifecycle.begin_shutdown().expect("Initialized → ShuttingDown");
    assert_eq!(lifecycle.current_state(), LifecycleState::ShuttingDown);
    lifecycle.complete_shutdown().expect("ShuttingDown → ShutDown");
    assert_eq!(lifecycle.current_state(), LifecycleState::ShutDown);
    assert!(!lifecycle.is_operational());

    // An error is a dead end that can only be left by re-initializing.
    let failed = LifecycleManager::new();
    failed
        .set_error(Some("Test error".to_string()))
        .expect_err("set_error reports the error it was given");
    assert_eq!(failed.current_state(), LifecycleState::Error);
    assert!(!failed.is_operational());
    assert!(failed.start().is_err());
    assert!(failed.can_initialize());
    failed.begin_initialization().expect("Error → Initializing");
    assert_eq!(failed.current_state(), LifecycleState::Initializing);
}
