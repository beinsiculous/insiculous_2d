//! Command-API frame hook: drain queued request lines once per
//! frame and answer them on stdout. The pure half
//! ([`EditorGame::answer_api_lines`]) is headless-testable; only the
//! stdout write lives in the frame hook. Writes route per line —
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
    ///
    /// `texture_path` is the session resolver's inverse (`None` = never
    /// issued): pure writes use it to refuse unissued handles, hosted
    /// saves to write the reference back.
    pub(super) fn answer_api_lines(
        &mut self,
        lines: &[String],
        world: &mut ecs::World,
        texture_path: &dyn Fn(u32) -> Option<String>,
    ) -> Vec<String> {
        let texture_known = |handle: u32| texture_path(handle).is_some();
        let texture_path_fn =
            |handle: u32| super::scene_io::texture_ref_for_save(handle, texture_path(handle));
        let mut responses = Vec::new();
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            let response = match parse_line(line) {
                Err(err) => error_response(&err),
                Ok(Request::Query(query)) => {
                    let ctx = command_api::QueryCtx {
                        world,
                        selection: &self.editor.selection,
                        scene_path: self.editor.scene_path(),
                        dirty: self.command_history.is_dirty(),
                        play_state: self.editor.play_state(),
                    };
                    command_api::answer_query(&query, &ctx)
                }
                Ok(Request::Write(WriteCmd::Pure(write))) => {
                    let play_state = self.editor.play_state();
                    let mut ctx = command_api::write::WriteCtx {
                        world,
                        history: &mut self.command_history,
                        selection: &mut self.editor.selection,
                        play_state,
                        batch: &mut self.api_batch,
                        texture_known: &texture_known,
                    };
                    match command_api::write::run(&write, &mut ctx) {
                        Ok(data) => ok_response(data),
                        Err(err) => error_response(&err),
                    }
                }
                Ok(Request::Write(WriteCmd::Hosted(hosted))) => {
                    match self.run_hosted_write(&hosted, world, &texture_path_fn) {
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
        // Hosted creates mutate the selection BEFORE their command is
        // recorded — note the pre-action selection first. Skipped
        // while a batch is open (the macro keeps the pre-batch image).
        if self.api_batch.is_none() {
            self.command_history.note_selection(&self.editor.selection);
        }
        match hosted {
            HostedWrite::Create { archetype, name, position } => {
                // Validate the name BEFORE spawning — a rejection after the
                // factory ran would leak an entity (same guard as `rename`).
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
                let entity = entity_ops::create_archetype(
                    *archetype,
                    world,
                    &mut self.editor.selection,
                    spawn,
                    &mut self.entity_counter,
                );
                // A requested Name lands BEFORE the create command captures,
                // so create+name is ONE undo entry.
                if let Some(name) = name {
                    world.add_component(&entity, ecs::Name::new(name)).ok();
                }
                let cmd = editor::commands::CreateEntityCommand::already_created(world, entity);
                command_api::write::record_executed(
                    &mut self.command_history,
                    &mut self.api_batch,
                    Box::new(cmd),
                );
                Ok(serde_json::json!({
                    "command": format!("create {}", archetype.kebab()),
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
        let texture_path = move |handle: u32| assets.texture_path(handle).map(str::to_string);
        let responses = self.answer_api_lines(&lines, ctx.world, &texture_path);
        // An API line may have changed the selection (`select`, `create`);
        // GUI commands recorded LATER this frame must see the current
        // selection as their before-image, not the frame-start note.
        self.command_history.note_selection(&self.editor.selection);
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
