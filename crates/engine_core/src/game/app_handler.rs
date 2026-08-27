//! winit `ApplicationHandler` wiring for `GameRunner`, split out of `game.rs`.
//!
//! Owns the event-loop callbacks: window + renderer bring-up on resume,
//! window events (input, resize, close), and frame driving — natively from
//! `about_to_wait`, on the web from `RedrawRequested` (which maps to
//! `requestAnimationFrame`).

use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::WindowId,
};

use crate::contexts::GameContext;

use super::{Game, GameRunner};

impl<G: Game> GameRunner<G> {
    /// One frame: update + render, honor exit requests, pace (native only),
    /// and re-arm the next redraw. Called from `about_to_wait` natively and
    /// from `RedrawRequested` on the web.
    fn drive_frame(&mut self, event_loop: &ActiveEventLoop) {
        self.update_and_render();
        self.save_input_settings_if_dirty();
        if self.exit_requested {
            self.shutdown(event_loop);
            return;
        }
        // Enforce GameConfig::target_fps by sleeping out the frame budget
        // (no-op on wasm — requestAnimationFrame paces the loop).
        self.game_loop_manager.throttle();
        self.window_manager.request_redraw();
    }

    /// Persist input bindings when a mutation happened this frame
    /// (write-through — a browser tab may never reach `shutdown`, so waiting
    /// for the clean-exit save would lose web rebindings). A failed save
    /// re-marks the settings dirty to retry next frame, warning only once
    /// per failure streak.
    fn save_input_settings_if_dirty(&mut self) {
        if !self.player_input.take_dirty() {
            return;
        }
        let Some(path) = &self.config.input_settings_path else {
            return;
        };
        match crate::input_settings_io::save(std::path::Path::new(path), &self.player_input) {
            Ok(()) => self.input_save_failing = false,
            Err(e) => {
                if !self.input_save_failing {
                    log::warn!("Could not save input settings to {}: {}", path, e);
                }
                self.input_save_failing = true;
                self.player_input.mark_dirty();
            }
        }
    }

    /// Clean shutdown: notify the game, persist input bindings, tear the
    /// scene down, and exit the event loop. Shared by the window close
    /// button and game-requested exits (`GameContext::exit_requested`).
    fn shutdown(&mut self, event_loop: &ActiveEventLoop) {
        self.game.on_exit();
        // Persist input bindings (incl. runtime pad re-assignments)
        if let Some(path) = &self.config.input_settings_path {
            if let Err(e) = crate::input_settings_io::save(
                std::path::Path::new(path),
                &self.player_input,
            ) {
                log::warn!("Could not save input settings to {}: {}", path, e);
            }
        }
        let _ = self.scene.stop();
        let _ = self.scene.shutdown();
        // The browser tab stays open with the last frame frozen on the
        // canvas — say so instead of looking crashed.
        #[cfg(target_arch = "wasm32")]
        crate::web::set_boot_status("Game ended — reload the page to play again");
        event_loop.exit();
    }
}

impl<G: Game> ApplicationHandler<()> for GameRunner<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Only create window once
        if self.window_manager.is_created() {
            return;
        }

        // Create window using window manager
        if let Err(e) = self.window_manager.create(event_loop) {
            log::error!("Failed to create window: {}", e);
            event_loop.exit();
            return;
        }

        // Initialize renderer: native blocks (pollster at the outer edge);
        // the web spawns the async wgpu setup and the frame driver adopts it
        // once it completes (game/web.rs).
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(e) = self.init_renderer() {
            log::error!("Failed to initialize renderer: {}", e);
            event_loop.exit();
            return;
        }
        #[cfg(target_arch = "wasm32")]
        self.spawn_renderer_init();

        // Initialize scene lifecycle
        if let Err(e) = self.scene.initialize() {
            log::error!("Scene init error: {}", e);
        }
        if let Err(e) = self.scene.start() {
            log::error!("Scene start error: {}", e);
        }

        log::info!("Game started: {}", self.config.title);

        // Kick the first frame. Harmless natively (about_to_wait drives the
        // loop anyway); required on the web where RedrawRequested drives it.
        self.window_manager.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // Only handle events for our window
        if !self.window_manager.is_our_window(window_id) {
            return;
        }

        // Forward to input handler
        self.input.handle_window_event(&event);

        match event {
            WindowEvent::CloseRequested => {
                self.shutdown(event_loop);
            }
            WindowEvent::Resized(size) => {
                // Update window manager's tracked size
                self.window_manager.resize(size.width, size.height);
                // Update render manager
                self.render_manager.resize(size.width, size.height);
                // Notify game
                self.game.on_resize(size.width, size.height);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.window_manager.set_scale_factor(scale_factor);
                log::info!("Scale factor changed to: {}", scale_factor);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    // Create context and call handlers
                    let window_size = self.window_size();
                    if let Some(asset_manager) = &mut self.asset_manager {
                        let mut ctx = GameContext {
                            input: &self.input,
                            players: &mut self.player_input,
                            world: &mut self.scene.world,
                            assets: asset_manager,
                            audio: &mut self.audio_manager,
                            ui: self.ui_manager.ui_context(),
                            delta_time: 0.0,
                            window_size,
                            chaos_mode: self.config.chaos_mode,
                            time_scale: self.time_scale,
                            exit_requested: false,
                            achievements: &mut self.achievements,
                            scores: &mut self.scores,
                            particles: &mut self.particles,
                            lines: &mut self.lines,
                            strings: &mut self.strings,
                        };

                        match event.state {
                            ElementState::Pressed => {
                                self.game.on_key_pressed(key, &mut ctx);
                            }
                            ElementState::Released => {
                                self.game.on_key_released(key, &mut ctx);
                            }
                        }

                        // Persist chaos-mode/time-scale/exit changes made in
                        // key handlers too.
                        self.config.chaos_mode = ctx.chaos_mode;
                        self.time_scale = ctx.time_scale;
                        self.exit_requested |= ctx.exit_requested;
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // Native: rendering is driven from about_to_wait; nothing here.
                // Web: this IS the frame driver — request_redraw() maps to
                // requestAnimationFrame, so drive_frame re-arming itself is
                // the browser-paced loop.
                #[cfg(target_arch = "wasm32")]
                self.drive_frame(event_loop);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Native frame driver (unchanged behavior). On the web this callback
        // has no vsync pacing and stalls while occluded differently, so
        // frames are driven from RedrawRequested instead.
        #[cfg(not(target_arch = "wasm32"))]
        self.drive_frame(event_loop);
        #[cfg(target_arch = "wasm32")]
        let _ = event_loop;
    }
}
