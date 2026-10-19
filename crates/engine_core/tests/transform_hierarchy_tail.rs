//! The frame tail propagates the transform hierarchy for every game: a
//! plain `run_game` game that parents entities gets real globals with no
//! editor and no system of its own.

use ecs::{EntityId, GlobalTransform2D};
use engine_core::prelude::{Game, GameConfig, GameContext, Transform2D, Vec2, WorldHierarchyExt};
use engine_core::scene_loader::SceneLoader;
use engine_core::test_support::GameHarness;

const FRAME: f32 = 1.0 / 60.0;

fn global_position(harness: &GameHarness<impl Game>, entity: EntityId) -> Option<Vec2> {
    harness.world().get::<GlobalTransform2D>(entity).map(|global| global.position)
}

/// Parents a child under a parent in `init`, and moves the parent on its
/// second `update`.
#[derive(Default)]
struct Parented {
    frames: usize,
    parent: Option<EntityId>,
    child: Option<EntityId>,
}

impl Game for Parented {
    fn init(&mut self, ctx: &mut GameContext) {
        let parent = ctx.world.create_entity();
        ctx.world.add_component(&parent, Transform2D::new(Vec2::new(100.0, 50.0))).ok();
        let child = ctx.world.create_entity();
        ctx.world.add_component(&child, Transform2D::new(Vec2::new(10.0, -5.0))).ok();
        ctx.world.set_parent(child, parent).ok();
        self.parent = Some(parent);
        self.child = Some(child);
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.frames += 1;
        let Some(parent) = self.parent else { return };
        if self.frames == 2 {
            if let Some(transform) = ctx.world.get_mut::<Transform2D>(parent) {
                transform.position = Vec2::new(-200.0, 0.0);
            }
        }
    }
}

#[test]
fn a_parented_childs_global_transform_follows_its_parent_with_no_editor_present() {
    let mut harness = GameHarness::new(Parented::default(), GameConfig::new("Parented"));

    harness.step(FRAME, &[]);
    let child = harness.game().child.expect("init parented the child");
    assert_eq!(
        global_position(&harness, child),
        Some(Vec2::new(110.0, 45.0)),
        "the first frame's tail propagates the parent's position into the child"
    );

    harness.step(FRAME, &[]);
    assert_eq!(
        global_position(&harness, child),
        Some(Vec2::new(-190.0, -5.0)),
        "the child followed the parent the game moved in update, in that frame"
    );
}

const NESTED_SCENE: &str = r#"SceneData(
    name: "Nested",
    entities: [
        EntityData(
            name: Some("parent"),
            components: [Transform2D(position: (30.0, 40.0))],
            children: [EntityData(name: Some("child"), components: [Transform2D(position: (1.0, 2.0))])],
        ),
    ],
)"#;

/// Loads a scene with an inline child in `init`.
#[derive(Default)]
struct SceneGame {
    child: Option<EntityId>,
}

impl Game for SceneGame {
    fn init(&mut self, ctx: &mut GameContext) {
        let data = SceneLoader::parse(NESTED_SCENE).expect("the scene parses");
        let instance = SceneLoader::instantiate(&data, ctx.world, ctx.assets).expect("the scene loads");
        self.child = instance.get_entity("child");
    }

    fn update(&mut self, _ctx: &mut GameContext) {}
}

#[test]
fn a_loaded_scenes_child_gets_its_global_from_the_first_tail_not_from_the_loader() {
    let mut harness = GameHarness::new(SceneGame::default(), GameConfig::new("Scene"));

    // `context` runs `init` and no frame around it.
    let child = harness.context(|game, _ctx| game.child).expect("the scene names its child");
    assert_eq!(
        global_position(&harness, child),
        None,
        "the loader writes no identity global for a parented entity"
    );

    harness.step(FRAME, &[]);
    assert_eq!(
        global_position(&harness, child),
        Some(Vec2::new(31.0, 42.0)),
        "the tail's first propagation adds the real global before the frame draws"
    );
}
