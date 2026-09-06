# Scripting in Insiculous 2D

Insiculous 2D provides an embedded scripting subsystem powered by [Rhai](https://rhai.rs) and native Rust descriptors. Scripts attach to entities via the `Scripts` component, providing lightweight game logic, movement controllers, collision handling, and state coordination.

## Script File Structure

Rhai scripts are stored as `.rhai` files within your project assets (for example `scripts/paddle.rhai`).

A script may define parameter headers, an `early_update` hook, and an `update` hook:

```rhai
// @param speed: f32 = 250.0
// @param tag: str = "player"

fn early_update(me, view, params, cmd, dt) {
    // Runs before the physics simulation step
}

fn update(me, view, params, cmd, dt) {
    // Runs after the physics simulation step and collision drain
}
```

Both hooks are optional. A script defining only `update` will only be called in the post-physics phase.

---

## Parameter Headers (`// @param`)

Scripts declare configurable parameters in their leading comments:

```
// @param <name>: <type> = <default_value>
```

Supported types and format examples:
- `f32`: `// @param speed: f32 = 150.0`
- `i32`: `// @param max_health: i32 = 3`
- `bool`: `// @param inverted: bool = false`
- `str`: `// @param title: str = "Hero"`
- `vec2`: `// @param offset: vec2 = (0.0, 10.0)`
- `entity`: `// @param target: entity = "ball"`
- `color`: `// @param tint: color = [1.0, 0.0, 0.0, 1.0]`

The editor inspector reads these declarations to expose typed fields with reset buttons. In code, parameters are accessible via `params.<name>` with fallback to their header defaults.

---

## Hook Signatures

Every script hook receives five arguments:

```rhai
fn update(me, view, params, cmd, dt)
```

### 1. `me` — Self View
Read-only snapshot of the entity executing the script:
- `me.name`: Entity's string name (empty string if unnamed)
- `me.transform`: Current `Transform2D`
- `me.position`: Shortcut for `me.transform.position` (`Vec2`)
- `me.rotation`: Shortcut for `me.transform.rotation` (`f32` in radians)
- `me.velocity`: Current linear velocity (`Vec2`)

### 2. `view` — World Query View
Read-only query methods across world state, input, and collisions:
- **Timing & Frame**:
  - `view.delta_time`: Frame delta time in seconds (the same value as the `dt` argument)
  - `view.frame`: Elapsed frame counter (`int`)
- **Entities**:
  - `view.has_entity(name)`: Returns `true` if an entity with this name exists
  - `view.transform(name)`: `Transform2D` of named entity
  - `view.position(name)`: `Vec2` position of named entity
  - `view.velocity(name)`: `Vec2` velocity of named entity
- **Shared State (Blackboard)**:
  - `view.blackboard_bool(key, default)`: Read boolean key
  - `view.blackboard_int(key, default)`: Read integer key
  - `view.blackboard_float(key, default)`: Read float key
  - `view.blackboard_str(key, default)`: Read string key
- **Input**:
  - `view.move_x(player_index)`: Horizontal axis (-1.0 to 1.0)
  - `view.move_y(player_index)`: Vertical axis (-1.0 to 1.0)
  - `view.is_active(player_index, action_name)`: Whether button/action is held
  - `view.just_activated(player_index, action_name)`: Whether action was pressed this frame
- **Collisions** (the `update` hook only; `early_update` runs before the physics step and sees none):
  - `view.has_collision(name_a, name_b)`: A contact between the named entities **started this frame**
  - `view.has_collision(me, other_name)`: A contact between `me` and the named entity started this frame
  - `view.has_collision_started(name_a, name_b)` / `view.has_collision_started(me, other)`: The same, spelled out
  - `view.has_collision_stopped(name_a, name_b)`: A contact ended this frame

  The view carries start and stop events, never ongoing contact: a ground check polled with
  `has_collision` is true for exactly one frame. Keep "standing" state on the blackboard.

### 3. `params` — Parameter Map
Map containing resolved parameter values:
- `params.<param_name>`: Reads the value specified in the inspector or falling back to the header default.

### 4. `cmd` — Command Buffer
Commands queue deferred actions executed cleanly at the end of the phase:
- **Positions & Transforms**:
  - `cmd.set_position(target, vec2)` or `cmd.set_position(target, x, y)`
  - `cmd.set_rotation(target, radians)`
- **Physics**:
  - `cmd.set_velocity(target, vec2)` or `cmd.set_velocity(target, vx, vy)`
  - `cmd.set_velocity_x(target, vx)`
  - `cmd.set_velocity_y(target, vy)`
  - `cmd.set_kinematic_target(target, vec2)` or `cmd.set_kinematic_target(target, x, y)`
  - `cmd.reset_body(target, vec2)` or `cmd.reset_body(target, x, y)`: Teleports body and zeroes velocity
- **Visuals & UI**:
  - `cmd.set_sprite_color(target, [r, g, b, a])`: four numbers, 0.0 to 1.0
  - `cmd.set_sprite_visible(target, bool)`
  - `cmd.set_label_text(target, text)`
- **Blackboard Writes**:
  - `cmd.set_blackboard_bool(key, bool)`
  - `cmd.set_blackboard_int(key, int)`: the value must fit an `i32`; a larger one is a runtime error
  - `cmd.set_blackboard_float(key, float)`
  - `cmd.set_blackboard_str(key, str)`
- **Lifecycle**:
  - `cmd.despawn(target)`

`target` can be `me`, an entity ID, or an entity's name string.

### 5. `dt` — Delta Time
Float delta time in seconds, equal to `view.delta_time`.

---

## Built-In Behaviors

The engine provides built-in behaviors registered in the script catalog:
- `engine::rotate`: Rotates the entity continuously.
  - Parameters: `degrees_per_second: f32 = 90.0`

---

## Worked example: pong

The Pong sample (`crates/playground/assets/projects/pong/`) implements gameplay entirely with data and four Rhai scripts:

1. **`paddle.rhai`** (`early_update`):
   - Parameters: `player: i32 = 0`, `x: f32 = -370.0`, `speed: f32 = 450.0`, `ai: bool = false`, `ai_speed: f32 = 255.0`, `dead_zone: f32 = 2.0`, `target: entity = "Ball"`.
   - Player control: reads vertical input via `view.move_y(params.player)` and scales by `params.speed * dt` (W/S for player 0, Up/Down arrows for player 1).
   - AI control: tracks `params.target` (`view.position(params.target).y`) at `params.ai_speed` outside `params.dead_zone`.
   - Motion: clamps vertical position to `[-230.0, 230.0]` and positions the paddle via `cmd.set_kinematic_target(me, vec2(params.x, new_y))`.
2. **`ball.rhai`** (`early_update` and `update`):
   - Parameters: `speed: f32 = 250.0`, `max_vertical: f32 = 500.0`.
   - Serve (`early_update`): launches ball on Action1 (`view.just_activated(0, "action1")` or `1`, Space/Enter) while `serving` is `true` and `game_over` is `false`. Launch direction heads toward the last scorer (`last_scorer` on the blackboard, the Rust game's rule; absent or after a restart it is `"right"`, so the first serve goes right) with pseudo-random vertical spread derived from `view.frame`. Sets velocity via `cmd.set_velocity(me, dir * params.speed)` and clears `serving` (`cmd.set_blackboard_bool("serving", false)`).
   - Speed maintenance (`update`): while not serving, pins horizontal speed to `sign * speed` and clamps vertical speed to `params.max_vertical`.
3. **`goal.rhai`** (`update`):
   - Parameters: `side: str = "left"`.
   - On `view.has_collision_started(me, "Ball")`, increments the opposite player's score on the blackboard (`right_score` for left goal, `left_score` for right goal), updates `last_scorer`, teleports the ball back to center via `cmd.reset_body("Ball", vec2(0.0, 0.0))`, and sets `cmd.set_blackboard_bool("serving", true)`.
4. **`scoreboard.rhai`** (`update`):
   - Formats and displays current score on a `UiLabel` entity: `${left} : ${right}`.
   - When either score reaches 7, declares the winner (`LEFT WINS — Action1 to restart` or `RIGHT WINS — Action1 to restart`) and sets `game_over` on the blackboard.
   - On Action1 press during `game_over`, resets scores to 0, clears `game_over`, sets `serving`, resets `last_scorer` to `"right"`, and updates the label to `0 : 0`.

**State Coordination & Timing**:
Shared match state is coordinated via blackboard keys: `left_score` (`i32`), `right_score` (`i32`), `last_scorer` (`str`), `serving` (`bool`), and `game_over` (`bool`). Commands applied in one phase are committed to the world and blackboard before the next phase executes.

---

## Error Handling & Quarantining

- **Syntax & Header Errors**: Reported on file save (the playground's Save status says "syntax OK — runtime errors show during Play"), and a project import refuses an archive whose `.rhai` fails the check. Rhai is dynamic, so an unknown function or a misspelled getter is a runtime error, not a syntax error.
- **Runtime Errors**: Surfaced in the editor status bar during Play mode, once per distinct error per Play session, and on the playground page through `playground_script_errors()`. A hook that errors has that call's commands discarded.
- **Runaway Loops**: Rhai execution is bounded by an operations limit (200,000 ops). Scripts exceeding this quota are automatically quarantined for the rest of the Play session to keep the engine and editor responsive.
- **Hook Failure Isolation**: If a script errors mid-hook, any commands queued by that failing hook are discarded without corrupting the ECS world.

---

## Running scripts outside the editor

The runner is engine-owned (`ctx.scripts`) but stepped by whoever owns the physics step,
because `early_update` must run before that step and `update` after it with the frame's
drained collisions. The editor's `ProjectHost` does this for every data project — the
playground included. A shipped game that wants scripts does the same in its own `update`:

```rust
// once, when a session starts
ctx.scripts.reset(ctx.world, ctx.assets.base_path());

// every frame
ctx.scripts.early_update(ctx.world, ctx.input, ctx.players, ctx.delta_time, Some(&mut physics));
physics.update(ctx.world, ctx.delta_time);
let collisions = physics.take_collision_events();
ctx.scripts.update(ctx.world, ctx.input, ctx.players, ctx.delta_time, &collisions, Some(&mut physics));
```

A game that never steps the runner runs no scripts, silently; inside the editor the status bar
says so at Play frame 60 ("scripts attached but the game never ran the script runner").
A `.rhai` source is compiled when Play starts and runs as compiled until the next Play, so a
file edited mid-Play takes effect on Stop → Play; a script that is missing or broken when
Play starts is reported once and likewise not retried until the next Play.

## Architecture: "One Bundle, Many Projects"

Scripts operate on standard ECS components and shared engine blackboards. The same engine binary and wasm bundle runs any project without custom compile-time dependencies.
