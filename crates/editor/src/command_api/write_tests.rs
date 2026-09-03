//! Stage B write-path contracts: every verb runs through `CommandHistory`
//! (so a script's edits are GUI-undoable), the per-verb refusals record
//! nothing, and a batch is one undo entry.

use ecs::behavior::Behavior;
use ecs::sprite_components::{Name, Sprite};
use ecs::World;
use glam::Vec2;
use physics::components::{Collider, ColliderShape};
use serde_json::Value;

use super::write::{ApiBatch, WriteCtx};
use super::{parse_line, ApiError, HostedWrite, QueryCtx, Request, WriteCmd};
use crate::commands::CommandHistory;
use crate::play_state::EditorPlayState;
use crate::selection::Selection;
use crate::test_support::named_entity;

const PLAYER_POS: Vec2 = Vec2::new(1.0, 2.0);

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
        named_entity(&mut self.world, "Player", PLAYER_POS)
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
            // The rig's "resolver" issued only the built-in #white.
            texture_known: &|handle| handle == 0,
        };
        super::write::run(&write, &mut ctx)
    }

    fn position(&self, entity: ecs::EntityId) -> Vec2 {
        self.world.get::<common::Transform2D>(entity).expect("transform").position
    }
}

ecs::define_component! {
    /// Stand-in game component for the dynamic-tier API tests.
    pub struct ApiDynTestBuff {
        pub strength: f32 = 1.0,
        pub active: bool = true,
    }
}

// ==================== set ====================

#[test]
fn test_set_patch_shallow_merges_and_each_line_is_one_undo_entry() -> Result<(), ApiError> {
    let mut rig = Rig::new();
    let e = rig.spawn_player();
    rig.world.add_component(&e, common::Camera::default()).ok();
    rig.world
        .add_component(
            &e,
            Behavior::PlayerPlatformer {
                move_speed: 100.0,
                jump_impulse: 50.0,
                jump_cooldown: 0.2,
                tag: "p1".into(),
            },
        )
        .ok();

    rig.run(r#"set Player Transform2D {"position": [40.0, 5.0]}"#)?;
    let t = rig.world.get::<common::Transform2D>(e).expect("transform");
    assert_eq!(t.position, Vec2::new(40.0, 5.0));
    assert_eq!(t.scale, Vec2::ONE, "unpatched fields survive the shallow merge");
    rig.run(r#"set Player Transform2D {"position": [20.0, 0.0]}"#)?;

    // A type with no typed Set*Command goes through the generic path.
    rig.run(r#"set Player Camera {"zoom": 2.5}"#)?;
    assert_eq!(rig.world.get::<common::Camera>(e).expect("camera").zoom, 2.5);

    // An externally-tagged enum patch replaces the whole value — that IS
    // how a script switches an entity's behavior variant.
    rig.run(r#"set Player Behavior {"FollowTagged": {"target_tag": "p1", "follow_distance": 30.0, "follow_speed": 90.0}}"#)?;
    assert!(
        matches!(rig.world.get::<Behavior>(e), Some(Behavior::FollowTagged { target_tag, .. }) if target_tag == "p1"),
        "the variant switches"
    );

    assert!(rig.history.undo(&mut rig.world), "each set line is its own undo entry");
    assert!(
        matches!(rig.world.get::<Behavior>(e), Some(Behavior::PlayerPlatformer { .. })),
        "undo restores the old variant"
    );
    assert!(rig.history.undo(&mut rig.world));
    assert_eq!(rig.world.get::<common::Camera>(e).expect("camera").zoom, 1.0);
    assert!(rig.history.undo(&mut rig.world));
    assert_eq!(rig.position(e), Vec2::new(40.0, 5.0), "the second set undoes on its own");
    assert!(rig.history.undo(&mut rig.world));
    assert_eq!(rig.position(e), PLAYER_POS, "the first set undoes to the original");
    assert!(!rig.history.can_undo());
    Ok(())
}

#[test]
fn test_set_refusals_name_the_fix_and_record_nothing() {
    let mut rig = Rig::new();
    let e = rig.spawn_player();

    let err = rig
        .run(r#"set Player Transform2D {"positon": [1.0, 1.0]}"#)
        .expect_err("unknown field");
    let ApiError::Invalid(msg) = err else {
        panic!("expected Invalid, got {err:?}");
    };
    assert!(msg.contains("positon"), "names the bad field: {msg}");
    assert!(msg.contains("position"), "lists the real fields: {msg}");

    let err = rig.run(r#"set Player Sprite {"depth": 1.0}"#).expect_err("absent component");
    assert!(matches!(err, ApiError::Invalid(msg) if msg.contains("add")), "directs to `add`");

    // Name goes through `rename` (validated) — never `set`.
    let err = rig.run(r#"set Player Name "Sneaky""#).expect_err("name via set");
    assert!(matches!(err, ApiError::Invalid(msg) if msg.contains("rename")), "directs to `rename`");

    assert_eq!(rig.position(e), PLAYER_POS, "the world is untouched");
    assert!(!rig.history.can_undo(), "a rejected set records nothing");
}

#[test]
fn test_set_rejects_non_finite_numbers() {
    let mut rig = Rig::new();
    let e = rig.spawn_player();

    // serde_json can't represent bare inf/nan literals, but a huge float
    // that parses to inf can arrive via 1e999 — reject it.
    let result = rig.run(r#"set Player Transform2D {"rotation": 1e999}"#);

    assert!(result.is_err(), "non-finite input must not reach the world");
    assert_eq!(rig.world.get::<common::Transform2D>(e).expect("transform").rotation, 0.0);
    assert!(!rig.history.can_undo());
}

#[test]
fn test_unissued_texture_handle_is_refused_at_set_and_add() -> Result<(), ApiError> {
    // #66: a handle the resolver never issued would save as `#texture_999`
    // and only fail on the next load — refuse it at the write instead.
    let mut rig = Rig::new();
    let e = rig.spawn_player();

    let err = rig.run(r#"add Player Sprite {"texture_handle": 42}"#).expect_err("unissued on add");
    assert!(matches!(err, ApiError::Invalid(_)), "{err:?}");
    assert!(rig.world.get::<Sprite>(e).is_none(), "the add is rolled back, no component left");
    assert!(!rig.history.can_undo(), "nothing recorded");

    // The built-in #white (handle 0) is always issued.
    rig.run(r#"add Player Sprite {"texture_handle": 0, "depth": 2.0}"#)?;
    assert_eq!(rig.world.get::<Sprite>(e).expect("sprite").depth, 2.0);
    let top_before = rig.history.undo_name().map(str::to_string);

    let err = rig.run(r#"set Player Sprite {"texture_handle": 999}"#).expect_err("unissued handle");
    assert!(matches!(err, ApiError::Invalid(ref msg) if msg.contains("999")), "{err:?}");
    assert_eq!(rig.world.get::<Sprite>(e).expect("sprite").texture_handle, 0, "world untouched");
    assert_eq!(rig.history.undo_name().map(str::to_string), top_before, "nothing recorded");

    // `Tilemap.tileset` lives in the same handle space.
    rig.run("add Player Tilemap")?;
    let err = rig.run(r#"set Player Tilemap {"tileset": 7}"#).expect_err("unissued tileset");
    assert!(matches!(err, ApiError::Invalid(ref msg) if msg.contains("7")), "{err:?}");
    assert_eq!(rig.world.get::<ecs::Tilemap>(e).expect("tilemap").tileset, 0, "world untouched");
    Ok(())
}

#[test]
fn test_set_sanitizes_collider_extents_to_the_gui_floor() -> Result<(), ApiError> {
    let mut rig = Rig::new();
    let e = rig.spawn_player();
    rig.world
        .add_component(&e, Collider::new(ColliderShape::Box { half_extents: Vec2::new(10.0, 10.0) }))
        .ok();

    rig.run(r#"set Player Collider {"shape": {"Box": {"half_extents": [0.0, 0.0]}}}"#)?;

    assert_eq!(
        rig.world.get::<Collider>(e).expect("collider").shape,
        ColliderShape::Box { half_extents: Vec2::new(0.5, 0.5) },
        "hard floor mirrors the GUI"
    );
    Ok(())
}

// ==================== add / remove / rename / delete / select ====================

#[test]
fn test_add_and_remove_round_trip_values_through_undo() -> Result<(), ApiError> {
    let mut rig = Rig::new();
    let e = rig.spawn_player();

    rig.run(r#"add Player Sprite {"depth": 7.0}"#)?;
    assert_eq!(rig.world.get::<Sprite>(e).expect("sprite").depth, 7.0);
    let err = rig.run("add Player Sprite").expect_err("duplicate add");
    assert!(matches!(err, ApiError::Invalid(msg) if msg.contains("set")), "directs to `set`");

    rig.run("remove Player Sprite")?;
    assert!(rig.world.get::<Sprite>(e).is_none());

    assert!(rig.history.undo(&mut rig.world));
    assert_eq!(
        rig.world.get::<Sprite>(e).expect("sprite").depth,
        7.0,
        "undo restores the removed component's VALUE, not a default"
    );
    assert!(rig.history.undo(&mut rig.world), "add+patch is ONE undo entry");
    assert!(rig.world.get::<Sprite>(e).is_none());
    assert!(!rig.history.can_undo());
    Ok(())
}

#[test]
fn test_rename_reaches_unnamed_entities_and_undo_restores_no_name() -> Result<(), ApiError> {
    let mut rig = Rig::new();
    let e = rig.world.create_entity();

    let out = rig.run(&format!("rename #{} Crate", e.value()))?;

    assert_eq!(out["entity"]["name"], Value::String("Crate".into()));
    assert_eq!(rig.world.get::<Name>(e).map(Name::as_str), Some("Crate"));
    assert!(rig.history.undo(&mut rig.world));
    assert!(rig.world.get::<Name>(e).is_none(), "undo restores no-Name");
    Ok(())
}

#[test]
fn test_delete_drops_the_selection_and_undo_resurrects_both() -> Result<(), ApiError> {
    let mut rig = Rig::new();
    let e = rig.spawn_player();
    rig.selection.select(e);

    rig.run("delete Player")?;
    assert!(rig.world.get_entity(&e).is_err());
    assert!(rig.selection.is_empty(), "selection drops the deleted entity");

    rig.run("undo")?;
    assert!(rig.world.get_entity(&e).is_ok(), "undo resurrects");
    assert_eq!(rig.world.get::<Name>(e).map(Name::as_str), Some("Player"));
    assert!(rig.selection.contains(e), "undoing the delete restores the pre-delete selection");
    Ok(())
}

#[test]
fn test_select_updates_the_selection_without_an_undo_entry() -> Result<(), ApiError> {
    let mut rig = Rig::new();
    let e = rig.spawn_player();

    rig.run("select Player")?;
    assert_eq!(rig.selection.primary(), Some(e));
    assert!(!rig.history.can_undo(), "selection is never on the undo stack");

    rig.run("select none")?;
    assert!(rig.selection.primary().is_none());
    Ok(())
}

// ==================== undo / redo verbs ====================

#[test]
fn test_undo_and_redo_verbs_name_the_command_and_report_null_when_empty() -> Result<(), ApiError> {
    let mut rig = Rig::new();
    rig.spawn_player();

    assert_eq!(rig.run("undo")?["undid"], Value::Null, "empty stack is a null, not an error");
    assert_eq!(rig.run("redo")?["redid"], Value::Null);

    rig.run(r#"set Player Transform2D {"rotation": 1.0}"#)?;
    assert_eq!(rig.run("undo")?["undid"], Value::String("Set Transform2D (API)".into()));
    assert_eq!(rig.run("redo")?["redid"], Value::String("Set Transform2D (API)".into()));
    Ok(())
}

// ==================== batches ====================

#[test]
fn test_batch_is_one_undo_entry_carrying_the_pre_batch_selection() -> Result<(), ApiError> {
    let mut rig = Rig::new();
    let e = rig.spawn_player();
    rig.selection.select(e);

    rig.run("batch begin setup")?;
    rig.run(r#"set Player Transform2D {"position": [5.0, 5.0]}"#)?;
    rig.run("add Player Sprite")?;
    // A frame boundary overwrites the pending note while the batch is
    // open — the user deselected everything mid-batch.
    rig.selection.clear();
    rig.history.note_selection(&rig.selection);
    let out = rig.run("batch end")?;
    assert_eq!(out["commands"], Value::Number(2.into()));

    rig.run("undo")?;
    assert!(rig.world.get::<Sprite>(e).is_none(), "the whole batch is ONE entry");
    assert_eq!(rig.position(e), PLAYER_POS);
    assert!(!rig.history.can_undo());
    assert!(
        rig.selection.contains(e),
        "undoing the batch restores the PRE-BATCH selection, not the frame note"
    );
    Ok(())
}

#[test]
fn test_batch_abort_rolls_back_in_reverse_and_records_nothing() -> Result<(), ApiError> {
    let mut rig = Rig::new();
    let e = rig.spawn_player();

    rig.run("batch begin oops")?;
    rig.run(r#"set Player Transform2D {"position": [9.0, 9.0]}"#)?;
    rig.run("add Player Sprite")?;
    let out = rig.run("batch abort")?;

    assert_eq!(out["aborted"], Value::Number(2.into()));
    assert!(rig.world.get::<Sprite>(e).is_none(), "abort undid the add");
    assert_eq!(rig.position(e), PLAYER_POS, "abort undid the set");
    assert!(!rig.history.can_undo(), "an aborted batch records nothing");
    Ok(())
}

#[test]
fn test_batch_refuses_unbalanced_delimiters_and_mid_batch_undo() -> Result<(), ApiError> {
    let mut rig = Rig::new();
    rig.spawn_player();

    assert!(matches!(rig.run("batch end"), Err(ApiError::Refused(_))));
    assert!(matches!(rig.run("batch abort"), Err(ApiError::Refused(_))));

    rig.run("batch begin a")?;
    assert!(matches!(rig.run("batch begin b"), Err(ApiError::Refused(_))));
    // Undoing mid-batch would desync the batch's commands from the world.
    rig.run(r#"set Player Transform2D {"rotation": 1.0}"#)?;
    assert!(matches!(rig.run("undo"), Err(ApiError::Refused(_))));
    assert!(matches!(rig.run("redo"), Err(ApiError::Refused(_))));

    rig.run("batch abort")?;
    assert_eq!(rig.run("undo")?["undid"], Value::Null, "after abort the stack is empty again");
    Ok(())
}

// ==================== guards ====================

#[test]
fn test_writes_refused_while_playing_and_allowed_while_paused() {
    let mut rig = Rig::new();
    rig.spawn_player();
    rig.play_state = EditorPlayState::Playing;

    let err = rig.run(r#"set Player Transform2D {"rotation": 1.0}"#).expect_err("playing");
    assert!(matches!(err, ApiError::Refused(_)));
    assert!(!rig.history.can_undo());

    // Paused edits stay allowed — inspector parity.
    rig.play_state = EditorPlayState::Paused;
    assert!(rig.run(r#"set Player Transform2D {"rotation": 1.0}"#).is_ok());

    // The read-only dispatch (query transport) never performs a write.
    let world = World::new();
    let selection = Selection::new();
    let ctx = QueryCtx {
        world: &world,
        selection: &selection,
        scene_path: None,
        dirty: false,
        play_state: EditorPlayState::Editing,
    };
    let response = super::dispatch_line("delete Player", &ctx).expect("response owed");
    assert!(response.contains("\"refused\""), "{response}");
}

#[test]
fn test_create_parses_to_a_hosted_write_and_refuses_unknown_archetypes() {
    let parsed = parse_line("create sprite Crate 100 40").expect("valid create");

    let Request::Write(WriteCmd::Hosted(HostedWrite::Create { archetype, name, position })) = parsed
    else {
        panic!("expected hosted create, got {parsed:?}");
    };
    assert_eq!(archetype, "sprite");
    assert_eq!(name.as_deref(), Some("Crate"));
    assert_eq!(position, Some((100.0, 40.0)));
    assert!(matches!(parse_line("create flying-toaster"), Err(ApiError::Invalid(_))));
}

// ==================== dynamic tier ====================

#[test]
fn test_add_set_remove_reach_game_registered_components() -> Result<(), ApiError> {
    ecs::register_components(|r| r.register::<ApiDynTestBuff>());
    let mut rig = Rig::new();
    let entity = rig.spawn_player();

    let err = rig.run("add Player NoSuchComponent").expect_err("unknown name");
    let ApiError::Invalid(msg) = err else {
        panic!("expected Invalid");
    };
    assert!(msg.contains("ApiDynTestBuff"), "the error lists dynamic names: {msg}");

    rig.run(r#"add Player ApiDynTestBuff {"strength": 4.0}"#)?;
    assert_eq!(rig.world.get::<ApiDynTestBuff>(entity).map(|b| b.strength), Some(4.0));

    rig.run(r#"set Player ApiDynTestBuff {"active": false}"#)?;
    let buff = rig.world.get::<ApiDynTestBuff>(entity).expect("still present");
    assert!(!buff.active);
    assert_eq!(buff.strength, 4.0, "merge keeps unpatched fields");

    rig.run("remove Player ApiDynTestBuff")?;
    assert!(rig.world.get::<ApiDynTestBuff>(entity).is_none());

    // The whole chain unwinds through CommandHistory.
    rig.run("undo")?;
    assert_eq!(rig.world.get::<ApiDynTestBuff>(entity).map(|b| b.active), Some(false));
    rig.run("undo")?;
    assert_eq!(rig.world.get::<ApiDynTestBuff>(entity).map(|b| b.active), Some(true));
    rig.run("undo")?;
    assert!(rig.world.get::<ApiDynTestBuff>(entity).is_none());
    Ok(())
}
