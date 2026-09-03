use std::path::Path;

use engine_core::scene_data::SceneData;

/// Why `ComponentData::GridBackdrop` is still a struct variant with hand-listed
/// serde defaults (`scene_data/grid_defaults.rs`): collapsing it to a newtype
/// over `ecs::GridBackdrop` needs ron's `UNWRAP_VARIANT_NEWTYPES`, and that
/// extension also unwraps `Option::Some`, so every `physics: Some(PhysicsSettings(..))`
/// in an existing scene stops parsing. If this test ever fails because the
/// extension-enabled parse succeeds, the newtype collapse has become possible.
#[test]
fn ron_unwrap_variant_newtypes_still_breaks_existing_scenes_so_grid_backdrop_stays_a_struct_variant() {
    let scene_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/assets/scenes/hello_world.scene.ron");
    let content = std::fs::read_to_string(&scene_path).expect("hello_world.scene.ron must exist");

    assert!(
        ron::Options::default().from_str::<SceneData>(&content).is_ok(),
        "the control: today's scene parses with the default options"
    );
    let unwrapping = ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::UNWRAP_VARIANT_NEWTYPES);
    assert!(
        unwrapping.from_str::<SceneData>(&content).is_err(),
        "the extension no longer breaks existing scenes: revisit collapsing GridBackdrop to a newtype variant"
    );
}
