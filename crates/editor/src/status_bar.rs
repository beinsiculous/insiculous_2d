//! Editor status bar displayed at the bottom of the window.
//!
//! Shows contextual status messages (left), runtime stats (center),
//! and version info (right). Status messages auto-clear after a timeout.

use glam::Vec2;
use ui::{Rect, UIContext};

use crate::theme::EditorTheme;

/// Height of the status bar in pixels.
pub const STATUS_BAR_HEIGHT: f32 = 22.0;

/// Duration in seconds before a status message auto-clears.
const MESSAGE_TIMEOUT: f32 = 3.0;

/// Runtime statistics for the status bar center section.
#[derive(Debug, Clone, Default)]
pub struct StatusBarStats {
    /// Number of entities in the world.
    pub entity_count: usize,
    /// Smoothed frames-per-second.
    pub fps: f32,
}

/// The editor status bar widget.
#[derive(Debug, Clone)]
pub struct StatusBar {
    /// Current status message (left section).
    message: Option<String>,
    /// Time remaining before the message auto-clears (seconds).
    message_timer: f32,
    /// Whether the current message should persist (errors).
    message_persistent: bool,
    /// Version string (right section).
    version: String,
    /// Runtime stats updated each frame.
    stats: StatusBarStats,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusBar {
    /// Create a new status bar.
    pub fn new() -> Self {
        Self {
            message: None,
            message_timer: 0.0,
            message_persistent: false,
            version: String::from("v0.1.0"),
            stats: StatusBarStats::default(),
        }
    }

    /// Set the version string displayed on the right.
    pub fn set_version(&mut self, version: impl Into<String>) {
        self.version = version.into();
    }

    /// Show a temporary status message (auto-clears after 3 seconds).
    pub fn show_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
        self.message_timer = MESSAGE_TIMEOUT;
        self.message_persistent = false;
    }

    /// Show a persistent status message (stays until explicitly cleared).
    pub fn show_error(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
        self.message_timer = 0.0;
        self.message_persistent = true;
    }

    /// Clear the current status message.
    pub fn clear_message(&mut self) {
        self.message = None;
        self.message_timer = 0.0;
        self.message_persistent = false;
    }

    /// Get the current message, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Update runtime stats.
    pub fn update_stats(&mut self, entity_count: usize, fps: f32) {
        self.stats.entity_count = entity_count;
        self.stats.fps = fps;
    }

    /// Tick the message timer. Call once per frame with delta time.
    pub fn update(&mut self, delta_time: f32) {
        if !self.message_persistent && self.message.is_some() {
            self.message_timer -= delta_time;
            if self.message_timer <= 0.0 {
                self.message = None;
                self.message_timer = 0.0;
            }
        }
    }

    /// Render the status bar. Returns the bar bounds for layout purposes.
    ///
    /// The status bar is positioned at the bottom of the given `window_size`.
    pub fn render(
        &self,
        ui: &mut UIContext,
        window_size: Vec2,
        theme: &EditorTheme,
    ) -> Rect {
        let bar = Rect::new(0.0, window_size.y - STATUS_BAR_HEIGHT, window_size.x, STATUS_BAR_HEIGHT);

        // Background
        ui.rect(bar, theme.status_bar_bg);

        // Top separator in the panel-border blue so the bar reads as chrome
        ui.line(
            Vec2::new(bar.x, bar.y),
            Vec2::new(bar.x + bar.width, bar.y),
            theme.border_panel,
            1.0,
        );

        let padding = 8.0;

        // Left section: status message
        let status_text = self.message.as_deref().unwrap_or("Ready");
        let status_color = if self.message_persistent {
            theme.error_red
        } else {
            theme.text_secondary
        };
        ui.label_in_bounds_styled(status_text, bar, ui::TextAlign::Left, status_color, theme.fonts.small, padding);

        // Center section: runtime stats
        let stats_text = format!(
            "Objects: {} | FPS: {:.0}",
            self.stats.entity_count,
            self.stats.fps,
        );
        ui.label_in_bounds_styled(&stats_text, bar, ui::TextAlign::Center, theme.text_muted, theme.fonts.small, padding);

        // Right section: version
        ui.label_in_bounds_styled(&self.version, bar, ui::TextAlign::Right, theme.accent_cyan, theme.fonts.small, padding);

        bar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Message lifetime: a plain message clears itself after
    /// `MESSAGE_TIMEOUT`, a newer message restarts the clock, and an error
    /// stays until the host clears it explicitly.
    #[test]
    fn test_message_auto_clears_but_an_error_persists_until_cleared() {
        let mut bar = StatusBar::new();

        bar.show_message("Saved");
        bar.update(MESSAGE_TIMEOUT - 1.0);
        assert_eq!(bar.message(), Some("Saved"), "still inside the timeout");
        bar.update(1.0);
        assert_eq!(bar.message(), None, "cleared once the timeout elapses");

        bar.show_message("First");
        bar.update(MESSAGE_TIMEOUT - 1.0);
        bar.show_message("Second");
        bar.update(MESSAGE_TIMEOUT - 1.0);
        assert_eq!(bar.message(), Some("Second"), "a new message restarts the timer");
        bar.update(1.0);
        assert_eq!(bar.message(), None);

        bar.show_error("Failed to save");
        bar.update(MESSAGE_TIMEOUT * 100.0);
        assert_eq!(bar.message(), Some("Failed to save"), "errors never time out");
        bar.clear_message();
        assert_eq!(bar.message(), None, "an explicit clear removes the error");
        bar.show_message("Next");
        bar.update(MESSAGE_TIMEOUT);
        assert_eq!(bar.message(), None, "clearing an error does not leave persistence sticky");
    }
}
