//! The game-world passes of one frame: what runs, in what order, and which
//! pass clears.
//!
//! The renderer owns the clear rather than each pass owning its own, because
//! there is exactly one per frame and it must happen before anything reads
//! the target. The behind-sprites pass is its owner and is therefore
//! unconditional: a frame with no behind lines still clears there, and every
//! later pass loads the result.

use crate::scissor::PassScissor;

/// A game-world pass, in the order the passes are encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldPass {
    /// Behind-sprites lines, and the frame's one clear of the HDR target.
    BehindLines,
    /// The sprite batches.
    Sprites,
    /// Over-sprites lines: the game's own geometry and the collider overlay.
    OverLines,
}

/// The order the game-world passes run in, whatever they draw.
pub const WORLD_PASS_ORDER: [WorldPass; 3] = [
    WorldPass::BehindLines,
    WorldPass::Sprites,
    WorldPass::OverLines,
];

/// One frame's game-world pass decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldPassPlan {
    clear_owner: WorldPass,
    behind_lines: bool,
    sprites: bool,
    over_lines: bool,
}

impl WorldPassPlan {
    /// Whether `pass` is encoded this frame.
    pub fn runs(&self, pass: WorldPass) -> bool {
        match pass {
            WorldPass::BehindLines => self.behind_lines,
            WorldPass::Sprites => self.sprites,
            WorldPass::OverLines => self.over_lines,
        }
    }

    /// The pass that clears the HDR color and depth target.
    pub fn clear_owner(&self) -> WorldPass {
        self.clear_owner
    }
}

/// Decide this frame's game-world passes.
///
/// An empty scissor (the editor's hidden scene panel) or an empty buffer
/// skips a pass's *draw*, never the clear.
pub fn plan_world_passes(scissor: PassScissor, over_line_vertices: u32) -> WorldPassPlan {
    WorldPassPlan {
        clear_owner: WorldPass::BehindLines,
        behind_lines: true,
        sprites: true,
        over_lines: over_line_vertices > 0 && scissor != PassScissor::Empty,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_behind_pass_runs_and_owns_the_clear_however_empty_the_draws_are() {
        // Pass order is the encoded order, and the clear belongs to the first
        // pass: a frame whose behind buffer is empty still clears there, and
        // every later pass loads that result.
        assert_eq!(
            WORLD_PASS_ORDER,
            [WorldPass::BehindLines, WorldPass::Sprites, WorldPass::OverLines]
        );

        for over_vertices in [2, 0] {
            let plan = plan_world_passes(PassScissor::Fullscreen, over_vertices);
            assert_eq!(plan.clear_owner(), WorldPass::BehindLines, "over {over_vertices}");
            assert!(plan.runs(WorldPass::BehindLines), "the clear pass is unconditional");
            assert!(plan.runs(WorldPass::Sprites), "sprites draw every frame");
            assert_eq!(
                plan.runs(WorldPass::OverLines),
                over_vertices > 0,
                "a pass with no vertices is skipped"
            );
        }
    }

    #[test]
    fn an_empty_scissor_skips_the_draws_and_never_the_clear() {
        // The editor's hidden scene panel: no game world is drawn, but the
        // pass that clears is still the frame's clear owner.
        let plan = plan_world_passes(PassScissor::Empty, 8);
        assert_eq!(plan.clear_owner(), WorldPass::BehindLines);
        assert!(plan.runs(WorldPass::BehindLines));
        assert!(plan.runs(WorldPass::Sprites));
        assert!(!plan.runs(WorldPass::OverLines), "a hidden viewport draws no lines");
    }
}
