//! A `Scripts` pseudo-row's ways to open its script: a double-click files the script's
//! source file, at no particular line, and a right press opens the context menu over it.
//! A script with no source path is a native one, compiled into the game, and says so.
//! The inspector's script rows are text fields, where a double-click selects a word;
//! they keep the Open Source button.

use common::clock::Instant;
use ecs::EntityId;
use glam::Vec2;

use crate::double_click::DoubleClick;
use crate::EditorContext;

/// Which button pressed a pseudo-row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptRowButton {
    Left,
    Right,
}

/// A press on a pseudo-row this frame, reported by the hierarchy's render.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptRowPress {
    pub entity: EntityId,
    pub index: usize,
    pub script_id: String,
    pub source_path: String,
    pub button: ScriptRowButton,
    pub position: Vec2,
}

/// The last left press on a pseudo-row, by its entity and its index in `Scripts`.
#[derive(Debug, Default)]
pub struct ScriptRowPresses {
    last_press: DoubleClick<(EntityId, usize)>,
}

/// Act on this frame's pseudo-row presses: a double-click files its script's source
/// file; a right press opens the context menu at the press, its items disabled on the
/// web (`desktop` false); either on a native script says it has no source file.
pub fn open_from_script_rows(editor: &mut EditorContext, presses: Vec<ScriptRowPress>, at: Instant, desktop: bool) {
    if editor.drag_drop.suppresses_click() {
        editor.hierarchy.script_presses.last_press.forget();
    }
    for press in presses {
        let opens = match press.button {
            ScriptRowButton::Left => {
                editor.hierarchy.script_presses.last_press.press((press.entity, press.index), at, press.position)
            }
            ScriptRowButton::Right => true,
        };
        if !opens {
            continue;
        }
        let source_path = press.source_path.trim();
        if source_path.is_empty() {
            editor
                .status_bar
                .show_message(format!("{} is a native script: it has no source file to open", press.script_id));
        } else if press.button == ScriptRowButton::Left {
            editor.open_requests.push(crate::open_file_request(source_path));
        } else {
            let config = &editor.external_tools;
            editor.context_menu.open_for_file(press.position, source_path, config, desktop);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external_tools::{Location, OpenRequest};
    use crate::test_support::entity;

    fn press(source_path: &str, button: ScriptRowButton) -> ScriptRowPress {
        ScriptRowPress {
            entity: entity(3),
            index: 0,
            script_id: "paddle".to_string(),
            source_path: source_path.to_string(),
            button,
            position: Vec2::new(40.0, 60.0),
        }
    }

    #[test]
    fn test_a_double_click_on_a_pseudo_row_files_its_scripts_source_and_a_native_one_says_it_has_none() {
        let mut editor = EditorContext::new();
        let at = Instant::now();
        for _ in 0..2 {
            open_from_script_rows(&mut editor, vec![press("scripts/paddle.rhai", ScriptRowButton::Left)], at, true);
        }
        assert_eq!(
            editor.open_requests,
            vec![OpenRequest { path: "scripts/paddle.rhai".into(), location: Location::None }],
            "one double-click, one request"
        );

        for _ in 0..2 {
            open_from_script_rows(&mut editor, vec![press("", ScriptRowButton::Left)], at, true);
        }
        assert_eq!(editor.open_requests.len(), 1, "a native script files nothing");
        assert_eq!(
            editor.status_bar.message(),
            Some("paddle is a native script: it has no source file to open")
        );
    }

    #[test]
    fn test_two_left_presses_drawn_on_a_pseudo_row_file_its_source_once() {
        use crate::test_support::{press_at, release};

        let mut world = ecs::World::new();
        let owner = world.create_entity();
        let mut script = ecs::ScriptRef::new("paddle");
        script.source_path = "scripts/paddle.rhai".to_string();
        world.add_component(&owner, ecs::Scripts(vec![script])).ok();
        let mut editor = EditorContext::new();
        let mut ui = ui::UIContext::new();
        let mut input = input::InputHandler::new();
        let bounds = common::Rect::new(0.0, 0.0, 220.0, 120.0);
        // The entity's row runs from the padding to one line below it; the pseudo-row is next.
        let on_script_row = Vec2::new(50.0, crate::layout::PADDING + crate::layout::LINE_HEIGHT * 1.5);

        let render = |editor: &mut EditorContext, ui: &mut ui::UIContext| {
            let response = editor.hierarchy.render(
                ui, &world, &mut editor.selection, bounds, &editor.theme, &mut editor.drag_drop,
            );
            open_from_script_rows(editor, response.script_presses, Instant::now(), true);
        };
        for _ in 0..2 {
            press_at(&mut ui, &mut input, on_script_row, |ui| render(&mut editor, ui));
            release(&mut ui, &mut input, |ui| render(&mut editor, ui));
        }

        assert_eq!(
            editor.open_requests,
            vec![OpenRequest { path: "scripts/paddle.rhai".into(), location: Location::None }]
        );
    }

    #[test]
    fn test_a_right_press_on_a_pseudo_row_opens_the_context_menu_over_its_source() {
        let mut editor = EditorContext::new();
        open_from_script_rows(&mut editor, vec![press("scripts/paddle.rhai", ScriptRowButton::Right)], Instant::now(), true);
        assert_eq!(editor.context_menu.labels(), vec!["Open with the system default", "Open containing folder"]);
        assert!(editor.open_requests.is_empty(), "the menu files nothing until an item is chosen");
    }
}
