//! Playground crate: web entry, IndexedDB project store, and bridge for the editor in the browser.

pub mod bridge;
pub mod persist;
pub mod projects;
pub mod store;

#[cfg(target_arch = "wasm32")]
pub mod web_entry;
