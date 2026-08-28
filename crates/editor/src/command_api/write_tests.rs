//! Stage B write-path tests: parse → execute through `CommandHistory`,
//! round-trip undo, batch semantics, and the per-verb guards.

use ecs::sprite_components::{Name, Sprite};
use ecs::World;
use glam::Vec2;
use serde_json::Value;

use super::write::{ApiBatch, WriteCtx};
use super::{parse_line, ApiError, Request, WriteCmd};
use crate::commands::CommandHistory;
use crate::play_state::EditorPlayState;
use crate::selection::Selection;

struct Rig {
    world: World,
    history: CommandHistory,
    selection: Selection,
    batch: Option<ApiBatch>,
    play_state: EditorPlayState,
}

impl Rig {
    fn new() -> Self {
        Self {
            world: World::new(),
            history: CommandHistory::new(),
            selection: Selection::new(),
            batch: None,
            play_state: EditorPlayState::Editing,
        }
    }

    fn spawn_player(&mut self) -> ecs::EntityId {
        let e = self.world.create_entity();
        self.world.add_component(&e, Name::new("Player")).ok();
        self.world
            .add_component(&e, common::Transform2D::new(Vec2::new(1.0, 2.0)))
            .ok();
        e
    }

    /// Parse and run one write line, panicking on a query.
    fn run(&mut self, line: &str) -> Result<Value, ApiError> {
        let request = parse_line(line)?;
        let Request::Write(WriteCmd::Pure(write)) = request else {
            panic!("expected a pure write: {line}");
        };
        let mut ctx = WriteCtx {
            world: &mut self.world,
            history: &mut self.history,
            selection: &mut self.selection,
            play_state: self.play_state,
            batch: &mut self.batch,
        };
        super::write::run(&write, &mut ctx)
    }
}

// ==================== set ====================

#[test]
fn test_set_patch_merges_fields_and_undoes() {
    let mut rig = Rig::new();
    let e = rig.spawn_player();

    rig.run(r#"set Player Transform2D {"position": [40.0, 5.0]}"#).unwrap();
    let t = rig.world.get::<common::Transform2D>(e).unwrap();
    assert_eq!(t.position, Vec2::new(40.0, 5.0));
    assert_eq!(t.scale, Vec2::ONE, "unpatched fields survive the shallow merge");

    assert!(rig.history.undo(&mut rig.world), "the set is one undo entry");
    let t = rig.world.get::<common::Transform2D>(e).unwrap();
    assert_eq!(t.position, Vec2::new(1.0, 2.0), "undo restores the old value");
}

#[test]
fn test_set_unknown_field_lists_valid_keys() {
    let mut rig = Rig::new();
    rig.spawn_player();

    let err = rig.run(r#"set Player Transform2D {"positon": [1.0, 1.0]}"#).unwrap_err();
    match err {
        ApiError::Invalid(msg) => {
            assert!(msg.contains("positon"), "{msg}");
            assert!(msg.contains("position"), "message lists the real fields: {msg}");
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
    assert!(!rig.history.can_undo(), "a rejected set records nothing");
}

#[test]
fn test_set_absent_component_directs_to_add() {
    let mut rig = Rig::new();
    rig.spawn_player();

    let err = rig.run(r#"set Player Sprite {"depth": 1.0}"#).unwrap_err();
    assert!(matches!(err, ApiError::Invalid(msg) if msg.contains("add")));
}

#[test]
fn test_set_readonly_registry_type_camera() {
    // The generic path covers types with no typed Set*Command — the API's
    // edge over the GUI.
    let mut rig = Rig::new();
    let e = rig.spawn_player();
    rig.world.add_component(&e, common::Camera::default()).ok();

    rig.run(r#"set Player Camera {"zoom": 2.5}"#).unwrap();
    assert_eq!(rig.world.get::<common::Camera>(e).unwrap().zoom, 2.5);
    assert!(rig.history.undo(&mut rig.world));
    assert_eq!(rig.world.get::<common::Camera>(e).unwrap().zoom, 1.0);
}

#[test]
fn test_two_sets_are_two_undo_steps() {
    let mut rig = Rig::new();
    let e = rig.spawn_player();

    rig.run(r#"set Player Transform2D {"position": [10.0, 0.0]}"#).unwrap();
    rig.run(r#"set Player Transform2D {"position": [20.0, 0.0]}"#).unwrap();

    assert!(rig.history.undo(&mut rig.world));
    assert_eq!(
        rig.world.get::<common::Transform2D>(e).unwrap().position,
        Vec2::new(10.0, 0.0),
        "each API set line is its own undo entry"
    );
    assert!(rig.history.undo(&mut rig.world));
    assert_eq!(
        rig.world.get::<common::Transform2D>(e).unwrap().position,
        Vec2::new(1.0, 2.0)
    );
}

#[test]
fn test_set_rejects_non_finite_numbers() {
    let mut rig = Rig::new();
    rig.spawn_player();
    // serde_json can't represent bare inf/nan literals, but a huge float
    // that parses to inf can arrive via 1e999 — reject it.
    let err = rig.run(r#"set Player Transform2D {"rotation": 1e999}"#);
    assert!(err.is_err(), "non-finite input must not reach the world");
    assert!(!rig.history.can_undo());
}

#[test]
fn test_set_sanitizes_collider_extents() {
    let mut rig = Rig::new();
    let e = rig.spawn_player();
    rig.world
        .add_component(&e, physics::components::Collider::new(
            physics::components::ColliderShape::Box { half_extents: Vec2::new(10.0, 10.0) },
        ))
        .ok();

    rig.run(r#"set Player Collider {"shape": {"Box": {"half_extents": [0.0, 0.0]}}}"#)
        .unwrap();
    match rig.world.get::<physics::components::Collider>(e).unwrap().shape {
        physics::components::ColliderShape::Box { half_extents } => {
            assert_eq!(half_extents, Vec2::new(0.5, 0.5), "hard floor mirrors the GUI");
        }
        ref other => panic!("expected Box, got {other:?}"),
    }
}

// ==================== add / remove / rename / delete / select ====================

#[test]
fn test_add_with_value_is_single_undo_step() {
    let mut rig = Rig::new();
    let e = rig.spawn_player();

    rig.run(r#"add Player Sprite {"depth": 3.0}"#).unwrap();
    assert_eq!(rig.world.get::<Sprite>(e).unwrap().depth, 3.0);

    assert!(rig.history.undo(&mut rig.world), "add+patch is ONE undo entry");
    assert!(rig.world.get::<Sprite>(e).is_none());
    assert!(!rig.history.can_undo());
}

#[test]
fn test_add_duplicate_component_is_invalid() {
    let mut rig = Rig::new();
    rig.spawn_player();
    rig.run("add Player Sprite").unwrap();
    let err = rig.run("add Player Sprite").unwrap_err();
    assert!(matches!(err, ApiError::Invalid(msg) if msg.contains("set")));
}

#[test]
fn test_remove_undo_restores_value() {
    let mut rig = Rig::new();
    let e = rig.spawn_player();
    rig.run(r#"add Player Sprite {"depth": 7.0}"#).unwrap();

    rig.run("remove Player Sprite").unwrap();
    assert!(rig.world.get::<Sprite>(e).is_none());

    assert!(rig.history.undo(&mut rig.world));
    assert_eq!(
        rig.world.get::<Sprite>(e).unwrap().depth,
        7.0,
        "undo restores the removed component's VALUE, not a default"
    );
}

#[test]
fn test_rename_reaches_unnamed_entities_and_undoes() {
    let mut rig = Rig::new();
    let e = rig.world.create_entity();

    let out = rig.run(&format!("rename #{} Crate", e.value())).unwrap();
    assert_eq!(out["entity"]["name"], Value::String("Crate".into()));

    assert!(rig.history.undo(&mut rig.world));
    assert!(rig.world.get::<Name>(e).is_none(), "undo restores no-Name");
}

#[test]
fn test_delete_undo_resurrects_and_selection_drops_it() {
    let mut rig = Rig::new();
    let e = rig.spawn_player();
    rig.selection.select(e);

    rig.run("delete Player").unwrap();
    assert!(rig.world.get_entity(&e).is_err());
    assert!(rig.selection.primary().is_none(), "selection drops the deleted entity");

    assert!(rig.history.undo(&mut rig.world));
    assert!(rig.world.get_entity(&e).is_ok(), "undo resurrects");
    assert_eq!(rig.world.get::<Name>(e).unwrap().as_str(), "Player");
}

#[test]
fn test_select_updates_selection_without_undo_entry() {
    let mut rig = Rig::new();
    let e = rig.spawn_player();

    rig.run("select Player").unwrap();
    assert_eq!(rig.selection.primary(), Some(e));
    assert!(!rig.history.can_undo(), "selection is never on the undo stack");

    rig.run("select none").unwrap();
    assert!(rig.selection.primary().is_none());
}

// ==================== undo / redo verbs ====================

#[test]
fn test_undo_empty_history_reports_null() {
    let mut rig = Rig::new();
    let out = rig.run("undo").unwrap();
    assert_eq!(out["undid"], Value::Null, "empty stack is a null, not an error");
    let out = rig.run("redo").unwrap();
    assert_eq!(out["redid"], Value::Null);
}

#[test]
fn test_undo_verb_names_the_undone_command() {
    let mut rig = Rig::new();
    rig.spawn_player();
    rig.run(r#"set Player Transform2D {"rotation": 1.0}"#).unwrap();

    let out = rig.run("undo").unwrap();
    assert_eq!(out["undid"], Value::String("Set Transform2D (API)".into()));
}

// ==================== batches ====================

#[test]
fn test_batch_groups_into_one_undo() {
    let mut rig = Rig::new();
    let e = rig.spawn_player();

    rig.run("batch begin setup").unwrap();
    rig.run(r#"set Player Transform2D {"position": [5.0, 5.0]}"#).unwrap();
    rig.run("add Player Sprite").unwrap();
    let out = rig.run("batch end").unwrap();
    assert_eq!(out["commands"], Value::Number(2.into()));

    assert!(rig.history.undo(&mut rig.world), "the whole batch is ONE entry");
    assert!(rig.world.get::<Sprite>(e).is_none());
    assert_eq!(
        rig.world.get::<common::Transform2D>(e).unwrap().position,
        Vec2::new(1.0, 2.0)
    );
    assert!(!rig.history.can_undo());
}

#[test]
fn test_batch_abort_rolls_back_in_reverse() {
    let mut rig = Rig::new();
    let e = rig.spawn_player();

    rig.run("batch begin oops").unwrap();
    rig.run(r#"set Player Transform2D {"position": [9.0, 9.0]}"#).unwrap();
    rig.run("add Player Sprite").unwrap();
    let out = rig.run("batch abort").unwrap();
    assert_eq!(out["aborted"], Value::Number(2.into()));

    assert!(rig.world.get::<Sprite>(e).is_none(), "abort undid the add");
    assert_eq!(
        rig.world.get::<common::Transform2D>(e).unwrap().position,
        Vec2::new(1.0, 2.0),
        "abort undid the set"
    );
    assert!(!rig.history.can_undo(), "an aborted batch records nothing");
}

#[test]
fn test_batch_end_without_begin_is_refused() {
    let mut rig = Rig::new();
    assert!(matches!(rig.run("batch end"), Err(ApiError::Refused(_))));
    assert!(matches!(rig.run("batch abort"), Err(ApiError::Refused(_))));
    rig.run("batch begin a").unwrap();
    assert!(matches!(rig.run("batch begin b"), Err(ApiError::Refused(_))));
}

// ==================== guards ====================

#[test]
fn test_writes_refused_while_playing() {
    let mut rig = Rig::new();
    rig.spawn_player();
    rig.play_state = EditorPlayState::Playing;

    let err = rig.run(r#"set Player Transform2D {"rotation": 1.0}"#).unwrap_err();
    assert!(matches!(err, ApiError::Refused(_)));
    assert!(!rig.history.can_undo());

    // Paused edits stay allowed — inspector parity.
    rig.play_state = EditorPlayState::Paused;
    assert!(rig.run(r#"set Player Transform2D {"rotation": 1.0}"#).is_ok());
}

#[test]
fn test_read_only_dispatch_refuses_writes() {
    let world = World::new();
    let selection = Selection::new();
    let ctx = super::QueryCtx {
        world: &world,
        selection: &selection,
        scene_path: None,
        dirty: false,
        play_state: EditorPlayState::Editing,
    };
    let response = super::dispatch_line("delete Player", &ctx).unwrap();
    assert!(response.contains("\"refused\""), "{response}");
}

// ==================== archetype drift ====================

#[test]
fn test_create_parse_shapes() {
    let ok = parse_line("create sprite Crate 100 40").unwrap();
    match ok {
        Request::Write(WriteCmd::Hosted(super::HostedWrite::Create { archetype, name, position })) => {
            assert_eq!(archetype, "sprite");
            assert_eq!(name.as_deref(), Some("Crate"));
            assert_eq!(position, Some((100.0, 40.0)));
        }
        other => panic!("expected hosted create, got {other:?}"),
    }
    assert!(matches!(parse_line("create flying-toaster"), Err(ApiError::Invalid(_))));
}

// ==================== review fixes (kimi batch-5) ====================

#[test]
fn test_set_behavior_switches_variant_by_whole_replace() {
    // Kimi F1: an externally-tagged enum patch replaces the whole value —
    // that IS how a script switches an entity's behavior variant.
    use ecs::behavior::Behavior;
    let mut rig = Rig::new();
    let e = rig.spawn_player();
    rig.world
        .add_component(&e, Behavior::PlayerPlatformer {
            move_speed: 100.0,
            jump_impulse: 50.0,
            jump_cooldown: 0.2,
            tag: "p1".into(),
        })
        .ok();

    rig.run(r#"set Player Behavior {"FollowTagged": {"target_tag": "p1", "follow_distance": 30.0, "follow_speed": 90.0}}"#)
        .unwrap();
    match rig.world.get::<Behavior>(e).unwrap() {
        Behavior::FollowTagged { target_tag, .. } => assert_eq!(target_tag, "p1"),
        other => panic!("variant must switch, got {other:?}"),
    }

    assert!(rig.history.undo(&mut rig.world));
    assert!(
        matches!(rig.world.get::<Behavior>(e), Some(Behavior::PlayerPlatformer { .. })),
        "undo restores the old variant"
    );
}

#[test]
fn test_set_name_is_rejected_toward_rename() {
    // Kimi F3: Name goes through `rename` (validated) — never `set`.
    let mut rig = Rig::new();
    rig.spawn_player();
    let err = rig.run(r#"set Player Name "Sneaky""#).unwrap_err();
    assert!(matches!(err, ApiError::Invalid(msg) if msg.contains("rename")));
    assert!(!rig.history.can_undo());
}

#[test]
fn test_undo_redo_refused_inside_open_batch() {
    // Kimi F5: undoing mid-batch would desync the batch's collected
    // commands from the world.
    let mut rig = Rig::new();
    rig.spawn_player();
    rig.run("batch begin b").unwrap();
    rig.run(r#"set Player Transform2D {"rotation": 1.0}"#).unwrap();
    assert!(matches!(rig.run("undo"), Err(ApiError::Refused(_))));
    assert!(matches!(rig.run("redo"), Err(ApiError::Refused(_))));
    rig.run("batch abort").unwrap();
    let out = rig.run("undo").unwrap();
    assert_eq!(out["undid"], Value::Null, "after abort the stack is empty again");
}
