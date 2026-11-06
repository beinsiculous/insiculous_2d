use super::*;
use std::sync::mpsc::sync_channel;

#[test]
fn test_validate_bridge_path_rules() {
    let root = Path::new("/playground/v1/assets/projects/pong");

    // Valid relative paths
    assert_eq!(
        validate_bridge_path(root, "scenes/main.scene.ron"),
        Ok(root.join("scenes/main.scene.ron"))
    );
    assert_eq!(
        validate_bridge_path(root, "scripts/sub/ball.rhai"),
        Ok(root.join("scripts/sub/ball.rhai"))
    );

    // Refused cases
    assert!(validate_bridge_path(root, "").is_err());
    assert!(validate_bridge_path(root, "/etc/passwd").is_err());
    assert!(validate_bridge_path(root, "\\windows\\system32").is_err());
    assert!(validate_bridge_path(root, "../sibling/file").is_err());
    assert!(validate_bridge_path(root, "scenes/../../outside").is_err());
}

#[test]
fn test_can_dispatch_refuses_empty_whitespace_and_full_channel() {
    let (request_sender, _request_receiver) = sync_channel::<String>(2);

    // Refuses empty/whitespace
    assert!(!can_dispatch("", &request_sender));
    assert!(!can_dispatch("   ", &request_sender));
    assert!(!can_dispatch("\t\n", &request_sender));

    // Accepts valid lines until capacity
    assert!(can_dispatch("query", &request_sender));
    assert!(can_dispatch("select 1", &request_sender));

    // Queue full (capacity 2): refuses next line
    assert!(!can_dispatch("create player", &request_sender));
}

#[test]
fn test_dirty_or_combination() {
    assert!(!dirty_or(false, false));
    assert!(dirty_or(true, false));
    assert!(dirty_or(false, true));
    assert!(dirty_or(true, true));
}

#[test]
fn test_relative_path_is_safe_rules() {
    assert!(relative_path_is_safe("assets/scenes/main.scene.ron"));
    assert!(relative_path_is_safe("project.ron"));
    assert!(!relative_path_is_safe(""));
    assert!(!relative_path_is_safe("/etc/passwd"));
    assert!(!relative_path_is_safe("\\windows\\system32"));
    assert!(!relative_path_is_safe("../escaped"));
    assert!(!relative_path_is_safe("assets/../../outside"));
    assert!(!relative_path_is_safe("C:drive"));
}
#[test]
fn test_file_changes_drain_once_without_consuming_command_responses() {
    let (request_sender, request_receiver) = sync_channel(2);
    let (response_sender, response_receiver) = sync_channel(2);
    let root = PathBuf::from("/projects/pong");
    setup_bridge(request_sender.clone(), response_receiver, root.clone());
    assert!(can_dispatch("query", &request_sender));
    response_sender.send("answer".to_string()).unwrap();

    record_file_change(&root.join("assets/scripts/paddle.rhai"));
    record_file_change(&root.join("assets/scenes/levels/.keep"));
    record_file_change(&root.join("assets/scripts/paddle.rhai"));
    record_file_change(Path::new("/projects/pong2/assets/scripts/other.rhai"));
    record_file_change(&root.join("../other.rhai"));
    record_file_change(&root);
    assert_eq!(take_file_changes(), vec!["assets/scripts/paddle.rhai", "assets/scenes/levels/.keep"]);
    assert!(take_file_changes().is_empty());
    assert_eq!(request_receiver.try_recv().unwrap(), "query");
    RESPONSE_RECEIVER.with(|receiver| {
        assert_eq!(receiver.borrow().as_ref().unwrap().try_recv().unwrap(), "answer");
    });

    record_file_change(&root.join("assets/scripts/paddle.rhai"));
    assert_eq!(take_file_changes(), vec!["assets/scripts/paddle.rhai"]);
}
