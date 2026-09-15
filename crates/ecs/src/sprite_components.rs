//! Sprite components for ECS integration

use crate::component_registry::ComponentMeta;
use ecs_macros::ComponentMeta as DeriveComponentMeta;
use glam::{Vec2, Vec4};
use serde::{Deserialize, Serialize};

// Re-export common types for ECS use
pub use common::{Camera, SheetGrid, Transform2D};

/// Name component for identifying entities in the editor hierarchy.
///
/// Entities with a Name component will display this name in the hierarchy panel
/// instead of a generic "Entity {id}" label.
#[derive(Debug, Clone, Serialize, Deserialize, DeriveComponentMeta, Default)]
pub struct Name(pub String);

impl Name {
    /// Create a new Name component with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Sprite component that defines visual appearance
#[derive(Debug, Clone, Serialize, Deserialize, DeriveComponentMeta)]
pub struct Sprite {
    /// Position offset from entity position
    pub offset: Vec2,
    /// Rotation in radians
    pub rotation: f32,
    /// Scale
    pub scale: Vec2,
    /// Texture region (x, y, width, height) in texture coordinates [0, 1].
    /// Omitted means the full texture, matching the scene-wire default — a
    /// plain serde default would be the empty region and render nothing.
    #[serde(default = "default_tex_region")]
    pub tex_region: [f32; 4],
    /// Color tint
    pub color: Vec4,
    /// Layer depth for sorting (higher values render on top)
    pub depth: f32,
    /// Whether this sprite is visible (invisible sprites are skipped during rendering)
    #[serde(default = "default_visible")]
    pub visible: bool,
    /// Emissive intensity — 0.0 disables glow, larger values bloom more strongly.
    /// `#[serde(default)]` keeps existing scene files (written before this field
    /// existed) loading cleanly.
    #[serde(default)]
    pub emissive: f32,
    /// Texture handle ID (resolved by the renderer)
    pub texture_handle: u32,
}

fn default_visible() -> bool { true }

fn default_tex_region() -> [f32; 4] {
    [0.0, 0.0, 1.0, 1.0]
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            tex_region: [0.0, 0.0, 1.0, 1.0], // Full texture
            color: Vec4::ONE, // White
            depth: 0.0,
            visible: true,
            emissive: 0.0,
            texture_handle: 0,
        }
    }
}

impl Sprite {
    /// Create a new sprite
    pub fn new(texture_handle: u32) -> Self {
        Self {
            texture_handle,
            ..Default::default()
        }
    }

    /// Set sprite offset
    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    /// Set sprite rotation
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set sprite scale
    pub fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    /// Set texture region (UV coordinates)
    pub fn with_tex_region(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.tex_region = [x, y, width, height];
        self
    }

    /// Set color tint
    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }

    /// Set depth
    pub fn with_depth(mut self, depth: f32) -> Self {
        self.depth = depth;
        self
    }

    /// Set visibility
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set emissive intensity (0.0 disables glow, larger values bloom more).
    pub fn with_emissive(mut self, emissive: f32) -> Self {
        self.emissive = emissive;
        self
    }
}

// Note: Transform2D and Camera2D are now re-exported from common crate
// This eliminates ~170 lines of duplicated code

/// One named animation clip: which sheet cells to show, how fast, and whether
/// it repeats.
///
/// A clip is **not** a component — clips live inside [`SpriteAnimation`], which
/// is. `frame_indices` are cell indices into that component's [`SheetGrid`],
/// so a clip is just a playlist over the sheet: `[0, 1, 2, 3]` plays the first
/// four cells in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationClip {
    /// Sheet cell indices, played in this order.
    pub frame_indices: Vec<u32>,
    /// Playback rate. Must be finite and positive to advance.
    pub fps: f32,
    /// Whether playback wraps back to the first frame at the end.
    pub looping: bool,
}

impl AnimationClip {
    /// Create a looping clip over the given cell indices.
    pub fn new(frame_indices: impl Into<Vec<u32>>, fps: f32) -> Self {
        Self {
            frame_indices: frame_indices.into(),
            fps,
            looping: true,
        }
    }

    /// Set whether the clip loops.
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }
}

/// Named-clip sprite animation over a sheet.
///
/// The component owns a [`SheetGrid`] describing how its sheet is cut into
/// cells, plus a set of named [`AnimationClip`]s. Game code plays clips **by
/// name** (`play("walk")`) and never touches raw cell indices — the names are
/// the stable API across art revisions.
///
/// # Ownership of `Sprite.tex_region`
///
/// While [`current_clip`](Self::current_clip) is `Some` and the active frame
/// resolves to a cell, this component **owns** the entity's `Sprite.tex_region`:
/// `SpriteAnimationSystem` overwrites it every frame, so manual writes are
/// lost. When [`current_uv`](Self::current_uv) yields `None` (nothing playing,
/// empty clip, or a frame index past the end of the grid) the sprite's region
/// is left exactly as it is.
#[derive(Debug, Clone, Serialize, Deserialize, DeriveComponentMeta)]
pub struct SpriteAnimation {
    /// How the sheet is cut into cells.
    pub grid: SheetGrid,
    /// Clips by name, in declaration order — ordered rather than a map so
    /// serialization and the inspector display are deterministic. Clip counts
    /// are single-digit, so lookup is a linear scan.
    pub clips: Vec<(String, AnimationClip)>,
    /// Path of the sheet PNG this animation came from, exactly as passed to
    /// `load_sprite_sheet` (base-path-relative). Its `.sheet.ron` sidecar —
    /// same stem, so `sprites/deion.png` → `sprites/deion.sheet.ron` — is the
    /// source of truth for `grid` and `clips` when a scene reloads.
    pub sheet: Option<String>,
    /// Name of the clip currently selected, or `None` when nothing is playing.
    pub current_clip: Option<String>,
    /// Whether the selected clip is advancing.
    pub playing: bool,
    /// Position within the active clip's `frame_indices` (not a cell index).
    pub current_frame: usize,
    /// Time carried over toward the next frame.
    pub time_accumulator: f32,
    /// Set by [`update`](Self::update) when a play-once clip runs past its
    /// last frame; cleared by [`play`](Self::play) and [`stop`](Self::stop).
    /// Kept apart from `playing` so a clip [`pause`](Self::pause)d on its last
    /// frame does not read as complete.
    #[serde(default)]
    pub finished: bool,
}

impl Default for SpriteAnimation {
    fn default() -> Self {
        Self {
            grid: SheetGrid::new(1, 1),
            clips: Vec::new(),
            sheet: None,
            current_clip: None,
            playing: false,
            current_frame: 0,
            time_accumulator: 0.0,
            finished: false,
        }
    }
}

impl SpriteAnimation {
    /// Create an animation over the given sheet grid, with no clips yet.
    pub fn new(grid: SheetGrid) -> Self {
        Self {
            grid,
            ..Default::default()
        }
    }

    /// Add a named clip.
    pub fn with_clip(mut self, name: impl Into<String>, clip: AnimationClip) -> Self {
        self.clips.push((name.into(), clip));
        self
    }

    /// Whether a clip with this name exists.
    pub fn has_clip(&self, name: &str) -> bool {
        self.clips.iter().any(|(n, _)| n == name)
    }

    /// The clip [`current_clip`](Self::current_clip) names, if any.
    pub fn active_clip(&self) -> Option<&AnimationClip> {
        let name = self.current_clip.as_deref()?;
        self.clips
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, clip)| clip)
    }

    /// Select a clip and **start it from the beginning**.
    ///
    /// This is a transition call, not a per-frame one: it always resets to
    /// frame 0 and clears the time accumulator, whether or not `name` is
    /// already the current clip and whether or not it was playing. Calling it
    /// every frame therefore pins the animation to frame 0 — use
    /// [`ensure_playing`](Self::ensure_playing) from state-machine code that
    /// re-asserts its clip each update, and [`resume`](Self::resume) to
    /// continue a paused clip.
    ///
    /// Returns `false` for an unknown clip name: the call is a warned no-op
    /// and the current clip keeps playing.
    #[must_use]
    pub fn play(&mut self, name: &str) -> bool {
        if !self.has_clip(name) {
            log::warn!(
                "SpriteAnimation::play: no clip named '{}' (keeping {:?}); known clips: {:?}",
                name,
                self.current_clip,
                self.clips.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>()
            );
            return false;
        }
        self.current_clip = Some(name.to_string());
        self.current_frame = 0;
        self.time_accumulator = 0.0;
        self.playing = true;
        self.finished = false;
        true
    }

    /// Play `name` unless it is already the clip that is playing — the
    /// per-frame-safe form of [`play`](Self::play).
    ///
    /// Safe to call every update: an already-running clip keeps advancing
    /// instead of restarting. A paused or finished clip of the same name does
    /// restart. Returns `false` for an unknown clip name.
    #[must_use]
    pub fn ensure_playing(&mut self, name: &str) -> bool {
        if self.playing && self.current_clip.as_deref() == Some(name) {
            return true;
        }
        self.play(name)
    }

    /// Stop playback and deselect the clip: nothing advances and
    /// [`current_uv`](Self::current_uv) yields `None`, so the sprite keeps
    /// whatever region it is showing.
    pub fn stop(&mut self) {
        self.playing = false;
        self.current_clip = None;
        self.current_frame = 0;
        self.time_accumulator = 0.0;
        self.finished = false;
    }

    /// Freeze on the current frame, keeping the clip and position.
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Continue a paused clip from where it stopped. No-op when no clip is
    /// selected.
    pub fn resume(&mut self) {
        if self.current_clip.is_some() {
            self.playing = true;
        }
    }

    /// Advance the active clip by `delta_time` seconds.
    ///
    /// Frame advance is computed arithmetically rather than by repeatedly
    /// subtracting a frame duration, so no combination of `fps` and
    /// `delta_time` can spin. Clips that cannot advance — no frames, or an
    /// `fps` that is zero, negative, or not finite — simply hold their frame.
    /// Authored clips are rejected at parse time; these guards are the second
    /// net for components built programmatically.
    pub fn update(&mut self, delta_time: f32) {
        if !self.playing || !delta_time.is_finite() || delta_time <= 0.0 {
            return;
        }
        let Some((fps, looping, frame_count)) = self
            .active_clip()
            .map(|clip| (clip.fps, clip.looping, clip.frame_indices.len()))
        else {
            return;
        };
        if frame_count == 0 || !fps.is_finite() || fps <= 0.0 {
            return;
        }

        self.time_accumulator += delta_time;
        let frame_duration = 1.0 / fps;
        // Float-to-int casts saturate in Rust, so even an absurd accumulator
        // yields a finite step count instead of looping forever.
        let advanced = (self.time_accumulator / frame_duration) as usize;
        if advanced == 0 {
            return;
        }
        self.time_accumulator -= advanced as f32 * frame_duration;

        let next = self.current_frame.saturating_add(advanced);
        if next < frame_count {
            self.current_frame = next;
        } else if looping {
            self.current_frame = next % frame_count;
        } else {
            self.current_frame = frame_count - 1;
            self.time_accumulator = 0.0;
            self.playing = false;
            self.finished = true;
        }
    }

    /// Whether a non-looping clip has run past its last frame and stopped.
    ///
    /// The update that lands on the last frame leaves the clip playing — that
    /// frame is shown for its own duration — and the update after it is the
    /// one that stops the clip and makes this true. False for a looping clip
    /// (it never ends), for a clip [`pause`](Self::pause)d anywhere, its last
    /// frame included, and when no clip is selected.
    ///
    /// [`play`](Self::play) and [`ensure_playing`](Self::ensure_playing)
    /// **restart** a finished clip of the same name, so code that re-asserts
    /// its clip every frame checks this first.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// The texture region of the current frame, or `None` when nothing
    /// resolves — no clip selected, an empty clip, or a frame index past the
    /// last cell of the grid.
    pub fn current_uv(&self) -> Option<[f32; 4]> {
        let clip = self.active_clip()?;
        let cell = *clip.frame_indices.get(self.current_frame)?;
        self.grid.uv_rect_checked(cell)
    }
}

// Note: Sprite and SpriteAnimation use #[derive(ComponentMeta)]
// Transform2D and Camera need manual impls since they're from the common crate

impl crate::component_registry::ComponentMeta for Transform2D {
    fn type_name() -> &'static str {
        "Transform2D"
    }

    fn field_names() -> &'static [&'static str] {
        &["position", "rotation", "scale"]
    }
}

impl crate::component_registry::ComponentMeta for Camera {
    fn type_name() -> &'static str {
        "Camera"
    }

    fn field_names() -> &'static [&'static str] {
        &["position", "rotation", "zoom", "viewport_size", "is_main_camera", "near", "far"]
    }
}

/// Set sprite visibility for a batch of entities — the shared mechanism for
/// "hide gameplay sprites while a menu state is active" (each game decides
/// WHICH entities and WHEN; the engine owns the loop). Entities without a
/// `Sprite` are skipped.
pub fn set_sprites_visible(
    world: &mut crate::World,
    entities: impl IntoIterator<Item = crate::EntityId>,
    visible: bool,
) {
    for entity in entities {
        if let Some(sprite) = world.get_mut::<Sprite>(entity) {
            sprite.visible = visible;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A three-cell sheet with a one-shot and a looping clip.
    fn animation() -> SpriteAnimation {
        SpriteAnimation::new(SheetGrid::new(3, 1))
            .with_clip("hurt", AnimationClip::new(vec![0, 1, 2], 10.0).with_looping(false))
            .with_clip("walk", AnimationClip::new(vec![0, 1, 2], 10.0))
    }

    #[test]
    fn test_is_finished_is_true_only_for_a_one_shot_clip_that_reached_its_last_frame() {
        let mut animation = animation();
        assert!(!animation.is_finished(), "no clip selected is never finished");

        assert!(animation.play("hurt"));
        assert!(!animation.is_finished(), "a clip that just started has not ended");
        animation.update(0.1);
        assert_eq!(animation.current_frame, 1, "one of three frames in");
        assert!(!animation.is_finished(), "mid-clip is not finished");
        assert!(animation.playing);

        // Showing the last frame is not the end of the clip: the update that
        // would advance past it is the one that stops it.
        animation.update(0.1);
        assert_eq!(animation.current_frame, 2, "the last frame is showing");
        assert!(animation.playing);
        assert!(!animation.is_finished());

        animation.update(0.1);
        assert_eq!(animation.current_frame, 2, "held on the last frame");
        assert!(!animation.playing);
        assert!(animation.is_finished());
        animation.update(1.0);
        assert!(animation.is_finished(), "a finished clip stays finished");

        // A looping clip runs on the same frames and never finishes.
        assert!(animation.play("walk"));
        for _ in 0..40 {
            animation.update(0.1);
        }
        assert!(animation.playing);
        assert!(!animation.is_finished());

        // A paused clip is not finished, and neither is one with no clip.
        animation.pause();
        assert!(!animation.is_finished());
        animation.stop();
        assert!(!animation.is_finished());
    }

    #[test]
    fn test_a_one_shot_paused_on_its_last_frame_is_not_finished() {
        // A freeze-frame on the last frame is a pause, not the clip's end: the
        // clip machine must not act on it. Resuming lets the end arrive.
        let mut animation = animation();
        assert!(animation.play("hurt"));
        animation.update(0.1);
        animation.update(0.1);
        assert_eq!(animation.current_frame, 2, "the last frame is showing");
        animation.pause();
        assert!(!animation.playing);
        assert!(!animation.is_finished(), "paused on the last frame is not finished");

        animation.resume();
        animation.update(0.1);
        assert!(animation.is_finished(), "the end arrives once playback resumes");
    }

    #[test]
    fn test_re_asserting_a_finished_clip_restarts_it_so_is_finished_comes_first() {
        // The ordering trap a state machine has to respect: `ensure_playing`
        // of the finished clip's own name starts it over, so the completion is
        // only observable if it is read before the clip is re-asserted.
        let mut animation = animation();
        assert!(animation.play("hurt"));
        for _ in 0..4 {
            animation.update(0.1);
        }
        assert!(animation.is_finished());

        let restarted = animation.ensure_playing("hurt");
        assert!(restarted);
        assert_eq!(animation.current_frame, 0, "the same name restarts from the top");
        assert!(!animation.is_finished());
    }

    #[test]
    fn test_set_sprites_visible_toggles_only_entities_that_carry_a_sprite() -> Result<(), crate::EcsError> {
        // The editor hides scene UI outside Play through this; an entity
        // without a Sprite in the list must be skipped, not panic.
        let mut world = crate::World::new();
        let drawn = world.create_entity();
        world.add_component(&drawn, Sprite::new(0))?;
        let bare = world.create_entity();

        set_sprites_visible(&mut world, [drawn, bare], false);
        assert!(!world.get::<Sprite>(drawn).expect("sprite").visible);

        set_sprites_visible(&mut world, [drawn, bare], true);
        assert!(world.get::<Sprite>(drawn).expect("sprite").visible);
        assert!(world.get::<Sprite>(bare).is_none(), "no sprite is ever created");
        Ok(())
    }
}
