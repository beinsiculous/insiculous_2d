//! Tests for the play-mode world snapshot (capture/restore round-trips).

use super::*;
use ecs::behavior::{Behavior, BehaviorState, EntityTag};
use ecs::tilemap::Tilemap;
use ecs::ui_components::{UiButton, UiLabel, UiPanel};
use glam::Vec2;
use physics::components::{Collider, RigidBody};

#[test]
fn test_snapshot_empty_world() {
    let world = World::new();
    let snapshot = WorldSnapshot::capture(&world);
    assert_eq!(snapshot.entity_count(), 0);
}

#[test]
fn test_snapshot_captures_entities() {
    let mut world = World::new();
    world.create_entity();
    world.create_entity();
    world.create_entity();

    let snapshot = WorldSnapshot::capture(&world);
    assert_eq!(snapshot.entity_count(), 3);
}

#[test]
fn test_snapshot_restore_preserves_entity_ids() {
    let mut world = World::new();
    let e1 = world.create_entity();
    let e2 = world.create_entity();
    world.add_component(&e1, common::Transform2D::new(Vec2::new(10.0, 20.0))).ok();
    world.add_component(&e2, common::Transform2D::new(Vec2::new(30.0, 40.0))).ok();

    let snapshot = WorldSnapshot::capture(&world);

    // Modify world
    world.clear();
    assert_eq!(world.entity_count(), 0);

    // Restore
    snapshot.restore(&mut world);

    assert_eq!(world.entity_count(), 2);
    let t1 = world.get::<common::Transform2D>(e1).unwrap();
    assert_eq!(t1.position, Vec2::new(10.0, 20.0));
    let t2 = world.get::<common::Transform2D>(e2).unwrap();
    assert_eq!(t2.position, Vec2::new(30.0, 40.0));
}

#[test]
fn test_snapshot_restore_discards_play_changes() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::ZERO)).ok();

    let snapshot = WorldSnapshot::capture(&world);

    // Simulate play-mode changes
    if let Some(t) = world.get_mut::<common::Transform2D>(entity) {
        t.position = Vec2::new(999.0, 999.0);
    }
    let new_entity = world.create_entity();
    world.add_component(&new_entity, common::Transform2D::new(Vec2::ONE)).ok();

    // Restore should undo play changes
    snapshot.restore(&mut world);

    assert_eq!(world.entity_count(), 1);
    let t = world.get::<common::Transform2D>(entity).unwrap();
    assert_eq!(t.position, Vec2::ZERO);
}

#[test]
fn test_snapshot_preserves_hierarchy() {
    use ecs::WorldHierarchyExt;

    let mut world = World::new();
    let parent = world.create_entity();
    let child = world.create_entity();
    world.set_parent(child, parent).unwrap();

    let snapshot = WorldSnapshot::capture(&world);
    world.clear();
    snapshot.restore(&mut world);

    // Hierarchy components should be restored
    let p = world.get::<Parent>(child).unwrap();
    assert_eq!(p.entity(), parent);
    let c = world.get::<Children>(parent).unwrap();
    assert!(c.entities().contains(&child));
}

#[test]
fn test_snapshot_preserves_physics_components() {
    let mut world = World::new();
    let entity = world.create_entity();
    let mut body = RigidBody::default();
    body.gravity_scale = 0.5;
    body.linear_damping = 2.0;
    world.add_component(&entity, body).ok();

    let mut collider = Collider::default();
    collider.friction = 0.9;
    collider.is_sensor = true;
    world.add_component(&entity, collider).ok();

    let snapshot = WorldSnapshot::capture(&world);
    world.clear();
    snapshot.restore(&mut world);

    let rb = world.get::<RigidBody>(entity).unwrap();
    assert_eq!(rb.gravity_scale, 0.5);
    assert_eq!(rb.linear_damping, 2.0);

    let col = world.get::<Collider>(entity).unwrap();
    assert_eq!(col.friction, 0.9);
    assert!(col.is_sensor);
}

#[test]
fn test_snapshot_preserves_ui_element_components() {
    let mut world = World::new();
    let entity = world.create_entity();
    world
        .add_component(&entity, UiLabel { text: "@hud.score".into(), ..Default::default() })
        .ok();
    world.add_component(&entity, UiPanel { border_width: 3.0, ..Default::default() }).ok();
    world
        .add_component(&entity, UiButton { id: "play".into(), ..Default::default() })
        .ok();

    let snapshot = WorldSnapshot::capture(&world);
    world.clear();
    snapshot.restore(&mut world);

    assert_eq!(world.get::<UiLabel>(entity).unwrap().text, "@hud.score");
    assert_eq!(world.get::<UiPanel>(entity).unwrap().border_width, 3.0);
    assert_eq!(world.get::<UiButton>(entity).unwrap().id, "play");
}

#[test]
fn test_snapshot_preserves_behavior_components() {
    let mut world = World::new();
    let entity = world.create_entity();
    let behavior = Behavior::PlayerPlatformer {
        move_speed: 150.0,
        jump_impulse: 500.0,
        jump_cooldown: 0.25,
        tag: "hero".to_string(),
    };
    world.add_component(&entity, behavior).ok();

    let state = BehaviorState {
        timer: 1.5,
        look_offset: glam::Vec2::new(120.0, -40.0),
        ..Default::default()
    };
    world.add_component(&entity, state).ok();

    world.add_component(&entity, EntityTag::new("hero")).ok();

    let snapshot = WorldSnapshot::capture(&world);
    world.clear();
    snapshot.restore(&mut world);

    // Behavior should survive the snapshot round-trip
    let b = world.get::<Behavior>(entity).unwrap();
    match b {
        Behavior::PlayerPlatformer { move_speed, tag, .. } => {
            assert_eq!(*move_speed, 150.0);
            assert_eq!(tag, "hero");
        }
        _ => panic!("Wrong behavior variant"),
    }

    let bs = world.get::<BehaviorState>(entity).unwrap();
    assert_eq!(bs.timer, 1.5);
    assert_eq!(
        bs.look_offset,
        glam::Vec2::new(120.0, -40.0),
        "camera look-ahead offset must survive play/stop"
    );

    let tag = world.get::<EntityTag>(entity).unwrap();
    assert!(tag.matches("hero"));
}

#[test]
fn test_snapshot_preserves_tilemap() {
    let mut world = World::new();
    let entity = world.create_entity();
    let tilemap = Tilemap {
        width: 3,
        height: 2,
        tile_size: 16.0,
        tileset: 7,
        tiles: vec![1, 0, 2, 0, 3, 0],
        ..Default::default()
    };
    world.add_component(&entity, tilemap).ok();

    let snapshot = WorldSnapshot::capture(&world);
    assert!(snapshot.loss_warning().is_none(), "Tilemap is a registry type");
    world.clear();
    snapshot.restore(&mut world);

    // A painted tilemap must survive Play -> Stop, not be deleted by it.
    let restored = world.get::<Tilemap>(entity).unwrap();
    assert_eq!(restored.width, 3);
    assert_eq!(restored.tileset, 7);
    assert_eq!(restored.tiles, vec![1, 0, 2, 0, 3, 0]);
}

#[test]
fn test_snapshot_reports_unregistered_component_types_once() {
    struct EnemyAi;

    let mut world = World::new();
    let e1 = world.create_entity();
    let e2 = world.create_entity();
    world.add_component(&e1, EnemyAi).ok();
    world.add_component(&e2, EnemyAi).ok();

    let snapshot = WorldSnapshot::capture(&world);

    // Present on two entities, reported once.
    assert_eq!(snapshot.uncaptured_types().len(), 1);
    assert!(snapshot.uncaptured_types()[0].contains("EnemyAi"));

    let warning = snapshot.loss_warning().expect("loss must be announced");
    assert!(warning.contains("EnemyAi"));
    assert!(warning.contains("lost on Stop"));

    let report = snapshot.drop_report().expect("drop must be reported");
    assert!(report.contains("EnemyAi"));
    assert!(report.contains("dropped 1"));
}

#[test]
fn test_snapshot_registry_and_hierarchy_types_not_reported() {
    use ecs::WorldHierarchyExt;

    let mut world = World::new();
    let parent = world.create_entity();
    let child = world.create_entity();
    world.set_parent(child, parent).unwrap();
    world.add_component(&parent, common::Transform2D::default()).ok();
    world.add_component(&parent, ecs::Sprite::default()).ok();
    world.add_component(&child, Tilemap::default()).ok();

    let snapshot = WorldSnapshot::capture(&world);

    // Registry types and Parent/Children must never false-positive.
    assert!(snapshot.uncaptured_types().is_empty());
    assert!(snapshot.loss_warning().is_none());
    assert!(snapshot.drop_report().is_none());
}

#[test]
fn test_restore_drops_unregistered_component_but_keeps_entity() {
    struct EnemyAi;

    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::new(5.0, 6.0))).ok();
    world.add_component(&entity, EnemyAi).ok();

    let snapshot = WorldSnapshot::capture(&world);
    world.clear();
    snapshot.restore(&mut world);

    // The entity and its registry components survive; the unregistered
    // component is the documented loss.
    let t = world.get::<common::Transform2D>(entity).unwrap();
    assert_eq!(t.position, Vec2::new(5.0, 6.0));
    assert!(world.get::<EnemyAi>(entity).is_none());
}

#[test]
fn test_display_names_fall_back_to_full_paths_on_collision() {
    let names = display_names(&["game::enemy::Ai", "game::player::Ai", "game::Brain<u32>"]);
    assert_eq!(
        names,
        vec!["game::enemy::Ai".to_string(), "game::player::Ai".to_string(), "Brain".to_string()]
    );
}
