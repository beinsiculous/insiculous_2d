//! Persistence seam for player saves (achievements, input bindings, high
//! scores): read/write string documents by slot. The roadmap-H6 store.
//!
//! A **slot** is a filesystem path natively and a localStorage key on
//! `wasm32-unknown-unknown` — `GameConfig`'s save-path strings flow through
//! unchanged, so native builds keep their JSON files byte-identical while web
//! builds persist the same JSON under keys like
//! `beinsiculous.games.pong.achievements` (see `docs/WEB_SAVES.md` for the
//! site contract).
//!
//! Semantics shared by both targets:
//! - [`read`] returns `Ok(None)` for an absent slot (missing file / no such
//!   key) — absence is not an error.
//! - [`write`] is atomic natively (tmp file + rename, parents created); on
//!   the web each successful localStorage write also dispatches a
//!   [`SAVE_EVENT`] `CustomEvent` on `window` so the hosting page can react
//!   (same-tab writes never fire the browser's own `storage` event).
//! - When localStorage is unavailable (private browsing, storage blocked),
//!   the web backend warns once and degrades to an in-memory [`MemoryStore`] —
//!   session-local persistence, no events.
//!
//! Errors are `std::io::Error` on every target (browser failures such as
//! quota map via `io::Error::other`), so callers keep their existing
//! `Io`-variant error enums. Slots are `Path`s: native IO uses them as-is
//! (arbitrary OS paths work); the wasm backend renders them to the string
//! key with `to_string_lossy` (lossless — the contract keys are ASCII).

use std::collections::HashMap;
use std::io;
use std::path::Path;

/// Name of the `CustomEvent` dispatched on `window` after every successful
/// localStorage persist (wasm only). `event.detail` is the storage key.
pub const SAVE_EVENT: &str = "insiculous-save";

/// Read a slot's contents. `Ok(None)` means the slot doesn't exist.
pub fn read(slot: &Path) -> io::Result<Option<String>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::fs::read_to_string(slot) {
            Ok(contents) => Ok(Some(contents)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let key = slot.to_string_lossy();
        BACKEND.with(|b| match &*b.borrow() {
            WebBackend::Local(storage) => storage
                .get_item(&key)
                .map_err(|e| io::Error::other(format!("localStorage read failed: {e:?}"))),
            WebBackend::Mem(mem) => Ok(mem.read(&key)),
        })
    }
}

/// Write a slot's contents, replacing any previous value.
///
/// Native: parent directories are created, and the write is atomic — the
/// contents go to a sibling `.tmp` file first, then rename over the target
/// (a failed rename removes the temp file and reports the rename error).
/// Consequences: the save *directory* must be writable, and a symlinked slot
/// becomes a regular file after the first save.
pub fn write(slot: &Path, contents: &str) -> io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(parent) = slot.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut tmp = slot.as_os_str().to_os_string();
        tmp.push(".tmp");
        let tmp = std::path::PathBuf::from(tmp);
        std::fs::write(&tmp, contents)?;
        std::fs::rename(&tmp, slot).inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
    }
    #[cfg(target_arch = "wasm32")]
    {
        let key = slot.to_string_lossy();
        BACKEND.with(|b| match &mut *b.borrow_mut() {
            WebBackend::Local(storage) => {
                storage
                    .set_item(&key, contents)
                    .map_err(|e| io::Error::other(format!("localStorage write failed: {e:?}")))?;
                dispatch_save_event(&key);
                Ok(())
            }
            WebBackend::Mem(mem) => {
                mem.write(&key, contents);
                Ok(())
            }
        })
    }
}

#[cfg(target_arch = "wasm32")]
enum WebBackend {
    /// Real browser localStorage — writes persist and fire [`SAVE_EVENT`].
    Local(web_sys::Storage),
    /// Fallback when storage is unavailable — session-local, no events.
    Mem(MemoryStore),
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static BACKEND: std::cell::RefCell<WebBackend> = std::cell::RefCell::new(pick_backend());
}

#[cfg(target_arch = "wasm32")]
fn pick_backend() -> WebBackend {
    match web_sys::window().map(|w| w.local_storage()) {
        Some(Ok(Some(storage))) => WebBackend::Local(storage),
        _ => {
            log::warn!(
                "localStorage unavailable (private browsing or storage blocked) — \
                 saves are in-memory for this session"
            );
            WebBackend::Mem(MemoryStore::default())
        }
    }
}

/// Fire the [`SAVE_EVENT`] `CustomEvent` on `window` with the storage key as
/// `detail`. Best-effort: the persist already succeeded, so a missing window
/// or a dispatch failure is a silent no-op (same posture as
/// `web::set_boot_status`).
#[cfg(target_arch = "wasm32")]
fn dispatch_save_event(key: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let init = web_sys::CustomEventInit::new();
    init.set_detail(&wasm_bindgen::JsValue::from_str(key));
    if let Ok(event) = web_sys::CustomEvent::new_with_event_init_dict(SAVE_EVENT, &init) {
        let _ = window.dispatch_event(&event);
    }
}

/// The in-memory backend the web falls back to when localStorage is
/// unavailable. Compiled (and unit tested) on every target so its exact
/// semantics are covered by the native suite, mirroring `common::vfs::MemFs`.
#[derive(Default)]
pub struct MemoryStore {
    map: HashMap<String, String>,
}

impl MemoryStore {
    /// The stored value for `slot`, if any.
    pub fn read(&self, slot: &str) -> Option<String> {
        self.map.get(slot).cloned()
    }

    /// Store `contents` under `slot`, replacing any previous value.
    pub fn write(&mut self, slot: &str, contents: &str) {
        self.map.insert(slot.to_string(), contents.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_then_read_round_trips_and_leaves_no_temp_file() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        // Parent directories are created on demand (`saves/` on first run).
        let slot = dir.path().join("deep/nested/save.json");
        assert_eq!(read(&slot)?, None, "a missing slot reads as None, not an error");

        write(&slot, "{\"a\":1}")?;

        assert_eq!(read(&slot)?.as_deref(), Some("{\"a\":1}"));
        assert!(
            !dir.path().join("deep/nested/save.json.tmp").exists(),
            "temp file must be renamed away"
        );

        write(&slot, "new")?;
        assert_eq!(read(&slot)?.as_deref(), Some("new"), "a write replaces the previous contents");
        Ok(())
    }

    #[test]
    fn test_memory_store_matches_slot_semantics() {
        let mut store = MemoryStore::default();
        assert_eq!(store.read("k"), None, "absent slot reads as None");
        store.write("k", "v1");
        assert_eq!(store.read("k").as_deref(), Some("v1"));
        store.write("k", "v2");
        assert_eq!(store.read("k").as_deref(), Some("v2"), "write replaces");
    }
}
