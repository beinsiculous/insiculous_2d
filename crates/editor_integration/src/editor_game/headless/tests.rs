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
    /// Stand-in game component for the headless dynamic-tier test.
    pub struct HeadlessDynTestMarker {
        pub level: f32 = 1.0,
    }
}

#[test]
fn test_full_authoring_loop_survives_a_reload() -> std::io::Result<()> {
    // Dynamic components must be registered by the hosting process — the
    // headless session sees whatever main() registered.
    ecs::register_components(|r| r.register::<HeadlessDynTestMarker>());
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("ship_point.ron");

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
    // mutation AND the dynamic component survived the round trip, and an
    // unknown verb is one error line that does not end the session.
    let responses = run_session(Some(path.clone()), "scene\nfrobnicate\ndescribe Hero\n");
    assert_eq!(responses[0]["ok"], true);
    assert!(
        responses[0]["data"]["path"]
            .as_str()
            .is_some_and(|p| p.ends_with("ship_point.ron")),
        "the opened scene's path is reported (the #53 seam): {}",
        responses[0]
    );
    assert_eq!(responses[1]["ok"], false, "unknown verb is one error line");
    let hero = &responses[2]["data"];
    assert_eq!(hero["components"]["Transform2D"]["rotation"], 0.5);
    assert_eq!(hero["components"]["HeadlessDynTestMarker"]["level"], 7.0);
    Ok(())
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
fn test_unissued_texture_handle_is_refused_and_never_reaches_the_file() -> std::io::Result<()> {
    // #66: only handles the session's resolver issued may be written. The
    // refusal is an ordinary error line; the session continues and the
    // built-in #white (handle 0) still saves.
    let dir = tempfile::tempdir()?;
    let scene = dir.path().join("refused.scene.ron");
    let script = format!(
        "create sprite Hero\nset Hero Sprite {{\"texture_handle\": 999}}\nset Hero Sprite {{\"texture_handle\": 0}}\nsave {}\n",
        scene.display()
    );

    let responses = run_session(None, &script);

    assert_eq!(responses.len(), 4, "{responses:?}");
    assert_eq!(responses[0]["ok"], true, "create: {}", responses[0]);
    assert_eq!(responses[1]["ok"], false, "unissued handle: {}", responses[1]);
    assert_eq!(responses[1]["error"]["kind"], "invalid", "{}", responses[1]);
    assert_eq!(responses[2]["ok"], true, "#white is always issued: {}", responses[2]);
    assert_eq!(responses[3]["ok"], true, "save: {}", responses[3]);
    let saved = std::fs::read_to_string(&scene)?;
    assert!(!saved.contains("#texture_"), "no placeholder ref may be saved: {saved}");
    Ok(())
}

#[test]
fn test_headless_assets_round_trip_references_verbatim() {
    let mut assets = HeadlessAssets::new();

    let white = assets.resolve_texture("#white").expect("built-in resolves");
    let solid = assets.resolve_texture("#solid:FF00FF").expect("solid resolves");
    let png = assets.resolve_texture("sprites/deion_16.png").expect("a path resolves");

    assert_eq!(white.id, 0, "#white is the built-in handle 0");
    assert_ne!(solid.id, png.id);
    assert_eq!(assets.resolve_texture("#solid:FF00FF").map(|h| h.id).ok(), Some(solid.id), "same ref, same handle");
    // The serializer's inverse returns the ref VERBATIM.
    assert_eq!(assets.texture_path(solid.id), Some("#solid:FF00FF"));
    assert_eq!(assets.texture_path(png.id), Some("sprites/deion_16.png"));
    assert_eq!(assets.texture_path(999), None);
}
