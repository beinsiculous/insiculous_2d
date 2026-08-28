//! Command-API frame hook (audit §9): drain queued request lines once per
//! frame and answer them on stdout. The pure half
//! ([`EditorGame::answer_api_lines`]) is headless-testable; only the
//! stdout write lives in the frame hook. Stage B: writes route per line —
//! pure writes through `editor::command_api::write` (always via
//! `CommandHistory`), hosted writes (create/save) through the same entity
//! factories and save choke point the GUI uses.

use std::io::Write as _;
use std::path::PathBuf;

use engine_core::contexts::GameContext;
use engine_core::Game;

use editor::command_api::{
    self, error_response, ok_response, parse_line, ApiError, HostedWrite, Request, WriteCmd,
};

use super::EditorGame;
use crate::entity_ops;

impl<G: Game> EditorGame<G> {
    /// Answer a batch of request lines against the current editor state.
    /// Blank lines produce no response; everything else produces exactly
    /// one line of JSON, in request order. Queries and pure writes run in
    /// the editor crate; hosted writes (create/save) run here.
    pub(super) fn answer_api_lines(
        &mut self,
        lines: &[String],
        world: &mut ecs::World,
        texture_path_fn: &dyn Fn(u32) -> String,
    ) -> Vec<String> {
        let mut responses = Vec::new();
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            let response = match parse_line(line) {
                Err(err) => error_response(&err),
                Ok(Request::Query(_)) => {
                    let ctx = command_api::QueryCtx {
                        world,
                        selection: &self.editor.selection,
                        scene_path: self.editor.scene_path(),
                        dirty: self.command_history.is_dirty(),
                        play_state: self.editor.play_state(),
                    };
                    // dispatch_line re-parses, but keeps the envelope in ONE
                    // place for queries; a blank was filtered above.
                    command_api::dispatch_line(line, &ctx)
                        .unwrap_or_else(|| error_response(&ApiError::Parse("empty".into())))
                }
                Ok(Request::Write(WriteCmd::Pure(write))) => {
                    let play_state = self.editor.play_state();
                    let mut ctx = command_api::write::WriteCtx {
                        world,
                        history: &mut self.command_history,
                        selection: &mut self.editor.selection,
                        play_state,
                        batch: &mut self.api_batch,
                    };
                    match command_api::write::run(&write, &mut ctx) {
                        Ok(data) => ok_response(data),
                        Err(err) => error_response(&err),
                    }
                }
                Ok(Request::Write(WriteCmd::Hosted(hosted))) => {
                    match self.run_hosted_write(&hosted, world, texture_path_fn) {
                        Ok(data) => ok_response(data),
                        Err(err) => error_response(&err),
                    }
                }
            };
            responses.push(response);
        }
        responses
    }

    /// Hosted writes: entity creation (factories + viewport spawn position
    /// live here) and scene save (the mandatory choke point).
    fn run_hosted_write(
        &mut self,
        hosted: &HostedWrite,
        world: &mut ecs::World,
        texture_path_fn: &dyn Fn(u32) -> String,
    ) -> Result<serde_json::Value, ApiError> {
        if self.editor.is_playing() {
            return Err(ApiError::Refused(
                "writes are refused while Playing — pause or stop first".to_string(),
            ));
        }
        match hosted {
            HostedWrite::Create { archetype, name, position } => {
                let action = archetype_action(archetype).ok_or_else(|| {
                    ApiError::Invalid(format!("unknown archetype \"{archetype}\""))
                })?;
                // Validate the name BEFORE spawning — a rejection after the
                // factory ran would leak an entity (same guard as `rename`,
                // kimi F4).
                let name = match name {
                    Some(raw) => {
                        let trimmed = raw.trim();
                        if trimmed.is_empty() {
                            return Err(ApiError::Invalid(
                                "create name must be non-empty (omit it for an unnamed entity)"
                                    .to_string(),
                            ));
                        }
                        Some(trimmed.to_string())
                    }
                    None => None,
                };
                let spawn = position
                    .map(|(x, y)| glam::Vec2::new(x, y))
                    .unwrap_or_else(|| self.editor.viewport.camera_position());
                let entity = entity_ops::handle_create_action(
                    action,
                    world,
                    &mut self.editor.selection,
                    spawn,
                    &mut self.entity_counter,
                )
                .ok_or_else(|| ApiError::Invalid(format!("create {archetype} failed")))?;
                // A requested Name lands BEFORE the create command captures,
                // so create+name is ONE undo entry.
                if let Some(name) = name {
                    world.add_component(&entity, ecs::Name::new(name)).ok();
                }
                let cmd = editor::commands::CreateEntityCommand::already_created(world, entity);
                match self.api_batch.as_mut() {
                    Some(batch) => batch.commands.push(Box::new(cmd)),
                    None => self.command_history.push_already_executed(Box::new(cmd)),
                }
                Ok(serde_json::json!({
                    "command": format!("create {archetype}"),
                    "entity": editor::command_api::entity_record(world, entity),
                }))
            }
            HostedWrite::Save { path } => {
                if self.api_batch.is_some() {
                    return Err(ApiError::Refused(
                        "a batch is open — `batch end` or `batch abort` before saving".to_string(),
                    ));
                }
                // Same default-path logic as save_scene, through the same
                // mandatory choke point.
                let target = match path {
                    Some(path) => PathBuf::from(path),
                    None => self
                        .editor
                        .scene_path()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| PathBuf::from(crate::constants::DEFAULT_SCENE_PATH)),
                };
                let result = self.save_scene_with(world, texture_path_fn, target);
                match result {
                    Ok(()) => Ok(serde_json::json!({
                        "saved": self.editor.scene_path().map(|p| p.display().to_string()),
                    })),
                    Err(e) => Err(ApiError::Refused(format!("save failed: {e}"))),
                }
            }
        }
    }

    /// Frame hook: drain the request channel and write responses to stdout.
    ///
    /// Skipped entirely while a gizmo drag is live — requests stay queued
    /// in the channel and are answered the next eligible frame, so an API
    /// read can never observe (or later, mutate) mid-drag state.
    pub(super) fn drain_api_requests(&mut self, ctx: &mut GameContext) {
        if self.editor.gizmo_has_priority() {
            return;
        }
        // Cap per-frame work so a piped flood of requests can't stall the
        // frame; the rest stays queued in the channel for later frames.
        const MAX_LINES_PER_FRAME: usize = 256;
        let lines = {
            let Some(rx) = &self.api_rx else { return };
            let mut lines = Vec::new();
            while lines.len() < MAX_LINES_PER_FRAME {
                let Ok(line) = rx.try_recv() else { break };
                lines.push(line);
            }
            lines
        };
        if lines.is_empty() {
            return;
        }

        let assets = &*ctx.assets;
        let texture_path_fn = move |handle: u32| -> String {
            assets
                .texture_path(handle)
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    if handle == 0 { "#white".to_string() } else { format!("#texture_{}", handle) }
                })
        };
        let responses = self.answer_api_lines(&lines, ctx.world, &texture_path_fn);
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

/// Kebab archetype name (see `editor::command_api::ARCHETYPES`) → the menu
/// action label the entity factories dispatch on. A drift test asserts
/// every archetype maps.
fn archetype_action(archetype: &str) -> Option<&'static str> {
    Some(match archetype {
        "empty" => "Create Empty",
        "sprite" => "Create Sprite",
        "camera" => "Create Camera",
        "static-body" => "Create Static Body",
        "dynamic-body" => "Create Dynamic Body",
        "kinematic-body" => "Create Kinematic Body",
        "ui-label" => "Create UI Label",
        "ui-panel" => "Create UI Panel",
        "ui-button" => "Create UI Button",
        _ => return None,
    })
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

        let clean = editor.answer_api_lines(&["scene".to_string()], &mut world, &|_| "#white".into());
        assert!(clean[0].contains("\"dirty\":false"), "clean history: {}", clean[0]);

        let cmd = editor::commands::CreateEntityCommand::already_created(&world, entity);
        editor.command_history.push_already_executed(Box::new(cmd));
        let dirty = editor.answer_api_lines(&["scene".to_string()], &mut world, &|_| "#white".into());
        assert!(dirty[0].contains("\"dirty\":true"), "recorded command: {}", dirty[0]);
    }

    #[test]
    fn test_answer_api_lines_describe_by_name() {
        let mut editor = EditorGame::new(DummyGame);
        let mut world = ecs::World::new();
        let entity = world.create_entity();
        world.add_component(&entity, ecs::sprite_components::Name::new("Player")).ok();
        world.add_component(&entity, common::Transform2D::new(glam::Vec2::ZERO)).ok();

        let responses = editor.answer_api_lines(
            &["describe Player".to_string(), "".to_string()],
            &mut world,
            &|_| "#white".into(),
        );

        assert_eq!(responses.len(), 1, "blank line owes no response");
        assert!(responses[0].contains("\"name\":\"Player\""));
        assert!(responses[0].contains("Transform2D"));
    }
}
