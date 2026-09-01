//! Self-description for the command API (`commands` query): a static doc
//! table plus the LIVE component/archetype name lists, so an agent can
//! discover the editor's vocabulary without reading source. Drift between
//! this table, the parser, and the registries is locked by tests.

use super::parse::ARCHETYPES;
use crate::stored_component::{settable_component_names, ComponentKind};

/// One verb's documentation row.
pub struct CommandDoc {
    pub id: &'static str,
    pub usage: &'static str,
    pub example: &'static str,
    pub summary: &'static str,
    /// Mutates editor state.
    pub writes: bool,
    /// Lands on the undo stack (as if clicked in the GUI).
    pub undoable: bool,
}

/// Every verb, documented. Kept in the parser's verb order.
pub fn command_docs() -> &'static [CommandDoc] {
    const DOCS: &[CommandDoc] = &[
        CommandDoc {
            id: "list",
            usage: "list [filter]",
            example: "list player",
            summary: "Entities whose display name contains the filter (case-insensitive); no filter = all.",
            writes: false,
            undoable: false,
        },
        CommandDoc {
            id: "describe",
            usage: "describe <entity>",
            example: "describe Player",
            summary: "Every registry component of one entity as JSON (name/display/id at top level).",
            writes: false,
            undoable: false,
        },
        CommandDoc {
            id: "selection",
            usage: "selection",
            example: "selection",
            summary: "Current selection (primary + all, insertion order).",
            writes: false,
            undoable: false,
        },
        CommandDoc {
            id: "scene",
            usage: "scene",
            example: "scene",
            summary: "Scene path, dirty state, entity count, play state.",
            writes: false,
            undoable: false,
        },
        CommandDoc {
            id: "commands",
            usage: "commands",
            example: "commands",
            summary: "This self-description, plus live component and archetype lists.",
            writes: false,
            undoable: false,
        },
        CommandDoc {
            id: "set",
            usage: "set <entity> <Component> <json>",
            example: "set Player Transform2D {\"position\": [40.0, 0.0]}",
            summary: "Shallow-patch a component with raw JSON (unknown fields are an error; enums replace whole; unissued texture handles are refused). One undo entry per line.",
            writes: true,
            undoable: true,
        },
        CommandDoc {
            id: "add",
            usage: "add <entity> <Component> [json]",
            example: "add Player Sprite {\"depth\": 1.0}",
            summary: "Add a component (default-valued, optionally patched; a patch with an unissued texture handle is refused) — one undo entry.",
            writes: true,
            undoable: true,
        },
        CommandDoc {
            id: "remove",
            usage: "remove <entity> <Component>",
            example: "remove Player Collider",
            summary: "Remove a component (undo restores its value).",
            writes: true,
            undoable: true,
        },
        CommandDoc {
            id: "rename",
            usage: "rename <entity> <name>",
            example: "rename #7 Goal",
            summary: "Assign or replace the entity's Name (works on unnamed entities; undo restores no-Name).",
            writes: true,
            undoable: true,
        },
        CommandDoc {
            id: "create",
            usage: "create <archetype> [name] [x y]",
            example: "create sprite Crate 100 40",
            summary: "Spawn an archetype at the viewport center or an explicit position — one undo entry.",
            writes: true,
            undoable: true,
        },
        CommandDoc {
            id: "delete",
            usage: "delete <entity>",
            example: "delete Crate",
            summary: "Delete an entity (children reparent to the grandparent; undo resurrects).",
            writes: true,
            undoable: true,
        },
        CommandDoc {
            id: "select",
            usage: "select <entity>|none",
            example: "select Player",
            summary: "Replace the selection (never on the undo stack — GUI parity).",
            writes: true,
            undoable: false,
        },
        CommandDoc {
            id: "undo",
            usage: "undo",
            example: "undo",
            summary: "Undo the top command; responds {\"undid\": name|null}.",
            writes: true,
            undoable: false,
        },
        CommandDoc {
            id: "redo",
            usage: "redo",
            example: "redo",
            summary: "Redo the last undone command; responds {\"redid\": name|null}.",
            writes: true,
            undoable: false,
        },
        CommandDoc {
            id: "save",
            usage: "save [path]",
            example: "save",
            summary: "Save the scene through the editor's save choke point (refused mid-play-session or with an open batch).",
            writes: true,
            undoable: false,
        },
        CommandDoc {
            id: "batch",
            usage: "batch begin [name] | batch end | batch abort",
            example: "batch begin level-setup",
            summary: "Group following writes into ONE undo entry (end) or reverse-undo them (abort). Not a transaction: a mid-batch error leaves earlier effects applied.",
            writes: true,
            undoable: false,
        },
    ];
    DOCS
}

/// The `commands` query payload: docs + live vocabularies.
pub(super) fn commands_response() -> serde_json::Value {
    let commands: Vec<serde_json::Value> = command_docs()
        .iter()
        .map(|d| {
            serde_json::json!({
                "id": d.id,
                "usage": d.usage,
                "example": d.example,
                "summary": d.summary,
                "writes": d.writes,
                "undoable": d.undoable,
            })
        })
        .collect();
    let addable: Vec<&'static str> =
        ComponentKind::ALL.iter().map(|k| k.display_name()).collect();
    serde_json::json!({
        "commands": commands,
        "settable": settable_component_names(),
        "addable": addable,
        "archetypes": ARCHETYPES,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::parse::VERBS;
    use crate::command_api::parse_line;

    #[test]
    fn test_every_doc_example_parses() {
        for doc in command_docs() {
            assert!(
                parse_line(doc.example).is_ok(),
                "example for \"{}\" fails to parse: {}",
                doc.id,
                doc.example
            );
        }
    }

    #[test]
    fn test_parser_verbs_match_docs() {
        // The parser's verb list and the doc table are the same set —
        // adding a verb without documenting it (or vice versa) breaks here.
        let doc_ids: Vec<&str> = command_docs().iter().map(|d| d.id).collect();
        for verb in VERBS {
            assert!(doc_ids.contains(&verb), "verb \"{verb}\" has no CommandDoc");
        }
        for id in &doc_ids {
            assert!(VERBS.contains(id), "doc \"{id}\" is not a parser verb");
        }
    }
}
