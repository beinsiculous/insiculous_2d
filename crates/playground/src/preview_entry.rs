//! The preview window's runtime: one scene, played, and nothing else.
//!
//! A page opened with `?mode=preview` boots none of the editor's machinery —
//! no project store, no persistence chains, no write observer, no bridge, no
//! asset preload. It waits for the editor to hand it an archive, unpacks that
//! into the VFS, and runs the game. Every save path of its `GameConfig` stays
//! `None`, so the preview writes no storage key of its own.

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

use editor_integration::{PreviewControls, PreviewHost};
use engine_core::prelude::GameConfig;
use engine_core::web::{boot_status, set_boot_status};
use wasm_bindgen::prelude::*;

use crate::preview::{preview_state, unpack_preview};
use crate::web_entry::{ASSET_BASE, BUNDLE_VERSION};

thread_local! {
    /// The controls of the one runtime this page may hold. `None` until the
    /// editor's archive arrives; winit refuses a second event loop, so a
    /// second load is refused here rather than crashing there.
    static PREVIEW_CONTROLS: RefCell<Option<Arc<PreviewControls>>> = const { RefCell::new(None) };
}

/// Tell the visitor the window is waiting, and nothing else.
pub fn announce_preview_mode() {
    set_boot_status("Waiting for the editor's snapshot…");
}

fn controls() -> Option<Arc<PreviewControls>> {
    PREVIEW_CONTROLS.with(|slot| slot.borrow().clone())
}

/// Unpack the editor's archive and start the game on the named scene.
///
/// `Ok` means the runtime was scheduled — ask `playground_preview_state` when
/// it is actually running.
#[wasm_bindgen]
pub fn playground_load_preview(bytes: Vec<u8>, scene_entry: String) -> Result<(), JsValue> {
    if controls().is_some() {
        return Err(JsValue::from_str("a preview is already running on this page"));
    }

    let unpacked = unpack_preview(&bytes, &scene_entry, ASSET_BASE, BUNDLE_VERSION)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;

    for (path, file_bytes) in unpacked.files {
        common::vfs::insert(path, file_bytes);
    }

    let preview_controls = Arc::new(PreviewControls::default());
    PREVIEW_CONTROLS.with(|slot| *slot.borrow_mut() = Some(Arc::clone(&preview_controls)));

    set_boot_status("Starting the preview…");
    let root = PathBuf::from(&unpacked.root);
    let asset_base = crate::projects::project_asset_base(&unpacked.root);
    let host = PreviewHost::new(root, unpacked.scene, preview_controls);
    let config = GameConfig::new("Preview")
        .with_size(1280, 800)
        .with_asset_base_path(&asset_base);

    // The frame driver is an animation frame, which a hidden document never
    // gets; the editor's tab is often the one in front when this boots.
    engine_core::web::install_hidden_frame_pump();

    if let Err(error) = engine_core::run_game(host, config) {
        // Nothing runs, so nothing holds the page: the next load must not be
        // refused as "already running".
        PREVIEW_CONTROLS.with(|slot| *slot.borrow_mut() = None);
        return Err(JsValue::from_str(&error.to_string()));
    }
    Ok(())
}

/// `"booting"`, `"running"`, or `"failed: <text>"`.
#[wasm_bindgen]
pub fn playground_preview_state() -> String {
    let preview_controls = controls();
    let scene_error = preview_controls.as_ref().and_then(|controls| controls.scene_error());
    let status = boot_status();
    preview_state(
        preview_controls.is_some(),
        preview_controls.map(|controls| controls.has_started()).unwrap_or(false),
        scene_error.as_deref(),
        status.as_deref(),
    )
}

/// Toggle the pause and report the state the preview is now in.
#[wasm_bindgen]
pub fn playground_preview_pause() -> bool {
    controls().map(|controls| controls.toggle_paused()).unwrap_or(false)
}

/// Reload the scene the preview was handed, from the top.
#[wasm_bindgen]
pub fn playground_preview_restart() -> Result<(), JsValue> {
    let controls =
        controls().ok_or_else(|| JsValue::from_str("no preview is running on this page"))?;
    controls.request_restart();
    Ok(())
}
