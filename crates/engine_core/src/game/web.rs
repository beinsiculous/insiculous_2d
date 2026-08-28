//! Wasm-side renderer bring-up for `GameRunner`, split out of `game.rs`.
//!
//! wgpu adapter/device requests are genuinely async on WebGPU and the
//! browser main thread cannot block, so `resumed()` spawns the setup as a
//! browser task; the frame driver adopts the finished renderer on a later
//! frame via [`GameRunner::drain_pending_renderer`]. Until then
//! `update_and_render` early-returns on its existing "no asset manager yet"
//! guard while the redraw chain keeps polling.

use std::cell::RefCell;
use std::rc::Rc;

use renderer::{Renderer, RendererError};
use winit::event::{ElementState, TouchPhase, WindowEvent};

use super::{Game, GameRunner};

/// Shared slot the spawned init task fills and the frame loop drains.
pub(super) type PendingRenderer = Rc<RefCell<Option<Result<Renderer, RendererError>>>>;

/// H7: failed audio-upgrade attempts before giving up. Retrying on later
/// gestures covers a transient in-gesture failure without unbounded
/// AudioContext churn on a permanently broken setup (adversarial review F1).
const MAX_AUDIO_GESTURE_ATTEMPTS: u8 = 5;

/// True for events that grant browser user activation (keydown / mousedown /
/// touchstart) — the only moments an `AudioContext` may start. Key
/// auto-repeat is excluded: it is not a fresh activation, and a held key
/// would otherwise retry at repeat rate. `CursorMoved`/`MouseWheel` grant no
/// activation and fire far too often to probe from.
fn is_activation_gesture(event: &WindowEvent) -> bool {
    match event {
        WindowEvent::KeyboardInput { event, .. } => {
            event.state == ElementState::Pressed && !event.repeat
        }
        WindowEvent::MouseInput { state: ElementState::Pressed, .. } => true,
        WindowEvent::Touch(touch) => touch.phase == TouchPhase::Started,
        _ => false,
    }
}

impl<G: Game> GameRunner<G> {
    /// H7: upgrade the disabled web audio manager to a real `OutputStream`
    /// on a user activation gesture. Runs synchronously inside winit's DOM
    /// event dispatch, which is what satisfies the browser autoplay policy
    /// (see coordination/H1_SPIKE.md — a startup `try_default()` succeeding
    /// proves nothing). Failures retry on later gestures up to the cap; the
    /// manager stays fully functional in disabled mode either way, and games
    /// can still call `ctx.audio.enable_output()` themselves.
    pub(super) fn upgrade_audio_on_gesture(&mut self, event: &WindowEvent) {
        if self.audio_manager.is_enabled()
            || self.audio_gesture_attempts >= MAX_AUDIO_GESTURE_ATTEMPTS
            || !is_activation_gesture(event)
        {
            return;
        }
        match self.audio_manager.enable_output() {
            Ok(()) => log::info!("Web audio enabled after user gesture"),
            Err(e) => {
                self.audio_gesture_attempts += 1;
                // Log the first failure and the give-up, not the retries.
                if self.audio_gesture_attempts == 1 {
                    log::warn!("Web audio init failed on first gesture: {e}. Retrying on later gestures.");
                } else if self.audio_gesture_attempts == MAX_AUDIO_GESTURE_ATTEMPTS {
                    log::warn!(
                        "Web audio init failed {MAX_AUDIO_GESTURE_ATTEMPTS} times; giving up. Audio stays disabled."
                    );
                }
            }
        }
    }

    /// Kick off async renderer creation as a browser task. Called from
    /// `resumed()`; re-fire is prevented by its window-created guard.
    pub(super) fn spawn_renderer_init(&mut self) {
        let Some(window) = self.window_manager.window_clone() else {
            log::error!("spawn_renderer_init: no window to render to");
            return;
        };
        let pending = Rc::clone(&self.pending_renderer);
        let renderer_config = renderer::RendererConfig { vsync: self.config.vsync };
        wasm_bindgen_futures::spawn_local(async move {
            let result = renderer::init_with_config(window.clone(), renderer_config).await;
            *pending.borrow_mut() = Some(result);
            // Wake the frame loop so the drain below runs promptly.
            window.request_redraw();
        });
    }

    /// Adopt a finished renderer (or surface its error). Called at the top
    /// of every frame until the render manager is initialized.
    pub(super) fn drain_pending_renderer(&mut self) {
        if self.render_manager.is_initialized() {
            return;
        }
        let taken = self.pending_renderer.borrow_mut().take();
        match taken {
            Some(Ok(renderer)) => {
                self.render_manager.complete_init(renderer, self.config.clear_color);
                self.finish_renderer_setup();
                // Force the surface to the game's configured size. The
                // canvas CANNOT be trusted here: winit's resize observer may
                // report 0 before first layout, the surface then configures
                // at the clamped 1x1, and wgpu's configure() writes those
                // 1x1 attrs back onto the canvas — which drives its CSS
                // layout, so the observer keeps confirming 1x1 forever.
                // Reconfiguring at the (always nonzero) GameConfig size
                // resets the canvas attrs and breaks that feedback loop;
                // genuine page-layout resizes still arrive as normal winit
                // Resized events afterwards.
                self.window_manager.resize(self.config.width, self.config.height);
                self.render_manager.resize(self.config.width, self.config.height);
                // The game draws its own frames from here on.
                crate::web::set_boot_status("");
            }
            Some(Err(e)) => {
                log::error!("Renderer init failed: {e}");
                crate::web::set_boot_status(&format!("Renderer init failed: {e}"));
            }
            None => {}
        }
    }
}
