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

    /// Get the inverse transformation matrix
    pub fn inverse_matrix(&self) -> Mat3 {
        self.matrix().inverse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite_components::Transform2D;
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn test_children_keep_insertion_order_across_remove_and_readd() {
        // Child order is load-bearing for the hierarchy panel and scene
        // serialization: a HashSet swap would pass a membership-only test.
        let (a, b, c) = (EntityId::new(), EntityId::new(), EntityId::new());
        let mut children = Children::new();

        children.add(a);
        children.add(b);
        children.add(c);
        children.add(b); // duplicate add: neither a second entry nor a reorder
        assert_eq!(children.entities(), [a, b, c]);

        children.remove(&a);
        assert_eq!(children.entities(), [b, c]);

        children.add(a);
        assert_eq!(children.entities(), [b, c, a], "a re-added child goes to the end");
    }

    #[test]
    fn test_global_transform_composes_parent_scale_and_rotation_onto_child_local() {
        // Parent scale 2x at (100, 50): child (10, 5) -> (100, 50) + (10, 5) * 2.
        let scaled = GlobalTransform2D::new(Vec2::new(100.0, 50.0), 0.0, Vec2::new(2.0, 2.0));
        let child = scaled.mul_transform(&Transform2D::new(Vec2::new(10.0, 5.0)));
        assert!(child.position.abs_diff_eq(Vec2::new(120.0, 60.0), 1e-3), "{:?}", child.position);
        assert_eq!(child.scale, Vec2::new(2.0, 2.0), "scale is inherited");

        // Parent rotated 90 degrees: child (10, 0) -> (0, 10), rotation added.
        let rotated = GlobalTransform2D::new(Vec2::ZERO, FRAC_PI_2, Vec2::ONE);
        let child = rotated.mul_transform(&Transform2D::new(Vec2::new(10.0, 0.0)).with_rotation(0.5));
        assert!(child.position.abs_diff_eq(Vec2::new(0.0, 10.0), 1e-3), "{:?}", child.position);
        assert!((child.rotation - (FRAC_PI_2 + 0.5)).abs() < 1e-6);
    }
}
