//! Script catalog construction combining native registry behaviors and project Rhai scripts.

use std::collections::BTreeMap;
use std::path::Path;

use editor::{AssetEntry, AssetKind, ScriptCatalogEntry};
use engine_core::scripting::{parse_param_header, ScriptRegistry};

/// Build the catalog of available scripts from registered native behaviors
/// and scanned project `.rhai` files.
pub fn build_script_catalog(
    entries: &[AssetEntry],
    registry: &ScriptRegistry,
    asset_base: &str,
) -> Vec<ScriptCatalogEntry> {
    let mut catalog = Vec::new();

    for descriptor in registry.descriptors() {
        let mut params = BTreeMap::new();
        for p in descriptor.params {
            params.insert(p.name.to_string(), p.default.clone());
        }
        catalog.push(ScriptCatalogEntry {
            id: descriptor.id.to_string(),
            display_name: descriptor.display_name.to_string(),
            category: descriptor.category.to_string(),
            params,
            source_path: None,
        });
    }

    for entry in entries {
        if entry.kind == AssetKind::Script && entry.relative_path.ends_with(".rhai") {
            let stem = Path::new(&entry.relative_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&entry.name);

            let full_path = if asset_base.is_empty() {
                Path::new(&entry.relative_path).to_path_buf()
            } else {
                Path::new(asset_base).join(&entry.relative_path)
            };

            let params = match common::vfs::read_to_string(&full_path) {
                Ok(source) => match parse_param_header(&source) {
                    Ok(parsed) => parsed,
                    Err(err) => {
                        log::warn!(
                            "Failed to parse script header for '{}': {err}",
                            entry.relative_path
                        );
                        BTreeMap::new()
                    }
                },
                Err(err) => {
                    log::warn!(
                        "Failed to read script file for '{}': {err}",
                        entry.relative_path
                    );
                    BTreeMap::new()
                }
            };

            catalog.push(ScriptCatalogEntry {
                id: stem.to_string(),
                display_name: stem.to_string(),
                category: "Project".to_string(),
                params,
                source_path: Some(entry.relative_path.clone()),
            });
        }
    }

    catalog
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecs::script::ScriptValue;

    #[test]
    fn test_build_script_catalog_lists_builtin_and_rhai() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let scripts_dir = temp_dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).expect("create scripts dir");
        let script_file = scripts_dir.join("mover.rhai");
        std::fs::write(
            &script_file,
            "// @param speed: f32 = 120.0\nfn update(me, view, params, out, dt) {}\n",
        )
        .expect("write script");

        let entries = editor::scan_assets(temp_dir.path());
        let registry = ScriptRegistry::new();
        let catalog = build_script_catalog(&entries, &registry, temp_dir.path().to_str().expect("path str"));

        assert!(catalog.iter().any(|e| e.id == "engine::rotate" && e.params.contains_key("degrees_per_second")));
        let mover = catalog.iter().find(|e| e.id == "mover").expect("mover script in catalog");
        assert_eq!(mover.params.get("speed"), Some(&ScriptValue::F32(120.0)));
        assert_eq!(mover.source_path, Some("scripts/mover.rhai".to_string()));
    }
}
