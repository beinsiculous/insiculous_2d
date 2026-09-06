//! Play-session script diagnostics: the status-bar surface for runner errors, the web
//! error mirror, and the lie-detector for a game that never steps the runner.

use ecs::script::Scripts;
use ecs::World;
use engine_core::scripting::ScriptRunner;
use engine_core::Game;

use super::EditorGame;

impl<G: Game> EditorGame<G> {
    /// Update script status per playing frame:
    /// 1. Lie detector: at play frame 60, check if entities have scripts attached while runner never ran.
    /// 2. Watermark: show new script errors on the status bar.
    /// 3. Mirror: sync script errors to the web error mirror.
    pub(super) fn track_script_status(&mut self, world: &World, scripts: &ScriptRunner) {
        if !self.editor.is_playing() {
            return;
        }

        self.play_frames += 1;

        if self.play_frames == 60 {
            let has_scripts = world.entities().iter().any(|&e| world.get::<Scripts>(e).is_some());
            if has_scripts && scripts.frames_run() == 0 {
                self.editor
                    .status_bar
                    .show_error("scripts attached but the game never ran the script runner");
            }
        }

        let errors = scripts.errors();
        if errors.len() > self.script_error_watermark {
            for err in &errors[self.script_error_watermark..] {
                self.editor.status_bar.show_error(err.to_string());
            }
            self.script_error_watermark = errors.len();
        }

        if let Some(mirror) = &self.script_errors {
            let string_errors: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            if let Ok(mut lock) = mirror.lock() {
                *lock = string_errors;
            }
        }
    }
}
