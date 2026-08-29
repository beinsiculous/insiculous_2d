//! The #45 ship point, proven headless: an agent drives the full authoring
//! loop — query → create → mutate → save — with no window, then a second
//! session reloads the file and reads the result back.

use std::io::Cursor;

use super::*;

fn run_session(scene: Option<PathBuf>, script: &str) -> Vec<serde_json::Value> {
    let mut out: Vec<u8> = Vec::new();
    run_headless_editor_api(None, scene, Cursor::new(script.to_string()), &mut out)
        .expect("session runs to EOF");
    String::from_utf8(out)
        .expect("utf8")
        .lines()
        .map(|l| serde_json::from_str(l).expect("every response is one JSON line"))
        .collect()
}

ecs::define_component! {
    /// Stand-in game component for the headless dynamic-tier test (#45).
    pub struct HeadlessDynTestMarker {
        pub level: f32 = 1.0,
    }
}

#[test]
fn test_full_authoring_loop_survives_a_reload() {
    // Dynamic components must be registered by the hosting process — the
    // headless session sees whatever main() registered (kimi R1-F1).
    ecs::register_components(|r| r.register::<HeadlessDynTestMarker>());

    let path = std::env::temp_dir().join("test_45_headless_ship_point.ron");
    let _ = std::fs::remove_file(&path);

    // Session 1: author a scene from nothing and save it.
    let script = format!(
        "scene\n\
         create sprite Hero 100 50\n\
         set Hero Transform2D {{\"rotation\": 0.5}}\n\
         add Hero HeadlessDynTestMarker {{\"level\": 7.0}}\n\
         save {}\n",
        path.display()
    );
    let responses = run_session(None, &script);
    assert_eq!(responses.len(), 5, "one JSON line per request");
    assert_eq!(responses[0]["data"]["play_state"], "editing");
    assert_eq!(responses[0]["data"]["path"], serde_json::Value::Null);
    for (i, r) in responses.iter().enumerate() {
        assert_eq!(r["ok"], true, "request {i} succeeded: {r}");
    }
    assert!(path.exists(), "save wrote the file");

    // Session 2: a fresh headless session opens the saved file — the
    // mutation AND the dynamic component survived the round trip.
    let responses = run_session(Some(path.clone()), "scene\ndescribe Hero\n");
    assert_eq!(responses[0]["ok"], true);
    assert!(
        responses[0]["data"]["path"]
            .as_str()
            .is_some_and(|p| p.ends_with("test_45_headless_ship_point.ron")),
        "the opened scene's path is reported (the #53 seam): {}",
        responses[0]
    );
    let hero = &responses[1]["data"];
    assert_eq!(hero["components"]["Transform2D"]["rotation"], 0.5);
    assert_eq!(hero["components"]["HeadlessDynTestMarker"]["level"], 7.0);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_unknown_verb_errors_and_the_session_continues() {
    let responses = run_session(None, "frobnicate\nscene\n");
    assert_eq!(responses[0]["ok"], false, "unknown verb is one error line");
    assert_eq!(responses[1]["ok"], true, "the session keeps answering");
}

#[test]
fn test_unreadable_startup_scene_fails_fast() {
    // An agent must never silently author against an empty world it
    // believes is the scene.
    let missing = std::env::temp_dir().join("test_45_no_such_scene.ron");
    let result = run_headless_editor_api(
        None,
        Some(missing),
        Cursor::new(String::new()),
        &mut Vec::new(),
    );
    let err = result.expect_err("missing scene refuses the session");
    assert!(err.contains("test_45_no_such_scene"), "error names the file: {err}");
}

#[test]
fn test_headless_assets_round_trip_references_verbatim() {
    let mut assets = HeadlessAssets::new();
    let white = assets.resolve_texture("#white").unwrap();
    assert_eq!(white.id, 0, "#white is the built-in handle 0");

    let solid = assets.resolve_texture("#solid:FF00FF").unwrap();
    let png = assets.resolve_texture("sprites/deion_16.png").unwrap();
    assert_ne!(solid.id, png.id);
    // Dedup: the same ref yields the same handle.
    assert_eq!(assets.resolve_texture("#solid:FF00FF").unwrap().id, solid.id);
    // The serializer's inverse returns the ref VERBATIM.
    assert_eq!(assets.texture_path(solid.id), Some("#solid:FF00FF"));
    assert_eq!(assets.texture_path(png.id), Some("sprites/deion_16.png"));
    assert_eq!(assets.texture_path(999), None);
}
