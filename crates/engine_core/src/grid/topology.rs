//! Honeycomb lattice construction for the deformable grid.
//!
//! Pointy-top hexagons built from zigzag rows: `spacing` is the hexagon
//! side length, columns advance by `spacing * sqrt(3)/2`, rows by
//! `spacing * 1.5`, and every node whose `(x + y)` is odd sits half a side
//! lower than its row neighbors. Horizontal springs trace each zigzag;
//! vertical springs drop from the zigzag low points only, so interior
//! nodes have degree exactly 3 and the springs outline regular hexagons
//! (width `spacing * sqrt(3)`, height `2 * spacing`). Every spring's rest
//! length is exactly `spacing`.
//!
//! Border nodes are pinned (inv_mass = 0) so the grid doesn't drift.

use glam::Vec2;

/// A single mass point in the grid.
#[derive(Debug, Clone, Copy)]
pub(super) struct GridNode {
    /// Initial position. Nodes return here when impulses fade.
    pub(super) rest: Vec2,
    /// Current position. Diverges from `rest` while the grid is excited.
    pub(super) position: Vec2,
    pub(super) velocity: Vec2,
    /// 0.0 = pinned (won't move). 1.0 = free node.
    pub(super) inv_mass: f32,
}

/// A spring connecting two nodes.
#[derive(Debug, Clone, Copy)]
pub(super) struct Spring {
    pub(super) a: u32,
    pub(super) b: u32,
    pub(super) rest_length: f32,
}

/// Build a honeycomb topology: `rows * cols` nodes in zigzag rows
/// (row-major, `idx = y * cols + x`), centered on `origin`. `cols` must be
/// even so the vertical-spring rule stays regular to the right edge.
///
/// Springs: `(cols - 1) * rows` horizontal + `(rows - 1) * cols / 2`
/// vertical (one per zigzag low point per row gap).
pub(super) fn build_hex_topology(
    cols: u32,
    rows: u32,
    spacing: f32,
    origin: Vec2,
) -> (Vec<GridNode>, Vec<Spring>) {
    let hx = spacing * 3.0_f32.sqrt() * 0.5;
    let hy = spacing * 1.5;
    let half_w = (cols - 1) as f32 * hx * 0.5;
    // The zigzag adds spacing/2 below the last row's base line.
    let half_h = ((rows - 1) as f32 * hy + spacing * 0.5) * 0.5;
    let zig = |x: u32, y: u32| if (x + y) % 2 == 1 { spacing * 0.5 } else { 0.0 };

    let mut nodes = Vec::with_capacity((cols * rows) as usize);
    for y in 0..rows {
        for x in 0..cols {
            let pos = origin
                + Vec2::new(
                    x as f32 * hx - half_w,
                    y as f32 * hy + zig(x, y) - half_h,
                );
            let pinned = x == 0 || y == 0 || x == cols - 1 || y == rows - 1;
            nodes.push(GridNode {
                rest: pos,
                position: pos,
                velocity: Vec2::ZERO,
                inv_mass: if pinned { 0.0 } else { 1.0 },
            });
        }
    }

    let idx = |x: u32, y: u32| -> u32 { y * cols + x };
    let mut springs =
        Vec::with_capacity(((cols - 1) * rows + (rows - 1) * cols / 2) as usize);
    for y in 0..rows {
        for x in 0..cols {
            if x + 1 < cols {
                springs.push(Spring {
                    a: idx(x, y),
                    b: idx(x + 1, y),
                    rest_length: spacing,
                });
            }
            // Verticals only from zigzag low points: their offset (+s/2)
            // meets the next row's base line exactly one side length down.
            if y + 1 < rows && (x + y) % 2 == 1 {
                springs.push(Spring {
                    a: idx(x, y),
                    b: idx(x, y + 1),
                    rest_length: spacing,
                });
            }
        }
    }
    (nodes, springs)
}

/// Build the classic square-lattice topology: `rows * cols` nodes on a
/// regular grid (row-major, `idx = y * cols + x`, centered on `origin`)
/// with springs to the right and down neighbors. Border nodes are pinned.
/// Unlike the honeycomb, any `cols >= 2` works — odd counts included.
pub(super) fn build_square_topology(
    cols: u32,
    rows: u32,
    spacing: f32,
    origin: Vec2,
) -> (Vec<GridNode>, Vec<Spring>) {
    let half_w = (cols - 1) as f32 * spacing * 0.5;
    let half_h = (rows - 1) as f32 * spacing * 0.5;
    let mut nodes = Vec::with_capacity((cols * rows) as usize);
    for y in 0..rows {
        for x in 0..cols {
            let pos = origin + Vec2::new(x as f32 * spacing - half_w, y as f32 * spacing - half_h);
            let pinned = x == 0 || y == 0 || x == cols - 1 || y == rows - 1;
            nodes.push(GridNode {
                rest: pos,
                position: pos,
                velocity: Vec2::ZERO,
                inv_mass: if pinned { 0.0 } else { 1.0 },
            });
        }
    }

    let idx = |x: u32, y: u32| -> u32 { y * cols + x };
    let mut springs = Vec::with_capacity(((cols - 1) * rows + cols * (rows - 1)) as usize);
    for y in 0..rows {
        for x in 0..cols {
            if x + 1 < cols {
                springs.push(Spring {
                    a: idx(x, y),
                    b: idx(x + 1, y),
                    rest_length: spacing,
                });
            }
            if y + 1 < rows {
                springs.push(Spring {
                    a: idx(x, y),
                    b: idx(x, y + 1),
                    rest_length: spacing,
                });
            }
        }
    }
    (nodes, springs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_springs_all_have_rest_length_equal_to_spacing() {
        let spacing = 10.0;
        let (nodes, springs) = build_hex_topology(6, 5, spacing, Vec2::new(3.0, -7.0));
        assert!(!springs.is_empty());
        for s in &springs {
            let d = nodes[s.a as usize].rest.distance(nodes[s.b as usize].rest);
            assert!(
                (d - spacing).abs() < 1e-4,
                "spring {}–{} rest distance {} != spacing",
                s.a, s.b, d
            );
            assert!((s.rest_length - spacing).abs() < 1e-4);
        }
    }

    #[test]
    fn hex_interior_nodes_have_degree_three() {
        let cols = 6;
        let rows = 5;
        let (nodes, springs) = build_hex_topology(cols, rows, 10.0, Vec2::ZERO);
        let mut degree = vec![0u32; nodes.len()];
        for s in &springs {
            degree[s.a as usize] += 1;
            degree[s.b as usize] += 1;
        }
        for y in 1..rows - 1 {
            for x in 1..cols - 1 {
                let i = (y * cols + x) as usize;
                assert_eq!(degree[i], 3, "interior node ({x},{y}) degree");
            }
        }
    }

    #[test]
    fn hex_grid_is_centered_on_origin() {
        let origin = Vec2::new(5.0, -3.0);
        let (nodes, _) = build_hex_topology(8, 6, 12.0, origin);
        let (mut min, mut max) = (nodes[0].rest, nodes[0].rest);
        for n in &nodes {
            min = min.min(n.rest);
            max = max.max(n.rest);
        }
        let center = (min + max) * 0.5;
        assert!((center - origin).length() < 1e-4, "bbox center {center} != origin");
    }

    #[test]
    fn hex_spring_count_matches_formula() {
        // (cols-1)*rows horizontal + (rows-1)*cols/2 vertical (even cols).
        for (cols, rows, expected) in [(6u32, 4u32, 5 * 4 + 3 * 3), (8, 5, 7 * 5 + 4 * 4)] {
            let (_, springs) = build_hex_topology(cols, rows, 10.0, Vec2::ZERO);
            assert_eq!(springs.len(), expected as usize, "{cols}x{rows}");
        }
    }

    #[test]
    fn square_springs_all_have_rest_length_equal_to_spacing() {
        let spacing = 10.0;
        let (nodes, springs) = build_square_topology(5, 4, spacing, Vec2::new(3.0, -7.0));
        // Odd column counts are legal for the square lattice.
        assert_eq!(nodes.len(), 20);
        assert_eq!(springs.len(), 4 * 4 + 5 * 3);
        for s in &springs {
            let d = nodes[s.a as usize].rest.distance(nodes[s.b as usize].rest);
            assert!((d - spacing).abs() < 1e-4);
            assert!((s.rest_length - spacing).abs() < 1e-4);
        }
    }

    #[test]
    fn square_interior_nodes_have_degree_four() {
        let cols = 5;
        let rows = 5;
        let (nodes, springs) = build_square_topology(cols, rows, 10.0, Vec2::ZERO);
        let mut degree = vec![0u32; nodes.len()];
        for s in &springs {
            degree[s.a as usize] += 1;
            degree[s.b as usize] += 1;
        }
        for y in 1..rows - 1 {
            for x in 1..cols - 1 {
                assert_eq!(degree[(y * cols + x) as usize], 4, "interior node ({x},{y}) degree");
            }
        }
    }

    #[test]
    fn hex_border_nodes_are_pinned_and_interior_free() {
        let cols = 6;
        let rows = 4;
        let (nodes, _) = build_hex_topology(cols, rows, 10.0, Vec2::ZERO);
        for y in 0..rows {
            for x in 0..cols {
                let border = x == 0 || y == 0 || x == cols - 1 || y == rows - 1;
                let inv_mass = nodes[(y * cols + x) as usize].inv_mass;
                if border {
                    assert_eq!(inv_mass, 0.0, "border ({x},{y}) must be pinned");
                } else {
                    assert_eq!(inv_mass, 1.0, "interior ({x},{y}) must be free");
                }
            }
        }
    }
}
