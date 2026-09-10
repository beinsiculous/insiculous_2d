//! Web boot phase: logging, asset prefetch, and DOM status reporting.
//!
//! On the web every asset must be in memory before the game constructs —
//! loaders are synchronous on all targets. A game's wasm entry point runs
//! [`preload_assets`] to fetch its generated `manifest.json` and every
//! listed file into the [`common::vfs`] map, then calls `run_game` exactly
//! like a native `main`.
//!
//! # Canonical VFS keys
//! Each manifest entry is stored under `{base_url}/{entry}` — the same
//! string runtime reads produce by joining `GameConfig.asset_base_path`
//! with a relative asset name, so a game configured with
//! `asset_base_path = base_url` hits every fetched byte without any
//! path translation.

use std::sync::atomic::{AtomicBool, Ordering};

use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

/// One-way latch: this page has been left (navigation away, tab close, or a
/// back/forward-cache freeze). The frame loop treats it as fatal — issue
/// Firefox's bfcache RESTORES a frozen game page and our rAF loop
/// would resume against a WebGPU queue the (in-parent on Linux) wgpu may
/// have dropped, and the first message to a dead queue id panics Firefox's
/// MAIN process. Never resume a page the browser took away.
static PAGE_EXITED: AtomicBool = AtomicBool::new(false);

/// Message shown when the page-exit latch stops the loop.
pub(crate) const PAGE_EXIT_STATUS: &str =
    "Game stopped by navigation — reload the page to play again";

/// Whether the page-exit latch has fired.
pub fn page_exited() -> bool {
    PAGE_EXITED.load(Ordering::Relaxed)
}

/// Install `pagehide`/`pageshow` listeners that stop the frame loop for
/// good once the page is left. Called by `run_game`'s web path before the
/// event loop is handed to the browser; idempotent enough (duplicate
/// listeners just re-set the same latch).
pub fn install_page_exit_guard() {
    use wasm_bindgen::closure::Closure;
    let Some(window) = web_sys::window() else { return };

    let on_pagehide = Closure::<dyn FnMut(web_sys::Event)>::new(move |_event| {
        PAGE_EXITED.store(true, Ordering::Relaxed);
    });
    let _ = window.add_event_listener_with_callback(
        "pagehide",
        on_pagehide.as_ref().unchecked_ref(),
    );
    on_pagehide.forget();

    // A bfcache restore (`pageshow` with persisted=true) would otherwise
    // resume the frozen loop; the latch is already set from the pagehide,
    // so just make the outcome legible immediately (the fatal path repeats
    // the same message when the next rAF tick lands).
    let on_pageshow =
        Closure::<dyn FnMut(web_sys::PageTransitionEvent)>::new(move |event: web_sys::PageTransitionEvent| {
            if event.persisted() {
                PAGE_EXITED.store(true, Ordering::Relaxed);
                set_boot_status(PAGE_EXIT_STATUS);
            }
        });
    let _ = window.add_event_listener_with_callback(
        "pageshow",
        on_pageshow.as_ref().unchecked_ref(),
    );
    on_pageshow.forget();
}

thread_local! {
    /// The running loop's proxy, published by `run_game` before the browser
    /// takes the loop over. `None` until then, and on a page that never ran
    /// a game.
    static WAKE_PROXY: std::cell::RefCell<Option<winit::event_loop::EventLoopProxy<crate::game::WakeUp>>> =
        const { std::cell::RefCell::new(None) };
    /// Whether a pump timer is scheduled, so however many callers ask —
    /// the installer, a visibility change, the proxy's arrival — one chain
    /// runs, never two driving double frames.
    static PUMP_ARMED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Publish the loop's proxy so the hidden-document timer can drive frames.
/// The pump is installed before the loop exists, so a page hidden all
/// through its boot is started from here, once there is a loop to wake.
pub fn set_wake_proxy(proxy: winit::event_loop::EventLoopProxy<crate::game::WakeUp>) {
    WAKE_PROXY.with(|slot| *slot.borrow_mut() = Some(proxy));
    start_pump_if_hidden();
}

/// Ask the loop for one frame. `false` once the loop is gone.
fn send_wake_up() -> bool {
    WAKE_PROXY.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|proxy| proxy.send_event(crate::game::WakeUp).is_ok())
            .unwrap_or(false)
    })
}

/// Keep frames coming while the document is hidden.
///
/// `request_redraw` is an animation frame under winit's web backend and a
/// hidden tab gets none, so a game whose page is backgrounded would freeze
/// mid-answer. The timer starts on the transition to hidden — a frame that
/// rAF had already scheduled and will never deliver does not matter — and
/// stops as soon as the document is visible again, leaving the ordinary
/// browser-paced loop in charge. A page that is already hidden when this
/// installs had its transition before anyone listened, so the pump also
/// starts right here in that case.
pub fn install_hidden_frame_pump() {
    use wasm_bindgen::closure::Closure;
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };

    let on_visibility_change = Closure::<dyn FnMut(web_sys::Event)>::new(move |_event| {
        let Some(window) = web_sys::window() else { return };
        let Some(document) = window.document() else { return };
        if document.visibility_state() != web_sys::VisibilityState::Hidden {
            return;
        }
        start_pump_if_hidden();
    });
    let _ = document.add_event_listener_with_callback(
        "visibilitychange",
        on_visibility_change.as_ref().unchecked_ref(),
    );
    on_visibility_change.forget();
    start_pump_if_hidden();
}

/// Start a pump chain unless one is already scheduled.
fn start_pump_if_hidden() {
    if PUMP_ARMED.with(|armed| armed.get()) {
        return;
    }
    pump_while_hidden();
}

/// One tick of the hidden-document pump: drive a frame, then re-arm in
/// 100 ms while the document is still hidden and the loop still lives. With
/// no proxy yet the chain simply ends; `set_wake_proxy` restarts it.
fn pump_while_hidden() {
    use wasm_bindgen::closure::Closure;
    PUMP_ARMED.with(|armed| armed.set(false));
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };
    if document.visibility_state() != web_sys::VisibilityState::Hidden || page_exited() {
        return;
    }
    if !send_wake_up() {
        return;
    }
    let on_timeout = Closure::once_into_js(pump_while_hidden);
    if window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            on_timeout.as_ref().unchecked_ref(),
            HIDDEN_FRAME_INTERVAL_MILLISECONDS,
        )
        .is_ok()
    {
        PUMP_ARMED.with(|armed| armed.set(true));
    }
}

/// How often a hidden document is driven. Browsers clamp background timers,
/// so a shorter interval buys nothing.
const HIDDEN_FRAME_INTERVAL_MILLISECONDS: i32 = 100;

/// A failure during the web boot phase (fetch, HTTP, or manifest parse).
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct WebBootError(pub String);

/// Install the panic hook and console logger. Idempotent — safe to call
/// from every entry point.
pub fn init_web_logging() {
    console_error_panic_hook::set_once();
    // A second init returns Err; ignoring it keeps this idempotent.
    let _ = console_log::init_with_level(log::Level::Info);
}

/// Write `text` to the page's `#game-loading` status element, if present.
/// Missing element is a silent no-op so bare test pages still work.
pub fn set_boot_status(text: &str) {
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("game-loading"))
    {
        el.set_text_content(Some(text));
    }
}

/// Read the page's `#game-loading` status text, if the element is there.
/// The boot status is where the renderer writes its own failure, so it is
/// also where a page reads whether the runtime came up.
pub fn boot_status() -> Option<String> {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("game-loading"))
        .and_then(|element| element.text_content())
}

/// Read a URL search query parameter value by key.
pub fn query_param(name: &str) -> Option<String> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    let search_params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    search_params.get(name)
}

/// Fetch `{base_url}/manifest.json` (a JSON array of relative paths) and
/// every listed file into the VFS under `{base_url}/{entry}`.
///
/// `base_url` MUST be the same string the game passes to
/// `GameConfig::with_asset_base_path` — keep both behind ONE constant (see
/// pong's `web_entry.rs::ASSET_BASE`). A divergence means every runtime
/// read misses the map and each loader reports "not in vfs".
pub async fn preload_assets(base_url: &str) -> Result<(), WebBootError> {
    set_boot_status("Loading assets…");
    let manifest_url = format!("{base_url}/manifest.json");
    let manifest_bytes = fetch_bytes(&manifest_url).await?;
    let entries: Vec<String> = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| WebBootError(format!("bad {manifest_url}: {e}")))?;

    let total = entries.len();
    for (index, entry) in entries.iter().enumerate() {
        set_boot_status(&format!("Loading assets… {}/{}", index + 1, total));
        let url = format!("{base_url}/{entry}");
        let bytes = fetch_bytes(&url).await?;
        common::vfs::insert(url, bytes);
    }
    set_boot_status("Starting…");
    Ok(())
}

/// Fetch one URL to bytes. HTTP errors (404 included) are ordinary
/// `WebBootError`s naming the URL — never a panic.
async fn fetch_bytes(url: &str) -> Result<Vec<u8>, WebBootError> {
    let window =
        web_sys::window().ok_or_else(|| WebBootError("no browser window".to_string()))?;
    let response_value = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|e| WebBootError(format!("fetch {url}: {}", js_to_string(&e))))?;
    let response: web_sys::Response = response_value
        .dyn_into()
        .map_err(|_| WebBootError(format!("fetch {url}: not a Response")))?;
    if !response.ok() {
        return Err(WebBootError(format!("fetch {url}: HTTP {}", response.status())));
    }
    let buffer_promise = response
        .array_buffer()
        .map_err(|e| WebBootError(format!("fetch {url}: {}", js_to_string(&e))))?;
    let buffer = JsFuture::from(buffer_promise)
        .await
        .map_err(|e| WebBootError(format!("fetch {url}: {}", js_to_string(&e))))?;
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

fn js_to_string(value: &JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}
