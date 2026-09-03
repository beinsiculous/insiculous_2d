//! Behavior Demo - Showcases the modular behavior system
//!
//! This example demonstrates all the built-in behaviors:
//! - PlayerTopDown: WASD/Arrow key movement in all directions
//! - ChasePlayer: Red enemies chase you when you get close
//! - Patrol: Orange guards walk back and forth
//! - FollowEntity: Green companion follows you around
//! - Collectible: Yellow coins (visual only for now)
//!
//! Controls: WASD/Arrow keys to move, ESC to exit
//!
//! Run with: cargo run --example behavior_demo

use engine_core::prelude::*;
use std::path::Path;

/// Anchor asset paths to the repository so the example runs from any
/// working directory.
const EXAMPLES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");

struct BehaviorDemo {
    behaviors: BehaviorRunner,
    physics: Option<PhysicsSystem>,
    scene_instance: Option<SceneInstance>,
}

impl BehaviorDemo {
    fn new() -> Self {
        Self {
            behaviors: BehaviorRunner::new(),
            physics: None,
            scene_instance: None,
        }
    }
}

impl Game for BehaviorDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        let scene_path = Path::new(EXAMPLES_DIR).join("assets/scenes/behavior_demo.scene.ron");

        match SceneLoader::load_and_instantiate(&scene_path, ctx.world, ctx.assets) {
            Ok(instance) => {
                println!("=== Behavior Demo ===");
                println!("Loaded scene '{}' with {} entities", instance.name, instance.entity_count);
                println!();
                println!("Behaviors demonstrated:");
                println!("  [BLUE]   Player    - You! Move with WASD/Arrow keys");
                println!("  [GREEN]  Companion - Follows you around");
                println!("  [RED]    Chasers   - Chase you when you get close");
                println!("  [ORANGE] Guards    - Patrol back and forth");
                println!("  [YELLOW] Coins     - Collectibles");
                println!();
                println!("Controls: WASD/Arrow keys to move, ESC to exit");

                // Create physics system from scene settings (zero gravity for top-down)
                if let Some(settings) = &instance.physics {
                    let config = PhysicsConfig::new(Vec2::new(settings.gravity.0, settings.gravity.1))
                        .with_scale(settings.pixels_per_meter);
                    self.physics = Some(PhysicsSystem::with_config(config));
                }

                // Set named entities for FollowEntity behavior
                self.behaviors.set_named_entities(instance.named_entities.clone());
                self.scene_instance = Some(instance);
            }
            Err(e) => {
                println!("Failed to load scene: {}", e);
                println!("Creating fallback player...");

                // Minimal fallback - just a player
                let player = ctx.world.create_entity();
                ctx.world.add_component(&player, Transform2D::new(Vec2::ZERO)).ok();
                ctx.world.add_component(&player, Sprite::new(0).with_color(Vec4::new(0.2, 0.6, 1.0, 1.0))).ok();
                ctx.world.add_component(&player, Behavior::PlayerTopDown { move_speed: 150.0, tag: "player".to_string() }).ok();
            }
        }

        // Initialize physics system
        if let Some(physics) = &mut self.physics {
            physics.initialize(ctx.world).ok();
        }
    }

    fn update(&mut self, ctx: &mut GameContext) {
        // Process all behaviors - sets velocities for physics entities
        self.behaviors.update(
            ctx.world,
            ctx.input,
            ctx.delta_time,
            self.physics.as_mut(),
        );

        // Step physics simulation (handles collision detection)
        if let Some(physics) = &mut self.physics {
            physics.update(ctx.world, ctx.delta_time);
        }
    }

    // render() uses default implementation
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = GameConfig::new("Behavior Demo - Insiculous 2D")
        .with_size(800, 600)
        .with_clear_color(0.1, 0.1, 0.12, 1.0)
        .with_asset_base_path(EXAMPLES_DIR);

    run_game(BehaviorDemo::new(), config)?;
    Ok(())
}
