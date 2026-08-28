//! Scene data structures for RON serialization
//!
//! This module defines the data structures used to serialize and deserialize
//! scene files in RON (Rusty Object Notation) format.

use std::collections::HashMap;

use common::SheetGrid;
use ecs::sprite_components::AnimationClip;
use serde::{Deserialize, Serialize};

// BehaviorData (+ its Behavior conversions) lives in `behavior_data.rs` for
// file-size reasons; re-exported here so the scene schema stays one import.
pub use crate::behavior_data::BehaviorData;

/// Editor-specific settings persisted with the scene
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EditorSettings {
    /// Camera position when scene was last saved
    #[serde(default)]
    pub camera_position: (f32, f32),
    /// Camera zoom level when scene was last saved
    #[serde(default = "default_zoom")]
    pub camera_zoom: f32,
}

/// Root structure for a scene file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneData {
    /// Scene name
    pub name: String,
    /// Physics settings for this scene
    #[serde(default)]
    pub physics: Option<PhysicsSettings>,
    /// Editor settings (camera position, zoom) - optional for backward compatibility
    #[serde(default)]
    pub editor: Option<EditorSettings>,
    /// Prefab definitions (reusable entity templates)
    #[serde(default)]
    pub prefabs: HashMap<String, PrefabData>,
    /// Entity instances
    #[serde(default)]
    pub entities: Vec<EntityData>,
}

impl Default for SceneData {
    fn default() -> Self {
        Self {
            name: "Untitled".to_string(),
            physics: None,
            editor: None,
            prefabs: HashMap::new(),
            entities: Vec::new(),
        }
    }
}

/// Physics world settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsSettings {
    /// Gravity vector (default: (0.0, -980.0) for platformer)
    #[serde(default = "default_gravity")]
    pub gravity: (f32, f32),
    /// Pixels per meter scale (default: 100.0)
    #[serde(default = "default_pixels_per_meter")]
    pub pixels_per_meter: f32,
    /// Physics timestep in seconds (default: 1/60)
    #[serde(default = "default_timestep")]
    pub timestep: f32,
}

fn default_gravity() -> (f32, f32) {
    (0.0, -980.0)
}

fn default_pixels_per_meter() -> f32 {
    100.0
}

fn default_timestep() -> f32 {
    1.0 / 60.0
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        Self {
            gravity: default_gravity(),
            pixels_per_meter: default_pixels_per_meter(),
            timestep: default_timestep(),
        }
    }
}

/// Prefab definition - a reusable entity template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabData {
    /// Components that make up this prefab
    pub components: Vec<ComponentData>,
}

/// Entity instance in a scene
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityData {
    /// Optional name for this entity (for lookup)
    #[serde(default)]
    pub name: Option<String>,
    /// Optional prefab to instantiate
    #[serde(default)]
    pub prefab: Option<String>,
    /// Optional parent entity name (for hierarchy)
    ///
    /// The parent must be defined earlier in the entities list so it exists
    /// when this entity is created. Use the parent entity's `name` field.
    #[serde(default)]
    pub parent: Option<String>,
    /// Component overrides (applied on top of prefab)
    #[serde(default)]
    pub overrides: Vec<ComponentData>,
    /// Inline components (if no prefab is used)
    #[serde(default)]
    pub components: Vec<ComponentData>,
    /// Child entities (alternative to using parent field - creates hierarchy inline)
    #[serde(default)]
    pub children: Vec<EntityData>,
}

/// Component data variants for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentData {
    /// Transform component
    Transform2D {
        #[serde(default)]
        position: (f32, f32),
        #[serde(default)]
        rotation: f32,
        #[serde(default = "default_scale")]
        scale: (f32, f32),
    },
    /// Sprite component
    Sprite {
        /// Texture reference: "#white", "#solid:RRGGBB", or file path
        #[serde(default = "default_texture")]
        texture: String,
        #[serde(default)]
        offset: (f32, f32),
        #[serde(default)]
        rotation: f32,
        #[serde(default = "default_scale")]
        scale: (f32, f32),
        #[serde(default = "default_color")]
        color: (f32, f32, f32, f32),
        #[serde(default)]
        depth: f32,
        /// Emissive strength for bloom (0.0 = no glow)
        #[serde(default)]
        emissive: f32,
        /// Texture sub-region (x, y, w, h) in normalized [0, 1] coordinates.
        /// Omitted means the full texture — a plain serde default would be
        /// the empty region (0,0,0,0) and render nothing. For an entity with
        /// a playing `SpriteAnimation` this is a frame snapshot the
        /// animation overwrites on load; it matters for static sheet props.
        #[serde(default = "default_tex_region")]
        tex_region: (f32, f32, f32, f32),
        /// Omitted means visible: a plain serde default would silently hide
        /// every sprite in scenes written before this field existed.
        #[serde(default = "default_true")]
        visible: bool,
    },
    /// Camera component
    Camera2D {
        #[serde(default)]
        position: (f32, f32),
        #[serde(default)]
        rotation: f32,
        #[serde(default = "default_zoom")]
        zoom: f32,
        #[serde(default = "default_viewport")]
        viewport_size: (f32, f32),
        #[serde(default)]
        is_main_camera: bool,
    },
    /// Named-clip sprite animation over a sheet.
    ///
    /// `sheet` names the PNG; its `.sheet.ron` sidecar (same stem) is the
    /// source of truth and its grid + clips win over the inline values on
    /// load, so an artist's sidecar edit reaches every scene that references
    /// it. The inline `grid`/`clips` are the baked snapshot used when the
    /// sheet is absent or its sidecar cannot be read.
    ///
    /// Playback position is deliberately not serialized: `current_frame` and
    /// `time_accumulator` are runtime state, and `autoplay` — the clip to
    /// start on load — is written only for an animation that was actually
    /// playing, so a paused one does not resurrect running.
    SpriteAnimation {
        #[serde(default)]
        sheet: Option<String>,
        #[serde(default)]
        grid: GridData,
        #[serde(default)]
        clips: Vec<(String, ClipData)>,
        #[serde(default)]
        autoplay: Option<String>,
    },
    /// Rigid body component
    RigidBody {
        #[serde(default)]
        body_type: RigidBodyTypeData,
        #[serde(default)]
        velocity: (f32, f32),
        #[serde(default)]
        angular_velocity: f32,
        #[serde(default = "default_gravity_scale")]
        gravity_scale: f32,
        #[serde(default)]
        linear_damping: f32,
        #[serde(default)]
        angular_damping: f32,
        #[serde(default = "default_true")]
        can_rotate: bool,
        #[serde(default)]
        ccd_enabled: bool,
    },
    /// Collider component
    Collider {
        #[serde(default)]
        shape: ColliderShapeData,
        #[serde(default)]
        offset: (f32, f32),
        #[serde(default)]
        is_sensor: bool,
        #[serde(default = "default_friction")]
        friction: f32,
        #[serde(default)]
        restitution: f32,
    },
    /// Tilemap component - grid of tile indices drawn from a tileset
    Tilemap {
        /// Tileset texture reference: "#white", "#solid:RRGGBB", or file path
        #[serde(default = "default_texture")]
        tileset: String,
        #[serde(default)]
        width: u32,
        #[serde(default)]
        height: u32,
        #[serde(default = "default_tile_size")]
        tile_size: f32,
        /// Row-major tile values, 0 = empty
        #[serde(default)]
        tiles: Vec<u32>,
        /// Fraction of the tileset per tile, e.g. (0.25, 0.25) for 4x4
        #[serde(default = "default_scale")]
        tile_uv_size: (f32, f32),
        #[serde(default = "default_tilemap_depth")]
        depth: f32,
    },
    /// Screen-space text label (data-driven UI; `@key` text localizes)
    UiLabel {
        #[serde(default)]
        text: String,
        #[serde(default)]
        anchor: ecs::UiAnchor,
        #[serde(default)]
        offset: (f32, f32),
        #[serde(default = "default_ui_font_size")]
        font_size: f32,
        #[serde(default = "default_color")]
        color: (f32, f32, f32, f32),
        #[serde(default = "default_true")]
        visible: bool,
    },
    /// Screen-space colored panel (data-driven UI)
    UiPanel {
        #[serde(default)]
        anchor: ecs::UiAnchor,
        #[serde(default)]
        offset: (f32, f32),
        #[serde(default = "default_ui_panel_size")]
        size: (f32, f32),
        #[serde(default = "default_ui_panel_background")]
        background: (f32, f32, f32, f32),
        #[serde(default = "default_color")]
        border: (f32, f32, f32, f32),
        #[serde(default = "default_ui_border_width")]
        border_width: f32,
        #[serde(default = "default_true")]
        visible: bool,
    },
    /// Screen-space clickable button (data-driven UI; presses surface as
    /// `UiButtonPressed` events carrying `id`)
    UiButton {
        #[serde(default)]
        text: String,
        #[serde(default)]
        id: String,
        #[serde(default)]
        anchor: ecs::UiAnchor,
        #[serde(default)]
        offset: (f32, f32),
        #[serde(default = "default_ui_button_size")]
        size: (f32, f32),
        #[serde(default = "default_true")]
        visible: bool,
    },
    /// Behavior component - defines how an entity responds to input/events
    Behavior(BehaviorData),
    /// Tag component for entity identification (targeted by behaviors)
    EntityTag {
        #[serde(default = "default_player_tag")]
        tag: String,
    },
    /// Dynamic component: any type registered in the ecs `ComponentRegistry`
    /// that has no concrete variant above (issue #43). This is how
    /// game-defined components and the audio components reach scene files.
    ///
    /// Concrete variants remain reserved for types with load-time resolve
    /// logic (texture references, `.sheet.ron` sidecars, physics shape
    /// enums) — a pure serde path cannot express that resolution.
    ///
    /// Loading a `Dynamic` whose name is not registered is a HARD error —
    /// authored data is never silently dropped. The payload is plain JSON
    /// (serde_json::Value round-trips through RON as a map; deliberately
    /// NOT `#[serde(flatten)]`, which is fragile through RON's enum
    /// representation).
    Dynamic {
        /// Component type name (must match registry)
        #[serde(rename = "type")]
        component_type: String,
        /// Component data as JSON
        data: serde_json::Value,
    },
}

/// Wire form of a [`SheetGrid`](common::SheetGrid) in scene RON: the cell
/// counts only.
///
/// The grid's normalized cell size is derived, never written — scene files
/// speak in columns and rows, `.sheet.ron` sidecars speak in pixel cell sizes,
/// and neither spells out UVs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridData {
    #[serde(default = "default_grid_axis")]
    pub cols: u32,
    #[serde(default = "default_grid_axis")]
    pub rows: u32,
}

impl Default for GridData {
    fn default() -> Self {
        Self { cols: 1, rows: 1 }
    }
}

fn default_grid_axis() -> u32 {
    1
}

impl From<GridData> for SheetGrid {
    fn from(data: GridData) -> Self {
        SheetGrid::new(data.cols, data.rows)
    }
}

impl From<SheetGrid> for GridData {
    fn from(grid: SheetGrid) -> Self {
        Self {
            cols: grid.cols,
            rows: grid.rows,
        }
    }
}

/// Wire form of an [`AnimationClip`] — one shape shared by scene RON and
/// `.sheet.ron` sidecars, so a clip reads and writes identically wherever it
/// appears.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipData {
    /// Sheet cell indices, in playback order.
    pub frames: Vec<u32>,
    pub fps: f32,
    /// Omitted means looping: most clips repeat, and the plain serde default
    /// for a bool would silently make every clip a one-shot.
    #[serde(default = "default_looping")]
    pub looping: bool,
}

/// Clips loop unless a file says otherwise.
pub fn default_looping() -> bool {
    true
}

impl From<ClipData> for AnimationClip {
    fn from(data: ClipData) -> Self {
        Self {
            frame_indices: data.frames,
            fps: data.fps,
            looping: data.looping,
        }
    }
}

impl From<AnimationClip> for ClipData {
    fn from(clip: AnimationClip) -> Self {
        Self {
            frames: clip.frame_indices,
            fps: clip.fps,
            looping: clip.looping,
        }
    }
}

// Default value functions
fn default_scale() -> (f32, f32) {
    (1.0, 1.0)
}

fn default_texture() -> String {
    "#white".to_string()
}

fn default_color() -> (f32, f32, f32, f32) {
    (1.0, 1.0, 1.0, 1.0)
}

fn default_tex_region() -> (f32, f32, f32, f32) {
    (0.0, 0.0, 1.0, 1.0)
}

fn default_zoom() -> f32 {
    1.0
}

fn default_viewport() -> (f32, f32) {
    (800.0, 600.0)
}

fn default_ui_font_size() -> f32 {
    16.0
}

fn default_ui_panel_size() -> (f32, f32) {
    (200.0, 120.0)
}

fn default_ui_panel_background() -> (f32, f32, f32, f32) {
    (0.1, 0.1, 0.15, 0.85)
}

fn default_ui_border_width() -> f32 {
    1.0
}

fn default_ui_button_size() -> (f32, f32) {
    (120.0, 32.0)
}

fn default_tile_size() -> f32 {
    32.0
}

fn default_tilemap_depth() -> f32 {
    -1.0
}

fn default_true() -> bool {
    true
}

fn default_gravity_scale() -> f32 {
    1.0
}

fn default_friction() -> f32 {
    0.5
}

fn default_player_tag() -> String {
    "player".to_string()
}

/// Rigid body type for serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RigidBodyTypeData {
    #[default]
    Dynamic,
    Static,
    Kinematic,
}

/// Collider shape for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColliderShapeData {
    Box { half_extents: (f32, f32) },
    Circle { radius: f32 },
    CapsuleY { half_height: f32, radius: f32 },
    CapsuleX { half_height: f32, radius: f32 },
}

impl Default for ColliderShapeData {
    fn default() -> Self {
        Self::Box {
            half_extents: (16.0, 16.0),
        }
    }
}

/// Error type for scene loading
#[derive(Debug, thiserror::Error)]
pub enum SceneLoadError {
    #[error("Failed to read scene file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse RON: {0}")]
    RonError(#[from] ron::error::SpannedError),

    #[error("Prefab not found: {0}")]
    PrefabNotFound(String),

    #[error("Invalid texture reference: {0}")]
    InvalidTextureRef(String),

    #[error("Failed to load texture: {0}")]
    TextureLoadError(String),

    #[error("Component error: {0}")]
    ComponentError(String),
}
