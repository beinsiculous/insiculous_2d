//! Cursor iteration and callback helpers for IndexedDB store operations.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Event, IdbCursor, IdbCursorWithValue, IdbKeyRange, IdbObjectStore, IdbRequest};

use crate::projects::ProjectManifest;
use crate::store::idb_transaction::{from_javascript_value, to_javascript_value, TransactionResultHandle};
use crate::store::{StoredFile, StoreError};

pub type EventClosure = Closure<dyn FnMut(Event)>;
pub type CallbackCell = Rc<RefCell<Option<EventClosure>>>;

/// Compute the bounded key range for files belonging to a project slug.
pub fn project_range(project_slug: &str) -> Result<IdbKeyRange, StoreError> {
    let lower = js_sys::Array::new();
    lower.push(&JsValue::from_str(project_slug));
    lower.push(&JsValue::from_str(""));

    let upper = js_sys::Array::new();
    upper.push(&JsValue::from_str(project_slug));
    upper.push(&JsValue::from_str("\u{ffff}"));

    IdbKeyRange::bound(&lower, &upper)
        .map_err(|error| StoreError::Backend(format!("IdbKeyRange bound error: {error:?}")))
}

/// Attach an event closure as `onsuccess` on an IndexedDB request and store it in a cell.
pub fn attach_request_callback(
    request: &IdbRequest,
    callback_cell: &CallbackCell,
    callback: EventClosure,
) {
    request.set_onsuccess(Some(callback.as_ref().unchecked_ref()));
    *callback_cell.borrow_mut() = Some(callback);
}

/// Extract request target and result value from a DOM event, without unwrap.
pub fn extract_request_target_and_result(
    event: &Event,
) -> Result<(IdbRequest, JsValue), StoreError> {
    let target: IdbRequest = event
        .target()
        .ok_or_else(|| StoreError::Backend("missing event target".to_string()))?
        .unchecked_into();
    let result = target
        .result()
        .map_err(|error| StoreError::Backend(format!("request result error: {error:?}")))?;
    Ok((target, result))
}

pub fn load_project_cursor(
    files_store: &IdbObjectStore,
    project_slug: &str,
    result_cell: TransactionResultHandle<Vec<StoredFile>>,
) -> Result<(), StoreError> {
    let range = project_range(project_slug)?;
    let cursor_request = files_store
        .open_cursor_with_range(&range)
        .map_err(|error| StoreError::Backend(format!("{error:?}")))?;

    let accumulated_files = Rc::new(RefCell::new(Vec::new()));
    let accumulated_clone = accumulated_files.clone();
    let cursor_callback: CallbackCell = Rc::new(RefCell::new(None));
    let cursor_callback_clone = cursor_callback.clone();

    let callback = Closure::wrap(Box::new(move |event: Event| {
        let (_, result) = match extract_request_target_and_result(&event) {
            Ok(pair) => pair,
            Err(error) => {
                *result_cell.borrow_mut() = Some(Err(error));
                let _ = cursor_callback_clone.borrow_mut().take();
                return;
            }
        };

        if result.is_null() || result.is_undefined() {
            let mut files = accumulated_clone.borrow_mut().clone();
            files.sort_by(|left_file: &StoredFile, right_file: &StoredFile| left_file.path.cmp(&right_file.path));
            *result_cell.borrow_mut() = Some(Ok(files));
            let _ = cursor_callback_clone.borrow_mut().take();
        } else {
            let cursor: IdbCursorWithValue = result.unchecked_into();
            if let Ok(value) = cursor.value() {
                if let Ok(file) = from_javascript_value::<StoredFile>(&value) {
                    accumulated_clone.borrow_mut().push(file);
                }
            }
            if let Err(error) = cursor.continue_() {
                *result_cell.borrow_mut() = Some(Err(StoreError::Backend(format!("cursor continue error: {error:?}"))));
                let _ = cursor_callback_clone.borrow_mut().take();
            }
        }
    }) as Box<dyn FnMut(Event)>);

    attach_request_callback(&cursor_request, &cursor_callback, callback);
    Ok(())
}

pub fn replace_project_cursor(
    files_store: &IdbObjectStore,
    projects_store: &IdbObjectStore,
    project_slug: &str,
    stored_files: Vec<StoredFile>,
    project_manifest: ProjectManifest,
    result_cell: TransactionResultHandle<()>,
) -> Result<(), StoreError> {
    let range = project_range(project_slug)?;
    let cursor_request = files_store
        .open_cursor_with_range(&range)
        .map_err(|error| StoreError::Backend(format!("{error:?}")))?;

    let inner_files_store = files_store.clone();
    let inner_projects_store = projects_store.clone();
    let callback_cell: CallbackCell = Rc::new(RefCell::new(None));
    let callback_clone = callback_cell.clone();

    let callback = Closure::wrap(Box::new(move |event: Event| {
        let (_, result) = match extract_request_target_and_result(&event) {
            Ok(pair) => pair,
            Err(error) => {
                *result_cell.borrow_mut() = Some(Err(error));
                let _ = callback_clone.borrow_mut().take();
                return;
            }
        };

        if result.is_null() || result.is_undefined() {
            for file in &stored_files {
                if let Ok(js_file) = to_javascript_value(file) {
                    let _ = inner_files_store.put(&js_file);
                }
            }
            if let Ok(js_manifest) = to_javascript_value(&project_manifest) {
                let _ = inner_projects_store.put(&js_manifest);
            }
            *result_cell.borrow_mut() = Some(Ok(()));
            let _ = callback_clone.borrow_mut().take();
        } else {
            let cursor: IdbCursor = result.unchecked_into();
            let _ = cursor.delete();
            let _ = cursor.continue_();
        }
    }) as Box<dyn FnMut(Event)>);

    attach_request_callback(&cursor_request, &callback_cell, callback);
    Ok(())
}

pub fn remove_project_cursor(
    files_store: &IdbObjectStore,
    projects_store: &IdbObjectStore,
    project_slug: &str,
    result_cell: TransactionResultHandle<()>,
) -> Result<(), StoreError> {
    let range = project_range(project_slug)?;
    let cursor_request = files_store
        .open_cursor_with_range(&range)
        .map_err(|error| StoreError::Backend(format!("{error:?}")))?;

    let inner_projects_store = projects_store.clone();
    let slug_string = project_slug.to_string();
    let callback_cell: CallbackCell = Rc::new(RefCell::new(None));
    let callback_clone = callback_cell.clone();

    let callback = Closure::wrap(Box::new(move |event: Event| {
        let (_, result) = match extract_request_target_and_result(&event) {
            Ok(pair) => pair,
            Err(error) => {
                *result_cell.borrow_mut() = Some(Err(error));
                let _ = callback_clone.borrow_mut().take();
                return;
            }
        };

        if result.is_null() || result.is_undefined() {
            let _ = inner_projects_store.delete(&JsValue::from_str(&slug_string));
            *result_cell.borrow_mut() = Some(Ok(()));
            let _ = callback_clone.borrow_mut().take();
        } else {
            let cursor: IdbCursor = result.unchecked_into();
            let _ = cursor.delete();
            let _ = cursor.continue_();
        }
    }) as Box<dyn FnMut(Event)>);

    attach_request_callback(&cursor_request, &callback_cell, callback);
    Ok(())
}

pub fn manifests_cursor(
    projects_store: &IdbObjectStore,
    result_cell: TransactionResultHandle<Vec<ProjectManifest>>,
) -> Result<(), StoreError> {
    let cursor_request = projects_store
        .open_cursor()
        .map_err(|error| StoreError::Backend(format!("{error:?}")))?;

    let accumulated_manifests = Rc::new(RefCell::new(Vec::new()));
    let accumulated_clone = accumulated_manifests.clone();
    let callback_cell: CallbackCell = Rc::new(RefCell::new(None));
    let callback_clone = callback_cell.clone();

    let callback = Closure::wrap(Box::new(move |event: Event| {
        let (_, result) = match extract_request_target_and_result(&event) {
            Ok(pair) => pair,
            Err(error) => {
                *result_cell.borrow_mut() = Some(Err(error));
                let _ = callback_clone.borrow_mut().take();
                return;
            }
        };

        if result.is_null() || result.is_undefined() {
            let mut list = accumulated_clone.borrow_mut().clone();
            list.sort_by(|left: &ProjectManifest, right: &ProjectManifest| left.slug.cmp(&right.slug));
            *result_cell.borrow_mut() = Some(Ok(list));
            let _ = callback_clone.borrow_mut().take();
        } else {
            let cursor: IdbCursorWithValue = result.unchecked_into();
            if let Ok(value) = cursor.value() {
                if let Ok(manifest) = from_javascript_value::<ProjectManifest>(&value) {
                    accumulated_clone.borrow_mut().push(manifest);
                }
            }
            let _ = cursor.continue_();
        }
    }) as Box<dyn FnMut(Event)>);

    attach_request_callback(&cursor_request, &callback_cell, callback);
    Ok(())
}

pub fn sweep_orphans_cursor(
    files_store: &IdbObjectStore,
    projects_store: &IdbObjectStore,
    bundled_slugs: &[String],
    result_cell: TransactionResultHandle<()>,
) -> Result<(), StoreError> {
    let bundled_set: std::collections::HashSet<String> = bundled_slugs.iter().cloned().collect();
    let projects_request = projects_store
        .get_all()
        .map_err(|error| StoreError::Backend(format!("{error:?}")))?;

    let inner_files_store = files_store.clone();
    let callback_cell: CallbackCell = Rc::new(RefCell::new(None));
    let callback_clone = callback_cell.clone();

    let callback = Closure::wrap(Box::new(move |event: Event| {
        let (target, _) = match extract_request_target_and_result(&event) {
            Ok(pair) => pair,
            Err(error) => {
                *result_cell.borrow_mut() = Some(Err(error));
                let _ = callback_clone.borrow_mut().take();
                return;
            }
        };

        let mut known_projects = bundled_set.clone();
        if let Ok(value) = target.result() {
            if let Ok(manifests) = from_javascript_value::<Vec<ProjectManifest>>(&value) {
                for manifest in manifests {
                    known_projects.insert(manifest.slug);
                }
            }
        }

        let cursor_request = match inner_files_store.open_cursor() {
            Ok(request) => request,
            Err(error) => {
                *result_cell.borrow_mut() = Some(Err(StoreError::Backend(format!("open cursor error: {error:?}"))));
                let _ = callback_clone.borrow_mut().take();
                return;
            }
        };

        let sweep_callback_cell: CallbackCell = Rc::new(RefCell::new(None));
        let sweep_callback_clone = sweep_callback_cell.clone();
        let sweep_result_cell = result_cell.clone();

        let sweep_callback = Closure::wrap(Box::new(move |cursor_event: Event| {
            let (_, cursor_result) = match extract_request_target_and_result(&cursor_event) {
                Ok(pair) => pair,
                Err(error) => {
                    *sweep_result_cell.borrow_mut() = Some(Err(error));
                    let _ = sweep_callback_clone.borrow_mut().take();
                    return;
                }
            };

            if cursor_result.is_null() || cursor_result.is_undefined() {
                *sweep_result_cell.borrow_mut() = Some(Ok(()));
                let _ = sweep_callback_clone.borrow_mut().take();
            } else {
                let cursor: IdbCursorWithValue = cursor_result.unchecked_into();
                if let Ok(value) = cursor.value() {
                    if let Ok(file) = from_javascript_value::<StoredFile>(&value) {
                        if !known_projects.contains(&file.project) {
                            let _ = cursor.delete();
                        }
                    }
                }
                let _ = cursor.continue_();
            }
        }) as Box<dyn FnMut(Event)>);

        attach_request_callback(&cursor_request, &sweep_callback_cell, sweep_callback);
        let _ = callback_clone.borrow_mut().take();
    }) as Box<dyn FnMut(Event)>);

    attach_request_callback(&projects_request, &callback_cell, callback);
    Ok(())
}
