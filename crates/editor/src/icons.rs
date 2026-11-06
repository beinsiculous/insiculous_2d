//! The toolbar strip's icons, drawn from strokes so they read the same in
//! every font the editor can be running.
//!
//! An icon is a list of [`Stroke`]s in a unit box — (0, 0) top-left to
//! (1, 1) bottom-right, y down — that [`draw_icon`] maps onto a button's
//! bounds. The ui crate draws lines, filled circles and rounded borders and
//! nothing else, so a curve is a polyline of short lines.

use glam::Vec2;
use ui::{Color, Rect, UIContext};

/// The editor's toolbar glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// The Select tool: a pointer arrow.
    Select,
    /// The Move tool: four arrows out of a cross.
    Move,
    /// The Rotate tool: an open ring with an arrowhead, turning clockwise.
    Rotate,
    /// The Scale tool: a small square with an arrow out of its corner.
    Scale,
    /// Start the play session: a triangle.
    Play,
    /// Pause the play session: two bars.
    Pause,
    /// Carry on a paused session: Play's triangle.
    Resume,
    /// End the play session: a square.
    Stop,
    /// The viewport follows the game camera: a reticle.
    Follow,
    /// The authoring grid: two lines each way.
    Grid,
    /// The collider outlines: a ring and a box overlapping.
    Colliders,
    /// The game frame: four corner brackets.
    GameFrame,
    /// Snap to the grid: a magnet.
    Snap,
    /// Put every panel back: a window split into panes.
    ResetLayout,
    /// The overflow menu's button: a double chevron.
    More,
    /// A directory: tab and body.
    Folder,
    /// Parent directory: chevron and stem.
    Up,
    /// Create an asset: a plus.
    New,
}

impl Icon {
    /// Every icon: the tools, the transport, the view toggles, Reset Layout
    /// and the overflow button.
    pub const ALL: [Icon; 18] = [
        Icon::Select, Icon::Move, Icon::Rotate, Icon::Scale,
        Icon::Play, Icon::Pause, Icon::Resume, Icon::Stop, Icon::Follow,
        Icon::Grid, Icon::Colliders, Icon::GameFrame, Icon::Snap,
        Icon::ResetLayout, Icon::More, Icon::Folder, Icon::Up, Icon::New,
    ];
}

/// One stroke of an icon, in the unit box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stroke {
    /// A straight stroke between two points, drawn with round caps.
    Line(Vec2, Vec2),
    /// A filled disc: its centre and radius.
    Disc(Vec2, f32),
    /// A circle's outline: its centre and radius, the stroke centred on it.
    Ring(Vec2, f32),
}

/// Lines to a quarter turn of an arc: short enough that the joints between
/// them never show at the strip's sizes.
const ARC_SEGMENTS_PER_QUARTER_TURN: f32 = 8.0;

/// The strokes of `icon` in the unit box. Every point, and every disc's and
/// ring's bounding square, lies inside it.
pub fn icon_strokes(icon: Icon) -> Vec<Stroke> {
    match icon {
        Icon::Select => closed_polyline(&[
            (0.22, 0.05), (0.22, 0.85), (0.42, 0.65), (0.57, 0.95), (0.67, 0.9), (0.52, 0.6), (0.77, 0.6),
        ]),
        Icon::Move => polylines(&[
            &[(0.5, 0.06), (0.5, 0.94)],
            &[(0.06, 0.5), (0.94, 0.5)],
            &[(0.34, 0.22), (0.5, 0.06), (0.66, 0.22)],
            &[(0.34, 0.78), (0.5, 0.94), (0.66, 0.78)],
            &[(0.22, 0.34), (0.06, 0.5), (0.22, 0.66)],
            &[(0.78, 0.34), (0.94, 0.5), (0.78, 0.66)],
        ]),
        Icon::Rotate => [
            arc(Vec2::new(0.5, 0.55), 0.35, 0.0, 270.0),
            polyline(&[(0.34, 0.06), (0.5, 0.2), (0.34, 0.34)]),
        ]
        .concat(),
        Icon::Scale => [
            closed_polyline(&[(0.08, 0.56), (0.44, 0.56), (0.44, 0.92), (0.08, 0.92)]),
            polylines(&[&[(0.44, 0.56), (0.9, 0.1)], &[(0.62, 0.1), (0.9, 0.1), (0.9, 0.38)]]),
        ]
        .concat(),
        Icon::Play | Icon::Resume => closed_polyline(&[(0.24, 0.1), (0.24, 0.9), (0.86, 0.5)]),
        Icon::Pause => polylines(&[&[(0.34, 0.14), (0.34, 0.86)], &[(0.66, 0.14), (0.66, 0.86)]]),
        Icon::Stop => closed_polyline(&[(0.16, 0.16), (0.84, 0.16), (0.84, 0.84), (0.16, 0.84)]),
        Icon::Follow => {
            let center = Vec2::new(0.5, 0.5);
            let ticks = polylines(&[
                &[(0.5, 0.0), (0.5, 0.2)],
                &[(0.5, 0.8), (0.5, 1.0)],
                &[(0.0, 0.5), (0.2, 0.5)],
                &[(0.8, 0.5), (1.0, 0.5)],
            ]);
            [vec![Stroke::Ring(center, 0.3), Stroke::Disc(center, 0.09)], ticks].concat()
        }
        Icon::Grid => polylines(&[
            &[(0.34, 0.08), (0.34, 0.92)],
            &[(0.66, 0.08), (0.66, 0.92)],
            &[(0.08, 0.34), (0.92, 0.34)],
            &[(0.08, 0.66), (0.92, 0.66)],
        ]),
        Icon::Colliders => [
            vec![Stroke::Ring(Vec2::new(0.36, 0.62), 0.28)],
            closed_polyline(&[(0.4, 0.1), (0.9, 0.1), (0.9, 0.6), (0.4, 0.6)]),
        ]
        .concat(),
        Icon::GameFrame => polylines(&[
            &[(0.06, 0.4), (0.06, 0.18), (0.28, 0.18)],
            &[(0.72, 0.18), (0.94, 0.18), (0.94, 0.4)],
            &[(0.94, 0.6), (0.94, 0.82), (0.72, 0.82)],
            &[(0.28, 0.82), (0.06, 0.82), (0.06, 0.6)],
        ]),
        Icon::Snap => [
            polyline(&[(0.22, 0.12), (0.22, 0.52)]),
            arc(Vec2::new(0.5, 0.52), 0.28, 180.0, 0.0),
            polylines(&[&[(0.78, 0.52), (0.78, 0.12)], &[(0.1, 0.32), (0.34, 0.32)], &[(0.66, 0.32), (0.9, 0.32)]]),
        ]
        .concat(),
        Icon::ResetLayout => [
            closed_polyline(&[(0.08, 0.14), (0.92, 0.14), (0.92, 0.86), (0.08, 0.86)]),
            polylines(&[&[(0.38, 0.14), (0.38, 0.86)], &[(0.38, 0.56), (0.92, 0.56)]]),
        ]
        .concat(),
        Icon::Folder => closed_polyline(&[(0.08, 0.25), (0.08, 0.82), (0.92, 0.82), (0.92, 0.33), (0.48, 0.33), (0.38, 0.18), (0.08, 0.18)]),
        Icon::Up => polylines(&[&[(0.22, 0.42), (0.5, 0.14), (0.78, 0.42)], &[(0.5, 0.14), (0.5, 0.86)]]),
        Icon::New => polylines(&[&[(0.5, 0.15), (0.5, 0.85)], &[(0.15, 0.5), (0.85, 0.5)]]),
        Icon::More => polylines(&[&[(0.2, 0.2), (0.48, 0.5), (0.2, 0.8)], &[(0.52, 0.2), (0.8, 0.5), (0.52, 0.8)]]),
    }
}

/// Draw `icon` centred in `bounds`, its strokes `weight` px wide, in `color`.
///
/// The unit box maps onto the square in `bounds.shrink(2.0)`, inset by half
/// the weight, so every stroke — its caps and a ring's outer edge included —
/// stays inside `bounds.shrink(2.0)`.
pub fn draw_icon(ui: &mut UIContext, icon: Icon, bounds: Rect, color: Color, weight: f32) {
    for stroke in placed_strokes(icon, bounds, weight) {
        match stroke {
            Stroke::Line(start, end) => {
                ui.line(start, end, color, weight);
                // A line is a quad with square ends; a disc on each end rounds
                // it, which also closes the notch two lines leave at a corner.
                ui.circle(start, weight / 2.0, color);
                ui.circle(end, weight / 2.0, color);
            }
            Stroke::Disc(center, radius) => ui.circle(center, radius, color),
            Stroke::Ring(center, radius) => {
                let square = Rect::new(center.x - radius, center.y - radius, radius * 2.0, radius * 2.0);
                // The border straddles its bounds, so the square it is drawn
                // on grows by half the weight each way; a corner radius short
                // of that grown half side leaves flat sides on the circle.
                ui.rect_border(square, color, weight, radius + weight / 2.0);
            }
        }
    }
}

/// `icon`'s strokes in `bounds`' pixels, as [`draw_icon`] draws them.
fn placed_strokes(icon: Icon, bounds: Rect, weight: f32) -> Vec<Stroke> {
    let inner = bounds.shrink(2.0 + weight / 2.0);
    let side = inner.width.min(inner.height).max(0.0);
    let origin = inner.center() - Vec2::splat(side / 2.0);
    let place = |point: Vec2| origin + point * side;
    icon_strokes(icon)
        .into_iter()
        .map(|stroke| match stroke {
            Stroke::Line(start, end) => Stroke::Line(place(start), place(end)),
            Stroke::Disc(center, radius) => Stroke::Disc(place(center), radius * side),
            Stroke::Ring(center, radius) => Stroke::Ring(place(center), radius * side),
        })
        .collect()
}

/// A line through each consecutive pair of `points`.
fn polyline(points: &[(f32, f32)]) -> Vec<Stroke> {
    points
        .windows(2)
        .map(|pair| {
            let (start, end) = (pair[0], pair[1]);
            Stroke::Line(Vec2::new(start.0, start.1), Vec2::new(end.0, end.1))
        })
        .collect()
}

/// A [`polyline`] through each of `paths`.
fn polylines(paths: &[&[(f32, f32)]]) -> Vec<Stroke> {
    paths.iter().flat_map(|path| polyline(path)).collect()
}

/// [`polyline`] with a last line back to the first point.
fn closed_polyline(points: &[(f32, f32)]) -> Vec<Stroke> {
    let mut closed = points.to_vec();
    if let Some(&first) = points.first() {
        closed.push(first);
    }
    polyline(&closed)
}

/// An arc of `radius` around `center` from `start_degrees` to `end_degrees`
/// as a polyline. Angles run clockwise on screen (y is down), so 0 is the
/// right, 90 the bottom; an end below the start runs anticlockwise.
fn arc(center: Vec2, radius: f32, start_degrees: f32, end_degrees: f32) -> Vec<Stroke> {
    let sweep_degrees = end_degrees - start_degrees;
    let segment_count = (sweep_degrees.abs() / 90.0 * ARC_SEGMENTS_PER_QUARTER_TURN).ceil().max(1.0) as usize;
    let point_at = |index: usize| {
        let angle = (start_degrees + sweep_degrees * index as f32 / segment_count as f32).to_radians();
        center + Vec2::new(angle.cos(), angle.sin()) * radius
    };
    (0..segment_count)
        .map(|index| Stroke::Line(point_at(index), point_at(index + 1)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::frame;

    /// Where a draw command reaches, its stroke included.
    fn command_extent(command: &ui::DrawCommand) -> Option<Rect> {
        match command {
            ui::DrawCommand::Line { start, end, width, .. } => {
                let half = width / 2.0;
                let min = start.min(*end) - Vec2::splat(half);
                let max = start.max(*end) + Vec2::splat(half);
                Some(Rect::new(min.x, min.y, max.x - min.x, max.y - min.y))
            }
            ui::DrawCommand::Circle { center, radius, .. } => {
                Some(Rect::new(center.x - radius, center.y - radius, radius * 2.0, radius * 2.0))
            }
            ui::DrawCommand::RectBorder { bounds, width, .. } => Some(bounds.expand(width / 2.0)),
            _ => None,
        }
    }

    /// Every icon draws something, and everything it draws — a line's caps,
    /// a ring's outer edge — stays inside `bounds.shrink(2.0)` at the three
    /// sizes the editor draws icons at, at every weight it uses, wherever
    /// the bounds sit.
    #[test]
    fn test_every_icon_draws_inside_its_shrunk_box_at_every_size() {
        let mut ui = UIContext::new();
        let input = input::InputHandler::new();
        for icon in Icon::ALL {
            for size in [16.0, 20.0, 24.0] {
                for weight in [1.0, 1.5, 2.0] {
                    let bounds = Rect::new(100.5, 40.25, size, size);
                    let allowed = bounds.shrink(2.0);
                    frame(&mut ui, &input, |ui| draw_icon(ui, icon, bounds, Color::WHITE, weight));
                    let extents: Vec<Rect> =
                        ui.draw_list().commands().iter().filter_map(command_extent).collect();
                    assert!(!extents.is_empty(), "{icon:?} draws nothing");
                    for extent in extents {
                        let tolerance = 1e-3;
                        assert!(
                            extent.x >= allowed.x - tolerance
                                && extent.y >= allowed.y - tolerance
                                && extent.right() <= allowed.right() + tolerance
                                && extent.bottom() <= allowed.bottom() + tolerance,
                            "{icon:?} at {size}px, weight {weight}: {extent:?} leaves {allowed:?}"
                        );
                    }
                }
            }
        }
    }

    /// The icons tell their buttons apart: no two share their strokes, but
    /// Resume, which is Play's own triangle by design.
    #[test]
    fn test_no_two_icons_share_their_strokes_but_play_and_resume() {
        for (index, &icon) in Icon::ALL.iter().enumerate() {
            for &other in &Icon::ALL[index + 1..] {
                let same = icon_strokes(icon) == icon_strokes(other);
                let expected = matches!((icon, other), (Icon::Play, Icon::Resume));
                assert_eq!(same, expected, "{icon:?} and {other:?}");
            }
        }
    }
}
