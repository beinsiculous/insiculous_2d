//! One-shot registration of engine-side components into the ecs dynamic
//! registry.
//!
//! The ecs crate registers its own builtins at first access; physics is
//! DOWNSTREAM of ecs and cannot self-register, so the engine wires it here.
//! Called from `run_game` and from `SceneLoader::instantiate` — idempotent
//! and cheap, so headless scene tests are covered without a running game.
//!
//! Downstream games register their own components the same way, in `main()`
//! before `run_game`:
//!
//! ```no_run
//! ecs::register_components(|r| {
//!     // r.register::<MyGameComponent>();
//!     let _ = r;
//! });
//! ```
//!
//! Note: the standalone editor binary never links a game crate, so it cannot
//! see game-registered components — scenes containing them load only from
//! the game's own executable (via `run_game_with_editor`).

/// Register every engine-owned component type into the global registry.
pub fn register_engine_components() {
    ecs::register_components(|registry| {
        #[cfg(feature = "physics")]
        physics::register::register_components(registry);
        let _ = registry;
    });
}
