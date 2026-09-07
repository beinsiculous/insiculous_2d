//! Window management for the renderer.

pub(crate) use winit::{
    dpi::PhysicalSize,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};

use crate::error::RendererError;

/// Configuration for the window
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Title of the window
    pub title: String,
    /// Width of the window
    pub width: u32,
    /// Height of the window
    pub height: u32,
    /// Whether the window is resizable
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "insiculous_2d v0.1".to_string(),
            width: 800,
            height: 600,
            resizable: true,
        }
    }
}

/// Create a new window with the given configuration using an ActiveEventLoop
///
/// On the web, winit creates a canvas but does NOT insert it into the DOM
/// (a silent blank page). We insert it ourselves: swapped in place of the
/// page's `#game-canvas` placeholder when present (the site-embed contract,
/// id and a11y attributes carried over), else appended to `<body>`. Letting
/// winit own the canvas — rather than adopting an existing element via
/// `with_canvas` — is the path the H1 spike verified end-to-end.
pub fn create_window_with_active_loop(
    config: &WindowConfig,
    event_loop: &ActiveEventLoop,
) -> Result<std::sync::Arc<Window>, RendererError> {
    // Create window attributes
    let mut attributes = WindowAttributes::default();
    attributes.title = config.title.clone();
    attributes.inner_size = Some(PhysicalSize::new(config.width, config.height).into());
    attributes.resizable = config.resizable;

    // Create the window using ActiveEventLoop's create_window method
    let window = event_loop
        .create_window(attributes)
        .map_err(|e| RendererError::WindowCreationError(e.to_string()))?;

    #[cfg(target_arch = "wasm32")]
    insert_canvas_into_dom(&window);

    // Wrap the window in an Arc to ensure it outlives the event loop callback
    Ok(std::sync::Arc::new(window))
}

/// Put winit's canvas into the page: replace `#game-canvas` if the page has
/// such a placeholder (keeping its id and accessibility attributes), else
/// append to `<body>`. Focuses the canvas so keyboard input works at once.
///
/// MUST be called for every winit window created on the web, whatever code
/// creates it — winit never inserts its canvas into the DOM, and a detached
/// canvas renders silently into nothing (every pass valid, page black).
#[cfg(target_arch = "wasm32")]
pub fn insert_canvas_into_dom(window: &Window) {
    use wasm_bindgen::JsCast;
    use winit::platform::web::WindowExtWebSys;

    let Some(canvas) = window.canvas() else {
        log::error!("winit window has no canvas on web");
        return;
    };
    // Idempotent: if this window's canvas is already in the DOM (a second
    // caller — e.g. both WindowManager and a direct renderer user — or a
    // repeated `resumed`), there is nothing to do.
    if canvas.is_connected() {
        return;
    }
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };

    if let Some(placeholder) = document.get_element_by_id("game-canvas") {
        // width/height carry the embed's intended pixel size — without them
        // the canvas starts at the browser default (300x150) until winit's
        // own sizing kicks in.
        for attr in ["width", "height", "tabindex", "role", "aria-label"] {
            if let Some(value) = placeholder.get_attribute(attr) {
                let _ = canvas.set_attribute(attr, &value);
            }
        }
        if placeholder.replace_with_with_node_1(canvas.unchecked_ref()).is_ok() {
            canvas.set_id("game-canvas");
        } else {
            log::error!("could not replace #game-canvas placeholder");
        }
    } else if let Some(body) = document.body() {
        log::warn!("no #game-canvas placeholder; appending canvas to <body>");
        let _ = canvas.set_attribute("tabindex", "0");
        canvas.set_id("game-canvas");
        let _ = body.append_child(&canvas);
    }

    // Keyboard input needs focus; do it here because the page's own
    // focus-by-id script may have run before this canvas existed. Without
    // scrolling: a page that wants its canvas in view on load anchors it
    // itself, and a load that jumps the page is the complaint, not the cure.
    let _ = canvas.focus_with_options(&focus_without_scroll());
    focus_before_winit_does(&document);
}

/// The size the canvas is shown at, in physical pixels: its client box times the device
/// pixel ratio, or `None` before it has a box. The client box is the content plus any
/// padding, and winit measures the content box and reads the pointer from the padding
/// edge, so the embed contract keeps the canvas free of padding and border: with neither,
/// the three agree exactly. A web `Resized` can carry the size the window
/// was created or asked for, and when the page's stylesheet holds the box smaller the
/// observer that would correct it never fires, because the box never changed. The surface,
/// the camera's viewport and the window's tracked size must all come from this box, or the
/// engine draws at one size, is shown at another, and reads the pointer in a third.
///
/// This is for the `Resized` event only. The boot's forcing resize must keep trusting the
/// configured size: before the first layout the box can be 1×1, and configuring the
/// surface to it writes 1×1 attributes back onto the canvas, which then IS 1×1 forever.
#[cfg(target_arch = "wasm32")]
pub fn shown_size(window: &Window) -> Option<(u32, u32)> {
    use winit::platform::web::WindowExtWebSys;
    let canvas = window.canvas()?;
    let ratio = web_sys::window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0);
    let width = (canvas.client_width() as f64 * ratio).round() as u32;
    let height = (canvas.client_height() as f64 * ratio).round() as u32;
    (width > 0 && height > 0).then_some((width, height))
}

/// Focus options that leave the page where it is. Plain `focus()` scrolls a partly
/// visible element into view, which is the pitfall `focus_before_winit_does` exists for.
#[cfg(target_arch = "wasm32")]
fn focus_without_scroll() -> web_sys::FocusOptions {
    let options = web_sys::FocusOptions::new();
    options.set_prevent_scroll(true);
    options
}

/// Focus a pressed canvas before winit does, and without scrolling.
///
/// winit focuses its canvas on every pointer press so keyboard input follows the click,
/// with a plain `focus()`, and a plain `focus()` scrolls a partly visible element into
/// view. That scroll lands between the press and the release, so the pointer's offset
/// into the canvas changes mid-click and the hit lands where the canvas WAS — above the
/// cursor by the distance scrolled. A viewport shorter than the canvas can never fit it,
/// so there every click re-scrolls and every hit is off. A capture-phase listener on the
/// document runs before winit's target-phase one; once it has focused the canvas with
/// `preventScroll`, winit's `focus()` finds it focused and moves nothing. One listener per
/// page, installed by the first canvas and focusing whichever canvas was pressed, so a
/// page that creates a window more than once accumulates nothing.
#[cfg(target_arch = "wasm32")]
fn focus_before_winit_does(document: &web_sys::Document) {
    use std::cell::Cell;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    thread_local! {
        static INSTALLED: Cell<bool> = const { Cell::new(false) };
    }
    if INSTALLED.with(|installed| installed.replace(true)) {
        return;
    }

    let on_pointer_down = Closure::<dyn FnMut(web_sys::Event)>::new(|event: web_sys::Event| {
        let Some(pressed) = event.target() else {
            return;
        };
        if let Ok(canvas) = pressed.dyn_into::<web_sys::HtmlCanvasElement>() {
            let _ = canvas.focus_with_options(&focus_without_scroll());
        }
    });
    let options = web_sys::AddEventListenerOptions::new();
    options.set_capture(true);
    let _ = document.add_event_listener_with_callback_and_add_event_listener_options(
        "pointerdown",
        on_pointer_down.as_ref().unchecked_ref(),
        &options,
    );
    // The one listener lives as long as the page.
    on_pointer_down.forget();
}
