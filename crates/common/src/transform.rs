//! 2D Transform type for position, rotation, and scale.

use glam::{Mat3, Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// 2D transformation component.
///
/// Represents position, rotation, and scale in 2D space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform2D {
    /// Position in world space
    pub position: Vec2,
    /// Rotation in radians
    pub rotation: f32,
    /// Scale factors
    pub scale: Vec2,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }
}

impl Transform2D {
    /// Create a new transform at the given position.
    #[inline]
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    /// Create a transform from position, rotation, and scale.
    #[inline]
    pub fn from_parts(position: Vec2, rotation: f32, scale: Vec2) -> Self {
        Self { position, rotation, scale }
    }

    /// Set rotation (builder pattern).
    #[inline]
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set scale (builder pattern).
    #[inline]
    pub fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    /// Set uniform scale (builder pattern).
    #[inline]
    pub fn with_uniform_scale(mut self, scale: f32) -> Self {
        self.scale = Vec2::splat(scale);
        self
    }

    /// Get the 3x3 transformation matrix (T * R * S order).
    pub fn matrix(&self) -> Mat3 {
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();

        // Rotation matrix
        let rot = Mat3::from_cols_array(&[
            cos_r, sin_r, 0.0,
            -sin_r, cos_r, 0.0,
            0.0, 0.0, 1.0,
        ]);

        // Scale matrix
        let scale = Mat3::from_diagonal(Vec3::new(self.scale.x, self.scale.y, 1.0));

        // Translation matrix
        let translate = Mat3::from_cols_array(&[
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            self.position.x, self.position.y, 1.0,
        ]);

        // Combine: T * R * S
        translate * rot * scale
    }

    /// Get the inverse transformation matrix.
    #[inline]
    pub fn inverse_matrix(&self) -> Mat3 {
        self.matrix().inverse()
    }

    /// Transform a point by this transform.
    #[inline]
    pub fn transform_point(&self, point: Vec2) -> Vec2 {
        let transformed = self.matrix() * Vec3::new(point.x, point.y, 1.0);
        Vec2::new(transformed.x, transformed.y)
    }

    /// Transform a point by the inverse of this transform.
    #[inline]
    pub fn inverse_transform_point(&self, point: Vec2) -> Vec2 {
        let transformed = self.inverse_matrix() * Vec3::new(point.x, point.y, 1.0);
        Vec2::new(transformed.x, transformed.y)
    }

    /// Transform a direction vector (rotation and scale only, no translation).
    ///
    /// Applies scale in local axes then rotation (the linear part of
    /// [`matrix`](Self::matrix)'s `T * R * S`), so directions agree with
    /// point transforms under non-uniform scale.
    #[inline]
    pub fn transform_direction(&self, direction: Vec2) -> Vec2 {
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();
        let scaled = direction * self.scale;
        Vec2::new(
            scaled.x * cos_r - scaled.y * sin_r,
            scaled.x * sin_r + scaled.y * cos_r,
        )
    }

    /// Get the forward direction (positive X axis rotated).
    #[inline]
    pub fn forward(&self) -> Vec2 {
        Vec2::new(self.rotation.cos(), self.rotation.sin())
    }

    /// Get the right direction (positive Y axis rotated).
    #[inline]
    pub fn right(&self) -> Vec2 {
        Vec2::new(-self.rotation.sin(), self.rotation.cos())
    }

    /// Translate by the given offset.
    #[inline]
    pub fn translate(&mut self, offset: Vec2) {
        self.position += offset;
    }

    /// Rotate by the given angle in radians.
    #[inline]
    pub fn rotate(&mut self, angle: f32) {
        self.rotation += angle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_3};

    #[test]
    fn test_inverse_transform_point_round_trips_translated_rotated_point() {
        let transform = Transform2D::new(Vec2::new(40.0, -15.0)).with_rotation(FRAC_PI_3);
        let local = Vec2::new(12.0, -7.0);

        let world = transform.transform_point(local);
        let back = transform.inverse_transform_point(world);

        assert!(
            (back - local).length() < 0.001,
            "expected {local:?}, got {back:?}"
        );
    }

    #[test]
    fn test_matrix_applies_scale_before_rotation_before_translation() {
        // T * R * S with non-uniform scale: (1,0) scales to (2,0),
        // rotates 90° to (0,2), translates to (10,22). Guards the
        // composition order — a T*S*R swap would yield (10,23).
        let transform = Transform2D::new(Vec2::new(10.0, 20.0))
            .with_rotation(FRAC_PI_2)
            .with_scale(Vec2::new(2.0, 3.0));

        let transformed = transform.transform_point(Vec2::new(1.0, 0.0));

        assert!(
            (transformed - Vec2::new(10.0, 22.0)).length() < 0.001,
            "expected (10, 22), got {transformed:?}"
        );
    }

    #[test]
    fn test_transform_direction_is_the_matrix_linear_part_and_ignores_translation() {
        // Directions must be the linear part of matrix(): the same
        // point-delta computed via transform_point, unaffected by position.
        let translated = Transform2D::new(Vec2::new(10.0, 20.0))
            .with_rotation(FRAC_PI_2)
            .with_scale(Vec2::new(2.0, 1.0));
        let at_origin = Transform2D::default()
            .with_rotation(FRAC_PI_2)
            .with_scale(Vec2::new(2.0, 1.0));
        let direction = Vec2::new(1.0, 0.0);

        let via_points = translated.transform_point(direction) - translated.transform_point(Vec2::ZERO);
        let via_direction = translated.transform_direction(direction);
        let via_origin = at_origin.transform_direction(direction);

        assert!(
            (via_direction - via_points).length() < 0.001,
            "direction {via_direction:?} disagrees with point delta {via_points:?}"
        );
        assert!(
            (via_direction - via_origin).length() < 0.001,
            "translation must not affect directions: {via_direction:?} vs {via_origin:?}"
        );
        assert!(
            (via_direction - Vec2::new(0.0, 2.0)).length() < 0.001,
            "scale (2,1) then 90° rotation should map +X to (0,2), got {via_direction:?}"
        );
    }

    #[test]
    fn test_forward_points_along_the_rotation_and_right_is_perpendicular() {
        // Asteroids aims every bullet with forward(); the contract is
        // (cos r, sin r) with right() a quarter turn counter-clockwise.
        let unrotated = Transform2D::default();
        let quarter_turn = Transform2D::default().with_rotation(FRAC_PI_2);
        let third_turn = Transform2D::default().with_rotation(FRAC_PI_3);

        let forward_unrotated = unrotated.forward();
        let forward_quarter = quarter_turn.forward();
        let forward_third = third_turn.forward();
        let right_third = third_turn.right();

        assert_eq!(forward_unrotated, Vec2::new(1.0, 0.0));
        assert!(
            (forward_quarter - Vec2::new(0.0, 1.0)).length() < 0.001,
            "90° should face +Y, got {forward_quarter:?}"
        );
        assert!(
            forward_third.dot(right_third).abs() < 0.001,
            "right() must be perpendicular to forward(): {forward_third:?} · {right_third:?}"
        );
        assert!(
            (forward_third.perp() - right_third).length() < 0.001,
            "right() is forward() rotated a quarter turn counter-clockwise, got {right_third:?}"
        );
    }
}
