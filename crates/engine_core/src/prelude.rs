//! Prelude re-exporting the most commonly used engine_core types.

pub use crate::{
    // Simple game API (recommended)
    game::{Game, run_game},
    contexts::{GameContext, RenderContext, GlyphCacheKey},
    game_config::GameConfig,
    chaos_mode::ChaosMode,
    chaos_theme::ChaosTheme,
    achievements::{Achievement, AchievementManager, AchievementError},
    scores::{ScoreEntry, Scores, ScoresError, MAX_SCORES_PER_MODE},
    // Asset management
    assets::{AssetManager, AssetConfig, AssetError},
    // Sprite sheets (`load_sprite_sheet` + the `.sheet.ron` schema)
    assets::sprite_sheet::SpriteSheet,
    sheet_file::SheetFile,
    // Scene serialization
    scene_data::{SceneData, PhysicsSettings, PrefabData, EntityData, ComponentData, BehaviorData, SceneLoadError},
    scene_loader::{SceneLoader, SceneInstance},
    // Behavior system
    behavior_runner::{BehaviorRunner, EntityCollected},
    // Particle system (CPU pool; spawn bursts or attach a ParticleEmitter)
    particles::{Particle, ParticleConfig, ParticleEmitter, ParticleManager, ParticleSystem},
    // Spring-mass grid (Geometry-Wars style deforming background)
    grid::{
        default_playfield_grid, request_backdrop_reset, ripple, step_and_emit_grid, GridImpulse,
        GridMesh,
    },
    // Localization
    localization::{LocaleFile, Strings},
    // Data-driven UI elements
    ui_element_system::{UiButtonPressed, UiElementsHidden},
    // Shared arcade scaffolding
    menu_input::MenuInput,
    menu_panel::{MenuMouse, MenuPanel, MenuStyle},
    pause::{PauseAction, PauseMenu, PauseMenuLabels},
    spawn_helpers::spawn_background,
    RENDER_UNIT,
    // Debug-draw helpers (collider outlines, etc.)
    debug,
    init,
    scene::Scene,
    EngineError,
};

// Re-export common types (Color, Transform2D, Camera2D, Rect)
pub use common::{Color, Transform2D, Camera, Rect};
pub use common::{hash_f32, hash_u32};

// Re-export commonly used types from dependencies
pub use glam::{Vec2, Vec4};

// Re-export ECS types
pub use ecs::{EntityId, World};
pub use ecs::sprite_components::{
    set_sprites_visible, AnimationClip, Name, SheetGrid, Sprite, SpriteAnimation,
};
pub use ecs::Tilemap;
pub use ecs::audio_components::{AudioSource, AudioListener, PlaySoundEffect};
pub use ecs::hierarchy_system::TransformHierarchySystem;
pub use ecs::lifetime::{Lifetime, LifetimeSystem};
pub use ecs::WorldHierarchyExt;
pub use ecs::System;
pub use ecs::behavior::{Behavior, EntityTag};
pub use ecs::{StateMachine, HierarchicalStateMachine, EventBus, ResourceStorage};

// Re-export input types (KeyCode/MouseButton re-exported through input crate, not directly from winit)
pub use input::prelude::{
    AxisDirection, GameAction, GamepadAxis, GamepadButton, InputSettings, KeyCode, MouseButton,
    PlayerId,
};

// Re-export renderer types
pub use renderer::{TextureFilter, TextureHandle, TextureLoadConfig};
pub use renderer::line_pipeline::LineVertex;

// Re-export audio types
pub use audio::{AudioManager, SoundHandle, SoundSettings};

// Re-export UI types (UIRect and UIColor are aliases to common types for backwards compatibility)
pub use ui::{UIContext, Theme as UITheme, WidgetId};
pub use common::Rect as UIRect;
pub use common::Color as UIColor;

// Re-export physics types when the physics feature is enabled
#[cfg(feature = "physics")]
pub use physics::{
    Collider, ColliderShape, CollisionData, CollisionEvent, ContactPoint,
    PhysicsConfig, PhysicsSystem, RigidBody, RigidBodyType,
};

// Generic pickup/collectible tracking (built on physics collision events)
#[cfg(feature = "physics")]
pub use crate::pickups::{EffectTimer, Pickup, Pickups};
