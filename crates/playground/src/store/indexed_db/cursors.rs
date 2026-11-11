//! Cursor iteration and callback helpers for IndexedDB store operations.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Event, IdbCursor, IdbCursorWithValue, IdbKeyRange, IdbObjectStore, IdbRequest, IdbTransaction};

use crate::projects::ProjectManifest;
use crate::store::idb_transaction::{from_javascript_value, to_javascript_value, TransactionResultHandle};
use crate::store::{replacement_revision, ManifestList, StoredFile, StoreError};

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

    let transaction = files_store.transaction();
    let accumulated_files = Rc::new(RefCell::new(Vec::new()));
    let accumulated_clone = accumulated_files.clone();
    let cursor_callback: CallbackCell = Rc::new(RefCell::new(None));
    let cursor_callback_clone = cursor_callback.clone();

    let callback = Closure::wrap(Box::new(move |event: Event| {
        let step = match extract_request_target_and_result(&event) {
            Err(error) => Err(error),
            Ok((_, result)) if result.is_null() || result.is_undefined() => {
                let mut files = accumulated_clone.borrow_mut().clone();
                files.sort_by(|left_file: &StoredFile, right_file: &StoredFile| left_file.path.cmp(&right_file.path));
                *result_cell.borrow_mut() = Some(Ok(files));
                let _ = cursor_callback_clone.borrow_mut().take();
                return;
            }
            Ok((_, result)) => {
                let cursor: IdbCursorWithValue = result.unchecked_into();
                decode_record::<StoredFile>(&cursor).and_then(|file| {
                    accumulated_clone.borrow_mut().push(file);
                    continue_cursor(&cursor)
                })
            }
        };
        if let Err(error) = step {
            fail_transaction(&transaction, &result_cell, error);
            let _ = cursor_callback_clone.borrow_mut().take();
        }
    }) as Box<dyn FnMut(Event)>);

    attach_request_callback(&cursor_request, &cursor_callback, callback);
    Ok(())
}

/// Replace a project's records in one transaction: every existing record is read for
/// its revision, tombstones included, and deleted; then each new file is written one
/// revision above the highest read. A record that will not decode aborts the whole
/// replacement, deletions included.
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

    let transaction = files_store.transaction();
    let inner_files_store = files_store.clone();
    let inner_projects_store = projects_store.clone();
    let callback_cell: CallbackCell = Rc::new(RefCell::new(None));
    let callback_clone = callback_cell.clone();
    let mut existing_revisions = Vec::new();

    let callback = Closure::wrap(Box::new(move |event: Event| {
        let step = match extract_request_target_and_result(&event) {
            Err(error) => Err(error),
            Ok((_, result)) if result.is_null() || result.is_undefined() => {
                let revision = replacement_revision(existing_revisions.iter().copied());
                let written = write_replacement(&inner_files_store, &inner_projects_store, &stored_files, &project_manifest, revision);
                if written.is_ok() {
                    *result_cell.borrow_mut() = Some(Ok(()));
                    let _ = callback_clone.borrow_mut().take();
                    return;
                }
                written
            }
            Ok((_, result)) => {
                let cursor: IdbCursorWithValue = result.unchecked_into();
                decode_record::<StoredFile>(&cursor).and_then(|existing| {
                    existing_revisions.push(existing.revision);
                    cursor
                        .delete()
                        .map_err(|error| StoreError::Backend(format!("record delete error: {error:?}")))?;
                    continue_cursor(&cursor)
                })
            }
        };
        if let Err(error) = step {
            fail_transaction(&transaction, &result_cell, error);
            let _ = callback_clone.borrow_mut().take();
        }
    }) as Box<dyn FnMut(Event)>);

    attach_request_callback(&cursor_request, &callback_cell, callback);
    Ok(())
}

fn write_replacement(
    files_store: &IdbObjectStore,
    projects_store: &IdbObjectStore,
    stored_files: &[StoredFile],
    project_manifest: &ProjectManifest,
    revision: u64,
) -> Result<(), StoreError> {
    for file in stored_files {
        let mut stamped = file.clone();
        stamped.revision = revision;
        files_store
            .put(&to_javascript_value(&stamped)?)
            .map_err(|error| StoreError::Backend(format!("file put error: {error:?}")))?;
    }
    projects_store
        .put(&to_javascript_value(project_manifest)?)
        .map_err(|error| StoreError::Backend(format!("manifest put error: {error:?}")))?;
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
    result_cell: TransactionResultHandle<ManifestList>,
) -> Result<(), StoreError> {
    let cursor_request = projects_store
        .open_cursor()
        .map_err(|error| StoreError::Backend(format!("{error:?}")))?;

    let transaction = projects_store.transaction();
    let accumulated = Rc::new(RefCell::new(ManifestList::default()));
    let accumulated_clone = accumulated.clone();
    let callback_cell: CallbackCell = Rc::new(RefCell::new(None));
    let callback_clone = callback_cell.clone();

    let callback = Closure::wrap(Box::new(move |event: Event| {
        let step = match extract_request_target_and_result(&event) {
            Err(error) => Err(error),
            Ok((_, result)) if result.is_null() || result.is_undefined() => {
                let mut list = accumulated_clone.borrow_mut().clone();
                list.manifests.sort_by(|left: &ProjectManifest, right: &ProjectManifest| left.slug.cmp(&right.slug));
                list.undecodable.sort();
                *result_cell.borrow_mut() = Some(Ok(list));
                let _ = callback_clone.borrow_mut().take();
                return;
            }
            Ok((_, result)) => {
                let cursor: IdbCursorWithValue = result.unchecked_into();
                read_value(&cursor).and_then(|value| {
                    match from_javascript_value::<ProjectManifest>(&value) {
                        Ok(manifest) => accumulated_clone.borrow_mut().manifests.push(manifest),
                        Err(_) => accumulated_clone.borrow_mut().undecodable.push(record_key_text(&cursor)?),
                    }
                    continue_cursor(&cursor)
                })
            }
        };
        if let Err(error) = step {
            fail_transaction(&transaction, &result_cell, error);
            let _ = callback_clone.borrow_mut().take();
        }
    }) as Box<dyn FnMut(Event)>);

    attach_request_callback(&cursor_request, &callback_cell, callback);
    Ok(())
}

/// Delete the files of every project not in `known` that has no manifest record now.
///
/// The record keys are read in this same transaction, decoded or not, so an import
/// another tab committed after this boot read its manifests is never taken for an
/// orphan. Each file's project comes from its key, never its value. Every failure
/// aborts the transaction, so a sweep that fails part-way deletes nothing.
pub fn sweep_orphans_cursor(
    files_store: &IdbObjectStore,
    projects_store: &IdbObjectStore,
    known: &[String],
    result_cell: TransactionResultHandle<()>,
) -> Result<(), StoreError> {
    let known_set: std::collections::HashSet<String> = known.iter().cloned().collect();
    let keys_request = projects_store
        .get_all_keys()
        .map_err(|error| StoreError::Backend(format!("{error:?}")))?;

    let transaction = files_store.transaction();
    let inner_files_store = files_store.clone();
    let callback_cell: CallbackCell = Rc::new(RefCell::new(None));
    let callback_clone = callback_cell.clone();

    let callback = Closure::wrap(Box::new(move |event: Event| {
        let opened = extract_request_target_and_result(&event)
            .and_then(|(_, keys)| manifest_keys(&keys))
            .and_then(|manifest_keys| {
                let mut keep = known_set.clone();
                keep.extend(manifest_keys);
                let cursor_request = inner_files_store
                    .open_cursor()
                    .map_err(|error| StoreError::Backend(format!("open cursor error: {error:?}")))?;
                attach_sweep_callback(&cursor_request, keep, transaction.clone(), result_cell.clone());
                Ok(())
            });
        if let Err(error) = opened {
            fail_transaction(&transaction, &result_cell, error);
        }
        let _ = callback_clone.borrow_mut().take();
    }) as Box<dyn FnMut(Event)>);

    attach_request_callback(&keys_request, &callback_cell, callback);
    Ok(())
}

fn attach_sweep_callback(
    cursor_request: &IdbRequest,
    keep: std::collections::HashSet<String>,
    transaction: IdbTransaction,
    result_cell: TransactionResultHandle<()>,
) {
    let sweep_callback_cell: CallbackCell = Rc::new(RefCell::new(None));
    let sweep_callback_clone = sweep_callback_cell.clone();
    let sweep_callback = Closure::wrap(Box::new(move |cursor_event: Event| {
        let step = match extract_request_target_and_result(&cursor_event) {
            Err(error) => Err(error),
            Ok((_, cursor_result)) if cursor_result.is_null() || cursor_result.is_undefined() => {
                *result_cell.borrow_mut() = Some(Ok(()));
                let _ = sweep_callback_clone.borrow_mut().take();
                return;
            }
            Ok((_, cursor_result)) => {
                let cursor: IdbCursor = cursor_result.unchecked_into();
                record_project(&cursor).and_then(|project| {
                    if !keep.contains(&project) {
                        cursor
                            .delete()
                            .map_err(|error| StoreError::Backend(format!("orphan delete error: {error:?}")))?;
                    }
                    continue_cursor(&cursor)
                })
            }
        };
        if let Err(error) = step {
            fail_transaction(&transaction, &result_cell, error);
            let _ = sweep_callback_clone.borrow_mut().take();
        }
    }) as Box<dyn FnMut(Event)>);
    attach_request_callback(cursor_request, &sweep_callback_cell, sweep_callback);
}

/// Put the error in the result cell and abort, so nothing the transaction wrote is kept
/// and the adapter's abort handler hands the error on.
fn fail_transaction<T>(transaction: &IdbTransaction, result_cell: &TransactionResultHandle<T>, error: StoreError) {
    *result_cell.borrow_mut() = Some(Err(error));
    let _ = transaction.abort();
}

fn continue_cursor(cursor: &IdbCursor) -> Result<(), StoreError> {
    cursor
        .continue_()
        .map_err(|error| StoreError::Backend(format!("cursor continue error: {error:?}")))
}

fn read_value(cursor: &IdbCursorWithValue) -> Result<JsValue, StoreError> {
    cursor.value().map_err(|error| {
        StoreError::Backend(format!("the stored record {} cannot be read: {error:?}", record_key_text(cursor).unwrap_or_default()))
    })
}

/// The record under the cursor, or an `Err` naming its key when it will not read or decode.
fn decode_record<T: serde::de::DeserializeOwned>(cursor: &IdbCursorWithValue) -> Result<T, StoreError> {
    let value = read_value(cursor)?;
    from_javascript_value::<T>(&value).map_err(|error| {
        StoreError::Backend(format!(
            "the stored record {} cannot be decoded: {error}",
            record_key_text(cursor).unwrap_or_default()
        ))
    })
}

/// The cursor's key as text: a project's slug, or a file's `project/path`.
fn record_key_text(cursor: &IdbCursor) -> Result<String, StoreError> {
    let key = cursor.key().map_err(|error| StoreError::Backend(format!("record key error: {error:?}")))?;
    let parts = match key.dyn_ref::<js_sys::Array>() {
        Some(parts) => parts.iter().collect::<Vec<JsValue>>(),
        None => vec![key],
    };
    parts
        .iter()
        .map(|part| {
            part.as_string()
                .ok_or_else(|| StoreError::Backend(format!("a record key is not text: {part:?}")))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|texts| texts.join("/"))
}

/// The project a file record belongs to, from its `[project, path]` key.
fn record_project(cursor: &IdbCursor) -> Result<String, StoreError> {
    let key = cursor.key().map_err(|error| StoreError::Backend(format!("record key error: {error:?}")))?;
    key.dyn_ref::<js_sys::Array>()
        .and_then(|parts| parts.get(0).as_string())
        .ok_or_else(|| StoreError::Backend(format!("a file record's key names no project: {key:?}")))
}

/// Every manifest record's key, which is its slug; a key that is not text fails the read.
fn manifest_keys(keys: &JsValue) -> Result<Vec<String>, StoreError> {
    let keys = keys
        .dyn_ref::<js_sys::Array>()
        .ok_or_else(|| StoreError::Backend("the manifest keys are not a list".to_string()))?;
    keys.iter()
        .map(|key| {
            key.as_string()
                .ok_or_else(|| StoreError::Backend(format!("a manifest key is not text: {key:?}")))
        })
        .collect()
}
