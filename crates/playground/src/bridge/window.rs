//! Two read-only questions the page asks around moving the editor to its own window:
//! whether this page's edits outlive it, and which project an archive would import as.
//! Neither changes anything.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// The slug an archive imports as, through the importer's whole validation: an archive
/// the import would refuse has no slug to fence, and the refusal is the import's own.
#[cfg(any(target_arch = "wasm32", test))]
fn archive_slug(bytes: &[u8], bundle_version: &str) -> Result<String, String> {
    crate::archive::import_project(bytes, bundle_version)
        .map(|(manifest, _)| manifest.slug)
        .map_err(|error| error.to_string())
}

/// Whether the store the engine chose at boot keeps edits past this page. False for a
/// page that fell back to memory, whose only copy of its edits would go with it.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_storage_persists() -> bool {
    crate::web_entry::storage_persists()
}

/// The project an archive would import as, read before anything is written, so the
/// page can take that project's lock first.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_archive_slug(bytes: Vec<u8>) -> Result<String, JsValue> {
    archive_slug(&bytes, crate::web_entry::BUNDLE_VERSION).map_err(|error| JsValue::from_str(&error))
}

#[cfg(test)]
mod tests {
    use super::archive_slug;
    use crate::projects::{ProjectManifest, ProjectOrigin};

    #[test]
    fn an_exported_archive_names_its_slug_and_a_refused_one_names_the_refusal() {
        let directory = tempfile::tempdir().expect("temporary directory");
        common::vfs::write(&directory.path().join("assets/scenes/main.scene.ron"), b"SceneData(name: \"x\")")
            .expect("scene written");
        let manifest = ProjectManifest {
            slug: "my-game".to_string(),
            title: "My Game".to_string(),
            bundle_version: "v6".to_string(),
            content_hash: String::new(),
            origin: ProjectOrigin::Imported,
        };
        let archive = crate::archive::export_project(directory.path(), &manifest).expect("exported");

        assert_eq!(archive_slug(&archive, "v6"), Ok("my-game".to_string()));
        assert!(archive_slug(b"not a zip", "v6").is_err(), "an archive the import refuses names no project");
    }
}
