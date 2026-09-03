//! The Play→Stop world snapshot: Stop hands the author back the world they
//! pressed Play on — same ids, same hierarchy, same values — and announces
//! exactly what it cannot bring back.

use super::*;
use ecs::behavior::{Behavior, BehaviorState, EntityTag};
use ecs::tilemap::Tilemap;
use ecs::ui_components::{UiButton, UiLabel, UiPanel};
use ecs::WorldHierarchyExt;
use glam::Vec2;
use physics::components::{Collider, RigidBody};

const TILES: [u32; 6] = [1, 0, 2, 0, 3, 0];

#[test]
fn test_snapshot_restore_rebuilds_entities_hierarchy_and_values_under_original_ids() {
    let mut world = World::new();
    let parent = world.create_entity();
    let child = world.create_entity();
    world.set_parent(child, parent).expect("reparent");
    world.add_component(&parent, common::Transform2D::new(Vec2::new(10.0, 20.0))).ok();
    world.add_component(&child, common::Transform2D::new(Vec2::new(30.0, 40.0))).ok();
    let mut body = RigidBody::default();
    body.gravity_scale = 0.5;
    world.add_component(&parent, body).ok();
    let mut collider = Collider::default();
    collider.friction = 0.9;
    collider.is_sensor = true;
    world.add_component(&parent, collider).ok();
    world.add_component(&parent, UiLabel { text: "@hud.score".into(), ..Default::default() }).ok();
    world.add_component(&parent, UiPanel { border_width: 3.0, ..Default::default() }).ok();
    world.add_component(&parent, UiButton { id: "play".into(), ..Default::default() }).ok();
    world
        .add_component(
            &child,
            Behavior::PlayerPlatformer { move_speed: 150.0, jump_impulse: 500.0, jump_cooldown: 0.25, tag: "hero".to_string() },
        )
        .ok();
    world
        .add_component(&child, BehaviorState { timer: 1.5, look_offset: Vec2::new(120.0, -40.0), ..Default::default() })
        .ok();
    world.add_component(&child, EntityTag::new("hero")).ok();
    world
        .add_component(
            &child,
            Tilemap { width: 3, height: 2, tile_size: 16.0, tileset: 7, tiles: TILES.to_vec(), ..Default::default() },
        )
        .ok();

    let snapshot = WorldSnapshot::capture(&world);
    assert!(snapshot.loss_warning().is_none(), "registry types and Parent/Children never false-positive");
    world.clear();
    assert_eq!(world.entity_count(), 0);
    snapshot.restore(&mut world);

    assert_eq!(world.entity_count(), 2);
    assert_eq!(world.get::<common::Transform2D>(parent).expect("parent transform").position, Vec2::new(10.0, 20.0));
    assert_eq!(world.get::<common::Transform2D>(child).expect("child transform").position, Vec2::new(30.0, 40.0));
    // Parent/Children are rebuilt explicitly — they are not registry types.
    assert_eq!(world.get::<Parent>(child).expect("child keeps its parent").entity(), parent);
    assert_eq!(world.get::<Children>(parent).expect("parent keeps its children").entities(), &[child]);
    // Value fidelity per family: physics, data-driven UI, behavior + its
    // runtime state (the camera look-ahead must survive Play/Stop), tilemap.
    assert_eq!(world.get::<RigidBody>(parent).expect("body").gravity_scale, 0.5);
    let collider = world.get::<Collider>(parent).expect("collider");
    assert_eq!((collider.friction, collider.is_sensor), (0.9, true));
    assert_eq!(world.get::<UiLabel>(parent).expect("label").text, "@hud.score");
    assert_eq!(world.get::<UiPanel>(parent).expect("panel").border_width, 3.0);
    assert_eq!(world.get::<UiButton>(parent).expect("button").id, "play");
    assert!(
        matches!(world.get::<Behavior>(child), Some(Behavior::PlayerPlatformer { move_speed, tag, .. }) if *move_speed == 150.0 && tag == "hero"),
        "behavior variant and fields survive"
    );
    let state = world.get::<BehaviorState>(child).expect("behavior state");
    assert_eq!((state.timer, state.look_offset), (1.5, Vec2::new(120.0, -40.0)));
    assert!(world.get::<EntityTag>(child).expect("tag").matches("hero"));
    let tilemap = world.get::<Tilemap>(child).expect("a painted tilemap survives Play -> Stop");
    assert_eq!((tilemap.width, tilemap.tileset), (3, 7));
    assert_eq!(tilemap.tiles, TILES.to_vec());
}

#[test]
fn test_snapshot_restore_discards_play_session_changes() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::ZERO)).ok();
    let snapshot = WorldSnapshot::capture(&world);

    // Gameplay moved the entity and spawned another.
    if let Some(t) = world.get_mut::<common::Transform2D>(entity) {
        t.position = Vec2::new(999.0, 999.0);
    }
    let spawned = world.create_entity();
    world.add_component(&spawned, common::Transform2D::new(Vec2::ONE)).ok();
    snapshot.restore(&mut world);

    assert_eq!(world.entity_count(), 1, "the spawned entity is gone");
    assert!(world.get_entity(&spawned).is_err());
    assert_eq!(world.get::<common::Transform2D>(entity).expect("transform").position, Vec2::ZERO);
}

#[test]
fn test_snapshot_reports_unregistered_types_once_and_drops_only_them_on_restore() {
    struct EnemyAi;
    let mut world = World::new();
    let e1 = world.create_entity();
    let e2 = world.create_entity();
    world.add_component(&e1, common::Transform2D::new(Vec2::new(5.0, 6.0))).ok();
    world.add_component(&e1, EnemyAi).ok();
    world.add_component(&e2, EnemyAi).ok();

    let snapshot = WorldSnapshot::capture(&world);

    // Present on two entities, reported once — announced BEFORE Stop.
    assert_eq!(snapshot.uncaptured_types().len(), 1);
    assert!(snapshot.uncaptured_types()[0].contains("EnemyAi"));
    let warning = snapshot.loss_warning().expect("loss must be announced");
    assert!(warning.contains("EnemyAi") && warning.contains("lost on Stop"), "{warning}");
    let report = snapshot.drop_report().expect("drop must be reported");
    assert!(report.contains("EnemyAi") && report.contains("dropped 1"), "{report}");

    // The entity and its registry components survive; the unregistered
    // component is the documented loss.
    world.clear();
    snapshot.restore(&mut world);
    assert_eq!(world.get::<common::Transform2D>(e1).expect("transform").position, Vec2::new(5.0, 6.0));
    assert!(world.get::<EnemyAi>(e1).is_none());
    assert!(world.get_entity(&e2).is_ok(), "an entity with only lost components still comes back");

    // Reports use short names unless two full paths collide.
    assert_eq!(
        display_names(&["game::enemy::Ai", "game::player::Ai", "game::Brain<u32>"]),
        vec!["game::enemy::Ai".to_string(), "game::player::Ai".to_string(), "Brain".to_string()]
    );
}
