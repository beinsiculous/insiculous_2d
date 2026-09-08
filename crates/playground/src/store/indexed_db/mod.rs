//! IndexedDB implementation of [`ProjectStore`] for wasm targets.
//!
//! Stores project records in database `beinsiculous.playground` (version 1)
//! with object stores:
//! - `files` keyed by compound path `["project", "path"]`
//! - `projects` keyed by `slug`

pub mod cursors;

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    Event, IdbDatabase, IdbOpenDbRequest, IdbTransaction, IdbTransactionMode,
    IdbVersionChangeEvent,
};

use crate::projects::{ProjectManifest, ProjectOrigin};
use crate::store::idb_transaction::{from_javascript_value, to_javascript_value, IdbTransactionFuture};
use crate::store::indexed_db::cursors::{
    attach_request_callback, extract_request_target_and_result, CallbackCell, EventClosure,
};
use crate::store::{Fut, ProjectStore, StoredFile, StoreError};

const DB_NAME: &str = "beinsiculous.playground";
const DB_VERSION: u32 = 1;
const STORE_FILES: &str = "files";
const STORE_PROJECTS: &str = "projects";

type OpenDbState = (Option<Result<(), StoreError>>, Option<std::task::Waker>);
type OpenDbStateCell = Rc<RefCell<OpenDbState>>;

/// IndexedDB-backed project store.
#[derive(Clone)]
pub struct IndexedDbStore {
    database: IdbDatabase,
}

impl IndexedDbStore {
    /// Open the IndexedDB database, creating object stores on upgrade.
    pub async fn open() -> Result<Self, StoreError> {
        let window = web_sys::window().ok_or(StoreError::Unavailable)?;
        let factory = window
            .indexed_db()
            .map_err(|error| StoreError::Backend(format!("{error:?}")))?
            .ok_or(StoreError::Unavailable)?;

        let open_request: IdbOpenDbRequest = factory
            .open_with_u32(DB_NAME, DB_VERSION)
            .map_err(|error| StoreError::Backend(format!("{error:?}")))?;

        let (database_cell, open_future) = open_request_future(&open_request);
        open_future.await?;
        let database = database_cell.borrow_mut().take().ok_or_else(|| {
            StoreError::Backend("database open completed without result".to_string())
        })?;

        Ok(Self { database })
    }

    fn transaction(&self, store_names: &[&str], mode: IdbTransactionMode) -> Result<IdbTransaction, StoreError> {
        let store_names_array = js_sys::Array::new();
        for name in store_names {
            store_names_array.push(&JsValue::from_str(name));
        }
        self.database
            .transaction_with_str_sequence_and_mode(&store_names_array, mode)
            .map_err(|error| StoreError::Backend(format!("transaction creation failed: {error:?}")))
    }
}

fn open_request_future(
    open_request: &IdbOpenDbRequest,
) -> (Rc<RefCell<Option<IdbDatabase>>>, impl std::future::Future<Output = Result<(), StoreError>>) {
    let database_cell = Rc::new(RefCell::new(None));
    let state_cell: OpenDbStateCell = Rc::new(RefCell::new((None, None)));

    let database_cell_upgrade = database_cell.clone();
    let state_upgrade = state_cell.clone();
    let onupgradeneeded = Closure::wrap(Box::new(move |event: IdbVersionChangeEvent| {
        let target: IdbOpenDbRequest = match event.target() {
            Some(target_val) => target_val.unchecked_into(),
            None => {
                let mut state_guard = state_upgrade.borrow_mut();
                state_guard.0 = Some(Err(StoreError::Backend("missing upgrade event target".to_string())));
                if let Some(waker) = state_guard.1.take() {
                    waker.wake();
                }
                return;
            }
        };

        let database: IdbDatabase = match target.result() {
            Ok(result_val) => result_val.unchecked_into(),
            Err(error) => {
                let mut state_guard = state_upgrade.borrow_mut();
                state_guard.0 = Some(Err(StoreError::Backend(format!("upgrade result error: {error:?}"))));
                if let Some(waker) = state_guard.1.take() {
                    waker.wake();
                }
                return;
            }
        };

        // 1. Files store with keyPath ["project", "path"]
        let key_path = js_sys::Array::new();
        key_path.push(&JsValue::from_str("project"));
        key_path.push(&JsValue::from_str("path"));
        let params = web_sys::IdbObjectStoreParameters::new();
        params.set_key_path(&key_path);
        if let Err(error) = database.create_object_store_with_optional_parameters(STORE_FILES, &params) {
            let mut state_guard = state_upgrade.borrow_mut();
            state_guard.0 = Some(Err(StoreError::Backend(format!("create files store error: {error:?}"))));
            if let Some(waker) = state_guard.1.take() {
                waker.wake();
            }
            return;
        }

        // 2. Projects store with keyPath "slug"
        let project_params = web_sys::IdbObjectStoreParameters::new();
        project_params.set_key_path(&JsValue::from_str("slug"));
        if let Err(error) = database.create_object_store_with_optional_parameters(STORE_PROJECTS, &project_params) {
            let mut state_guard = state_upgrade.borrow_mut();
            state_guard.0 = Some(Err(StoreError::Backend(format!("create projects store error: {error:?}"))));
            if let Some(waker) = state_guard.1.take() {
                waker.wake();
            }
            return;
        }

        *database_cell_upgrade.borrow_mut() = Some(database);
    }) as Box<dyn FnMut(IdbVersionChangeEvent)>);

    let database_cell_success = database_cell.clone();
    let state_success = state_cell.clone();
    let onsuccess = Closure::wrap(Box::new(move |event: Event| {
        let target: IdbOpenDbRequest = match event.target() {
            Some(target_val) => target_val.unchecked_into(),
            None => {
                let mut state_guard = state_success.borrow_mut();
                state_guard.0 = Some(Err(StoreError::Backend("missing open success target".to_string())));
                if let Some(waker) = state_guard.1.take() {
                    waker.wake();
                }
                return;
            }
        };

        let database: IdbDatabase = match target.result() {
            Ok(result_val) => result_val.unchecked_into(),
            Err(error) => {
                let mut state_guard = state_success.borrow_mut();
                state_guard.0 = Some(Err(StoreError::Backend(format!("open result error: {error:?}"))));
                if let Some(waker) = state_guard.1.take() {
                    waker.wake();
                }
                return;
            }
        };

        *database_cell_success.borrow_mut() = Some(database);
        let mut state_guard = state_success.borrow_mut();
        state_guard.0 = Some(Ok(()));
        if let Some(waker) = state_guard.1.take() {
            waker.wake();
        }
    }) as Box<dyn FnMut(Event)>);

    let state_error = state_cell.clone();
    let onerror = Closure::wrap(Box::new(move |event: Event| {
        let mut state_guard = state_error.borrow_mut();
        state_guard.0 = Some(Err(StoreError::Backend(format!("open DB error: {event:?}"))));
        if let Some(waker) = state_guard.1.take() {
            waker.wake();
        }
    }) as Box<dyn FnMut(Event)>);

    open_request.set_onupgradeneeded(Some(onupgradeneeded.as_ref().unchecked_ref()));
    open_request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
    open_request.set_onerror(Some(onerror.as_ref().unchecked_ref()));

    struct OpenFuture {
        state: OpenDbStateCell,
        _onupgrade: Closure<dyn FnMut(IdbVersionChangeEvent)>,
        _onsuccess: EventClosure,
        _onerror: EventClosure,
    }
    impl std::future::Future for OpenFuture {
        type Output = Result<(), StoreError>;
        fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            let mut state_guard = self.state.borrow_mut();
            if let Some(result) = state_guard.0.take() {
                std::task::Poll::Ready(result)
            } else {
                state_guard.1 = Some(cx.waker().clone());
                std::task::Poll::Pending
            }
        }
    }

    (database_cell, OpenFuture {
        state: state_cell,
        _onupgrade: onupgradeneeded,
        _onsuccess: onsuccess,
        _onerror: onerror,
    })
}

impl ProjectStore for IndexedDbStore {
    fn load_project(&self, project_slug: &str) -> Fut<Result<Vec<StoredFile>, StoreError>> {
        let transaction = match self.transaction(&[STORE_FILES], IdbTransactionMode::Readonly) {
            Ok(transaction) => transaction,
            Err(error) => return Box::pin(async move { Err(error) }),
        };

        let (result_cell, future) = IdbTransactionFuture::new(&transaction);
        let files_store = match transaction.object_store(STORE_FILES) {
            Ok(store) => store,
            Err(error) => return Box::pin(async move { Err(StoreError::Backend(format!("{error:?}"))) }),
        };

        if let Err(error) = cursors::load_project_cursor(&files_store, project_slug, result_cell) {
            return Box::pin(async move { Err(error) });
        }

        Box::pin(future)
    }

    fn put(
        &self,
        file: StoredFile,
        base_revision: u64,
        manifest: &ProjectManifest,
    ) -> Fut<Result<u64, StoreError>> {
        let transaction = match self.transaction(&[STORE_FILES, STORE_PROJECTS], IdbTransactionMode::Readwrite) {
            Ok(transaction) => transaction,
            Err(error) => return Box::pin(async move { Err(error) }),
        };

        let (result_cell, future) = IdbTransactionFuture::new(&transaction);
        let files_store = match transaction.object_store(STORE_FILES) {
            Ok(store) => store,
            Err(error) => return Box::pin(async move { Err(StoreError::Backend(format!("{error:?}"))) }),
        };
        let projects_store = match transaction.object_store(STORE_PROJECTS) {
            Ok(store) => store,
            Err(error) => return Box::pin(async move { Err(StoreError::Backend(format!("{error:?}"))) }),
        };

        let key = js_sys::Array::new();
        key.push(&JsValue::from_str(&file.project));
        key.push(&JsValue::from_str(&file.path));

        let get_request = match files_store.get(&key) {
            Ok(request) => request,
            Err(error) => return Box::pin(async move { Err(StoreError::Backend(format!("{error:?}"))) }),
        };

        let manifest_value = manifest.clone();
        let result_cell_clone = result_cell.clone();
        let transaction_clone = transaction.clone();
        let callback_cell: CallbackCell = Rc::new(RefCell::new(None));
        let callback_clone = callback_cell.clone();

        let callback = Closure::wrap(Box::new(move |event: Event| {
            let (target, result) = match extract_request_target_and_result(&event) {
                Ok(pair) => pair,
                Err(error) => {
                    let _ = transaction_clone.abort();
                    *result_cell_clone.borrow_mut() = Some(Err(error));
                    let _ = callback_clone.borrow_mut().take();
                    return;
                }
            };

            let stored_revision = if result.is_null() || result.is_undefined() {
                0
            } else if let Ok(stored_file) = from_javascript_value::<StoredFile>(&result) {
                stored_file.revision
            } else {
                0
            };

            if stored_revision != base_revision {
                let _ = target.transaction().map(|transaction| transaction.abort());
                *result_cell_clone.borrow_mut() = Some(Err(StoreError::StaleRevision {
                    stored: stored_revision,
                    base: base_revision,
                }));
                let _ = callback_clone.borrow_mut().take();
                return;
            }

            let new_revision = base_revision + 1;
            let mut updated_file = file.clone();
            updated_file.revision = new_revision;

            let project_request = match projects_store.get(&JsValue::from_str(&file.project)) {
                Ok(request) => request,
                Err(error) => {
                    let _ = target.transaction().map(|transaction| transaction.abort());
                    *result_cell_clone.borrow_mut() = Some(Err(StoreError::Backend(format!("get project error: {error:?}"))));
                    let _ = callback_clone.borrow_mut().take();
                    return;
                }
            };

            let inner_projects_store = projects_store.clone();
            let inner_files_store = files_store.clone();
            let inner_result_cell = result_cell_clone.clone();
            let inner_manifest = manifest_value.clone();
            let project_callback_cell: CallbackCell = Rc::new(RefCell::new(None));
            let project_callback_clone = project_callback_cell.clone();

            let project_callback = Closure::wrap(Box::new(move |project_event: Event| {
                let (_, project_result) = match extract_request_target_and_result(&project_event) {
                    Ok(pair) => pair,
                    Err(error) => {
                        *inner_result_cell.borrow_mut() = Some(Err(error));
                        let _ = project_callback_clone.borrow_mut().take();
                        return;
                    }
                };

                if project_result.is_null() || project_result.is_undefined() {
                    let mut upserted_manifest = inner_manifest.clone();
                    upserted_manifest.origin = ProjectOrigin::Saved;
                    if let Ok(js_manifest) = to_javascript_value(&upserted_manifest) {
                        let _ = inner_projects_store.put(&js_manifest);
                    }
                }
                if let Ok(js_file) = to_javascript_value(&updated_file) {
                    let _ = inner_files_store.put(&js_file);
                }
                *inner_result_cell.borrow_mut() = Some(Ok(new_revision));
                let _ = project_callback_clone.borrow_mut().take();
            }) as Box<dyn FnMut(Event)>);

            attach_request_callback(&project_request, &project_callback_cell, project_callback);
            let _ = callback_clone.borrow_mut().take();
        }) as Box<dyn FnMut(Event)>);

        attach_request_callback(&get_request, &callback_cell, callback);
        Box::pin(future)
    }

    fn replace_project(
        &self,
        project_slug: &str,
        stored_files: Vec<StoredFile>,
        project_manifest: ProjectManifest,
    ) -> Fut<Result<(), StoreError>> {
        let transaction = match self.transaction(&[STORE_FILES, STORE_PROJECTS], IdbTransactionMode::Readwrite) {
            Ok(transaction) => transaction,
            Err(error) => return Box::pin(async move { Err(error) }),
        };

        let (result_cell, future) = IdbTransactionFuture::new(&transaction);
        let files_store = match transaction.object_store(STORE_FILES) {
            Ok(store) => store,
            Err(error) => return Box::pin(async move { Err(StoreError::Backend(format!("{error:?}"))) }),
        };
        let projects_store = match transaction.object_store(STORE_PROJECTS) {
            Ok(store) => store,
            Err(error) => return Box::pin(async move { Err(StoreError::Backend(format!("{error:?}"))) }),
        };

        if let Err(error) = cursors::replace_project_cursor(
            &files_store,
            &projects_store,
            project_slug,
            stored_files,
            project_manifest,
            result_cell,
        ) {
            return Box::pin(async move { Err(error) });
        }

        Box::pin(future)
    }

    fn remove_project(&self, project_slug: &str) -> Fut<Result<(), StoreError>> {
        let transaction = match self.transaction(&[STORE_FILES, STORE_PROJECTS], IdbTransactionMode::Readwrite) {
            Ok(transaction) => transaction,
            Err(error) => return Box::pin(async move { Err(error) }),
        };

        let (result_cell, future) = IdbTransactionFuture::new(&transaction);
        let files_store = match transaction.object_store(STORE_FILES) {
            Ok(store) => store,
            Err(error) => return Box::pin(async move { Err(StoreError::Backend(format!("{error:?}"))) }),
        };
        let projects_store = match transaction.object_store(STORE_PROJECTS) {
            Ok(store) => store,
            Err(error) => return Box::pin(async move { Err(StoreError::Backend(format!("{error:?}"))) }),
        };

        if let Err(error) = cursors::remove_project_cursor(
            &files_store,
            &projects_store,
            project_slug,
            result_cell,
        ) {
            return Box::pin(async move { Err(error) });
        }

        Box::pin(future)
    }

    fn manifests(&self) -> Fut<Vec<ProjectManifest>> {
        // An empty list here is indistinguishable from "nothing stored" to the page, so every
        // failure path says why before returning it.
        let transaction = match self.transaction(&[STORE_PROJECTS], IdbTransactionMode::Readonly) {
            Ok(transaction) => transaction,
            Err(error) => {
                log::warn!("manifests: {error}");
                return Box::pin(async move { Vec::new() });
            }
        };

        let (result_cell, future) = IdbTransactionFuture::new(&transaction);
        let projects_store = match transaction.object_store(STORE_PROJECTS) {
            Ok(store) => store,
            Err(error) => {
                log::warn!("manifests: projects store unavailable: {error:?}");
                return Box::pin(async move { Vec::new() });
            }
        };

        if let Err(error) = cursors::manifests_cursor(&projects_store, result_cell) {
            log::warn!("manifests: {error}");
            return Box::pin(async move { Vec::new() });
        }

        Box::pin(async move {
            future.await.unwrap_or_else(|error| {
                log::warn!("manifests: {error}");
                Vec::new()
            })
        })
    }

    fn sweep_orphans(&self, bundled_slugs: &[String]) -> Fut<Result<(), StoreError>> {
        let transaction = match self.transaction(&[STORE_FILES, STORE_PROJECTS], IdbTransactionMode::Readwrite) {
            Ok(transaction) => transaction,
            Err(error) => return Box::pin(async move { Err(error) }),
        };

        let (result_cell, future) = IdbTransactionFuture::new(&transaction);
        let files_store = match transaction.object_store(STORE_FILES) {
            Ok(store) => store,
            Err(error) => return Box::pin(async move { Err(StoreError::Backend(format!("{error:?}"))) }),
        };
        let projects_store = match transaction.object_store(STORE_PROJECTS) {
            Ok(store) => store,
            Err(error) => return Box::pin(async move { Err(StoreError::Backend(format!("{error:?}"))) }),
        };

        if let Err(error) = cursors::sweep_orphans_cursor(
            &files_store,
            &projects_store,
            bundled_slugs,
            result_cell,
        ) {
            return Box::pin(async move { Err(error) });
        }

        Box::pin(future)
    }
}
