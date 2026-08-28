//! Command API Stage A contract tests: parse grammar, name-first entity
//! resolution, query payload shapes, and the response envelope.

use ecs::sprite_components::{Name, Sprite};
use ecs::World;
use glam::Vec2;
use serde_json::Value;

use super::parse::parse_line;
use super::*;

fn named_entity(world: &mut World, name: &str) -> ecs::EntityId {
    let entity = world.create_entity();
    world.add_component(&entity, Name::new(name)).ok();
    world.add_component(&entity, common::Transform2D::new(Vec2::new(3.0, 4.0))).ok();
    entity
}

fn ctx<'a>(world: &'a World, selection: &'a Selection) -> QueryCtx<'a> {
    QueryCtx {
        world,
        selection,
        scene_path: Some(std::path::Path::new("assets/scenes/level.ron")),
        dirty: false,
        play_state: EditorPlayState::Editing,
    }
}

fn dispatch_data(line: &str, ctx: &QueryCtx<'_>) -> Value {
    let response = dispatch_line(line, ctx).expect("non-blank line gets a response");
    let parsed: Value = serde_json::from_str(&response).expect("response is valid JSON");
    assert_eq!(parsed["ok"], Value::Bool(true), "expected success: {response}");
    parsed["data"].clone()
}

fn dispatch_error(line: &str, ctx: &QueryCtx<'_>) -> Value {
    let response = dispatch_line(line, ctx).expect("non-blank line gets a response");
    let parsed: Value = serde_json::from_str(&response).expect("response is valid JSON");
    assert_eq!(parsed["ok"], Value::Bool(false), "expected error: {response}");
    parsed["error"].clone()
}

// ==================== Parsing ====================

#[test]
fn test_parse_list_bare_and_with_filter() {
    assert_eq!(parse_line("list").unwrap(), Request::Query(Query::ListEntities { filter: None }));
    assert_eq!(
        parse_line("list paddle").unwrap(),
        Request::Query(Query::ListEntities { filter: Some("paddle".to_string()) })
    );
}

#[test]
fn test_parse_describe_quoted_name_keeps_spaces() {
    assert_eq!(
        parse_line("describe \"Left Paddle\"").unwrap(),
        Request::Query(Query::Describe { entity: EntityRef::Name("Left Paddle".to_string()) })
    );
}

#[test]
fn test_parse_describe_hash_id() {
    assert_eq!(
        parse_line("describe #42").unwrap(),
        Request::Query(Query::Describe { entity: EntityRef::Id(42) })
    );
    assert!(matches!(parse_line("describe #nope"), Err(ApiError::Parse(_))));
}

#[test]
fn test_parse_unknown_query_and_trailing_tokens_are_errors() {
    assert!(matches!(parse_line("frobnicate"), Err(ApiError::Parse(_))));
    assert!(matches!(parse_line("scene extra"), Err(ApiError::Parse(_))));
    assert!(matches!(parse_line("describe \"unterminated"), Err(ApiError::Parse(_))));
}

#[test]
fn test_blank_line_gets_no_response() {
    let world = World::new();
    let selection = Selection::default();
    assert_eq!(dispatch_line("   ", &ctx(&world, &selection)), None);
}

// ==================== Entity resolution ====================

#[test]
fn test_resolve_name_unique_and_missing() {
    let mut world = World::new();
    let entity = named_entity(&mut world, "Player");

    assert_eq!(EntityRef::Name("Player".to_string()).resolve(&world).unwrap(), entity);
    assert!(matches!(
        EntityRef::Name("Ghost".to_string()).resolve(&world),
        Err(ApiError::NotFound(_))
    ));
}

#[test]
fn test_resolve_name_ambiguous_lists_matches() {
    let mut world = World::new();
    let a = named_entity(&mut world, "Brick");
    let b = named_entity(&mut world, "Brick");

    match EntityRef::Name("Brick".to_string()).resolve(&world) {
        Err(ApiError::AmbiguousName { matches, .. }) => {
            assert_eq!(matches.len(), 2);
            assert!(matches.contains(&a.value()) && matches.contains(&b.value()));
        }
        other => panic!("expected AmbiguousName, got {other:?}"),
    }
}

#[test]
fn test_resolve_id_by_value() {
    let mut world = World::new();
    let entity = named_entity(&mut world, "Player");

    assert_eq!(EntityRef::Id(entity.value()).resolve(&world).unwrap(), entity);
    assert!(matches!(
        EntityRef::Id(entity.value() + 999).resolve(&world),
        Err(ApiError::NotFound(_))
    ));
}

// ==================== Queries ====================

#[test]
fn test_list_reports_ids_names_display() {
    let mut world = World::new();
    let named = named_entity(&mut world, "Player");
    let anonymous = world.create_entity();
    world.add_component(&anonymous, Sprite::new(0)).ok();
    let selection = Selection::default();

    let data = dispatch_data("list", &ctx(&world, &selection));
    let entities = data["entities"].as_array().expect("entities array");
    assert_eq!(entities.len(), 2);

    let player = entities.iter().find(|e| e["name"] == "Player").expect("named entity listed");
    assert_eq!(player["id"], named.value());
    assert_eq!(player["display"], "Player");

    let anon = entities.iter().find(|e| e["name"].is_null()).expect("anonymous entity listed");
    assert_eq!(anon["display"], format!("Sprite (Entity {})", anonymous.value()));
}

#[test]
fn test_list_filter_is_case_insensitive() {
    let mut world = World::new();
    named_entity(&mut world, "Left Paddle");
    named_entity(&mut world, "Ball");
    let selection = Selection::default();

    let data = dispatch_data("list PADDLE", &ctx(&world, &selection));
    let entities = data["entities"].as_array().expect("entities array");
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0]["name"], "Left Paddle");
}

#[test]
fn test_describe_contains_component_values_and_lifts_name() {
    let mut world = World::new();
    named_entity(&mut world, "Player");
    let selection = Selection::default();

    let data = dispatch_data("describe Player", &ctx(&world, &selection));
    assert_eq!(data["name"], "Player");
    // Transform2D is a registry component: its serialized fields come through.
    assert_eq!(data["components"]["Transform2D"]["position"][0], 3.0);
    // Name is a hidden registry entry: top-level field, not a component key.
    assert!(data["components"].get("Name").is_none());
}

#[test]
fn test_selection_reports_primary_and_all() {
    let mut world = World::new();
    let first = named_entity(&mut world, "A");
    let second = named_entity(&mut world, "B");
    let mut selection = Selection::default();
    selection.select(first);
    selection.toggle(second);

    let data = dispatch_data("selection", &ctx(&world, &selection));
    assert_eq!(data["primary"]["name"], "A");
    let all = data["all"].as_array().expect("all array");
    assert_eq!(all.len(), 2);
    assert_eq!(all[0]["name"], "A");
    assert_eq!(all[1]["name"], "B");
}

#[test]
fn test_selection_empty_primary_is_null() {
    let world = World::new();
    let selection = Selection::default();
    let data = dispatch_data("selection", &ctx(&world, &selection));
    assert!(data["primary"].is_null());
}

#[test]
fn test_scene_info_shape() {
    let mut world = World::new();
    named_entity(&mut world, "Player");
    let selection = Selection::default();

    let data = dispatch_data("scene", &ctx(&world, &selection));
    assert_eq!(data["path"], "assets/scenes/level.ron");
    assert_eq!(data["dirty"], false);
    assert_eq!(data["entity_count"], 1);
    assert_eq!(data["play_state"], "editing");
}

// ==================== Envelope ====================

#[test]
fn test_responses_are_single_line() {
    let mut world = World::new();
    named_entity(&mut world, "Player");
    let selection = Selection::default();
    let c = ctx(&world, &selection);

    for line in ["list", "describe Player", "selection", "scene", "bogus"] {
        let response = dispatch_line(line, &c).expect("response owed");
        assert!(!response.contains('\n'), "response must be one line: {response}");
    }
}

#[test]
fn test_error_envelope_kind_and_message() {
    let world = World::new();
    let selection = Selection::default();
    let c = ctx(&world, &selection);

    let error = dispatch_error("describe Ghost", &c);
    assert_eq!(error["kind"], "not_found");
    assert!(error["message"].as_str().expect("message").contains("Ghost"));

    let error = dispatch_error("frobnicate", &c);
    assert_eq!(error["kind"], "parse");
}

#[test]
fn test_ambiguous_name_error_carries_matches() {
    let mut world = World::new();
    named_entity(&mut world, "Brick");
    named_entity(&mut world, "Brick");
    let selection = Selection::default();

    let error = dispatch_error("describe Brick", &ctx(&world, &selection));
    assert_eq!(error["kind"], "ambiguous_name");
    assert_eq!(error["matches"].as_array().expect("matches").len(), 2);
}

// ==================== Name filtering (#32) ====================

#[test]
fn test_describe_omits_name_component_but_keeps_top_level_name() {
    // Name became an editable registry component in #32, so
    // capture_all_values now emits it — but describe must keep surfacing the
    // name ONLY as the record's top-level field (the API's entity address),
    // never as a duplicate component entry.
    let mut world = World::new();
    named_entity(&mut world, "Player");
    let selection = Selection::new();
    let ctx = ctx(&world, &selection);

    let data = dispatch_data("describe Player", &ctx);
    assert_eq!(data["name"], Value::String("Player".into()));
    let components = data["components"].as_object().expect("components object");
    assert!(!components.contains_key("Name"), "Name must not appear as a component");
    assert!(components.contains_key("Transform2D"), "real components still listed");
}
