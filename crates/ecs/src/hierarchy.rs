//! Entity hierarchy components for parent-child relationships
//!
//! This module provides components for building scene graphs with transform propagation.
//! Entities can have parent-child relationships where children inherit their parent's transform.

use crate::entity::EntityId;
use glam::{Mat3, Vec2};
use serde::{Deserialize, Serialize};

/// Component that stores an entity's parent reference
///
/// When an entity has a Parent component, its transform will be relative to its parent's
/// world-space transform.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Parent {
    /// The parent entity ID
    entity: EntityId,
}

impl Parent {
    /// Create a new Parent component
    pub fn new(entity: EntityId) -> Self {
        Self { entity }
    }

    /// Get the parent entity ID
    pub fn entity(&self) -> EntityId {
        self.entity
    }

    /// Set the parent entity ID
    pub fn set(&mut self, entity: EntityId) {
        self.entity = entity;
    }
}

/// Component that stores an entity's children
///
/// This component is automatically managed by the hierarchy system. You typically
/// don't need to add it manually - use the World hierarchy methods instead.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Children {
    /// List of child entity IDs
    entities: Vec<EntityId>,
}

impl Children {
    /// Create a new Children component
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a Children component with initial children
    pub fn with_children(children: Vec<EntityId>) -> Self {
        Self { entities: children }
    }

    /// Get the child entity IDs
    pub fn entities(&self) -> &[EntityId] {
        &self.entities
    }

    /// Add a child entity
    pub fn add(&mut self, child: EntityId) {
        if !self.entities.contains(&child) {
            self.entities.push(child);
        }
    }

    /// Remove a child entity
    pub fn remove(&mut self, child: &EntityId) {
        self.entities.retain(|e| e != child);
    }

    /// Check if an entity is a child
    pub fn contains(&self, child: &EntityId) -> bool {
        self.entities.contains(child)
    }

    /// Get the number of children
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Check if there are no children
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Iterate over children
    pub fn iter(&self) -> impl Iterator<Item = &EntityId> {
        self.entities.iter()
    }
}

/// Component storing the computed world-space transform
///
/// This is automatically updated by the TransformHierarchySystem. For root entities
/// (those without a Parent), this equals their local Transform2D. For child entities,
/// this is the result of multiplying the parent's GlobalTransform2D with the child's
/// local Transform2D.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GlobalTransform2D {
    /// World-space position
    pub position: Vec2,
    /// World-space rotation in radians
    pub rotation: f32,
    /// World-space scale
    pub scale: Vec2,
}

impl crate::component_registry::ComponentMeta for GlobalTransform2D {
    fn type_name() -> &'static str {
        "GlobalTransform2D"
    }

    fn field_names() -> &'static [&'static str] {
        &["position", "rotation", "scale"]
    }
}

impl Default for GlobalTransform2D {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }
}

impl GlobalTransform2D {
    /// Create a new global transform
    pub fn new(position: Vec2, rotation: f32, scale: Vec2) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    /// Create from a local Transform2D (for root entities)
    pub fn from_transform(transform: &crate::sprite_components::Transform2D) -> Self {
        Self {
            position: transform.position,
            rotation: transform.rotation,
            scale: transform.scale,
        }
    }

    /// Get the transformation matrix
    pub fn matrix(&self) -> Mat3 {
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();

        // Rotation matrix
        let rot = Mat3::from_cols_array(&[cos_r, sin_r, 0.0, -sin_r, cos_r, 0.0, 0.0, 0.0, 1.0]);

        // Scale matrix
        let scale = Mat3::from_diagonal(glam::Vec3::new(self.scale.x, self.scale.y, 1.0));

        // Translation matrix
        let translate = Mat3::from_cols_array(&[
            1.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            self.position.x,
            self.position.y,
            1.0,
        ]);

        // Combine: T * R * S
        translate * rot * scale
    }

    /// Multiply this transform with a local transform to produce a child's global transform
    pub fn mul_transform(&self, local: &crate::sprite_components::Transform2D) -> Self {
        // Combine rotations
        let rotation = self.rotation + local.rotation;

        // Combine scales
        let scale = self.scale * local.scale;

        // Rotate and scale the local position by parent's transform, then add parent's position
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();
        let rotated_pos = Vec2::new(
            local.position.x * cos_r - local.position.y * sin_r,
            local.position.x * sin_r + local.position.y * cos_r,
        );
        let position = self.position + rotated_pos * self.scale;

        Self {
            position,
            rotation,
            scale,
        }
    }

    /// Transform a point from local to world space
    pub fn transform_point(&self, point: Vec2) -> Vec2 {
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();
        let rotated = Vec2::new(
            point.x * cos_r - point.y * sin_r,
            point.x * sin_r + point.y * cos_r,
        );
        self.position + rotated * self.scale
    }

    /// Get the inverse transformation matrix
    pub fn inverse_matrix(&self) -> Mat3 {
        self.matrix().inverse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite_components::Transform2D;

    #[test]
    fn test_parent_component() {
        let id = EntityId::new();
        let parent = Parent::new(id);
        assert_eq!(parent.entity(), id);
    }

    #[test]
    fn test_children_component() {
        let mut children = Children::new();
        assert!(children.is_empty());
        assert_eq!(children.len(), 0);

        let child1 = EntityId::new();
        let child2 = EntityId::new();

        children.add(child1);
        assert!(children.contains(&child1));
        assert!(!children.contains(&child2));
        assert_eq!(children.len(), 1);

        children.add(child2);
        assert_eq!(children.len(), 2);

        // Adding duplicate should not increase count
        children.add(child1);
        assert_eq!(children.len(), 2);

        children.remove(&child1);
        assert!(!children.contains(&child1));
        assert_eq!(children.len(), 1);
    }

    #[test]
    fn test_global_transform_identity() {
        let global = GlobalTransform2D::default();
        assert_eq!(global.position, Vec2::ZERO);
        assert_eq!(global.rotation, 0.0);
        assert_eq!(global.scale, Vec2::ONE);
    }

    #[test]
    fn test_global_transform_from_local() {
        let local = Transform2D::new(Vec2::new(100.0, 50.0))
            .with_rotation(0.5)
            .with_scale(Vec2::new(2.0, 2.0));

        let global = GlobalTransform2D::from_transform(&local);
        assert_eq!(global.position, local.position);
        assert_eq!(global.rotation, local.rotation);
        assert_eq!(global.scale, local.scale);
    }

    #[test]
    fn test_global_transform_mul() {
        // Parent at (100, 50), no rotation, scale 2x
        let parent = GlobalTransform2D::new(Vec2::new(100.0, 50.0), 0.0, Vec2::new(2.0, 2.0));

        // Child at (10, 5) local
        let child_local = Transform2D::new(Vec2::new(10.0, 5.0));

        let child_global = parent.mul_transform(&child_local);

        // Child world position should be parent + child_local * parent_scale
        // = (100, 50) + (10, 5) * 2 = (120, 60)
        assert!((child_global.position.x - 120.0).abs() < 0.001);
        assert!((child_global.position.y - 60.0).abs() < 0.001);
        assert_eq!(child_global.scale, Vec2::new(2.0, 2.0));
    }

    #[test]
    fn test_global_transform_mul_with_rotation() {
        use std::f32::consts::FRAC_PI_2;

        // Parent at (0, 0), rotated 90 degrees, scale 1x
        let parent = GlobalTransform2D::new(Vec2::ZERO, FRAC_PI_2, Vec2::ONE);

        // Child at (10, 0) local - should end up at (0, 10) after 90 degree rotation
        let child_local = Transform2D::new(Vec2::new(10.0, 0.0));

        let child_global = parent.mul_transform(&child_local);

        // After 90 degree rotation: (10, 0) -> (0, 10)
        assert!((child_global.position.x).abs() < 0.001);
        assert!((child_global.position.y - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_transform_point() {
        let global = GlobalTransform2D::new(Vec2::new(100.0, 100.0), 0.0, Vec2::new(2.0, 2.0));

        let local_point = Vec2::new(10.0, 5.0);
        let world_point = global.transform_point(local_point);

        // (100, 100) + (10, 5) * 2 = (120, 110)
        assert!((world_point.x - 120.0).abs() < 0.001);
        assert!((world_point.y - 110.0).abs() < 0.001);
    }
}
