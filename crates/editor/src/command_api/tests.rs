//! Command API Stage A contracts: name-first entity resolution, the query
//! payload shapes, and the response envelope a script parses.

use ecs::sprite_components::Sprite;
use ecs::World;
use glam::Vec2;
use serde_json::Value;

use super::*;
use crate::test_support::named_entity;

const PLAYER_POS: Vec2 = Vec2::new(3.0, 4.0);

fn ctx<'a>(world: &'a World, selection: &'a Selection) -> QueryCtx<'a> {
    QueryCtx {
        world,
        selection,
        scene_path: Some(std::path::Path::new("assets/scenes/level.ron")),
        dirty: false,
        play_state: EditorPlayState::Editing,
    }
}

fn dispatch_json(line: &str, ctx: &QueryCtx<'_>) -> Value {
    let response = dispatch_line(line, ctx).expect("non-blank line gets a response");
    serde_json::from_str(&response).expect("response is valid JSON")
}

fn dispatch_data(line: &str, ctx: &QueryCtx<'_>) -> Value {
    let parsed = dispatch_json(line, ctx);
    assert_eq!(parsed["ok"], Value::Bool(true), "expected success: {parsed}");
    parsed["data"].clone()
}

fn dispatch_error(line: &str, ctx: &QueryCtx<'_>) -> Value {
    let parsed = dispatch_json(line, ctx);
    assert_eq!(parsed["ok"], Value::Bool(false), "expected error: {parsed}");
    parsed["error"].clone()
}

#[test]
fn test_list_and_describe_surface_name_only_as_the_top_level_field() {
    let mut world = World::new();
    let named = named_entity(&mut world, "Left Paddle", PLAYER_POS);
    let anonymous = world.create_entity();
    world.add_component(&anonymous, Sprite::new(0)).ok();
    let selection = Selection::default();
    let ctx = ctx(&world, &selection);

    let listed = dispatch_data("list", &ctx);
    let entities = listed["entities"].as_array().expect("entities array");
    assert_eq!(entities.len(), 2, "every entity is listed");
    let paddle = entities
        .iter()
        .find(|e| e["name"] == "Left Paddle")
        .expect("named entity listed");
    assert_eq!(paddle["id"], named.value());
    assert_eq!(paddle["display"], "Left Paddle");
    let anon = entities
        .iter()
        .find(|e| e["name"].is_null())
        .expect("anonymous entity listed");
    assert_eq!(anon["display"], format!("Sprite (Entity {})", anonymous.value()));

    let filtered = dispatch_data("list PADDLE", &ctx);
    let filtered = filtered["entities"].as_array().expect("entities array");
    assert_eq!(filtered.len(), 1, "the filter is case-insensitive");
    assert_eq!(filtered[0]["name"], "Left Paddle");

    // A quoted name keeps its spaces and `#id` addresses by id. Name is an
    // editable registry component, but describe surfaces it ONLY as the
    // record's top-level field — never as a duplicate component entry.
    let by_id = format!("describe #{}", named.value());
    for line in ["describe \"Left Paddle\"", by_id.as_str()] {
        let data = dispatch_data(line, &ctx);
        assert_eq!(data["name"], "Left Paddle", "{line}");
        assert_eq!(
            data["components"]["Transform2D"]["position"][0],
            PLAYER_POS.x,
            "registry component values come through: {line}"
        );
        let components = data["components"].as_object().expect("components object");
        assert!(
            !components.contains_key("Name"),
            "Name must not appear as a component: {line}"
        );
    }
}

#[test]
fn test_selection_and_scene_queries_report_session_state() {
    let mut world = World::new();
    let first = named_entity(&mut world, "A", PLAYER_POS);
    let second = named_entity(&mut world, "B", PLAYER_POS);
    let mut selection = Selection::default();
    selection.select(first);
    selection.toggle(second);

    let data = dispatch_data("selection", &ctx(&world, &selection));
    assert_eq!(data["primary"]["name"], "A");
    let all = data["all"].as_array().expect("all array");
    assert_eq!(all.len(), 2);
    assert_eq!(all[0]["name"], "A", "selection order is insertion order");
    assert_eq!(all[1]["name"], "B");

    let empty = Selection::default();
    let data = dispatch_data("selection", &ctx(&world, &empty));
    assert!(data["primary"].is_null(), "no selection reads as null, not an error");

    let data = dispatch_data("scene", &ctx(&world, &empty));
    assert_eq!(data["path"], "assets/scenes/level.ron");
    assert_eq!(data["dirty"], false);
    assert_eq!(data["entity_count"], 2);
    assert_eq!(data["play_state"], "editing");
}

#[test]
fn test_error_envelope_carries_kind_message_and_matches_per_typed_error() {
    let mut world = World::new();
    let brick_a = named_entity(&mut world, "Brick", PLAYER_POS);
    let brick_b = named_entity(&mut world, "Brick", PLAYER_POS);
    let selection = Selection::default();
    let ctx = ctx(&world, &selection);

    for line in ["frobnicate", "scene extra", "describe \"unterminated", "describe #nope"] {
        assert_eq!(dispatch_error(line, &ctx)["kind"], "parse", "{line}");
    }

    let missing_id = format!("#{}", brick_b.value() + 999);
    for reference in ["Ghost", missing_id.as_str()] {
        let error = dispatch_error(&format!("describe {reference}"), &ctx);
        assert_eq!(error["kind"], "not_found", "{reference}");
        assert!(
            error["message"].as_str().expect("message").contains(reference),
            "the message names the missing reference: {error}"
        );
    }

    let error = dispatch_error("describe Brick", &ctx);
    assert_eq!(error["kind"], "ambiguous_name");
    let matches: Vec<u64> = error["matches"]
        .as_array()
        .expect("matches")
        .iter()
        .map(|m| m.as_u64().expect("id"))
        .collect();
    assert_eq!(matches, vec![brick_a.value(), brick_b.value()], "every match is listed");
}

#[test]
fn test_every_response_is_one_line_and_blank_input_gets_none() {
    let mut world = World::new();
    named_entity(&mut world, "Player", PLAYER_POS);
    let selection = Selection::default();
    let ctx = ctx(&world, &selection);

    for line in ["list", "describe Player", "selection", "scene", "bogus"] {
        let response = dispatch_line(line, &ctx).expect("response owed");
        assert!(!response.contains('\n'), "response must be one line: {response}");
    }
    assert_eq!(dispatch_line("   ", &ctx), None, "a blank line is not a request");
}
