# Insiculous 2D - AI Pair Programming Guide

## Context
Insiculous 2D is a lightweight, modular game engine designed for creating 2D games with Rust. It aims to provide a simple yet powerful API that allows developers to focus on game logic rather than boilerplate code. The engine prioritizes performance, cross-platform compatibility, and a clean, intuitive interface. The architecture follows a component-based design with clear separation of concerns between systems.

## Directory Map
- **crates/engine_core/** - Core functionality including Game API, lifecycle management, and scene coordination
  - `game.rs` - **Simple Game API** - `Game` trait, `run_game()`, GameRunner orchestration
  - `game_config.rs` - `GameConfig` (window, clear color, target FPS, chaos mode, asset base path, input settings path)
  - `input_settings_io.rs` - JSON load/save for player input bindings (defaults written when missing; never panics on bad files)
  - `game_loop_manager.rs` - Game loop timing and frame delta management (the only frame timer)
  - `render_manager.rs` / `window_manager.rs` - Focused managers (renderer lifecycle, window creation)
  - `contexts.rs` - `GameContext` / `RenderContext` passed to Game methods
  - `assets.rs` - Asset loading and caching (tracks handle→path for scene save)
  - `assets/sprite_sheet.rs` - `AssetManager::load_sprite_sheet` (PNG + sidecar → `SpriteSheet`), `SidecarCache` (one sidecar read per path per scene load, cleared at the top of every load)
  - `sheet_file.rs` - **the `.sheet.ron` schema** (`SheetFile` v1, `parse_sheet_file`, `into_parts`, `sidecar_path_for`); all sidecar validation lives here
  - `texture_filter_serde.rs` - shared serde bridge for `TextureFilter` (renderer stays serde-free); used by `GameConfig.texture_filter` and `SheetFile.filter`
  - `scene.rs` / `scene_loader.rs` / `scene_data.rs` / `scene_serializer.rs` - Scene lifecycle, RON load, schema, World→RON save (the ONLY save pipeline)
  - `behavior_runner/` (`mod.rs` + `handlers.rs` + `camera.rs`) / `behavior_data.rs` - Behavior system + Behavior↔BehaviorData `From` pair
  - `chaos_mode.rs` - `ChaosMode` enum (cross-game intensity theme)
  - `menu_input.rs` - `MenuInput` shared menu navigation (read keys once, wraparound `navigate`; scans every connected gamepad)
  - `pause.rs` - `PauseMenu`/`PauseAction` shared pause mechanism (see Pause Pattern)
  - `menu_panel.rs` - `MenuPanel`/`MenuStyle` shared menu window chrome (see Menu Chrome Pattern)
  - `spawn_helpers.rs` - Shared entity recipes (`spawn_background`); crate root exports `RENDER_UNIT = 80.0` (pixels per world unit)
  - `pickups.rs` - Generic pickup tracking (`Pickups<K>`, `EffectTimer`)
  - `achievements/` - Achievement registry, atomic JSON persistence (temp file + rename), toasts
  - `particles/` - Pooled particle system (ring buffer, config builder)
  - `grid/` - Deformable spring-mass grid effect (general-purpose visual)
  - `glyph_texture_cache.rs` - UI glyph bitmap → GPU texture cache
  - `texture_ref.rs` - Scene texture references (`#white`, `#solid:RRGGBB`, paths); the `TextureResolver` trait is the GPU/filesystem seam — also `sheet_for()` (sidecar → `SheetData`) and `clear_sidecar_cache()`
  - `tilemap_render.rs` - Tilemap → sprite-batch expansion (default `Game::render` calls it; one batch per tileset)
  - `ui_integration.rs` - UI-to-renderer bridge; `debug.rs` - collider outline drawing
  - `lifecycle.rs` - FSM for scene lifecycle

- **crates/renderer/** - WGPU-based rendering system with instancing, batching, HDR + bloom
  - `renderer.rs` - WGPU device/queue/surface lifecycle, `RendererConfig`
  - `sprite.rs` + `sprite/batch.rs` + `sprite/pipeline.rs` - Sprite type, CPU batching, GPU pipeline
  - `sprite_data.rs` - GPU data structures (Vertex, Instance, DynamicBuffer)
  - `texture.rs` - Texture manager with caching (`TextureHandle::WHITE`), `TextureFilter` (Linear/Nearest sampling knob)
  - `line_pipeline.rs` - Line-list geometry (grid effect); `bloom.rs` + `render_targets.rs` - HDR post-processing
  - `shaders/` - WGSL shaders

- **crates/ecs/** - Entity Component System for game object management
  - `world.rs` - World: entity/component CRUD, queries, resources, events
  - `component.rs` - Component trait and HashMap-based per-type storage (`ComponentStore`)
  - `component_registry.rs` - Global type registry (create components by name), `define_component!`
  - `query.rs` - Type-safe query types (Single, Pair, Triple)
  - `hierarchy.rs` / `hierarchy_extension.rs` / `hierarchy_system.rs` - Parent/child links, `WorldHierarchyExt`, transform propagation
  - `sprite_components.rs` - Sprite, Transform2D, Camera, plus `AnimationClip` + `SpriteAnimation` (named clips over a `SheetGrid`); `sprite_system.rs` - `SpriteAnimationSystem` advances clips and writes `Sprite.tex_region` (driven from engine_core's `frame_tail.rs`)
  - `behavior.rs` - Scene-driven `Behavior` enum + `BehaviorState` (incl. `CameraFollow`)
  - `tilemap.rs` - `Tilemap` component (row-major tile grid over a tileset; `sprite_instances()` yields plain per-tile data)
  - `state_machine.rs` - `StateMachine` / `HierarchicalStateMachine` FSM components
  - `event.rs` - `EventBus` (typed per-frame event queue)
  - `audio_components.rs` - AudioSource/AudioListener (editor-inspectable data only)

- **crates/input/** - Event-based input with action-mapping + player-aware settings layers
  - `input_handler.rs` - Device state (keyboard/mouse/gamepads) + event queue
  - `input_mapping.rs` - Generic `InputMapping<A>` action bindings (games define action enums); `InputSource` incl. `GamepadAxis` (axis-as-button, threshold 0.5)
  - `player.rs` - `InputSettings`/`PlayerBindings`/`PlayerId`: per-player device routing (device-relative `PlayerSource`, one `pad` id per player), `move_x/move_y` merged digital+analog — the layer games consume via `ctx.players`
  - `button_tracker.rs` - Shared `ButtonTracker<T>` (pressed/just_pressed/just_released)
  - `keyboard.rs` / `mouse.rs` / `gamepad.rs` - Per-device state (gamepad axes keep a prev-frame snapshot for edges; stick +Y = up)

- **crates/ui/** - Immediate-mode UI framework
  - `context/` - UIContext: lifecycle + primitives (`mod.rs`), labels (`text.rs`), widgets (`widgets.rs`)
  - `font/` - FontManager facade (`mod.rs`), `glyph_cache.rs`, `layout.rs` (fontdue)
  - `draw.rs` - Draw command generation
  - `interaction.rs` - Widget state, mouse interaction, focus
  - `style.rs` - Theme definitions (`Color`/`Rect` re-exported from `common`)

- **crates/physics/** - Rapier2d-based 2D physics integration
  - `lib.rs` - Physics system integration with ECS
  - `physics_world/` - Rapier2d world wrapper (bodies, stepping, queries)
  - `physics_system/` - ECS system for physics updates (sync, update loop)
  - `components.rs` - RigidBody, Collider ECS components
  - `presets.rs` - Pre-configured physics presets for common scenarios

- **crates/audio/** - Audio playback via rodio
  - `lib.rs` - crate docs + re-exports
  - `manager.rs` - Sound loading, playback, volume buses; `sound.rs` - SoundHandle, SoundSettings; `error.rs` - AudioError
  - Note: AudioSource/AudioListener/PlaySoundEffect live in `crates/ecs/src/audio_components.rs`
    and are editor-inspectable data only — no runtime system consumes them yet

- **crates/common/** - Shared types and utilities
  - `rect.rs` / `color.rs` / `transform.rs` / `camera.rs` / `time.rs` / `macros.rs` - Rect, Color, transforms, CameraUniform, Time

- **crates/editor/** - Editor data + widgets (panels, inspector, gizmos, undo/redo). NO engine_core dep. See `crates/editor/CLAUDE.md` for the file map; `stored_component.rs` holds the component registry macro (single source of truth).

- **crates/editor_integration/** - Bridges editor and engine_core (`EditorGame<G>` wrapper, `run_game_with_editor()`, panel rendering, inspector writeback). See its `CLAUDE.md`.

- **crates/ecs_macros/** - `#[derive(ComponentMeta)]` proc macro

- **examples/** - Working demonstrations of engine features
  - `hello_world.rs` - Physics platformer with UI, audio, ECS, and scene graph
  - `editor_demo.rs` - Editor wrapping the platformer (`--features editor`)
  - `behavior_demo.rs` - Scene-driven behaviors
  - Real games live in `../games/` (pong, breakout, space_invaders, snake, asteroids) — see `PROJECT_ROADMAP.md`

## Coding Guidelines

### Single Responsibility Principle (SRP)
Each module, struct, and function should have a single, well-defined responsibility. For example, the `Renderer` handles graphics rendering, while `InputHandler` manages input, with no overlap between them.

### Don't Repeat Yourself (DRY)
Extract common functionality into reusable components. Use traits for shared behavior and create utility functions for repeated operations.

### Explicit Naming
Use descriptive names that clearly communicate purpose:
- Functions should be verbs: `start()`, `update()`, `render()`
- Boolean methods should be questions: `is_running()`, `has_component()`
- Getters should use noun form: `device()`, `keyboard()`
- Mutable getters should use `_mut` suffix: `keyboard_mut()`
- **No abbreviations**, loop variables and closure parameters included:
  `keyboard` not `kb`, `image` not `img`, `manager` not `mgr`, `button` not
  `btn`, `delta_time` not `dt`, `center` not `c`. One-letter bindings are
  for tight math in `common` only.
- **A converted value names its unit**: `position_meters`, `half_extents_meters`,
  `clamped_delta_time` — the reader must not have to find the conversion.
- **A binding names one thing.** Never reuse `b` for a button and then a
  behavior in the same function.

### Comment Policy
A comment earns its line by saying something the code cannot.
- `///` states the contract in a few lines: what the caller may rely on, what
  it must not do. Module docs describe the module's responsibility, not its
  history. A compiled doc example is a test and stays when it is the only
  executable pin on a public API.
- `//` survives only as a **pitfall**, an **invariant**, or a **failed approach**
  (the Firefox WebGPU crash rationale, the two-batcher painter's reason, the
  `Box<dyn Component>` downcast trap, the RwLock re-entrancy guard, the tie
  rule in `scores.rs`). Narration that restates the next line is deleted;
  numbered section headers (`// 3. Menu bar`) become function names.
- **Keep the reason, drop the tag.** Issue numbers, reviewer tags (`kimi F3`),
  audit sections, sprint names and pattern codes (`GPP-15`) never appear in
  source; the sentence that explains the decision does. A comment that is
  only a tag is deleted. The gate (bare `#42` counts; six-digit hex is
  spared by the pattern — a match inside a string or a code span, like the
  command API's `#<id>` examples, is inspected by hand, not excluded):
  ```sh
  grep -riEn "kimi|issue #[0-9]+|GPP-[0-9]+|audit §|\(#[0-9]+\)|#[0-9]{1,4}\b|Sprint [0-9]" crates src examples --include=*.rs
  ```
  prints nothing. Nothing runs it at commit time: it is `/finish-task` Gate 3
  and part of every batch gate, run by hand.
- Changelog prose belongs in `log_archive.md`, dated, not in a module doc.

### No `#[allow]`
A lint is a design signal, not noise to silence.
- `too_many_arguments`: bundle the per-call inputs into a context struct
  built once by the caller (`HandlerFrame { input, delta_time, physics }`)
  or pass the owning value (`&Behavior`) and destructure inside. Never a
  tuple parameter — that is the allow in another costume.
- `should_implement_trait`: rename (`WidgetId::hashed`, not `from_str`).
- A destructure that cannot fail by construction still logs before an early
  return; a silent `let … else { return }` hides a dispatch bug.
- The one standing exception, `arc_with_non_send_sync` in `renderer.rs`,
  carries its reason in the comment above it.

### Rust Idioms
- Prefer `impl From<T>` over custom conversion functions
- Use `Option<T>` for values that might be absent
- Use `Result<T, E>` for operations that might fail
- Implement traits like `Default` where appropriate
- Use `#[derive]` for common traits when possible
- Prefer ownership with borrowing over raw pointers
- Use strong typing over stringly-typed code

## Patterns Reference

### Manager Pattern (January 2026)
Extracts responsibilities from monolithic classes into focused managers following SRP:

```rust
// BEFORE: GameRunner had 15+ responsibilities
// AFTER: Responsibilities distributed to focused managers

// GameLoopManager - Frame timing and delta calculation
let mut loop_mgr = GameLoopManager::new();
let delta_time = loop_mgr.update();

// RenderManager - Renderer lifecycle
let mut render_mgr = RenderManager::new();
render_mgr.init(&window, clear_color)?;
render_mgr.render(&batches, &textures)?;

// WindowManager - Window management
let mut win_mgr = WindowManager::new(config);
let window = win_mgr.create(event_loop)?;
```

**Files:** `game_loop_manager.rs`, `render_manager.rs`, `window_manager.rs`

### Simple Game API Pattern
The primary way to create games. Implement the `Game` trait:

```rust
use engine_core::prelude::*;

struct MyGame;

impl Game for MyGame {
    fn init(&mut self, ctx: &mut GameContext) {
        // Load assets, create entities, set initial state
        let player = ctx.world.create_entity();
        ctx.world.add_component(&player, Transform2D::new(Vec2::ZERO)).ok();
        ctx.world.add_component(&player, Sprite::new(0)).ok();
        
        // Load textures (init returns (), so handle the Result explicitly)
        let player_tex = ctx.assets.load_texture("player.png").expect("missing player.png");
    }

    fn update(&mut self, ctx: &mut GameContext) {
        // Game logic - run every frame
        // Access input, update entities, handle collisions
        if ctx.input.is_key_pressed(KeyCode::KeyD) {
            // Move player
        }
        
        // UI
        ctx.ui.label("Score: 100", Vec2::new(10.0, 10.0));
        if ctx.ui.button("play_btn", "Play", button_rect) {
            // Button clicked
        }
    }
}

fn main() {
    run_game(MyGame, GameConfig::new("My Game")).unwrap();
}
```

**Files:** `game.rs`, `game_loop_manager.rs`, `GameContext` struct

### ECS Pattern
HashMap-based per-type component storage with type-safe queries:

```rust
// Components stored in per-type HashMaps via ComponentRegistry
world.add_component(&entity, Transform2D::new(pos)).ok();
world.add_component(&entity, Sprite::new(texture_handle)).ok();

// Type-safe queries - get entities with specific components
use ecs::{Single, Pair, Triple};
let transforms = world.query_entities::<Single<Transform2D>>();
let sprites_with_transform = world.query_entities::<Pair<Transform2D, Sprite>>();
let full_entities = world.query_entities::<Triple<Transform2D, Sprite, RigidBody>>();

// Type-safe component access — get/get_mut take EntityId by value, return Option
if let Some(transform) = world.get::<Transform2D>(entity) {
    println!("position: {:?}", transform.position);
}
// To update one component from another, read first, then get_mut sequentially
let offset = world.get::<Sprite>(entity).map(|s| s.offset);
if let (Some(offset), Some(transform)) = (offset, world.get_mut::<Transform2D>(entity)) {
    transform.position += offset;
}

// Efficient entity iteration
for entity in world.entities() {
    // Process entities
}

// Hierarchy methods via WorldHierarchyExt trait
use ecs::WorldHierarchyExt;
world.set_parent(child, parent)?;
let children = world.get_children(parent);
let descendants = world.get_descendants(root);
let roots = world.get_root_entities();
```

**Files:** `ecs/lib.rs`, `ecs/component.rs`, `ecs/entity.rs`, `ecs/world.rs`, `ecs/hierarchy_extension.rs`

### Immediate-Mode UI Pattern
UI described every frame rather than retained state:

```rust
// Begin frame
ui.begin_frame(&input, window_size);

// Create UI elements
ui.panel(UIRect::new(10.0, 10.0, 200.0, 150.0));
ui.label("Volume:", Vec2::new(20.0, 30.0));
let volume = ui.slider("volume", current_volume, slider_rect);

// If button clicked, returns true
if ui.button("play", "Play", button_rect) {
    // Handle click
}

// End frame (collects draw commands)
ui.end_frame();
```

**Files:** `ui/lib.rs`, `ui/context/`, `ui/draw.rs`

### Asset Manager Pattern
Centralized asset loading with caching:

```rust
// Load textures
let texture = ctx.assets.load_texture("player.png")?;

// Create programmatic textures
let white = ctx.assets.create_solid_color(1, 1, [255, 255, 255, 255])?;
let checkerboard = ctx.assets.create_checkerboard(
    64, 64, [100, 100, 100, 255], [150, 150, 150, 255], 8
)?;
// Raw RGBA8 pixel data (nearest-filtered — built for tileset strips/pixel art;
// path records the non-unique "#rgba" sentinel, not usable for scene save/load)
let tileset = ctx.assets.create_texture_from_rgba(64, 16, &rgba_bytes)?;

// Use assets in ECS
world.add_component(&entity, Sprite::new(texture.id)).ok();
```

**Files:** `assets.rs`, `texture.rs`

### Sprite Sheet Pattern (`.sheet.ron` + named clips)
A sheet is a PNG plus a `.sheet.ron` sidecar under the **same stem**
(`sprites/deion_16.png` ↔ `sprites/deion_16.sheet.ron`). The sidecar declares
the pixel cell size, the sampling filter, and the clips; the grid is derived at
load from the PNG's dimensions ÷ the cell size, so normalized UVs never appear
in any file.

```ron
// assets/sprites/deion_16.sheet.ron
SheetFile(
    version: 1,
    cell: (16, 16),          // PIXEL cell size — what the artist knows
    filter: Nearest,         // omitted defaults to Nearest (sheets are pixel art)
    clips: [
        ("idle", (frames: [0, 1, 2, 3], fps: 6.0, looping: true)),
        ("walk", (frames: [8, 9, 10, 11], fps: 10.0)),  // `looping` defaults true
    ],
)
```

```rust
// One call loads the PNG (with the sidecar's filter) and its clips.
let sheet = ctx.assets.load_sprite_sheet("sprites/deion_16.png")?;

let hero = ctx.world.create_entity();
ctx.world.add_component(&hero, Transform2D::new(Vec2::ZERO)).ok();
ctx.world.add_component(&hero, sheet.sprite()).ok();   // bound to the texture, first cell

let mut animation = sheet.animation();                  // grid + clips + sheet path
animation.play("walk");                                 // clip NAMES are the API
ctx.world.add_component(&hero, animation).ok();
```

Playback rules worth knowing:
- **`play(name)` always restarts** at frame 0 — it's a transition call. From
  code that re-asserts its clip every frame (a state machine), use
  **`ensure_playing(name)`**, which only restarts when the named clip isn't
  already the one playing. `resume()` continues a paused clip; `stop()`
  deselects it entirely.
- Both return `false` for an unknown clip name: a warned no-op that leaves the
  current clip running, never a panic.
- While a clip is selected and its frame resolves, `SpriteAnimation` **owns**
  `Sprite.tex_region` — `SpriteAnimationSystem` overwrites it every frame from
  `frame_tail.rs`, using the same time-scaled delta as particles, so a paused
  game (`ctx.time_scale = 0.0`) freezes animations for free.
- Authored sidecars fail loud at load: unknown version, zero cell size, empty
  `frames`, a non-finite/non-positive `fps`, or a frame index past the last
  cell is an error naming the file and the clip. `load_sprite_sheet` validates
  **before** loading the texture, so a bad sheet leaves no handle behind.

In scene RON the component stores its sheet path plus a baked snapshot; the
sidecar wins on load, so re-cutting a sheet propagates to every scene without
re-saving any of them:

```ron
SpriteAnimation(
    sheet: Some("sprites/deion_16.png"),
    grid: (cols: 4, rows: 2),        // baked fallback if the sidecar is missing
    clips: [("walk", (frames: [0, 1, 2, 3], fps: 8.0, looping: true))],
    autoplay: Some("walk"),          // written only for an animation that was playing
)
```

**Files:** `engine_core/sheet_file.rs` (schema SSOT), `engine_core/assets/sprite_sheet.rs`,
`ecs/sprite_components.rs`, `ecs/sprite_system.rs`, `engine_core/scene_data.rs` (`ClipData`/`GridData`)

### Generic Component Inspector Pattern
Display any Serialize component without hardcoding field display logic:

```rust
use editor::inspector::{inspect_component, InspectorStyle};

// Works with any component that implements Serialize
let style = InspectorStyle::default();

// Display component fields automatically
if let Some(transform) = world.get::<Transform2D>(entity) {
    y = inspect_component(ui, "Transform2D", transform, x, y, &style);
}
if let Some(sprite) = world.get::<Sprite>(entity) {
    y = inspect_component(ui, "Sprite", sprite, x, y, &style);
}

// Inspector automatically:
// - Extracts fields via JSON serialization (serde)
// - Handles nested objects (Vec2, Vec3, Vec4)
// - Displays arrays with count and inline formatting
// - Formats floats with 2 decimal precision
// - Recursively renders complex types with indentation
```

**Benefits:**
- Single implementation handles all component types
- New components automatically work in inspector
- No per-component display code needed

**Files:** `editor/src/inspector.rs`

### Scene Serialization Pattern
Load entire game levels from RON files:

```rust
// Scene definition (hello_world.scene.ron)
SceneData(
    name: "Hello World",
    physics: Some(PhysicsSettings(
        gravity: (0.0, -980.0),
        pixels_per_meter: 100.0,
    )),
    prefabs: {
        "Player": PrefabData(
            components: [
                Transform2D(position: (0.0, 0.0)),
                Sprite(texture: "#white", color: (0.2, 0.4, 1.0, 1.0)),
                RigidBody(body_type: Dynamic, linear_damping: 5.0, can_rotate: false),
            ],
        ),
    },
    entities: [
        EntityData(
            name: Some("player"),
            prefab: Some("Player"),
            overrides: [Transform2D(position: (-200.0, 100.0))],
        ),
    ],
)

// Load in code
let instance = SceneLoader::load_and_instantiate("scene.ron", &mut world, &mut assets)?;

// Spawn more copies of a prefab at runtime (Prototype pattern) — overrides
// replace matching component types, same semantics as scene-file overrides
let ball = instance.spawn_prefab(&mut world, &mut assets, "Player", &[])?;
```

**Files:** `scene_loader.rs`, `scene_data.rs`, `examples/assets/scenes/*.ron` (games keep theirs in `../games/<name>/assets/scenes/`)

### Physics Integration Pattern
ECS-friendly 2D physics via rapier2d. Game-facing movement API is a single
function: **`PhysicsSystem::set_velocity(entity, linear, angular)`** — use it
for every "launch / move this body" case. Safe on entities spawned the same
frame (velocity buffered, applied once the body syncs). The older
`apply_impulse` pass-through is gone; if a behavior genuinely needs mass-aware
impulse semantics, reach into `physics_world_mut().apply_impulse(...)`.

```rust
// Add physics components
world.add_component(&entity, RigidBody::player_platformer()).ok();
world.add_component(&entity, Collider::player_box(80.0, 80.0)).ok();

// Physics presets for common scenarios
RigidBody::player_platformer()        // Dynamic with damping
Collider::player_box(width, height)   // Player box with high friction
Collider::platform(width, height)     // Static ground/platform

// Collision events: after each update, drain the frame's events ONCE and
// share the Vec among all consumers (gameplay reactions, pickups, ...).
// The Vec is owned — no borrow of physics is held, so handlers can freely
// call set_velocity / destroy_entity while iterating.
physics_system.update(&mut world, delta_time);
let collisions = physics_system.take_collision_events();
for c in &collisions {
    if c.event.started { /* destroy brick, play sound, ... */ }
}
```

**Files:** `physics/lib.rs`, `physics/components.rs`, `physics/presets.rs`, `physics/physics_system/`

### Input Mapping Pattern
Action-based input bindings. `InputMapping<A>` is generic — **games define their
own action enums** and evaluate them against the engine's `InputHandler` device
state. A new mapping is empty; nothing is bound implicitly.

```rust
use input::{InputMapping, InputSource};
use winit::keyboard::KeyCode;

// Define actions (any Copy + Eq + Hash type)
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum MyAction {
    MoveUp,
    Jump,
    Shoot,
}

// Bind inputs to actions (own this mapping in your game struct)
let mut actions = InputMapping::new();
actions.bind(MyAction::MoveUp, InputSource::Keyboard(KeyCode::KeyW));
actions.bind(MyAction::MoveUp, InputSource::Keyboard(KeyCode::ArrowUp));
actions.bind(MyAction::Jump, InputSource::Keyboard(KeyCode::Space));

// Check actions against device state (instead of raw inputs)
if actions.is_active(MyAction::Jump, ctx.input) {
    // Player is holding jump
}
if actions.just_activated(MyAction::Jump, ctx.input) {
    // Jump became active this frame (strict edge: pressing a second
    // bound key while one is already held does NOT re-trigger)
}
```

The engine ships an optional `GameAction` preset
(`InputMapping::with_default_bindings()` — WASD/arrows movement, Space/Enter
actions, gamepad 0 equivalents) used by scene-defined behaviors like
`PlayerControlled`; rebind via `BehaviorRunner::actions_mut()`.

**Files:** `input/input_mapping.rs`, `input/input_handler.rs`

### Player-Aware Input Pattern (the universal layer — use this in games)
`GameContext.players` carries an engine-owned `InputSettings`: per-player
bindings over the fixed `GameAction` vocabulary with device routing
(default: P1 = WASD + Space + mouse + gamepad 0; P2 = arrows + Enter +
gamepad 1). Bindings persist to JSON via `GameConfig::with_input_settings_path`
(file is hand-editable; defaults written on first run).

```rust
use engine_core::prelude::*;

fn update(&mut self, ctx: &mut GameContext) {
    // Digital queries, per player (Action1 = primary: fire/serve/confirm)
    if ctx.players.just_activated(PlayerId::P2, GameAction::Action1, ctx.input) {
        // player 2 fired
    }
    // Shared actions: either player can pause/restart
    if ctx.players.just_activated_any(GameAction::Menu, ctx.input) { /* pause */ }

    // Merged movement in -1.0..=1.0 (+y = up): digital keys/dpad merged
    // with the analog stick, clamped — analog paddle/ship control for free
    let dx = ctx.players.move_x(PlayerId::P1, ctx.input);

    // Single-player convention: the lone player listens to BOTH slots so
    // WASD, arrows, and either pad all work
    let axis = (ctx.players.move_y(PlayerId::P1, ctx.input)
        + ctx.players.move_y(PlayerId::P2, ctx.input)).clamp(-1.0, 1.0);

    // Runtime pad re-assignment (press-A-to-join screens)
    ctx.players.assign_pad(PlayerId::P2, Some(3));
}
```

Rules of thumb: gameplay never reads raw `KeyCode`s (debug keys like F1 are
the exception); menus keep using `MenuInput::read(ctx.input)` (it already
scans every connected pad); the mouse belongs to P1 (raw `ctx.input` reads
for pointer takeover are P1-only).

**Files:** `input/player.rs`, `engine_core/input_settings_io.rs`, `engine_core/contexts.rs`

### Pause Pattern (universal — every game ships it)
The engine owns the mechanism (`PauseMenu`: Menu/Esc/Start toggles, MenuInput
navigation, Resume/Restart/Quit-to-Title/Exit-Game); games own the meaning.
Title-screen Exit items work the same way: set `ctx.exit_requested = true`
and the engine performs the same clean shutdown as closing the window. Gate your
entire gameplay update on it, in pausable states only (GameOver keeps its
direct Menu→title exit):

```rust
// Top of the gameplay update, BEFORE physics/timers (pausable states only):
if matches!(self.state, GameState::Playing) {
    let action = self.pause.update(ctx.players, ctx.input, ctx.window_size);
    ctx.time_scale = self.pause.time_scale();   // freezes engine particles
    match action {
        PauseAction::Restart => { self.start_game(ctx); return; }
        PauseAction::QuitToTitle => { self.reset_to_title(ctx.world); return; }
        PauseAction::ExitGame => { ctx.exit_requested = true; return; }
        PauseAction::Resumed => return, // resume takes effect NEXT frame —
                                        // the keypress can't leak into gameplay
        PauseAction::Idle => {}
    }
    if self.pause.is_active() { return; } // frozen: no physics/input/logic
}

// End of the gameplay UI pass (frozen world stays visible underneath):
if self.pause.is_active() {
    self.pause.draw(ctx.ui, ctx.window_size, &self.menu_style());
}
```

**Files:** `engine_core/pause.rs` (mechanism + tests), any game's `gameplay/mod.rs` + `drawing.rs`/`ui.rs` (integration)

### Menu Chrome Pattern (windows, not floating text)
Menu screens draw inside a `MenuPanel`: opaque theme-tinted window with a
border, accent separator, corner ticks, ▶-cursor highlight rows, and a hint
footer. Style derives from the game's chaos theme so each game keeps its
identity:

```rust
fn menu_style(&self) -> MenuStyle { MenuStyle::from_theme(&self.current_theme()) }

fn draw_title(&self, ctx: &mut GameContext, selection: u8) {
    let style = self.menu_style();
    let panel = MenuPanel::new("MY GAME", ctx.window_size / 2.0, 360.0, 3);
    let mut y = panel.begin(ctx.ui, &style);
    for (i, item) in ["1 Player", "2 Player", "Achievements"].iter().enumerate() {
        y = panel.item(ctx.ui, y, item, i as u8 == selection, &style);
    }
    panel.hint(ctx.ui, "W/S or D-Pad navigate - SPACE/ENTER or click confirm", &style);
}
```
`item_colored` keeps caller-chosen row colors (breakout's level roster);
`line` adds non-selectable rows (game-over panels); `draw_as_overlay` wraps
the window in an input-blocking dimmed overlay (how the pause menu renders).

**Menus are mouse-clickable (Aug 2026).** In the input half of a menu screen,
build the SAME panel geometry the drawing half renders (title text doesn't
matter, center/width/rows do) and read `panel.mouse_select(ctx.input)`:
`hovered` (reported only on frames the mouse moved, so a resting cursor never
fights keyboard navigation) overwrites the selection; `clicked` selects that
row AND confirms it — treat it exactly like `MenuInput.confirm`. `PauseMenu`
does this internally, which is why `pause.update` takes `ctx.window_size`.

```rust
let input = MenuInput::read(ctx.input);          // Space/Enter/NumpadEnter, pads
let mouse = panel.mouse_select(ctx.input);       // hover + click
if let Some(row) = mouse.hovered { self.selection = row; }
self.selection = input.navigate(self.selection, 3);
if input.confirm || mouse.clicked.is_some() {
    if let Some(row) = mouse.clicked { self.selection = row; }
    // enter the selected item
}
```

**Files:** `engine_core/menu_panel.rs`, examples in every game's `drawing.rs`/`ui.rs`

### Chaos Mode Pattern (Cross-Game Theme)
Every game built on the engine is expected to support a four-tier intensity
theme: **Normal / Insane / Ridiculous / Insiculous** (= both insane and
ridiculous). The engine carries the *selection*; each game decides what the
variants *mean*.

```rust
use engine_core::prelude::*;

// Startup: set once via the config...
let config = GameConfig::new("My Game").with_chaos_mode(ChaosMode::Insane);

// ...or let the player pick at runtime and store on your own struct.
// Branch gameplay with the helper predicates:
if self.chaos_mode.is_insane()     { /* per-game "insane" behavior */ }
if self.chaos_mode.is_ridiculous() { /* per-game "ridiculous" behavior */ }

// Iterate all four variants for menus:
for mode in ChaosMode::ALL { println!("{}", mode.label()); }
```

`is_insane()` / `is_ridiculous()` both return true for Insiculous, so wiring
both flags independently gives you "both behaviors at once" for Insiculous
mode for free.

For the *look* of each mode, `ChaosTheme::for_mode(mode)` (prelude) provides
the shared per-mode palette (bg/structure/accent/grid colors, HUD banner,
particle-count multiplier); games override individual fields with
struct-update syntax where their art differs.

**Files:** `crates/engine_core/src/chaos_mode.rs`, `game_config.rs`, `contexts.rs`

### Resource Acquisition Is Initialization (RAII)
Used throughout for resource management:

```rust
// WebGPU resources managed via Arc
let texture: Arc<Texture> = device.create_texture(&desc);
let view: Arc<TextureView> = texture.create_view(&Default::default());

// Auto-cleanup when last Arc dropped
drop(view); // Texture still alive if others hold Arc
drop(texture); // Now fully cleaned up
```

**Files:** `renderer/renderer.rs`, `renderer/texture.rs`

### Component Registry Pattern
Unified component definition with metadata for scene serialization and editor inspection:

```rust
// Option 1: Derive macro (preferred for new components)
use ecs::DeriveComponentMeta;
use ecs::component_registry::ComponentMeta;

#[derive(Debug, Clone, Serialize, Deserialize, DeriveComponentMeta)]
pub struct Health {
    pub value: f32,
    pub max: f32,
}

// Option 2: define_component! macro (includes Default impl)
define_component! {
    pub struct Stamina {
        pub value: f32 = 100.0,
        pub max: f32 = 100.0,
    }
}

// ComponentMeta trait provides runtime type info
assert_eq!(Health::type_name(), "Health");
assert_eq!(Health::field_names(), &["value", "max"]);

// Global registry (built-ins registered at first access). GAMES REGISTER
// THEIR OWN COMPONENTS in main(), before run_game — one line buys scene
// save/load, editor snapshot/clipboard survival, the Add Component popup's
// "Game" section, and command-API set/add/remove/describe (issue #43):
ecs::register_components(|r| r.register::<Health>());
// (register_transient::<T>() = editable but never written to scene files.
// Rules: T needs ComponentMeta + Serialize + Deserialize + Default; never
// store raw EntityId fields — ids aren't stable across save/load; and the
// standalone editor binary can't see game types — edit game scenes from the
// game's own binary via run_game_with_editor.)

ecs::with_global_registry(|registry| {
    assert!(registry.is_registered("Transform2D"));
    assert!(registry.is_registered("Health"));
});

// Create/attach components dynamically by name from JSON
let json = serde_json::json!({"value": 50.0, "max": 100.0});
ecs::with_global_registry(|r| r.insert_component(&mut world, entity, "Health", json))?;
```

**Built-in Components with ComponentMeta:**
- `Transform2D` - position, rotation, scale
- `Sprite` - texture_handle, offset, rotation, scale, color, depth, tex_region
- `Camera` - position, rotation, zoom, viewport_size, is_main_camera, near, far
- `SpriteAnimation` - fps, frames, playing, loop_animation, current_frame, time_accumulator

**Files:** `ecs/src/component_registry.rs`, `ecs/src/sprite_components.rs`, `ecs_macros/src/lib.rs`

## Current Known Limitations

Open technical debt lives on the **Studio Board** as issues with the
`tech-debt` label (`gh issue list -R beinsiculous/insiculous_2d --label
tech-debt`; games debt on the game's own repo). Resolved history (incl. the
closed Jul 2026 Game Programming Patterns audit): `log_archive.md`. Do not
list debt here — this file documents APIs and patterns, not status.

---

## Quick Reference

### File Structure
```
/home/jedi/RustroverProjects/insiculous_2d/
├── CLAUDE.md                   # Main agent ruleset (AGENTS.md is a symlink to it)
├── training.md                 # This file - AI pair programming guide
├── PROJECT_ROADMAP.md          # Vision + settled decisions (open tasks = Studio Board issues)
├── log_archive.md              # Completed/resolved work (history)
├── README.md                   # Project overview
├── crates/
│   ├── engine_core/           # Core engine (Game API, managers, scene pipeline)
│   ├── renderer/              # WGPU rendering (sprites, batching, bloom)
│   ├── ecs/                   # Entity Component System
│   ├── ecs_macros/            # #[derive(ComponentMeta)]
│   ├── input/                 # Input handling + generic action mapping
│   ├── ui/                    # Immediate-mode UI framework
│   ├── physics/               # Rapier2d physics integration
│   ├── audio/                 # Audio playback via rodio
│   ├── common/                # Shared types (Rect, Color, Time, CameraUniform)
│   ├── editor/                # Editor panels, inspector, gizmos, undo/redo
│   └── editor_integration/    # EditorGame<G> wrapper, run_game_with_editor()
├── examples/                  # hello_world, editor_demo, behavior_demo
└── ../games/                  # Sibling dir: one cargo project per game (pong, breakout, space_invaders, snake, asteroids)
```

### Key Commands
```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p engine_core
cargo test -p renderer
cargo test -p ecs

# Run demos
cargo run --example hello_world
cargo run --example editor_demo --features editor

# Check for TODOs (should be 0)
grep -r "TODO:" crates/ --include="*.rs" | wc -l

# Run with output
cargo test -- --nocapture
```

### Test Status Summary
Do not trust hardcoded test counts in docs — run `cargo test --workspace` for the
truth. The invariant that matters: **0 failed, 0 ignored** (GPU/window-bound doc
examples are `no_run`, never `ignore`). Current count lives in `CLAUDE.md`.

For more game programming patterns, refer to the [Game Programming Patterns](https://gameprogrammingpatterns.com/) book.

## Example Docstrings

### Crate-Level Documentation
```rust
//! Core functionality for the insiculous_2d game engine.
//! 
//! This crate provides the game loop, timing, and world state management.
```

### Struct Documentation
```rust
/// Manages game loop timing and frame delta calculations
pub struct GameLoopManager {
    last_frame_time: Instant,
    delta_time: f32,
    frame_count: u64,
    total_time: f32,
    /// Minimum duration of a frame, derived from the target FPS.
    /// `None` means uncapped (rely on vsync / present mode).
    min_frame_time: Option<Duration>,
}
```

### Method Documentation
```rust
/// Cap the frame rate at `fps`. Pass 0 to uncap (vsync still applies).
pub fn set_target_fps(&mut self, fps: u32) {
    self.min_frame_time =
        (fps > 0).then(|| Duration::from_secs_f64(1.0 / fps as f64));
}
```

### Error Type Documentation
```rust
/// Errors that can occur in the engine core
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Failed to initialize engine: {0}")]
    InitializationError(String),

    #[error("Game loop error: {0}")]
    GameLoopError(String),
}
```

## Prompt Recipes

### Adding a New System

```
I want to add a new particle system to the engine. It should:
1. Be a component that can be attached to entities
2. Support configurable emission rate, particle lifetime, and velocity
3. Integrate with the renderer to draw particles efficiently
4. Follow the existing patterns in the codebase

Please help me design and implement this system.
```

### Refactoring Existing Code

```
I need to refactor the input system to support input mapping configurations. Users should be able to:
1. Define custom key/button mappings for actions
2. Save and load these configurations
3. Query input by action name rather than specific keys/buttons

Please help me refactor the input_handler.rs file while maintaining backward compatibility.
```

### Generating Documentation

```
Please generate comprehensive documentation for the ECS world implementation. Include:
1. Overview of the world's responsibility
2. Explanation of how entities, components, and systems interact
3. Code examples showing common usage patterns
4. Performance considerations and best practices
```

### Writing Tests

A failing test must mean a **player- or author-visible contract** broke, not
that a field assignment still assigns. Name tests after the behavior
(`test_entity_despawns_when_lifetime_crosses_zero`), not the method
(`test_new`). Models: `crates/ecs/src/lifetime.rs`, `crates/ecs/src/sprite_system.rs`.

A test **stays** if a game author or player would notice the failure:

- Lifecycle / state machines (play→pause→stop, just-pressed clearing, lifetime despawn)
- Cross-component wiring (`SpriteAnimationSystem` writes `Sprite.tex_region`)
- Persistence (legacy JSON/RON still loads; texture-filter aliases)
- Non-obvious math (capsule half-height, UV cells, camera bounds, attenuation)
- Typed error paths (`AudioError::IoError` vs `DecodeError`)
- GPU layout sizes the shader assumes
- Derive-macro output (`type_name` / `field_names`)

Do **not** add tests that only:

- Echo a constructor/builder field (`with_x(v); assert_eq!(x, v)`)
- Assert `Default` / `label()` / `type_name()` with no downstream contract
- Construct and return (no assert, or “doesn’t panic” with no state check)
- Reimplement production `if` logic inside the test
- Duplicate another crate’s tests of the same type
- Check `is_nan` / `is_finite` instead of the named behavior

### Implementing a New Feature

```
I want to implement a scene graph system that:
1. Allows entities to have parent-child relationships
2. Propagates transformations down the hierarchy
3. Supports efficient querying of entities by their position in the graph
4. Integrates with the existing ECS architecture

Please help me design and implement this feature.
```
