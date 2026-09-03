# Input Crate — Agent Context

Event-based input system with a generic action-mapping layer and a
player-aware settings layer (the universal mapping games consume).

## Key Types
- `InputHandler` — raw device state: keyboard, mouse, gamepads + event queue
- `InputSettings` (player.rs) — **the layer games use**: one `PlayerBindings`
  per local player (`PlayerId::P1/P2`), device-relative sources
  (`PlayerSource::PadButton/PadAxis` resolve against the player's single
  `pad: Option<u32>` at query time — `assign_pad` never rewrites bindings).
  Queries: `is_active / just_activated (player, GameAction, &input)`,
  `just_activated_any`, and `move_x/move_y(player, &input) -> f32` (digital
  merged with left stick, clamped −1..1, +y = up).
  `default_two_player()`: P1 = WASD+Space+mouse+pad 0, P2 = arrows+Enter+pad 1.
  Persisted via engine_core `input_settings_io` (JSON; `GameConfig::input_settings_path`).
  Dirty tracking: real mutations (`set_pad` change, new `bind`, effective `unbind`)
  flag the settings; the engine polls `take_dirty()` each frame and saves on change
  (`mark_dirty()` re-queues a failed save). Constructors/`from_players` start clean.
- `InputMapping<A>` — generic action bindings; games may define private enums:
  `mapping.bind(MyAction::Jump, InputSource::Keyboard(KeyCode::Space))`
- `GameAction` — the fixed engine action vocabulary (MoveUp/Down/Left/Right,
  Action1..4, Menu, Cancel, Select, Custom); also usable as an `InputMapping`
  preset via `with_default_bindings()` (used by `BehaviorRunner`)
- `InputSource::GamepadAxis(id, axis, AxisDirection)` — analog axis as a
  digital source, active past `AXIS_ACTIVATION_THRESHOLD` (0.5); edges come
  from `GamepadState`'s previous-frame axis snapshot
- `ButtonTracker<T>` — shared pressed/just_pressed/just_released tracker composed
  by `KeyboardState`, `MouseState`, `GamepadState`

## Frame Lifecycle
1. `handle_window_event()` queues events (engine does this automatically)
2. `process_queued_events()` at start of frame
3. Game logic reads state
4. `end_frame()` clears one-shot flags + mouse movement/wheel deltas

## Per-Frame Queries
- `is_key_pressed(key)` / `is_key_just_pressed(key)` / `is_key_just_released(key)`
- `is_source_pressed(&source)` family — works for any `InputSource`
- `mapping.is_active(action, &input)` — any bound source held
- `mapping.just_activated(action, &input)` — a bound source was pressed this frame and the action was not active at the end of the previous frame
- `mouse_movement_delta()` — accumulated over the frame, zero when mouse idle

## Design Notes
- `InputMapping::new()` is **empty** — no implicit default bindings
- Gamepads auto-register on first event; `InputEvent::GamepadConnected/
  Disconnected` also register/drop state (disconnect = sources read released,
  no just-released edge). The hardware backend is engine_core's
  `gamepad_backend.rs` (gilrs poll → these events); this crate stays
  hardware-agnostic, and tests drive gamepads via `queue_event()`
- Winit types (`KeyCode`, `MouseButton`) used directly by design (documented
  on `InputEvent`); serde derives on all binding types (winit `serde` feature)
- Scroll deltas normalized to lines (`PixelDelta` ÷ 16)
- Stick Y follows gilrs convention: **positive = up**

## Testing
- `cargo test -p input` — 0 failed, 0 ignored
