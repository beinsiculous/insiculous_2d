//! Collider shape types: the geometry a collider hands to rapier, and the
//! editor's variant cycling over it.
//!
//! Every shape is authored in **pixels in the collider's own frame**:
//! `Transform2D.scale` plays no part, and `Collider.offset` moves the whole
//! shape.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// The capsule the editor's shape selector cycles to: a 32-pixel segment
/// with an 8-pixel cap, the size an unedited sprite usually wants.
const CYCLE_CAPSULE_HALF_LENGTH: f32 = 16.0;
/// Cap radius of the cycled-to capsule — half the default box's half-extent.
const CYCLE_CAPSULE_RADIUS: f32 = 8.0;

/// Collider shape types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColliderShape {
    /// A box with half-extents (width/2, height/2)
    Box { half_extents: Vec2 },
    /// A circle with radius
    Circle { radius: f32 },
    /// A capsule aligned along the Y axis
    CapsuleY { half_height: f32, radius: f32 },
    /// A capsule aligned along the X axis
    CapsuleX { half_height: f32, radius: f32 },
    /// A capsule between two points of the collider's frame — the general
    /// form of the two axis-aligned ones, for a shape that is not straight.
    /// The points are cap centres, so the outline reaches `radius` beyond
    /// each of them.
    Capsule { a: Vec2, b: Vec2, radius: f32 },
    /// Several shapes in the collider's frame, built and drawn as one
    /// collider. A compound's parts are always leaves: nesting is flattened
    /// away, because rapier refuses a compound as another compound's part.
    Compound(Vec<ColliderShape>),
}

impl ColliderShape {
    /// Variant names in cycle order (drives the editor's shape selector).
    pub const VARIANT_NAMES: [&'static str; 6] =
        ["Box", "Circle", "CapsuleY", "CapsuleX", "Capsule", "Compound"];

    /// Index of this shape's variant within [`Self::VARIANT_NAMES`].
    pub fn variant_index(&self) -> usize {
        match self {
            ColliderShape::Box { .. } => 0,
            ColliderShape::Circle { .. } => 1,
            ColliderShape::CapsuleY { .. } => 2,
            ColliderShape::CapsuleX { .. } => 3,
            ColliderShape::Capsule { .. } => 4,
            ColliderShape::Compound(_) => 5,
        }
    }

    /// Display name of this shape's variant.
    pub fn variant_name(&self) -> &'static str {
        Self::VARIANT_NAMES[self.variant_index()]
    }

    /// Build the shape for `variant_index`, carrying this shape's tuned
    /// dimensions across where a sensible mapping exists (a cycled collider
    /// should stay roughly the same size, not snap to defaults). The
    /// mapping is lossy — e.g. a Box's two half-extents collapse into one
    /// radius — which is acceptable because each cycle is a single undo
    /// entry.
    ///
    /// A capsule and a compound are built rather than measured: cycling to
    /// them takes the shape selector's defaults (a straight capsule of the
    /// default box's size; a compound that starts as the shape being left
    /// behind), and cycling away from them carries the half-extents of the
    /// box that contains them.
    pub fn variant_with_carried_dimensions(&self, variant_index: usize) -> ColliderShape {
        // Capsule ↔ capsule is a pure axis swap — going through the
        // bounding-box reduction below would corrupt the half-height.
        match (self, variant_index) {
            (ColliderShape::CapsuleY { half_height, radius }, 3) => {
                return ColliderShape::CapsuleX { half_height: *half_height, radius: *radius };
            }
            (ColliderShape::CapsuleX { half_height, radius }, 2) => {
                return ColliderShape::CapsuleY { half_height: *half_height, radius: *radius };
            }
            _ => {}
        }
        match variant_index {
            4 => {
                return ColliderShape::capsule(
                    Vec2::new(0.0, -CYCLE_CAPSULE_HALF_LENGTH),
                    Vec2::new(0.0, CYCLE_CAPSULE_HALF_LENGTH),
                    CYCLE_CAPSULE_RADIUS,
                );
            }
            5 => return ColliderShape::compound(vec![self.clone()]),
            _ => {}
        }
        // The current shape reduced to a bounding half-width/half-height.
        let extent = self.bounding_half_extents();
        match variant_index {
            0 => ColliderShape::Box { half_extents: extent },
            1 => ColliderShape::Circle { radius: extent.x.max(extent.y) },
            // A zero cylinder section is a valid capsule (= a ball) and is
            // what keeps Circle → Capsule → Circle exact instead of
            // accumulating a floor's worth of drift per lap.
            2 => ColliderShape::CapsuleY {
                half_height: (extent.y - extent.x).max(0.0),
                radius: extent.x,
            },
            3 => ColliderShape::CapsuleX {
                half_height: (extent.x - extent.y).max(0.0),
                radius: extent.y,
            },
            _ => self.clone(),
        }
    }

    /// The half-extents of the box, sitting on the collider's origin, that
    /// contains this shape — what a cycle into another variant carries.
    ///
    /// A shape that is not centred on the origin (a capsule between two
    /// arbitrary points, a compound touching one side) is measured from the
    /// origin anyway: the carried value is a centred box, not the shape's
    /// true bounds.
    fn bounding_half_extents(&self) -> Vec2 {
        match self {
            ColliderShape::Box { half_extents } => *half_extents,
            ColliderShape::Circle { radius } => Vec2::splat(*radius),
            ColliderShape::CapsuleY { half_height, radius } => {
                Vec2::new(*radius, half_height + radius)
            }
            ColliderShape::CapsuleX { half_height, radius } => {
                Vec2::new(half_height + radius, *radius)
            }
            ColliderShape::Capsule { a, b, radius } => {
                a.abs().max(b.abs()) + Vec2::splat(*radius)
            }
            ColliderShape::Compound(parts) => parts
                .iter()
                .fold(Vec2::ZERO, |extent, part| extent.max(part.bounding_half_extents())),
        }
    }
}

impl Default for ColliderShape {
    fn default() -> Self {
        Self::Box {
            half_extents: Vec2::new(16.0, 16.0),
        }
    }
}

impl ColliderShape {
    /// Create a box collider
    pub fn box_shape(width: f32, height: f32) -> Self {
        Self::Box {
            half_extents: Vec2::new(width / 2.0, height / 2.0),
        }
    }

    /// Create a circle collider
    pub fn circle(radius: f32) -> Self {
        Self::Circle { radius }
    }

    /// Create a vertical capsule collider
    pub fn capsule_y(total_height: f32, radius: f32) -> Self {
        Self::CapsuleY {
            half_height: (total_height - 2.0 * radius).max(0.0) / 2.0,
            radius,
        }
    }

    /// Create a horizontal capsule collider
    pub fn capsule_x(total_width: f32, radius: f32) -> Self {
        Self::CapsuleX {
            half_height: (total_width - 2.0 * radius).max(0.0) / 2.0,
            radius,
        }
    }

    /// Create a capsule between two cap centres, in the collider's frame.
    ///
    /// The endpoints are taken as given — coincident ones are a ball, which
    /// is a valid capsule, so the constructor neither orders nor separates
    /// them.
    pub fn capsule(a: Vec2, b: Vec2, radius: f32) -> Self {
        Self::Capsule { a, b, radius }
    }

    /// Create a compound of `parts`, flattened: nested compounds become
    /// their leaves.
    pub fn compound(parts: Vec<ColliderShape>) -> Self {
        Self::Compound(parts).flattened()
    }

    /// This shape with every nested compound replaced by its leaves, so a
    /// [`ColliderShape::Compound`] returned here holds nothing but leaves.
    ///
    /// Rapier panics on a compound passed as another compound's part, so
    /// flattening happens here — on the shape — rather than at build time.
    pub fn flattened(&self) -> Self {
        let ColliderShape::Compound(parts) = self else {
            return self.clone();
        };
        let mut leaves = Vec::new();
        for part in parts {
            match part.flattened() {
                ColliderShape::Compound(nested) => leaves.extend(nested),
                leaf => leaves.push(leaf),
            }
        }
        ColliderShape::Compound(leaves)
    }

    /// Whether this is a compound with no parts — a shape with nothing to
    /// collide with, which rapier cannot build and the engine refuses to
    /// stand in for.
    pub fn is_empty_compound(&self) -> bool {
        matches!(self, ColliderShape::Compound(parts) if parts.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_constructors_take_full_sizes_and_store_half_dimensions() {
        assert_eq!(
            ColliderShape::box_shape(32.0, 64.0),
            ColliderShape::Box { half_extents: Vec2::new(16.0, 32.0) },
            "a box is authored by full width and height"
        );
        // A capsule's half-height is the cylinder section only: the two
        // end caps (2 × radius) come off the total before halving.
        assert_eq!(
            ColliderShape::capsule_y(50.0, 10.0),
            ColliderShape::CapsuleY { half_height: 15.0, radius: 10.0 }
        );
        assert_eq!(
            ColliderShape::capsule_x(50.0, 10.0),
            ColliderShape::CapsuleX { half_height: 15.0, radius: 10.0 }
        );
        assert_eq!(
            ColliderShape::capsule_y(10.0, 10.0),
            ColliderShape::CapsuleY { half_height: 0.0, radius: 10.0 },
            "a capsule shorter than its caps is a ball, never a negative cylinder"
        );
    }

    #[test]
    fn test_capsule_between_two_points_keeps_its_endpoints_as_given() {
        let hinge = Vec2::new(-8.0, 0.0);
        let tip = Vec2::new(-40.0, 60.0);
        assert_eq!(
            ColliderShape::capsule(hinge, tip, 6.0),
            ColliderShape::Capsule { a: hinge, b: tip, radius: 6.0 },
            "the endpoints are stored as given: the axis is a → b, not an ordering"
        );
    }

    #[test]
    fn test_compound_constructor_flattens_nested_compounds_into_leaves() {
        let left = ColliderShape::capsule(Vec2::ZERO, Vec2::new(-30.0, 40.0), 6.0);
        let right = ColliderShape::capsule(Vec2::ZERO, Vec2::new(30.0, 40.0), 6.0);
        let nested = ColliderShape::compound(vec![
            ColliderShape::Compound(vec![left.clone(), right.clone()]),
            ColliderShape::circle(4.0),
        ]);
        assert_eq!(
            nested,
            ColliderShape::Compound(vec![left, right, ColliderShape::circle(4.0)]),
            "a part that is itself a compound contributes its leaves, not itself"
        );

        // Flattening is total: a compound built by hand (never through the
        // constructor) is flattened too, one level at a time.
        let hand_built = ColliderShape::Compound(vec![ColliderShape::Compound(vec![
            ColliderShape::Compound(vec![ColliderShape::circle(4.0)]),
        ])]);
        assert_eq!(
            hand_built.flattened(),
            ColliderShape::Compound(vec![ColliderShape::circle(4.0)])
        );
    }

    #[test]
    fn test_only_a_compound_without_parts_is_empty() {
        assert!(ColliderShape::compound(vec![]).is_empty_compound());
        assert!(
            !ColliderShape::compound(vec![ColliderShape::circle(4.0)]).is_empty_compound()
        );
        assert!(
            ColliderShape::Compound(vec![ColliderShape::compound(vec![])])
                .flattened()
                .is_empty_compound(),
            "an empty compound contributes no leaves, so it cannot rescue a compound that holds it"
        );
        for shape in [
            ColliderShape::Box { half_extents: Vec2::ZERO },
            ColliderShape::Circle { radius: 0.0 },
            ColliderShape::CapsuleY { half_height: 0.0, radius: 0.0 },
            ColliderShape::capsule(Vec2::ZERO, Vec2::ZERO, 1.0),
        ] {
            assert!(!shape.is_empty_compound(), "{} is not a compound", shape.variant_name());
        }
    }

    #[test]
    fn test_editor_selector_indices_round_trip_through_their_variant_tables() {
        // The inspector cycles by index into VARIANT_NAMES: an index that
        // disagrees with its table jumps the selector to the wrong variant.
        let shapes = [
            ColliderShape::Box { half_extents: Vec2::new(1.0, 2.0) },
            ColliderShape::Circle { radius: 3.0 },
            ColliderShape::CapsuleY { half_height: 4.0, radius: 1.0 },
            ColliderShape::CapsuleX { half_height: 5.0, radius: 2.0 },
            ColliderShape::capsule(Vec2::new(0.0, -4.0), Vec2::new(0.0, 4.0), 1.0),
            ColliderShape::compound(vec![ColliderShape::circle(4.0)]),
        ];
        assert_eq!(shapes.len(), ColliderShape::VARIANT_NAMES.len());
        for (index, shape) in shapes.iter().enumerate() {
            assert_eq!(shape.variant_index(), index);
            assert_eq!(shape.variant_name(), ColliderShape::VARIANT_NAMES[index]);
        }
    }

    #[test]
    fn test_shape_cycle_carries_tuned_dimensions_and_clean_round_trips_are_exact() {
        // A wide box cycled to a circle keeps its footprint (max extent),
        // not the default radius — level tuning survives a cycle + undo.
        let wide = ColliderShape::Box { half_extents: Vec2::new(40.0, 20.0) };
        assert_eq!(wide.variant_with_carried_dimensions(1), ColliderShape::Circle { radius: 40.0 });
        // Box → CapsuleY: radius from the half-width, the rest of the height
        // in the cylinder (a wide box yields a zero cylinder = ball).
        assert_eq!(wide.variant_with_carried_dimensions(2), ColliderShape::CapsuleY { half_height: 0.0, radius: 40.0 });
        let tall = ColliderShape::Box { half_extents: Vec2::new(10.0, 50.0) };
        assert_eq!(tall.variant_with_carried_dimensions(2), ColliderShape::CapsuleY { half_height: 40.0, radius: 10.0 });
        assert_eq!(wide.variant_with_carried_dimensions(9), wide, "an out-of-range index is a no-op");

        // Cycling away and straight back must return the same shape — a
        // designer previewing shapes gets their collider back (the old 0.5
        // capsule floor grew a Circle by 0.5 per lap).
        let circle = ColliderShape::Circle { radius: 5.0 };
        let capsule = ColliderShape::CapsuleY { half_height: 30.0, radius: 10.0 };
        let round_trips = [
            (&circle, 2, 1),
            (&circle, 0, 1),
            (&tall, 2, 0),
            (&capsule, 3, 2),
            (&capsule, 0, 2),
        ];
        for (shape, away, back) in round_trips {
            assert_eq!(
                &shape.variant_with_carried_dimensions(away).variant_with_carried_dimensions(back),
                shape,
                "{} → {} → back changed the shape",
                shape.variant_name(),
                ColliderShape::VARIANT_NAMES[away]
            );
        }
    }

    #[test]
    fn test_cycle_to_capsule_and_compound_builds_the_selector_defaults() {
        let box_shape = ColliderShape::Box { half_extents: Vec2::new(40.0, 20.0) };
        assert_eq!(
            box_shape.variant_with_carried_dimensions(4),
            ColliderShape::Capsule {
                a: Vec2::new(0.0, -16.0),
                b: Vec2::new(0.0, 16.0),
                radius: 8.0,
            },
            "a capsule is a new shape, not the box measured"
        );
        assert_eq!(
            box_shape.variant_with_carried_dimensions(5),
            ColliderShape::Compound(vec![box_shape.clone()]),
            "a compound starts as the shape being cycled away from"
        );
        assert_eq!(
            ColliderShape::capsule(Vec2::ZERO, Vec2::new(0.0, 8.0), 2.0)
                .variant_with_carried_dimensions(5),
            ColliderShape::compound(vec![
                ColliderShape::capsule(Vec2::ZERO, Vec2::new(0.0, 8.0), 2.0)
            ]),
        );
    }

    #[test]
    fn test_cycle_from_capsule_or_compound_carries_the_box_that_contains_it() {
        // The jaw's V: two arms from a shared hinge, tips 60 out and 60 up.
        let left = ColliderShape::capsule(Vec2::ZERO, Vec2::new(-30.0, 40.0), 6.0);
        let right = ColliderShape::capsule(Vec2::ZERO, Vec2::new(30.0, 40.0), 6.0);
        let v = ColliderShape::compound(vec![left, right]);

        // The segment reaches 40 up, plus the 6 cap; 36 out, plus the cap.
        assert_eq!(
            v.variant_with_carried_dimensions(0),
            ColliderShape::Box { half_extents: Vec2::new(36.0, 46.0) }
        );
        // A circle takes the larger extent, as it does from any shape.
        assert_eq!(v.variant_with_carried_dimensions(1), ColliderShape::Circle { radius: 46.0 });

        let vertical = ColliderShape::capsule(Vec2::new(0.0, -20.0), Vec2::new(0.0, 30.0), 5.0);
        assert_eq!(
            vertical.variant_with_carried_dimensions(0),
            ColliderShape::Box { half_extents: Vec2::new(5.0, 35.0) }
        );

        // A segment off the origin is measured from the origin: only the far
        // cap counts, because the carried value is a centred box.
        let offset = ColliderShape::capsule(Vec2::new(10.0, 0.0), Vec2::new(10.0, 40.0), 5.0);
        assert_eq!(
            offset.variant_with_carried_dimensions(0),
            ColliderShape::Box { half_extents: Vec2::new(15.0, 45.0) }
        );
    }
}
