//! Browser entry point for the Web Playground.
//!
//! # Version contract
//! `ASSET_BASE` and `BUNDLE_VERSION` are tied to deployment routes:
//! 1. `ASSET_BASE` = `"/playground/v1/assets"`
//! 2. `BUNDLE_VERSION` = `"v1"`
//! 3. Deployed assets live at `insiculous_web/public/playground/v1/assets/`
//! 4. Projects metadata served from `/playground/v1/assets/projects.json`
//! 5. `scripts/build_wasm.sh` produces `playground/v1/`

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, sync_channel};
use std::sync::Arc;

use editor_integration::{find_first_scene, run_game_with_editor_opts, EditorRunOptions, ProjectHost};
use engine_core::prelude::GameConfig;
use engine_core::web::{init_web_logging, preload_assets, query_param, set_boot_status};
use wasm_bindgen::prelude::*;

use crate::persist::{drive_started_puts, install_listeners, set_active_chains, set_dom_banner, with_active_chains, Chains};
use crate::projects::{project_asset_base, project_root, ProjectManifest, ProjectOrigin};
use crate::store::indexed_db::IndexedDbStore;
use crate::store::memory::MemoryStore;
use crate::store::ProjectStore;

/// Canonical asset base path for the deployed playground.
pub const ASSET_BASE: &str = "/playground/v1/assets";
/// Bundle version contract string.
pub const BUNDLE_VERSION: &str = "v1";

thread_local! {
    static ACTIVE_STORE: RefCell<Option<Arc<dyn ProjectStore>>> = const { RefCell::new(None) };
    static BUNDLED_MANIFESTS: RefCell<Vec<ProjectManifest>> = const { RefCell::new(Vec::new()) };
    static STORED_MANIFESTS: RefCell<Vec<ProjectManifest>> = const { RefCell::new(Vec::new()) };
    static DIRTY_FLAG: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) };
}

pub fn active_store() -> Option<Arc<dyn ProjectStore>> {
    ACTIVE_STORE.with(|store_cell| store_cell.borrow().clone())
}

pub fn bundled_manifests() -> Vec<ProjectManifest> {
    BUNDLED_MANIFESTS.with(|manifests_cell| manifests_cell.borrow().clone())
}

pub fn stored_manifests() -> Vec<ProjectManifest> {
    STORED_MANIFESTS.with(|manifests_cell| manifests_cell.borrow().clone())
}

pub fn dirty_flag() -> Option<Arc<AtomicBool>> {
    DIRTY_FLAG.with(|flag_cell| flag_cell.borrow().clone())
}

#[wasm_bindgen(start)]
pub fn start() {
    init_web_logging();
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = run_playground().await {
            log::error!("playground startup failed: {error}");
            set_boot_status(&format!("Startup failed: {error}"));
        }
    });
}

async fn run_playground() -> Result<(), String> {
    set_boot_status("Loading playground assets…");
    if let Err(error) = preload_assets(ASSET_BASE).await {
        return Err(format!("asset preload failed: {error:?}"));
    }

    // 1. Open IndexedDB store or fall back to MemoryStore
    let store: Arc<dyn ProjectStore> = match IndexedDbStore::open().await {
        Ok(indexed_db_store) => Arc::new(indexed_db_store),
        Err(error) => {
            log::warn!("IndexedDB open failed ({error}); falling back to memory store");
            set_dom_banner("Storage unavailable (private browsing?) — edits will not persist across reloads");
            Arc::new(MemoryStore::new())
        }
    };
    ACTIVE_STORE.with(|store_cell| *store_cell.borrow_mut() = Some(store.clone()));

    // 2. Read bundled projects.json
    let projects_json_path = format!("{ASSET_BASE}/projects.json");
    let bundled_manifests_list: Vec<ProjectManifest> = match common::vfs::read_to_string(std::path::Path::new(&projects_json_path)) {
        Ok(json_content) => match serde_json::from_str(&json_content) {
            Ok(manifests) => manifests,
            Err(error) => {
                log::warn!("failed to parse {projects_json_path}: {error}; falling back to default manifest");
                vec![ProjectManifest {
                    slug: "examples".to_string(),
                    title: "Examples".to_string(),
                    bundle_version: BUNDLE_VERSION.to_string(),
                    content_hash: String::new(),
                    origin: ProjectOrigin::Bundled,
                }]
            }
        },
        Err(error) => {
            log::warn!("failed to read {projects_json_path}: {error}; falling back to default manifest");
            vec![ProjectManifest {
                slug: "examples".to_string(),
                title: "Examples".to_string(),
                bundle_version: BUNDLE_VERSION.to_string(),
                content_hash: String::new(),
                origin: ProjectOrigin::Bundled,
            }]
        }
    };
    BUNDLED_MANIFESTS.with(|manifests_cell| *manifests_cell.borrow_mut() = bundled_manifests_list.clone());

    // 3. Sweep orphans (files of slugs absent from projects store and projects.json)
    let bundled_slugs: Vec<String> = bundled_manifests_list
        .iter()
        .map(|manifest| manifest.slug.clone())
        .collect();
    let _ = store.sweep_orphans(&bundled_slugs).await;

    // 4. Stored manifests
    let stored_manifests_list = store.manifests().await;
    STORED_MANIFESTS.with(|manifests_cell| *manifests_cell.borrow_mut() = stored_manifests_list.clone());

    // 5. Select project from query param
    let first_bundled_slug = bundled_manifests_list
        .first()
        .map(|manifest| manifest.slug.clone())
        .unwrap_or_else(|| "examples".to_string());

    let requested_slug = query_param("project");
    let project_slug = match requested_slug {
        Some(ref slug) => {
            let is_bundled = bundled_manifests_list.iter().any(|manifest| &manifest.slug == slug);
            let is_stored = stored_manifests_list.iter().any(|manifest| &manifest.slug == slug);
            if is_bundled || is_stored {
                slug.clone()
            } else {
                log::warn!("requested project '{slug}' is unknown; redirecting to '{first_bundled_slug}'");
                if let Some(window) = web_sys::window() {
                    let location = window.location();
                    let _ = location.set_search(&format!("?project={first_bundled_slug}"));
                }
                return Ok(());
            }
        }
        None => first_bundled_slug,
    };

    let stored_manifest_option = stored_manifests_list
        .iter()
        .find(|manifest| manifest.slug == project_slug)
        .cloned();
    let has_stored_manifest = stored_manifest_option.is_some();

    let manifest = stored_manifest_option
        .or_else(|| bundled_manifests_list.iter().find(|manifest| manifest.slug == project_slug).cloned())
        .unwrap_or_else(|| ProjectManifest {
            slug: project_slug.clone(),
            title: project_slug.clone(),
            bundle_version: BUNDLE_VERSION.to_string(),
            content_hash: String::new(),
            origin: ProjectOrigin::Saved,
        });

    let root_string = project_root(ASSET_BASE, &project_slug);
    let root_path = PathBuf::from(&root_string);
    let asset_base_string = project_asset_base(&root_string);

    // 6. Load stored files and overwrite onto MemFs
    let stored_files = store.load_project(&project_slug).await.unwrap_or_default();
    let stored_files_for_chains = if !stored_files.is_empty() && !has_stored_manifest {
        log::warn!("project '{project_slug}' has stored files but no stored manifest (an interrupted import); removing them and loading bundled only");
        if let Err(error) = store.remove_project(&project_slug).await {
            log::warn!("could not remove the manifest-less files of '{project_slug}': {error}");
        }
        Vec::new()
    } else {
        for file in &stored_files {
            let vfs_path = format!("{root_string}/{}", file.path);
            common::vfs::insert(vfs_path, file.bytes.clone());
        }
        stored_files
    };

    // 7. Chains and persistence listeners
    let dirty_atomic = Arc::new(AtomicBool::new(false));
    let pending_atomic = Arc::new(AtomicBool::new(false));
    DIRTY_FLAG.with(|flag_cell| *flag_cell.borrow_mut() = Some(dirty_atomic.clone()));

    let mut chains = Chains::new(
        project_slug.clone(),
        root_string.clone(),
        BUNDLE_VERSION.to_string(),
        manifest,
        store,
        Some(pending_atomic.clone()),
    );
    chains.seed(&stored_files_for_chains);
    set_active_chains(chains);

    // Install VFS write observer to feed chains
    common::vfs::set_write_observer(|path| {
        let bytes = common::vfs::read(path).unwrap_or_default();
        let write_result = with_active_chains(|chains| {
            chains.on_vfs_write(path, &bytes)
        });
        if let Some(Err(error_message)) = write_result {
            log::warn!("{error_message}");
            set_dom_banner(&error_message);
        }
        drive_started_puts();
    });

    install_listeners();

    // 8. Bridge setup (FIFO 1024)
    let (request_sender, request_receiver) = sync_channel::<String>(1024);
    let (response_sender, response_receiver) = channel::<String>();
    crate::bridge::setup_bridge(request_sender, response_receiver, root_path.clone());

    // The page's `await init()` resolves before this spawned future has run: without
    // this event a select populated right after `init()` reads empty manifests forever.
    if let Some(window) = web_sys::window() {
        if let Ok(event) = web_sys::Event::new("playground-ready") {
            let _ = window.dispatch_event(&event);
        }
    }

    // 9. Initial scene and options
    let scenes_directory = root_path.join("assets").join("scenes");
    let initial_scene = find_first_scene(&scenes_directory);

    let editor_options = EditorRunOptions {
        api_rx: Some(request_receiver),
        initial_scene,
        api_responses: Some(response_sender),
        prefs_slot: Some(PathBuf::from("beinsiculous.playground.editor_prefs")),
        dirty_flag: Some(dirty_atomic),
        persist_pending: Some(pending_atomic),
    };

    let config = GameConfig::new("Insiculous Playground")
        .with_size(1280, 800)
        .with_asset_base_path(&asset_base_string);

    let host = ProjectHost::new(root_path);
    run_game_with_editor_opts(host, config, editor_options).map_err(|error| format!("{error}"))
}
