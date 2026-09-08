//! Open script source file in external IDE / editor.

use std::path::Path;
use engine_core::Game;

impl<G: Game> super::EditorGame<G> {
    /// If an "Open source" request was triggered this frame, open the script's
    /// source file in the user's configured IDE or display a status hint.
    pub(super) fn open_pending_source(&mut self) {
        let Some(source_path) = self.editor.pending_open_source.take() else {
            return;
        };

        let resolved = self.resolve_asset_path(Path::new(&source_path));
        let ide_command = self
            .last_saved_prefs
            .as_ref()
            .and_then(|prefs| prefs.ide_command.as_deref())
            .map(str::trim)
            .filter(|cmd| !cmd.is_empty());

        let Some(command_str) = ide_command else {
            self.editor.status_bar.show_message(format!(
                "Source: {} (set ide_command in the editor prefs to open it)",
                resolved.display()
            ));
            return;
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut parts = command_str.split_whitespace();
            if let Some(prog) = parts.next() {
                let rest: Vec<&str> = parts.collect();
                match std::process::Command::new(prog)
                    .args(&rest)
                    .arg(&resolved)
                    .spawn()
                {
                    Ok(mut child) => {
                        // IDE launchers fork and exit at once; an unwaited
                        // child stays a zombie for the editor's lifetime.
                        std::thread::spawn(move || {
                            let _ = child.wait();
                        });
                        self.editor.status_bar.show_message(format!(
                            "Opened {} with {}",
                            resolved.display(),
                            prog
                        ));
                    }
                    Err(error) => {
                        self.editor.status_bar.show_error(format!(
                            "Failed to open {} with {}: {}",
                            resolved.display(),
                            prog,
                            error
                        ));
                    }
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = (command_str, resolved);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::editor_game;
    use editor::EditorPreferences;
    use std::path::PathBuf;

    #[test]
    fn test_open_pending_source_without_ide_command_shows_hint() {
        let mut session = editor_game();
        session.asset_base = PathBuf::from("my_game");
        session.editor.pending_open_source = Some("scripts/player.rhai".to_string());
        session.last_saved_prefs = Some(EditorPreferences::default());

        session.open_pending_source();

        assert_eq!(
            session.editor.status_bar.message(),
            Some("Source: my_game/scripts/player.rhai (set ide_command in the editor prefs to open it)")
        );
        assert!(session.editor.pending_open_source.is_none());
    }

    #[test]
    fn test_open_pending_source_with_whitespace_ide_command_shows_hint() {
        let mut session = editor_game();
        session.asset_base = PathBuf::from("my_game");
        session.editor.pending_open_source = Some("scripts/enemy.rhai".to_string());
        let prefs = EditorPreferences {
            ide_command: Some("   \t  ".to_string()),
            ..Default::default()
        };
        session.last_saved_prefs = Some(prefs);

        session.open_pending_source();

        assert_eq!(
            session.editor.status_bar.message(),
            Some("Source: my_game/scripts/enemy.rhai (set ide_command in the editor prefs to open it)")
        );
        assert!(session.editor.pending_open_source.is_none());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_open_pending_source_with_failed_spawn_shows_error() {
        let mut session = editor_game();
        session.editor.pending_open_source = Some("scripts/test.rhai".to_string());
        let prefs = EditorPreferences {
            ide_command: Some("nonexistent_command_that_cannot_be_found_xyz123".to_string()),
            ..Default::default()
        };
        session.last_saved_prefs = Some(prefs);

        session.open_pending_source();

        assert!(session.editor.status_bar.message().is_some());
        let msg = session.editor.status_bar.message().unwrap();
        assert!(msg.starts_with("Failed to open scripts/test.rhai with nonexistent_command_that_cannot_be_found_xyz123"));
    }
}
