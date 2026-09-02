//! Integration tests: InputMapping's binding table, over a game-defined action enum.

use input::prelude::*;

/// Games define their own action types — InputMapping is generic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TestAction {
    Jump,
    Shoot,
    Pause,
}

#[test]
fn test_new_mapping_binds_nothing_implicitly() {
    let mapping: InputMapping<TestAction> = InputMapping::new();
    let mut input = InputHandler::new();
    input.queue_event(InputEvent::KeyPressed(KeyCode::Space));
    input.process_queued_events();

    assert!(mapping.is_empty());
    assert!(!mapping.has_binding(TestAction::Jump));
    assert!(mapping.bindings(TestAction::Jump).is_empty());
    assert!(!mapping.is_active(TestAction::Jump, &input));
    assert!(!mapping.just_activated(TestAction::Jump, &input));
}

#[test]
fn test_bindings_are_many_to_many_deduplicated_and_unbind_in_every_form() {
    let mut mapping = InputMapping::new();
    let space = InputSource::Keyboard(KeyCode::Space);
    let pad_a = InputSource::Gamepad(0, GamepadButton::A);
    let enter = InputSource::Keyboard(KeyCode::Enter);
    let click = InputSource::Mouse(MouseButton::Left);

    // Many sources per action, many actions per source; a repeated pair collapses
    mapping.bind(TestAction::Jump, space);
    mapping.bind(TestAction::Jump, space);
    mapping.bind(TestAction::Jump, pad_a);
    mapping.bind(TestAction::Shoot, space);
    mapping.bind(TestAction::Pause, enter);
    mapping.bind(TestAction::Pause, click);
    assert!(mapping.has_binding(TestAction::Jump));
    assert_eq!(mapping.bindings(TestAction::Jump), &[space, pad_a]);
    let space_actions = mapping.actions_for(&space);
    assert_eq!(space_actions.len(), 2);
    assert!(space_actions.contains(&TestAction::Jump));
    assert!(space_actions.contains(&TestAction::Shoot));
    let mut input = InputHandler::new();
    input.queue_event(InputEvent::KeyPressed(KeyCode::Space));
    input.process_queued_events();
    assert!(mapping.is_active(TestAction::Jump, &input), "the shared source drives Jump");
    assert!(mapping.is_active(TestAction::Shoot, &input), "and Shoot");
    assert!(!mapping.is_active(TestAction::Pause, &input));

    // Unbinding one source keeps the rest; unbinding the last removes the action
    mapping.unbind(TestAction::Jump, &space);
    assert_eq!(mapping.bindings(TestAction::Jump), &[pad_a]);
    mapping.unbind(TestAction::Jump, &pad_a);
    assert!(!mapping.has_binding(TestAction::Jump));

    // unbind_source strips a source from every action and leaves the others alone
    mapping.bind(TestAction::Jump, space);
    mapping.unbind_source(&space);
    assert!(!mapping.has_binding(TestAction::Jump));
    assert!(!mapping.has_binding(TestAction::Shoot));
    assert!(mapping.actions_for(&space).is_empty());
    assert_eq!(mapping.bindings(TestAction::Pause), &[enter, click]);

    // unbind_action drops every source of one action; clear empties the table
    mapping.unbind_action(TestAction::Pause);
    assert!(!mapping.has_binding(TestAction::Pause));
    mapping.bind(TestAction::Shoot, click);
    mapping.clear();
    assert!(mapping.is_empty());
}
