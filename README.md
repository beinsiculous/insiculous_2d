# Insiculous 2D

A lightweight, modular 2D game engine for Rust. Focus on game logic, not boilerplate.

## Quick Start

```bash
git clone https://github.com/yourusername/insiculous_2d.git
cd insiculous_2d
cargo run --example hello_world                      # physics platformer demo
cargo run --example editor_demo --features editor    # same demo inside the visual editor
```

## Create Your First Game

```rust
use engine_core::prelude::*;

struct MyGame {
    player: Option<EntityId>,
}

impl Game for MyGame {
    fn init(&mut self, ctx: &mut GameContext) {
        // Create a player entity with position and sprite
        let player = ctx.world.create_entity();
        ctx.world.add_component(&player, Transform2D::new(Vec2::new(0.0, 0.0))).ok();
        ctx.world.add_component(&player, Sprite::new(0).with_color(Vec4::new(0.2, 0.4, 1.0, 1.0))).ok();
        self.player = Some(player);
    }

    fn update(&mut self, ctx: &mut GameContext) {
        // Move player with WASD
        if let Some(player) = self.player {
            if let Some(transform) = ctx.world.get_mut::<Transform2D>(player) {
                let speed = 200.0 * ctx.delta_time;
                if ctx.input.is_key_pressed(KeyCode::KeyW) { transform.position.y += speed; }
                if ctx.input.is_key_pressed(KeyCode::KeyS) { transform.position.y -= speed; }
                if ctx.input.is_key_pressed(KeyCode::KeyA) { transform.position.x -= speed; }
                if ctx.input.is_key_pressed(KeyCode::KeyD) { transform.position.x += speed; }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_game(MyGame { player: None }, GameConfig::new("My Game"))
}
```

That's it! The engine handles window creation, event loops, rendering, and input automatically.

## Game API Reference

| Type | Purpose |
|------|---------|
| `Game` | Trait to implement - `init()`, `update()`, optionally `render()` |
| `GameConfig` | Window settings: title, size, clear color, FPS, asset paths, chaos mode |
| `GameContext` | Access `input`, `world` (ECS), `assets`, `audio`, `ui`, `achievements`, `chaos_mode`, `delta_time`, `window_size` |
| `run_game()` | Single function to start everything |
| `run_game_with_editor()` | Same game, wrapped in the visual editor (see below) |

### GameConfig Options

```rust
let config = GameConfig::new("My Game")
    .with_size(1280, 720)                   // Window dimensions
    .with_clear_color(0.1, 0.1, 0.2, 1.0)   // Background color
    .with_fps(60)                           // Target frame rate
    .with_asset_base_path("assets")         // Resolve relative asset paths (cwd-independent)
    .with_chaos_mode(ChaosMode::Insane);    // Normal / Insane / Ridiculous / Insiculous
```

### Game Trait Methods

```rust
impl Game for MyGame {
    // Required: called every frame
    fn update(&mut self, ctx: &mut GameContext) { }

    // Optional: called once at startup
    fn init(&mut self, ctx: &mut GameContext) { }

    // Optional: custom sprite rendering (default extracts from ECS)
    fn render(&mut self, ctx: &mut RenderContext) { }

    // Optional: key event callbacks
    fn on_key_pressed(&mut self, key: KeyCode, ctx: &mut GameContext) { }
    fn on_key_released(&mut self, key: KeyCode, ctx: &mut GameContext) { }

    // Optional: window events
    fn on_resize(&mut self, width: u32, height: u32) { }
    fn on_exit(&mut self) { }
}
```

## Visual Editor

The engine ships a dockable scene editor that wraps **any** `Game` without
changing its code: hierarchy panel, inspector with editable components and
undo/redo, transform gizmos, play/pause/stop with world snapshot restore,
scene save/load (RON), grid, and physics collider visualization.

```bash
cargo run --example editor_demo --features editor
```

### Run Your Own Game Inside the Editor

`run_game_with_editor(game, config)` is a drop-in replacement for
`run_game(game, config)`. Gate it behind a cargo feature so your shipping
binary stays editor-free:

```toml
# Cargo.toml of your game crate
[features]
editor = ["dep:editor_integration"]

[dependencies]
engine_core = { path = "../insiculous_2d/crates/engine_core" }
editor_integration = { path = "../insiculous_2d/crates/editor_integration", optional = true }
```

```rust
fn main() {
    let config = GameConfig::new("My Game");

    #[cfg(feature = "editor")]
    editor_integration::run_game_with_editor(MyGame::default(), config).unwrap();
    #[cfg(not(feature = "editor"))]
    run_game(MyGame::default(), config).unwrap();
}
```

```bash
cargo run                     # plain game
cargo run --features editor   # the same game inside the editor
```

The Pong game (`games/pong`, sibling to this repo) is wired exactly this way —
run `cargo run --features editor` from its directory to open it in the editor.

### Editor Controls

| Keys | Action |
|------|--------|
| `Q` / `W` / `E` / `R` | Select / Move / Rotate / Scale tool |
| `F5`, `Ctrl+P` | Play (`Ctrl+P` also toggles pause while playing) |
| `Ctrl+Shift+P` | Stop — restores the world to its pre-play snapshot |
| `Ctrl+Z` / `Ctrl+Y` | Undo / Redo |
| `Ctrl+D`, `Del` | Duplicate / delete selected entities |
| `Ctrl+S` / `Ctrl+Shift+S` | Save / Save As scene (RON) |
| `Ctrl+O` / `Ctrl+N` | Open / New scene |
| `G` | Toggle grid |
| `C` | Toggle collider outlines |
| `+` / `-` / `0` | Zoom in / out / reset camera |

### Collider Visualization

The scene view overlays every `Collider` with its outline, exactly as the
physics simulation places it: green for solid colliders, cyan for sensors,
yellow for the selected entity. Collider sizes are **absolute pixels** —
physics ignores `Transform2D.scale` — so if a sprite is sized via scale, the
overlay instantly shows any sprite-vs-collider mismatch. Shape dimensions
(box half-extents, circle radius, capsule height/radius) and offset are
editable in the inspector with full undo support.

## Architecture

```
insiculous_2d/
├── crates/
│   ├── engine_core/        # Game trait, lifecycle managers, scenes, behaviors, achievements
│   ├── renderer/           # WGPU 28.0 sprite rendering with instancing and batching
│   ├── ecs/                # Entity Component System (HashMap-based per-type storage)
│   ├── ecs_macros/         # #[derive(ComponentMeta)] for editor/scene-aware components
│   ├── input/              # Keyboard, mouse, gamepad with generic action mapping
│   ├── physics/            # rapier2d 2D physics integration with presets
│   ├── audio/              # Rodio-based sound and music playback
│   ├── ui/                 # Immediate-mode UI with fontdue font rendering
│   ├── common/             # Shared types (colors, transforms, math utilities)
│   ├── editor/             # Editor panels, inspector, gizmos, undo/redo, theme
│   └── editor_integration/ # run_game_with_editor() — wires the editor to a running game
└── examples/               # Demo applications
```

## Advanced Usage

For more control, you can use the lower-level APIs directly.

### Direct ECS Access

```rust
// Create entities with components
let entity = world.create_entity();
world.add_component(&entity, Transform2D::new(Vec2::new(100.0, 200.0)))?;
world.add_component(&entity, Sprite::new(0).with_color(Vec4::ONE))?;

// Query components
if let Some(transform) = world.get_mut::<Transform2D>(entity) {
    transform.position.x += 10.0;
}

// Iterate all entities
for entity_id in world.entities() {
    if let Some(sprite) = world.get::<Sprite>(entity_id) {
        // Process sprite...
    }
}
```

### Physics Integration

```rust
use engine_core::prelude::*;

// Create physics entity
let player = world.create_entity();
world.add_component(&player, Transform2D::new(Vec2::ZERO))?;
world.add_component(&player, RigidBody::player_platformer())?;
world.add_component(&player, Collider::player_box(80.0, 80.0))?;
world.add_component(&player, Sprite::new(0).with_color(Vec4::new(0.2, 0.4, 1.0, 1.0)))?;

// Physics system handles movement, gravity, collisions.
// Collider sizes are absolute pixels; Transform2D.scale only affects sprites.
```

### Audio System

```rust
// In init()
self.jump_sound = ctx.audio.load_sound("assets/jump.wav").ok();

// In update() - play sound
if ctx.input.is_key_just_pressed(KeyCode::Space) {
    if let Some(sound) = &self.jump_sound {
        ctx.audio.play_with_settings(sound, SoundSettings::default());
    }
}

// Background music
ctx.audio.play_music("assets/music.ogg")?;
ctx.audio.set_volume(0.5);
```

### UI System

```rust
fn update(&mut self, ctx: &mut GameContext) {
    // Panel background
    ctx.ui.panel(Rect::new(10.0, 10.0, 200.0, 150.0));

    // Button
    if ctx.ui.button("play_btn", "Play", Rect::new(20.0, 30.0, 180.0, 40.0)) {
        self.start_game();
    }

    // Slider
    self.volume = ctx.ui.slider("volume_slider", self.volume, Rect::new(20.0, 90.0, 180.0, 20.0));

    // Label
    ctx.ui.label(&format!("Volume: {:.0}%", self.volume * 100.0), Vec2::new(20.0, 120.0));
}
```

### Scene Files (RON)

Load entire levels from scene files:

```rust
use engine_core::SceneLoader;
use std::path::Path;

fn init(&mut self, ctx: &mut GameContext) {
    let scene_path = Path::new("assets/scenes/level1.scene.ron");
    match SceneLoader::load_and_instantiate(scene_path, &mut ctx.world, ctx.assets) {
        Ok(instance) => {
            println!("Loaded scene '{}' with {} entities", instance.name, instance.entity_count);
            // Access named entities
            if let Some(player) = instance.get_entity("player") {
                // Work with player entity...
            }
        }
        Err(e) => eprintln!("Failed to load scene: {}", e),
    }
}
```

Example scene file format:
```ron
SceneData(
    name: "Level 1",
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
```

Scenes edited in the visual editor are saved back to this same format
(`Ctrl+S`).

### Entity Behaviors

Attach behaviors to entities for common game logic:

```rust
use ecs::behavior::Behavior;

// Player-controlled platformer (WASD + Space to jump)
world.add_component(&player, Behavior::PlayerPlatformer {
    move_speed: 120.0,
    jump_impulse: 420.0,
    jump_cooldown: 0.3,
    tag: "player".to_string(),
})?;

// AI that follows the player
world.add_component(&enemy, Behavior::ChaseTagged {
    target_tag: "player".to_string(),
    detection_range: 200.0,
    chase_speed: 80.0,
    lose_interest_range: 300.0,
})?;

// Process behaviors in update()
self.behaviors.update(&mut ctx.world, ctx.input, ctx.delta_time, self.physics.as_mut());
```

Available behaviors:
- `PlayerPlatformer` - WASD movement with jumping
- `PlayerTopDown` - WASD movement in all directions
- `ChaseTagged` - Chase entities with a specific tag
- `FollowEntity` - Follow a named entity
- `FollowTagged` - Follow the nearest entity with a tag
- `Patrol` - Patrol between two points
- `Collectible` - Items that can be collected

### Hierarchy/Scene Graph

Create parent-child relationships between entities:

```rust
use ecs::WorldHierarchyExt;

// Set parent-child relationship
world.set_parent(child_entity, parent_entity)?;

// Get children of an entity
let children = world.get_children(parent_entity);

// Get all descendants (children, grandchildren, etc.)
let descendants = world.get_descendants(root_entity);

// Get all root entities (no parent)
let roots = world.get_root_entities();

// Check for cycles (prevented automatically)
```

Child transforms are automatically updated when parent transforms change.
Note: entities with a `RigidBody` must stay at the root — physics treats
their transform as world-space.

### Custom Rendering

Override `render()` to control sprite rendering:

```rust
fn render(&mut self, ctx: &mut RenderContext) {
    // Add sprites manually
    let sprite = renderer::Sprite::new(TextureHandle { id: 0 })
        .with_position(Vec2::new(100.0, 100.0))
        .with_scale(Vec2::new(64.0, 64.0))
        .with_color(Vec4::new(1.0, 0.0, 0.0, 1.0));

    ctx.sprites.add_sprite(&sprite);

    // Or extract from specific entities
    if let Some(transform) = ctx.world.get::<Transform2D>(self.player) {
        // Custom sprite logic...
    }
}
```

### Input System

```rust
// In update()
if ctx.input.is_key_pressed(KeyCode::Space) {
    // Key is held down
}
if ctx.input.is_key_just_pressed(KeyCode::Enter) {
    // Key was just pressed this frame
}

// Mouse position
let mouse_pos = ctx.input.mouse_position();

// Action-based input: define your own actions, own the mapping in your game
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum MyAction { MoveLeft, Fire }

let mut actions = InputMapping::new();
actions.bind(MyAction::MoveLeft, InputSource::Keyboard(KeyCode::KeyA));
actions.bind(MyAction::Fire, InputSource::Mouse(MouseButton::Left));

if actions.is_active(MyAction::MoveLeft, ctx.input) {
    // Move left
}
if actions.just_activated(MyAction::Fire, ctx.input) {
    // Fire triggered this frame
}
```

### Chaos Mode

Every game built on the engine can support the four-tier intensity theme:
**Normal / Insane / Ridiculous / Insiculous** (= both). The engine carries
the selection; each game decides what the variants mean.

```rust
let config = GameConfig::new("My Game").with_chaos_mode(ChaosMode::Insane);

// In game logic — both predicates return true for Insiculous:
if ctx.chaos_mode.is_insane()     { /* e.g. double speed */ }
if ctx.chaos_mode.is_ridiculous() { /* e.g. spawn extras */ }
```

## Test Summary

| Crate | Tests | Coverage |
|-------|-------|----------|
| ECS | 174 | Entity lifecycle, components, queries, hierarchy, systems |
| Editor | 241 | Panels, inspector, gizmos, undo/redo, collider overlay, theme |
| Engine Core | 161 | Game API, lifecycle, scenes, behaviors, achievements, timing |
| Renderer | 70 | Sprite batching, instancing, camera, textures |
| UI | 70 | Widgets, interaction, draw commands, font rendering |
| Editor Integration | 64 | Editor↔game wiring, play/pause, scene save/load |
| Input | 62 | Key states, action mapping, events |
| Physics | 61 | Body types, colliders, simulation, presets |
| Common / Audio / Macros | 52 | Math, transforms, sound playback, derive macros |
| **Total** | **955** | **All passing, 0 ignored** |

Run all tests: `cargo test --workspace` — everything runs headless, no GPU
or window required.

## Project Status

**Current State:** Functional editor (Phase 1 complete); work follows the UX-audit sprint order — see `PROJECT_ROADMAP.md` § "Editor — UX Audit & Work Order" and `docs/EDITOR_UX_AUDIT.md`

- **Test Status:** `cargo test --workspace` — 0 failed, 0 ignored is the invariant
- **Lint Status:** `cargo clippy --workspace --all-targets` clean
- **Editor:** entity CRUD, component management, undo/redo, play/pause/stop, scene save/load, collider visualization
- **Architecture:** Manager pattern, SRP-refactored core, all files under 600 lines

See [PROJECT_ROADMAP.md](PROJECT_ROADMAP.md) for detailed technical debt tracking and priorities.

## Examples

- **hello_world** - Physics platformer with UI, audio, ECS, scene files, and behaviors
- **behavior_demo** - Demonstrates all built-in entity behaviors
- **editor_demo** - The hello_world platformer running inside the visual editor

Run an example:
```bash
cargo run --example hello_world
cargo run --example behavior_demo
cargo run --example editor_demo --features editor
```

## Design Patterns Used

This codebase implements patterns from [Robert Nystrom's Game Programming Patterns](https://gameprogrammingpatterns.com/):

| Pattern | Implementation |
|---------|---------------|
| **Component** | ECS with entity-component storage |
| **Update Method** | Single game loop with centralized update |
| **Command** | Editor undo/redo, input events, UI draw commands as data |
| **Manager Pattern** | GameLoopManager, RenderManager, WindowManager |

## Requirements

- Recent stable Rust toolchain
- Cargo
- GPU with Vulkan, Metal, or DX12 support (games/editor only — tests run headless)

## License

See LICENSE file for details.
