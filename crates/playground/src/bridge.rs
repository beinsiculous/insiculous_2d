//! Wasm-bindgen bridge exports and transport-agnostic helper functions.
//!
//! Provides the Stage D JavaScript command and file interface for the web editor.

use std::cell::RefCell;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, SyncSender};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Callback to syntax-check Rhai script source.
pub type SourceCheckFn = fn(&str) -> Result<(), String>;
/// Callback to query current script runtime error messages.
pub type ScriptErrorsFn = Rc<dyn Fn() -> Vec<String>>;

/// The hooks the web entry installs: the syntax check a `.rhai` write runs, the reader of
/// the runner's error list, the editor's snapshot mailbox, and the flag that reserves the
/// simulation for a preview window. All `None` until `set_hooks` runs.
#[derive(Default)]
pub struct Hooks {
    pub source_check: Option<SourceCheckFn>,
    pub script_errors: Option<ScriptErrorsFn>,
    pub scene_snapshot: Option<std::sync::Arc<editor_integration::SceneSnapshotRequest>>,
    pub preview_open: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

thread_local! {
    static REQUEST_SENDER: RefCell<Option<SyncSender<String>>> = const { RefCell::new(None) };
    static RESPONSE_RECEIVER: RefCell<Option<Receiver<String>>> = const { RefCell::new(None) };
    static CURRENT_PROJECT_ROOT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static HOOKS: RefCell<Hooks> = RefCell::new(Hooks::default());
}

/// Set the channel endpoints and active project root for the bridge.
pub fn setup_bridge(
    request_sender: SyncSender<String>,
    response_receiver: Receiver<String>,
    project_root: PathBuf,
) {
    REQUEST_SENDER.with(|sender_cell| *sender_cell.borrow_mut() = Some(request_sender));
    RESPONSE_RECEIVER.with(|receiver_cell| *receiver_cell.borrow_mut() = Some(response_receiver));
    CURRENT_PROJECT_ROOT.with(|root_cell| *root_cell.borrow_mut() = Some(project_root));
}

/// Set the scripting hooks for syntax checking and runtime errors.
pub fn set_hooks(hooks: Hooks) {
    HOOKS.with(|h| *h.borrow_mut() = hooks);
}

/// Pure helper: validate that a relative path does not escape the project root.
pub fn validate_bridge_path(project_root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    if relative_path.is_empty() {
        return Err("empty path is invalid".to_string());
    }
    if relative_path.starts_with('/') || relative_path.starts_with('\\') {
        return Err("absolute path is not permitted; path must be project-relative".to_string());
    }

    let path = Path::new(relative_path);
    for component in path.components() {
        match component {
            Component::ParentDir => return Err("parent directory '..' is forbidden".to_string()),
            Component::RootDir | Component::Prefix(_) => {
                return Err("root/prefix path components are forbidden".to_string())
            }
            Component::CurDir => {}
            Component::Normal(segment) => {
                // On this target a Windows drive letter is an ordinary segment, so `C:x`
                // never reaches the `Prefix` arm; an archive written on Windows can carry one.
                let segment_bytes = segment.as_encoded_bytes();
                if segment_bytes.len() >= 2
                    && segment_bytes[1] == b':'
                    && segment_bytes[0].is_ascii_alphabetic()
                {
                    return Err("drive prefix is forbidden".to_string());
                }
            }
        }
    }

    Ok(project_root.join(path))
}

/// The same rule as [`validate_bridge_path`] with no root to join: the archive importer
/// asks it about every entry name before anything is stored.
pub fn relative_path_is_safe(relative_path: &str) -> bool {
    validate_bridge_path(Path::new(""), relative_path).is_ok()
}

/// Pure helper: decide whether an incoming line can be dispatched.
///
/// Refuses whitespace-only lines (which the engine skips without a response)
/// and lines when the channel is full.
pub fn can_dispatch(line: &str, sender: &SyncSender<String>) -> bool {
    if line.trim().is_empty() {
        return false;
    }
    sender.try_send(line.to_string()).is_ok()
}

/// Pure helper: compute combined dirtiness.
pub fn dirty_or(dirty_flag: bool, persist_pending: bool) -> bool {
    dirty_flag || persist_pending
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_dispatch(line: String) -> bool {
    REQUEST_SENDER.with(|sender_slot| {
        if let Some(sender) = sender_slot.borrow().as_ref() {
            can_dispatch(&line, sender)
        } else {
            false
        }
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_poll_responses() -> Vec<JsValue> {
    RESPONSE_RECEIVER.with(|receiver_slot| {
        let mut responses = Vec::new();
        if let Some(receiver) = receiver_slot.borrow().as_ref() {
            while let Ok(line) = receiver.try_recv() {
                responses.push(JsValue::from_str(&line));
            }
        }
        responses
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_is_dirty() -> bool {
    let pending = crate::persist::is_pending();
    crate::web_entry::dirty_flag()
        .map(|flag_cell| flag_cell.load(Ordering::Relaxed))
        .unwrap_or(false)
        || pending
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_write_file(path: String, text: String) -> Result<(), JsValue> {
    let project_root = CURRENT_PROJECT_ROOT.with(|root_cell| {
        root_cell.borrow().clone().ok_or_else(|| JsValue::from_str("no active project root"))
    })?;

    let full_path = validate_bridge_path(&project_root, &path)
        .map_err(|error| JsValue::from_str(&error))?;

    // Checked before it is written: a refused script must not land in the store.
    if path.ends_with(".rhai") {
        let check = HOOKS.with(|hooks_cell| hooks_cell.borrow().source_check);
        if let Some(check_fn) = check {
            check_fn(&text).map_err(|error| JsValue::from_str(&error))?;
        }
    }

    common::vfs::write_string(&full_path, &text)
        .map_err(|error| JsValue::from_str(&format!("failed to write file: {error}")))?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_read_file(path: String) -> Result<String, JsValue> {
    let project_root = CURRENT_PROJECT_ROOT.with(|root_cell| {
        root_cell.borrow().clone().ok_or_else(|| JsValue::from_str("no active project root"))
    })?;

    let full_path = validate_bridge_path(&project_root, &path)
        .map_err(|error| JsValue::from_str(&error))?;

    common::vfs::read_to_string(&full_path)
        .map_err(|error| JsValue::from_str(&format!("failed to read file: {error}")))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_list_files() -> Result<Vec<String>, JsValue> {
    let project_root = CURRENT_PROJECT_ROOT.with(|root_cell| {
        root_cell.borrow().clone().ok_or_else(|| JsValue::from_str("no active project root"))
    })?;

    let files = common::vfs::list_files(&project_root)
        .map_err(|error| JsValue::from_str(&format!("failed to list files: {error}")))?;

    let mut result = Vec::new();
    for file_path in files {
        if let Ok(relative) = file_path.strip_prefix(&project_root) {
            result.push(relative.to_string_lossy().into_owned());
        }
    }
    Ok(result)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_list_projects() -> JsValue {
    let bundled = crate::web_entry::bundled_manifests();
    let stored = crate::web_entry::stored_manifests();
    let entries = crate::projects::list_projects(&bundled, &stored);
    to_javascript_value(&entries)
}

#[cfg(target_arch = "wasm32")]
fn to_javascript_value<T: serde::Serialize>(value: &T) -> JsValue {
    let json_string = serde_json::to_string(value).unwrap_or_default();
    js_sys::JSON::parse(&json_string).unwrap_or(JsValue::UNDEFINED)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_open_project(slug: String) -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        if !crate::projects::validate_slug(&slug) {
            return Err(JsValue::from_str(&format!("invalid project slug: {slug}")));
        }
        crate::persist::drain_then_epoch().await.map_err(|error| JsValue::from_str(&error))?;
        Ok(JsValue::TRUE)
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_reset_project(slug: String) -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        if !crate::projects::validate_slug(&slug) {
            return Err(JsValue::from_str(&format!("invalid project slug: {slug}")));
        }
        crate::persist::drain_then_epoch().await.map_err(|error| JsValue::from_str(&error))?;
        if let Some(store) = crate::web_entry::active_store() {
            store.remove_project(&slug).await.map_err(|error| JsValue::from_str(&error.to_string()))?;
        }
        Ok(JsValue::TRUE)
    })
}


/// How long a caller waits for the editor's next frame before giving up.
#[cfg(target_arch = "wasm32")]
const SNAPSHOT_TIMEOUT_SECONDS: u64 = 5;

/// How often a waiting caller checks the mailbox.
#[cfg(target_arch = "wasm32")]
const SNAPSHOT_POLL_MILLISECONDS: i32 = 50;

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// Generations the bridge files for itself, when an Export has to ask for
    /// a snapshot nobody else asked for. Only one request is ever in flight,
    /// so an id is only required to be unique among the live ones.
    static BRIDGE_GENERATION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Set the preview reservation. While it is raised the editor refuses Play,
/// so only the page may lower it — on `preview-failed`, `preview-closed`, or
/// a window it finds closed. Never on silence.
#[cfg(target_arch = "wasm32")]
fn set_preview_reservation(open: bool) {
    HOOKS.with(|hooks_cell| {
        if let Some(flag) = &hooks_cell.borrow().preview_open {
            flag.store(open, Ordering::Relaxed);
        }
    });
}

/// The live scene as an archive: file a snapshot request, wait for the
/// editor's next frame, then rebuild the project zip with the live bytes in
/// place of the saved scene.
#[cfg(target_arch = "wasm32")]
async fn live_scene(generation: u64) -> Result<(String, Vec<u8>), String> {
    let request = HOOKS
        .with(|hooks_cell| hooks_cell.borrow().scene_snapshot.clone())
        .ok_or_else(|| "no editor is running on this page".to_string())?;

    let completion = match request.subscribe(generation) {
        // Someone already filed this generation; join their wait.
        Some(shared) if request.pending_generation() == Some(generation) => shared,
        _ => {
            if !request.file(generation) {
                return Err("a snapshot is already in flight".to_string());
            }
            request
                .subscribe(generation)
                .ok_or_else(|| "the snapshot request was replaced".to_string())?
        }
    };

    let started_at = common::clock::Instant::now();
    let outcome = loop {
        if let Some(outcome) = completion.outcome() {
            break outcome;
        }
        if started_at.elapsed() > std::time::Duration::from_secs(SNAPSHOT_TIMEOUT_SECONDS) {
            request.cancel(generation);
            return Err("the editor did not answer — is its tab visible?".to_string());
        }
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            if let Some(window) = web_sys::window() {
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    &resolve,
                    SNAPSHOT_POLL_MILLISECONDS,
                );
            }
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    };

    let snapshot = outcome?;
    let project_root = CURRENT_PROJECT_ROOT
        .with(|root_cell| root_cell.borrow().clone())
        .ok_or_else(|| "no active project root".to_string())?;
    let manifest = crate::web_entry::active_manifest()
        .ok_or_else(|| "no active project manifest".to_string())?;
    let bytes = crate::archive::export_snapshot(
        &project_root,
        &manifest,
        &snapshot.scene_entry,
        &snapshot.ron,
    )
    .map_err(|error| error.to_string())?;

    Ok((snapshot.scene_entry, bytes))
}

/// The archive the preview window is to load, as `{ sceneEntry, bytes }`.
///
/// The reservation is taken here, at the click, rather than when the window
/// hands back a handshake: the editor's own Play must already be refused
/// while the preview is still booting.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_snapshot(generation: u64) -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        set_preview_reservation(true);
        match live_scene(generation).await {
            Ok((scene_entry, bytes)) => {
                let envelope = js_sys::Object::new();
                let _ = js_sys::Reflect::set(
                    &envelope,
                    &JsValue::from_str("sceneEntry"),
                    &JsValue::from_str(&scene_entry),
                );
                let _ = js_sys::Reflect::set(
                    &envelope,
                    &JsValue::from_str("bytes"),
                    &js_sys::Uint8Array::from(bytes.as_slice()).into(),
                );
                Ok(envelope.into())
            }
            Err(error) => {
                set_preview_reservation(false);
                Err(JsValue::from_str(&error))
            }
        }
    })
}

/// Raise or lower the preview reservation from the page.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_set_preview_open(open: bool) {
    set_preview_reservation(open);
}

/// Export the project with the scene as the visitor left it, unsaved edits
/// included — the saved scene would drop the very edit they just previewed.
///
/// A snapshot already in flight is joined rather than refused: Export a
/// moment after Play ↗ is an ordinary two-click sequence.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_export_zip() -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        let generation = match HOOKS
            .with(|hooks_cell| hooks_cell.borrow().scene_snapshot.clone())
            .and_then(|request| request.pending_generation())
        {
            Some(pending) => pending,
            None => BRIDGE_GENERATION.with(|counter| {
                let next = counter.get().wrapping_add(1);
                counter.set(next);
                next
            }),
        };
        match live_scene(generation).await {
            Ok((_, bytes)) => Ok(js_sys::Uint8Array::from(bytes.as_slice()).into()),
            Err(error) => Err(JsValue::from_str(&error)),
        }
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_import_zip(bytes: Vec<u8>) -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        let (manifest, files) = crate::archive::import_project(&bytes, crate::web_entry::BUNDLE_VERSION)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;

        let store = crate::web_entry::active_store()
            .ok_or_else(|| JsValue::from_str("no active project store"))?;

        crate::persist::drain_then_epoch()
            .await
            .map_err(|error| JsValue::from_str(&error))?;

        let slug = manifest.slug.clone();
        if let Err(store_error) = store.replace_project(&slug, files, manifest).await {
            crate::persist::with_active_chains(|chains| chains.restore_epoch());
            return Err(JsValue::from_str(&store_error.to_string()));
        }

        Ok(JsValue::from_str(&slug))
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_read_file_bytes(path: String) -> Result<Vec<u8>, JsValue> {
    let project_root = CURRENT_PROJECT_ROOT.with(|root_cell| {
        root_cell.borrow().clone().ok_or_else(|| JsValue::from_str("no active project root"))
    })?;

    let full_path = validate_bridge_path(&project_root, &path)
        .map_err(|error| JsValue::from_str(&error))?;

    common::vfs::read(&full_path)
        .map_err(|error| JsValue::from_str(&format!("failed to read file: {error}")))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_conflicted_paths() -> Vec<JsValue> {
    crate::persist::with_active_chains(|chains| {
        chains
            .conflicted_paths()
            .into_iter()
            .map(|path| JsValue::from_str(&path))
            .collect()
    })
    .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn playground_script_errors() -> Vec<String> {
    HOOKS.with(|hooks_cell| {
        if let Some(errors_fn) = &hooks_cell.borrow().script_errors {
            errors_fn()
        } else {
            Vec::new()
        }
    })
}

#[cfg(test)]
mod tests {
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
}
