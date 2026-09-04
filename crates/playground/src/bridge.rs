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

/// Hooks for future scripting support (batch 7). Both remain `None` in batch 3.
#[derive(Default)]
pub struct Hooks {
    pub source_check: Option<SourceCheckFn>,
    pub script_errors: Option<ScriptErrorsFn>,
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
            Component::CurDir | Component::Normal(_) => {}
        }
    }

    Ok(project_root.join(path))
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
}
