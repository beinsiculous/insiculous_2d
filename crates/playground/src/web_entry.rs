//! Browser entry point for the Web Playground.
//!
//! # Version contract
//! `ASSET_BASE` and `BUNDLE_VERSION` are tied to deployment routes:
//! 1. `ASSET_BASE` = `"/playground/v7/assets"`
//! 2. `BUNDLE_VERSION` = `"v7"`
//! 3. Deployed assets live at `insiculous_web/public/playground/v7/assets/`
//! 4. Projects metadata served from `/playground/v7/assets/projects.json`
//! 5. `scripts/build_wasm.sh` produces `playground/v7/`

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, sync_channel};
use std::sync::{Arc, Mutex};

use editor_integration::{run_game_with_editor_opts, EditorRunOptions, ProjectHost};
use engine_core::prelude::{GameConfig, SceneLoader};
use engine_core::web::{init_web_logging, preload_assets, query_param, set_boot_status};
use wasm_bindgen::prelude::*;

use crate::persist::{drive_started_operations, install_listeners, set_active_chains, set_dom_banner, with_active_chains, Chains};
use crate::projects::{project_asset_base, project_root, ProjectManifest, ProjectOrigin};
use crate::store::boot;
use crate::store::indexed_db::IndexedDbStore;
use crate::store::memory::MemoryStore;
use crate::store::ProjectStore;

/// Canonical asset base path for the deployed playground.
pub const ASSET_BASE: &str = "/playground/v7/assets";
/// Bundle version contract string.
pub const BUNDLE_VERSION: &str = "v7";

thread_local! {
    static ACTIVE_STORE: RefCell<Option<Arc<dyn ProjectStore>>> = const { RefCell::new(None) };
    static BUNDLED_MANIFESTS: RefCell<Vec<ProjectManifest>> = const { RefCell::new(Vec::new()) };
    static STORED_MANIFESTS: RefCell<Vec<ProjectManifest>> = const { RefCell::new(Vec::new()) };
    static DIRTY_FLAG: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) };
    static HISTORY_DIRTY_FLAG: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) };
    static ACTIVE_MANIFEST: RefCell<Option<ProjectManifest>> = const { RefCell::new(None) };
    static STORAGE_PERSISTS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn active_store() -> Option<Arc<dyn ProjectStore>> {
    ACTIVE_STORE.with(|store_cell| store_cell.borrow().clone())
}

pub fn active_manifest() -> Option<ProjectManifest> {
    ACTIVE_MANIFEST.with(|manifest_cell| manifest_cell.borrow().clone())
}

pub fn bundled_manifests() -> Vec<ProjectManifest> {
    BUNDLED_MANIFESTS.with(|manifests_cell| manifests_cell.borrow().clone())
}

pub fn stored_manifests() -> Vec<ProjectManifest> {
    STORED_MANIFESTS.with(|manifests_cell| manifests_cell.borrow().clone())
}

/// Whether the store this page chose keeps edits past the page: false before the boot
/// chose one and for the whole life of a page that fell back to memory.
pub fn storage_persists() -> bool {
    STORAGE_PERSISTS.with(std::cell::Cell::get)
}

pub fn dirty_flag() -> Option<Arc<AtomicBool>> {
    DIRTY_FLAG.with(|flag_cell| flag_cell.borrow().clone())
}

/// The editor's command history alone, without the persist layer's pending
/// puts: a put that a reader catches between completing and the next frame's
/// mirror would otherwise read as unsaved edits.
pub fn history_dirty_flag() -> Option<Arc<AtomicBool>> {
    HISTORY_DIRTY_FLAG.with(|flag_cell| flag_cell.borrow().clone())
}

#[wasm_bindgen(start)]
pub fn start() {
    init_web_logging();
    // The preview is a second runtime on the same bundle, not a second
    // entry point: winit allows one event loop per page, so this branch
    // returns before anything the editor owns is built.
    if query_param("mode").as_deref() == Some("preview") {
        crate::preview_entry::announce_preview_mode();
        return;
    }
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

    let is_move_boot = boot::is_move_boot(query_param("move").as_deref());
    let store: Arc<dyn ProjectStore> = match IndexedDbStore::open().await {
        Ok(indexed_db_store) => {
            STORAGE_PERSISTS.with(|persists| persists.set(true));
            Arc::new(indexed_db_store)
        }
        Err(error) => {
            if let Some(refusal) = boot::memory_fallback_refusal(is_move_boot) {
                log::error!("IndexedDB open failed ({error}) on a hand-off's destination");
                return Err(refusal.to_string());
            }
            log::warn!("IndexedDB open failed ({error}); falling back to memory store");
            set_dom_banner("Storage unavailable (private browsing?) — edits will not persist across reloads");
            Arc::new(MemoryStore::new())
        }
    };
    ACTIVE_STORE.with(|store_cell| *store_cell.borrow_mut() = Some(store.clone()));

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

    let opened = match boot::plan_boot(store.as_ref(), &bundled_manifests_list, query_param("project").as_deref()).await? {
        boot::BootPlan::Open(opened) => opened,
        boot::BootPlan::Redirect(first_bundled_slug) => {
            if let Some(window) = web_sys::window() {
                let _ = window.location().set_search(&format!("?project={first_bundled_slug}"));
            }
            return Ok(());
        }
    };
    STORED_MANIFESTS.with(|manifests_cell| *manifests_cell.borrow_mut() = opened.stored_manifests.clone());
    let project_slug = opened.slug;

    let manifest = opened
        .stored_manifest
        .or_else(|| bundled_manifests_list.iter().find(|manifest| manifest.slug == project_slug).cloned())
        .unwrap_or_else(|| ProjectManifest {
            slug: project_slug.clone(),
            title: project_slug.clone(),
            bundle_version: BUNDLE_VERSION.to_string(),
            content_hash: String::new(),
            origin: ProjectOrigin::Saved,
        });

    ACTIVE_MANIFEST.with(|manifest_cell| *manifest_cell.borrow_mut() = Some(manifest.clone()));

    let root_string = project_root(ASSET_BASE, &project_slug);
    let root_path = PathBuf::from(&root_string);
    let asset_base_string = project_asset_base(&root_string);

    let stored_files_for_chains = opened.stored_files;
    // Before the records are laid down, so a stored file at an old key replaces its alias.
    crate::legacy_paths::lay_aliases(
        &root_string,
        &crate::legacy_paths::selected_aliases(&project_slug, &stored_files_for_chains),
        common::vfs::read,
        common::vfs::exists,
        common::vfs::insert,
    );
    for file in &stored_files_for_chains {
        let vfs_path = format!("{root_string}/{}", file.path);
        if file.deleted {
            // A tombstone keeps a removed file's revision for the chains and no bytes,
            // and unseats the bundled copy the preload put here; the observers are not
            // installed yet, so nothing is queued.
            let _ = common::vfs::remove_prefix(std::path::Path::new(&vfs_path));
        } else {
            common::vfs::insert(vfs_path, file.bytes.clone());
        }
    }

    let dirty_atomic = Arc::new(AtomicBool::new(false));
    let history_dirty_atomic = Arc::new(AtomicBool::new(false));
    let pending_atomic = Arc::new(AtomicBool::new(false));
    DIRTY_FLAG.with(|flag_cell| *flag_cell.borrow_mut() = Some(dirty_atomic.clone()));
    HISTORY_DIRTY_FLAG
        .with(|flag_cell| *flag_cell.borrow_mut() = Some(history_dirty_atomic.clone()));

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

    // The VFS write and remove observers feed the chains
    common::vfs::set_write_observer(|path| {
        crate::bridge::record_file_change(path);
        let bytes = common::vfs::read(path).unwrap_or_default();
        let write_result = with_active_chains(|chains| {
            chains.on_vfs_write(path, &bytes)
        });
        if let Some(Err(error_message)) = write_result {
            log::warn!("{error_message}");
            set_dom_banner(&error_message);
        }
        drive_started_operations();
    });
    common::vfs::set_remove_observer(|path| {
        crate::bridge::record_file_change(path);
        with_active_chains(|chains| chains.on_vfs_remove(path));
        drive_started_operations();
    });

    install_listeners();

    let (request_sender, request_receiver) = sync_channel::<String>(1024);
    let (response_sender, response_receiver) = channel::<String>();
    crate::bridge::setup_bridge(request_sender, response_receiver, root_path.clone());

    let script_errors_mirror = Arc::new(Mutex::new(Vec::<String>::new()));
    let errors_for_hooks = Arc::clone(&script_errors_mirror);
    let scene_snapshot = Arc::new(editor_integration::SceneSnapshotRequest::default());
    let preview_open = Arc::new(AtomicBool::new(false));
    let open_song = Arc::new(Mutex::new(None));
    let song_import = Arc::new(Mutex::new(None));
    let hooks = crate::bridge::Hooks {
        source_check: Some(|source: &str| {
            engine_core::scripting::check_source(source).map_err(|e| e.to_string())
        }),
        script_errors: Some(Rc::new(move || {
            errors_for_hooks
                .lock()
                .map(|guard| guard.clone())
                .unwrap_or_default()
        })),
        scene_snapshot: Some(Arc::clone(&scene_snapshot)),
        preview_open: Some(Arc::clone(&preview_open)),
        open_song: Some(Arc::clone(&open_song)),
        song_import: Some(Arc::clone(&song_import)),
    };
    crate::bridge::set_hooks(hooks);

    // The page's `await init()` resolves before this spawned future has run: without
    // this event a select populated right after `init()` reads empty manifests forever.
    if let Some(window) = web_sys::window() {
        if let Ok(event) = web_sys::Event::new("playground-ready") {
            let _ = window.dispatch_event(&event);
        }
    }

    let scenes_directory = root_path.join("assets").join("scenes");
    let initial_scene = SceneLoader::first_scene_in(&scenes_directory);

    let editor_options = EditorRunOptions {
        api_rx: Some(request_receiver),
        initial_scene,
        api_responses: Some(response_sender),
        prefs_slot: Some(PathBuf::from("beinsiculous.playground.editor_prefs")),
        dirty_flag: Some(dirty_atomic),
        history_dirty_flag: Some(history_dirty_atomic),
        persist_pending: Some(pending_atomic),
        script_errors: Some(script_errors_mirror),
        scene_snapshot: Some(scene_snapshot),
        preview_open: Some(preview_open),
        open_song: Some(open_song),
        song_import: Some(song_import),
        file_mutation_guard: Some(crate::bridge::files::guard_file_mutation),
    };

    let config = GameConfig::new("Insiculous Playground")
        .with_size(1280, 800)
        .with_asset_base_path(&asset_base_string);

    // Opening the preview window hides this tab, and a hidden tab gets no
    // animation frames: the frame that answers the snapshot must still come.
    engine_core::web::install_hidden_frame_pump();

    let host = ProjectHost::new(root_path);
    run_game_with_editor_opts(host, config, editor_options).map_err(|error| format!("{error}"))
}
