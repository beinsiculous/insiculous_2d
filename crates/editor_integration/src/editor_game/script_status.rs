//! Play-session script diagnostics: runner errors reported to the status bar and the
//! console, a `.rhai` file's linked at its line, the web error mirror, and the
//! lie-detector for a game that never steps the runner.

use ecs::script::Scripts;
use ecs::World;
use editor::console::Severity;
use engine_core::scripting::ScriptRunner;
use engine_core::Game;

use super::console::script_error_location;
use super::EditorGame;

impl<G: Game> EditorGame<G> {
    /// Update script status per playing frame:
    /// 1. Lie detector: at play frame 60, check if entities have scripts attached while runner never ran.
    /// 2. Watermark: report new script errors on the status bar and in the console.
    /// 3. Mirror: sync script errors to the web error mirror.
    pub(super) fn track_script_status(&mut self, world: &World, scripts: &ScriptRunner) {
        if !self.editor.is_playing() {
            return;
        }

        self.play_frames += 1;

        if self.play_frames == 60 {
            let has_scripts = world.entity_ids().any(|entity| world.get::<Scripts>(entity).is_some());
            if has_scripts && scripts.frames_run() == 0 {
                self.report(Severity::Error, "scripts attached but the game never ran the script runner", None);
            }
        }

        let errors = scripts.errors();
        if errors.len() > self.script_error_watermark {
            for error in &errors[self.script_error_watermark..] {
                self.report(Severity::Error, error.to_string(), script_error_location(error));
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
