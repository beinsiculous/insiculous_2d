//! Entity picking for the scene viewport.
//!
//! Handles click-to-select and rectangle selection for entities in the scene.
//! Uses CPU-based AABB intersection with camera coordinate conversion.

use ecs::EntityId;
use glam::Vec2;

use crate::viewport::SceneViewport;

/// An axis-aligned bounding box in world coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    /// Minimum corner (bottom-left in world coords)
    pub min: Vec2,
    /// Maximum corner (top-right in world coords)
    pub max: Vec2,
}

impl AABB {
    /// Create a new AABB from min and max corners.
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Create an AABB from center and half-extents.
    pub fn from_center_half_extents(center: Vec2, half_extents: Vec2) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    /// Create an AABB from position and size.
    pub fn from_position_size(position: Vec2, size: Vec2) -> Self {
        let half = size * 0.5;
        Self::from_center_half_extents(position, half)
    }

    /// Get the center of the AABB.
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// Get the size (width, height) of the AABB.
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    /// Check if a point is inside the AABB.
    pub fn contains_point(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Check if this AABB intersects another AABB.
    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    /// Expand the AABB by a margin on all sides.
    pub fn expand(&self, margin: f32) -> Self {
        Self {
            min: self.min - Vec2::splat(margin),
            max: self.max + Vec2::splat(margin),
        }
    }
}

/// Data needed for picking an entity.
#[derive(Debug, Clone)]
pub struct PickableEntity {
    /// Entity ID
    pub entity_id: EntityId,
    /// Position in world coordinates
    pub position: Vec2,
    /// Size (scale) in world units
    pub size: Vec2,
    /// Depth for sorting (higher = in front)
    pub depth: f32,
}

impl PickableEntity {
    /// Create a new pickable entity.
    pub fn new(entity_id: EntityId, position: Vec2, size: Vec2, depth: f32) -> Self {
        Self {
            entity_id,
            position,
            size,
            depth,
        }
    }

    /// Get the AABB for this entity. Uses the absolute size so sprites
    /// flipped via a negative scale stay clickable (a raw negative size
    /// would give `min > max` and never contain any point).
    pub fn aabb(&self) -> AABB {
        AABB::from_position_size(self.position, self.size.abs())
    }
}

/// Result of a pick operation.
#[derive(Debug, Clone, Default)]
pub struct PickResult {
    /// Entities hit by the pick (sorted by depth, front to back)
    pub hits: Vec<EntityId>,
}

impl PickResult {
    /// Get the topmost (front) entity hit.
    pub fn topmost(&self) -> Option<EntityId> {
        self.hits.first().copied()
    }

    /// Check if any entities were hit.
    pub fn is_empty(&self) -> bool {
        self.hits.is_empty()
    }

    /// Number of entities hit.
    pub fn len(&self) -> usize {
        self.hits.len()
    }
}

/// Handles entity picking in the scene viewport.
#[derive(Debug, Clone)]
pub struct EntityPicker {
    /// Margin added to entity bounds for easier picking (in world units)
    pub pick_margin: f32,
    /// Index for cycling through overlapping entities on repeated clicks
    cycle_index: usize,
    /// Last pick position (for cycle detection)
    last_pick_pos: Option<Vec2>,
    /// Distance threshold for considering a click at the same position
    same_position_threshold: f32,
}

impl Default for EntityPicker {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityPicker {
    /// Create a new entity picker.
    pub fn new() -> Self {
        Self {
            pick_margin: 2.0,
            cycle_index: 0,
            last_pick_pos: None,
            same_position_threshold: 5.0,
        }
    }

    /// Set the pick margin (tolerance for clicking near entities).
    pub fn with_pick_margin(mut self, margin: f32) -> Self {
        self.pick_margin = margin;
        self
    }

    /// Pick entities at a screen position.
    ///
    /// Returns entities sorted by depth (front to back).
    pub fn pick_at_screen_pos(
        &mut self,
        viewport: &SceneViewport,
        screen_pos: Vec2,
        entities: &[PickableEntity],
    ) -> PickResult {
        let world_pos = viewport.screen_to_world(screen_pos);
        self.pick_at_world_pos(world_pos, screen_pos, entities)
    }

    /// Pick entities at a world position.
    pub fn pick_at_world_pos(
        &mut self,
        world_pos: Vec2,
        screen_pos: Vec2,
        entities: &[PickableEntity],
    ) -> PickResult {
        // Check if this is a repeat click at the same position
        let is_same_position = self.last_pick_pos.is_some_and(|last| {
            (screen_pos - last).length() < self.same_position_threshold
        });

        // Find all entities that contain the point
        let mut hits: Vec<(EntityId, f32)> = entities
            .iter()
            .filter(|e| {
                let aabb = e.aabb().expand(self.pick_margin);
                aabb.contains_point(world_pos)
            })
            .map(|e| (e.entity_id, e.depth))
            .collect();

        // Sort by depth (higher depth = in front); ids break equal-depth ties
        // so downstream selection order (and thus the primary) is deterministic.
        hits.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.value().cmp(&b.0.value())));

        // Handle cycling for overlapping entities
        if is_same_position && hits.len() > 1 {
            self.cycle_index = (self.cycle_index + 1) % hits.len();
            // Rotate the list so the cycled entity is first
            let cycled: Vec<_> = hits
                .iter()
                .cycle()
                .skip(self.cycle_index)
                .take(hits.len())
                .map(|(id, _)| *id)
                .collect();

            self.last_pick_pos = Some(screen_pos);
            return PickResult { hits: cycled };
        }

        // Reset cycle for new position
        self.cycle_index = 0;
        self.last_pick_pos = Some(screen_pos);

        PickResult {
            hits: hits.into_iter().map(|(id, _)| id).collect(),
        }
    }

    /// Pick all entities within a screen rectangle.
    pub fn pick_in_screen_rect(
        &self,
        viewport: &SceneViewport,
        screen_start: Vec2,
        screen_end: Vec2,
        entities: &[PickableEntity],
    ) -> PickResult {
        // Convert screen rect to world rect
        let world_start = viewport.screen_to_world(screen_start);
        let world_end = viewport.screen_to_world(screen_end);

        // Create selection AABB (handle any corner order)
        let selection_aabb = AABB::new(
            Vec2::new(world_start.x.min(world_end.x), world_start.y.min(world_end.y)),
            Vec2::new(world_start.x.max(world_end.x), world_start.y.max(world_end.y)),
        );

        self.pick_in_world_rect(selection_aabb, entities)
    }

    /// Pick all entities within a world rectangle.
    pub fn pick_in_world_rect(&self, rect: AABB, entities: &[PickableEntity]) -> PickResult {
        let mut hits: Vec<(EntityId, f32)> = entities
            .iter()
            .filter(|e| {
                let aabb = e.aabb();
                aabb.intersects(&rect)
            })
            .map(|e| (e.entity_id, e.depth))
            .collect();

        // Sort by depth (higher depth = in front); ids break equal-depth ties
        // so downstream selection order (and thus the primary) is deterministic.
        hits.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.value().cmp(&b.0.value())));

        PickResult {
            hits: hits.into_iter().map(|(id, _)| id).collect(),
        }
    }

    /// Reset cycling state (call when selection changes).
    pub fn reset_cycle(&mut self) {
        self.cycle_index = 0;
        self.last_pick_pos = None;
    }
}

#[cfg(test)]
mod tests;
