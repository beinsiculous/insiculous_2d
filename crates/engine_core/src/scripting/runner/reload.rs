//! A script's source recompiled in the middle of a session, for the editor's hot reload, and
//! the rule for an instance's first frame on the new source.

use std::collections::HashSet;
use std::path::Path;

use ecs::EntityId;

use super::{ScriptError, ScriptErrorKind, ScriptErrors, ScriptRunner};

/// How the message of a runtime error on a reloaded instance's first frame ends.
pub(super) const RELOADED_FIRST_FRAME: &str =
    "reloaded script failed on its first frame — Stop and Play to reset its state";

/// Whether a hook's error quarantines its instance for the rest of the session: a runaway
/// loop or a blown limit recurs every frame, and so does any error on the first frame after
/// a reload, where the new source meets the state the old one left — that one's message
/// says so.
pub(super) fn hook_error_quarantines(
    reloaded: &HashSet<(EntityId, usize)>,
    key: (EntityId, usize),
    error: &mut ScriptError,
) -> bool {
    if reloaded.contains(&key) {
        error.message = format!("{} — {RELOADED_FIRST_FRAME}", error.message);
        return true;
    }
    matches!(error.kind, ScriptErrorKind::RunawayLoop | ScriptErrorKind::ResourceLimit)
}

impl ScriptErrors {
    /// Forget the keys recorded for `file`, so its next errors are reported as new. The
    /// errors already recorded stay, and with them a reader's place in the list.
    fn forget_file(&mut self, file: &str) {
        self.seen.retain(|(seen_file, _, _)| seen_file != file);
    }
}

impl ScriptRunner {
    /// Read `source_path` (as a `Scripts` component names it) through the asset base and
    /// recompile it, mid-session.
    ///
    /// On success every instance of that path runs the new unit from its next hook, the
    /// blackboard, the values a component set and [`frames_run`](Self::frames_run) kept; a
    /// changed `@param` default applies at once, since the params are composed every frame.
    /// The path's quarantined instances and failed resolves get a new try, and its errors are
    /// reported fresh. On failure the old unit keeps running and the error is returned, never
    /// recorded: the caller reports it, so the same error coming back after a fix is not
    /// swallowed by the session's one-report-per-line rule.
    pub fn reload_source(&mut self, source_path: &str) -> Result<(), ScriptError> {
        let source = self.read_source(source_path)?;
        self.rhai.compile(source_path, &source)?;
        let swapped: Vec<(EntityId, usize)> = self
            .instances
            .iter()
            .filter(|(_, instance)| instance.source_path == source_path)
            .map(|(key, _)| *key)
            .collect();
        for key in &swapped {
            self.quarantined.remove(key);
        }
        self.failed.retain(|_, (_, failed_path)| failed_path != source_path);
        self.errors.forget_file(source_path);
        self.reloaded.extend(swapped);
        Ok(())
    }

    /// A `.rhai` script's text, read through the asset base.
    pub(super) fn read_source(&self, source_path: &str) -> Result<String, ScriptError> {
        let resolved_path = self.resolve_source_path(source_path);
        common::vfs::read_to_string(Path::new(&resolved_path)).map_err(|error| ScriptError {
            file: source_path.to_string(),
            line: 0,
            kind: ScriptErrorKind::Runtime,
            message: format!("could not read script at {resolved_path}: {error}"),
        })
    }
}
