//! Future adapter for an IndexedDB transaction.
//!
//! Wraps transaction `complete`, `error`, and `abort` events into a Rust
//! [`Future`]. Dependent IndexedDB requests are issued synchronously inside
//! request `onsuccess` callbacks before returning to the event loop, avoiding
//! `TransactionInactiveError`.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Event, IdbTransaction};

use crate::store::StoreError;

struct TransactionShared<T> {
    result: Option<Result<T, StoreError>>,
    waker: Option<Waker>,
}

/// A Rust future that resolves when the IndexedDB transaction completes or errors.
pub struct IdbTransactionFuture<T> {
    state: Rc<RefCell<TransactionShared<T>>>,
    _oncomplete: Closure<dyn FnMut(Event)>,
    _onerror: Closure<dyn FnMut(Event)>,
    _onabort: Closure<dyn FnMut(Event)>,
}

/// Handle for setting transaction result across callbacks.
pub type TransactionResultHandle<T> = Rc<RefCell<Option<Result<T, StoreError>>>>;

impl<T: 'static> IdbTransactionFuture<T> {
    /// Bind a transaction and return the shared state handle and future.
    pub fn new(transaction: &IdbTransaction) -> (TransactionResultHandle<T>, Self) {
        let external_result: TransactionResultHandle<T> = Rc::new(RefCell::new(None));
        let state = Rc::new(RefCell::new(TransactionShared {
            result: None,
            waker: None,
        }));

        let complete_result = external_result.clone();
        let complete_state = state.clone();
        let oncomplete = Closure::wrap(Box::new(move |_event: Event| {
            let mut state_guard = complete_state.borrow_mut();
            if state_guard.result.is_none() {
                let extracted_result = complete_result.borrow_mut().take();
                state_guard.result = extracted_result.or(Some(Err(StoreError::Backend("transaction completed without result".to_string()))));
            }
            if let Some(waker) = state_guard.waker.take() {
                waker.wake();
            }
        }) as Box<dyn FnMut(Event)>);

        let error_state = state.clone();
        let onerror = Closure::wrap(Box::new(move |event: Event| {
            let mut state_guard = error_state.borrow_mut();
            let message = format!("transaction error event: {event:?}");
            state_guard.result = Some(Err(StoreError::Backend(message)));
            if let Some(waker) = state_guard.waker.take() {
                waker.wake();
            }
        }) as Box<dyn FnMut(Event)>);

        let abort_state = state.clone();
        let abort_result = external_result.clone();
        let onabort = Closure::wrap(Box::new(move |_event: Event| {
            let mut state_guard = abort_state.borrow_mut();
            if state_guard.result.is_none() {
                // Only a refusal survives an abort: the CAS path writes StaleRevision and aborts on
                // purpose, but a put callback writes Ok before the transaction commits, and a
                // transaction that aborts after that (quota, eviction) stored nothing.
                let refusal = abort_result.borrow_mut().take().filter(|result| result.is_err());
                state_guard.result = refusal.or(Some(Err(StoreError::Backend("transaction aborted".to_string()))));
            }
            if let Some(waker) = state_guard.waker.take() {
                waker.wake();
            }
        }) as Box<dyn FnMut(Event)>);

        transaction.set_oncomplete(Some(oncomplete.as_ref().unchecked_ref()));
        transaction.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        transaction.set_onabort(Some(onabort.as_ref().unchecked_ref()));

        let future = Self {
            state,
            _oncomplete: oncomplete,
            _onerror: onerror,
            _onabort: onabort,
        };

        (external_result, future)
    }
}

impl<T> Future for IdbTransactionFuture<T> {
    type Output = Result<T, StoreError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state_guard = self.state.borrow_mut();
        if let Some(result) = state_guard.result.take() {
            Poll::Ready(result)
        } else {
            state_guard.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

/// Convert a serializable Rust value into a JavaScript object via JSON parsing.
pub fn to_javascript_value<T: serde::Serialize>(value: &T) -> Result<JsValue, StoreError> {
    let json_string = serde_json::to_string(value)
        .map_err(|error| StoreError::Backend(format!("JSON serialize error: {error}")))?;
    js_sys::JSON::parse(&json_string)
        .map_err(|error| StoreError::Backend(format!("JSON parse error: {error:?}")))
}

/// Parse a JavaScript object into a deserializable Rust value via JSON stringification.
pub fn from_javascript_value<T: serde::de::DeserializeOwned>(value: &JsValue) -> Result<T, StoreError> {
    let json_string = js_sys::JSON::stringify(value)
        .map_err(|error| StoreError::Backend(format!("JSON stringify error: {error:?}")))?
        .as_string()
        .ok_or_else(|| StoreError::Backend("stringify returned non-string".to_string()))?;
    serde_json::from_str(&json_string)
        .map_err(|error| StoreError::Backend(format!("JSON deserialize error: {error}")))
}
