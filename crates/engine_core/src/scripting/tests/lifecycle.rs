//! Lifecycle tests: scripts hot-swapping and early despawn semantics.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

use ecs::blackboard::Blackboard;
use ecs::script::{ScriptRef, ScriptValue, Scripts};


use crate::scripting::commands::{ScriptCommand, ScriptCommands, Target};
use crate::scripting::registry::{ScriptBehavior, ScriptDescriptor};
use crate::scripting::runner::ScriptRunner;
use crate::scripting::view::{ScriptView, SelfView};

use super::{test_inputs, test_world};

struct ValueWriterA;
impl ScriptBehavior for ValueWriterA {
    fn update(
        &mut self,
        _me: &SelfView,
        _view: &ScriptView,
        _params: &BTreeMap<String, ScriptValue>,
        commands: &mut ScriptCommands,
    ) {
        commands.commands.push(ScriptCommand::SetBlackboard {
            key: "key_a".to_string(),
            value: ScriptValue::I32(100),
        });
    }
}

struct ValueWriterB;
impl ScriptBehavior for ValueWriterB {
    fn update(
        &mut self,
        _me: &SelfView,
        _view: &ScriptView,
        _params: &BTreeMap<String, ScriptValue>,
        commands: &mut ScriptCommands,
    ) {
        commands.commands.push(ScriptCommand::SetBlackboard {
            key: "key_b".to_string(),
            value: ScriptValue::I32(200),
        });
    }
}

fn make_writer_a() -> Box<dyn ScriptBehavior> {
    Box::new(ValueWriterA)
}

fn make_writer_b() -> Box<dyn ScriptBehavior> {
    Box::new(ValueWriterB)
}

#[test]
fn test_scripts_edit_between_frames_swaps_refs_runs_each_under_own_ref() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let mut runner = ScriptRunner::new();

    runner.registry_mut().register(ScriptDescriptor {
        id: "script_a",
        display_name: "Script A",
        category: "Test",
        params: &[],
        make: make_writer_a,
    });
    runner.registry_mut().register(ScriptDescriptor {
        id: "script_b",
        display_name: "Script B",
        category: "Test",
        params: &[],
        make: make_writer_b,
    });
    runner.reset(&mut world, "");

    let entity = world.create_entity();
    world
        .add_component(
            &entity,
            Scripts(vec![ScriptRef::new("script_a"), ScriptRef::new("script_b")]),
        )
        .unwrap();

    runner.early_update(&mut world, &input, &players, 0.016, None);
    runner.update(&mut world, &input, &players, 0.016, &[], None);

    assert_eq!(
        world.resource::<Blackboard>().and_then(|b| b.get("key_a")),
        Some(&ScriptValue::I32(100))
    );
    assert_eq!(
        world.resource::<Blackboard>().and_then(|b| b.get("key_b")),
        Some(&ScriptValue::I32(200))
    );

    // Swap refs
    {
        let scripts = world.get_mut::<Scripts>(entity).unwrap();
        scripts.0.swap(0, 1);
    }

    runner.early_update(&mut world, &input, &players, 0.016, None);
    runner.update(&mut world, &input, &players, 0.016, &[], None);

    assert_eq!(
        world.resource::<Blackboard>().and_then(|b| b.get("key_a")),
        Some(&ScriptValue::I32(100))
    );
    assert_eq!(
        world.resource::<Blackboard>().and_then(|b| b.get("key_b")),
        Some(&ScriptValue::I32(200))
    );
}

static DESPAWN_UPDATE_RAN: AtomicBool = AtomicBool::new(false);

struct DespawnerBehavior;

impl ScriptBehavior for DespawnerBehavior {
    fn early_update(
        &mut self,
        me: &SelfView,
        _view: &ScriptView,
        _params: &BTreeMap<String, ScriptValue>,
        commands: &mut ScriptCommands,
    ) {
        commands.commands.push(ScriptCommand::Despawn {
            target: Target::Entity(me.entity),
        });
    }

    fn update(
        &mut self,
        _me: &SelfView,
        _view: &ScriptView,
        _params: &BTreeMap<String, ScriptValue>,
        _commands: &mut ScriptCommands,
    ) {
        DESPAWN_UPDATE_RAN.store(true, Ordering::SeqCst);
    }
}

fn make_despawner() -> Box<dyn ScriptBehavior> {
    Box::new(DespawnerBehavior)
}

#[test]
fn test_entity_despawned_in_early_update_gets_no_update_call() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let mut runner = ScriptRunner::new();

    DESPAWN_UPDATE_RAN.store(false, Ordering::SeqCst);

    runner.registry_mut().register(ScriptDescriptor {
        id: "test::despawner",
        display_name: "Despawner",
        category: "Test",
        params: &[],
        make: make_despawner,
    });
    runner.reset(&mut world, "");

    let entity = world.create_entity();
    world
        .add_component(
            &entity,
            Scripts(vec![ScriptRef::new("test::despawner")]),
        )
        .unwrap();

    runner.early_update(&mut world, &input, &players, 0.016, None);
    assert!(
        world.validate_entity(&entity).is_err(),
        "Entity should be despawned after early_update"
    );

    runner.update(&mut world, &input, &players, 0.016, &[], None);
    assert!(
        !DESPAWN_UPDATE_RAN.load(Ordering::SeqCst),
        "Despawned entity must not run update"
    );
}
