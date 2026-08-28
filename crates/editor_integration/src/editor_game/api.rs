//! Command-API frame hook (audit §9 Stage A): drain queued request lines
//! once per frame and answer them on stdout. The pure half
//! ([`EditorGame::answer_api_lines`]) is headless-testable; only the
//! stdout write lives in the frame hook.

use std::io::Write as _;

use engine_core::contexts::GameContext;
use engine_core::Game;

use super::EditorGame;

impl<G: Game> EditorGame<G> {
    /// Answer a batch of request lines against the current editor state.
    /// Blank lines produce no response; everything else produces exactly
    /// one line of JSON, in request order.
    pub(super) fn answer_api_lines(&self, lines: &[String], world: &ecs::World) -> Vec<String> {
        let ctx = editor::command_api::QueryCtx {
            world,
            selection: &self.editor.selection,
            scene_path: self.editor.scene_path(),
            dirty: self.command_history.is_dirty(),
            play_state: self.editor.play_state(),
        };
        lines
            .iter()
            .filter_map(|line| editor::command_api::dispatch_line(line, &ctx))
            .collect()
    }

    /// Frame hook: drain the request channel and write responses to stdout.
    ///
    /// Skipped entirely while a gizmo drag is live — requests stay queued
    /// in the channel and are answered the next eligible frame, so an API
    /// read can never observe (or later, mutate) mid-drag state.
    pub(super) fn drain_api_requests(&mut self, ctx: &mut GameContext) {
        let Some(rx) = &self.api_rx else { return };
        if self.editor.gizmo_has_priority() {
            return;
        }
        // Cap per-frame work so a piped flood of requests can't stall the
        // frame; the rest stays queued in the channel for later frames.
        const MAX_LINES_PER_FRAME: usize = 256;
        let mut lines = Vec::new();
        while lines.len() < MAX_LINES_PER_FRAME {
            let Ok(line) = rx.try_recv() else { break };
            lines.push(line);
        }
        if lines.is_empty() {
            return;
        }

        let responses = self.answer_api_lines(&lines, ctx.world);
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        for response in responses {
            // .ok(): a closed pipe must not crash the editor.
            writeln!(out, "{response}").ok();
        }
        // Piped stdout is block-buffered — without this the caller hangs
        // waiting for a response that sits in the buffer.
        out.flush().ok();
    }
}

#[cfg(test)]
mod tests {
    use engine_core::contexts::GameContext;
    use engine_core::Game;

    use super::super::EditorGame;

    struct DummyGame;
    impl Game for DummyGame {
        fn update(&mut self, _ctx: &mut GameContext) {}
    }

    #[test]
    fn test_answer_api_lines_scene_info_reflects_dirty_history() {
        let mut editor = EditorGame::new(DummyGame);
        let mut world = ecs::World::new();
        let entity = world.create_entity();
        world.add_component(&entity, common::Transform2D::new(glam::Vec2::ZERO)).ok();

        let clean = editor.answer_api_lines(&["scene".to_string()], &world);
        assert!(clean[0].contains("\"dirty\":false"), "clean history: {}", clean[0]);

        let cmd = editor::commands::CreateEntityCommand::already_created(&world, entity);
        editor.command_history.push_already_executed(Box::new(cmd));
        let dirty = editor.answer_api_lines(&["scene".to_string()], &world);
        assert!(dirty[0].contains("\"dirty\":true"), "recorded command: {}", dirty[0]);
    }

    #[test]
    fn test_answer_api_lines_describe_by_name() {
        let editor = EditorGame::new(DummyGame);
        let mut world = ecs::World::new();
        let entity = world.create_entity();
        world.add_component(&entity, ecs::sprite_components::Name::new("Player")).ok();
        world.add_component(&entity, common::Transform2D::new(glam::Vec2::ZERO)).ok();

        let responses =
            editor.answer_api_lines(&["describe Player".to_string(), "".to_string()], &world);

        assert_eq!(responses.len(), 1, "blank line owes no response");
        assert!(responses[0].contains("\"name\":\"Player\""));
        assert!(responses[0].contains("Transform2D"));
    }
}
