//! Editor status bar displayed at the bottom of the window.
//!
//! Shows contextual status messages (left), runtime stats (center),
//! and version info (right). Status messages auto-clear after a timeout.
//! An error naming a file carries the request that opens it, and a click on
//! the message reports it.

use glam::Vec2;
use ui::{Rect, UIContext};

use crate::external_tools::{Location, OpenRequest};
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
    /// What a click on the shown error opens; any other message drops it, so a click on
    /// "Saved" never opens a stale script.
    link: Option<OpenRequest>,
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
            link: None,
            version: String::from("v0.1.0"),
            stats: StatusBarStats::default(),
        }
    }

    /// Show a temporary status message (auto-clears after 3 seconds).
    pub fn show_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
        self.message_timer = MESSAGE_TIMEOUT;
        self.message_persistent = false;
        self.link = None;
    }

    /// Show a persistent status message (stays until explicitly cleared or replaced by a
    /// user's action), drawn with an "Error: " prefix and logged at `error`, so the terminal
    /// or the browser's console carries it too.
    pub fn show_error(&mut self, message: impl Into<String>) {
        self.show_linked_error(message, None);
    }

    /// [`Self::show_error`], with the request a click on the message files while it shows.
    pub fn show_linked_error(&mut self, message: impl Into<String>, link: Option<OpenRequest>) {
        let message = message.into();
        log::error!("{message}");
        self.message = Some(message);
        self.message_timer = 0.0;
        self.message_persistent = true;
        self.link = link;
    }

    /// A message the editor shows by itself rather than for a user's action: it never
    /// displaces an error, which the user has not yet read.
    pub fn show_notice(&mut self, message: impl Into<String>) {
        if self.is_showing_error() {
            log::info!("{}", message.into());
            return;
        }
        self.show_message(message);
    }

    /// The text the bar draws: the message, an error's with its "Error: " prefix; `None`
    /// while it says nothing.
    pub fn shown_text(&self) -> Option<String> {
        let message = self.message.as_deref()?;
        Some(if self.message_persistent {
            format!("Error: {message}")
        } else {
            message.to_string()
        })
    }

    /// Clear the current status message.
    pub fn clear_message(&mut self) {
        self.message = None;
        self.message_timer = 0.0;
        self.message_persistent = false;
        self.link = None;
    }

    /// What a click on the shown message opens, if it names a file.
    pub fn link(&self) -> Option<&OpenRequest> {
        self.link.as_ref()
    }

    /// The message's tooltip: the whole text when it was `ellipsized`, then, for a linked
    /// error, what a click opens, naming the file and the line.
    fn message_tooltip(&self, ellipsized: bool) -> Option<String> {
        let whole = self.shown_text()?;
        let hint = self.link.as_ref().map(|link| format!("Click to open {}", describe_request(link)));
        match (ellipsized, hint) {
            (true, Some(hint)) => Some(format!("{whole} — {hint}")),
            (true, None) => Some(whole),
            (false, hint) => hint,
        }
    }

    /// Get the current message, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Whether the bar is showing an error, which stays until cleared. A
    /// hover hint must not write over it: `show_message` would clear the
    /// persistence along with the text.
    pub fn is_showing_error(&self) -> bool {
        self.message_persistent && self.message.is_some()
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

    /// Render the status bar at the bottom of the given `window_size`. Returns the request
    /// a click on a linked error filed this frame, which the host queues. The click goes
    /// through the ui's interaction, so a dialog's scrim or an open menu blocks it.
    pub fn render(
        &self,
        ui: &mut UIContext,
        window_size: Vec2,
        theme: &EditorTheme,
    ) -> Option<OpenRequest> {
        let bar = Rect::new(0.0, window_size.y - STATUS_BAR_HEIGHT, window_size.x, STATUS_BAR_HEIGHT);

        // Background
        ui.rect(bar, theme.status_bar_bg);

        // Top separator so the bar reads as chrome
        ui.line(
            Vec2::new(bar.x, bar.y),
            Vec2::new(bar.x + bar.width, bar.y),
            theme.border_subtle,
            1.0,
        );

        let padding = crate::layout::PADDING;
        let font_size = theme.fonts.small;
        let measure = |text: &str| ui.measure_text_styled(text, font_size).x;

        let shown_message = self.shown_text();
        let status_text = shown_message.as_deref().unwrap_or("Ready");
        let status_color = if self.message_persistent {
            theme.status_error
        } else {
            theme.text_secondary
        };
        let stats_text = format!(
            "Objects: {} | FPS: {:.0}",
            self.stats.entity_count,
            self.stats.fps,
        );
        // The message has the room up to the version: an error cut short loses the words
        // that say what to do, so the stats step aside while a long one shows, and only a
        // message longer than that is shortened, its whole text kept as the tooltip.
        let message_room = (bar.width - measure(&self.version) - 4.0 * padding).max(0.0);
        let shown = crate::row_layout::ellipsize(status_text, message_room, measure);
        let stats_left = bar.center().x - measure(&stats_text) / 2.0;
        let shown_width = measure(&shown);
        let stats_fit = padding + shown_width + padding <= stats_left;
        let message_bounds = Rect::new(bar.x, bar.y, message_room + padding, bar.height);
        ui.label_in_bounds_styled(&shown, message_bounds, ui::TextAlign::Left, status_color, font_size, padding);
        // The text's own rect, not the message's room, which runs under the stats: the
        // tooltip says what a click does, so it must not show where a click does nothing.
        let text_bounds = Rect::new(bar.x, bar.y, shown_width + 2.0 * padding, bar.height);
        let clicked = self.link.as_ref().and_then(|link| {
            let result = ui.interact(ui::WidgetId::hashed("status_bar_link"), text_bounds, true);
            if matches!(result.state, ui::WidgetState::Hovered | ui::WidgetState::Active) {
                ui.request_cursor(ui::CursorIcon::Pointer);
            }
            result.clicked.then(|| link.clone())
        });
        if let Some(tooltip) = self.message_tooltip(shown != status_text) {
            ui.tooltip(text_bounds, &tooltip);
        }
        if stats_fit {
            ui.label_in_bounds_styled(&stats_text, bar, ui::TextAlign::Center, theme.text_muted, font_size, padding);
        }
        ui.label_in_bounds_styled(&self.version, bar, ui::TextAlign::Right, theme.accent_cyan, font_size, padding);

        clicked
    }
}

/// A request as the tooltip names it: the file, and its line when it has one.
fn describe_request(request: &OpenRequest) -> String {
    match request.location {
        Location::Line { line, .. } => format!("{} at line {line}", request.path.display()),
        _ => request.path.display().to_string(),
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

    /// The texts a frame drew, for the layout tests below.
    fn drawn(ui: &UIContext) -> Vec<String> {
        ui.draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                ui::DrawCommand::TextPlaceholder { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    /// A long message keeps its words: the stats step aside for it rather than draw over
    /// it, and a short one leaves them where they were.
    #[test]
    fn test_a_long_message_moves_the_stats_aside_and_a_short_one_keeps_them() {
        let theme = EditorTheme::default();
        let input = input::InputHandler::new();
        let window = Vec2::new(1280.0, 720.0);
        for (message, stats_drawn) in [
            ("Saved", true),
            ("Song save failed: the song has no provenance: a song needs its author and license before it is saved", false),
        ] {
            let mut bar = StatusBar::new();
            bar.show_error(message);
            let mut ui = UIContext::new();
            ui.begin_frame(&input, window);
            bar.render(&mut ui, window, &theme);
            let texts = drawn(&ui);
            ui.end_frame();
            let shown = format!("Error: {message}");
            assert!(texts.contains(&shown), "the whole message is drawn: {texts:?}");
            assert_eq!(texts.iter().any(|text| text.starts_with("Objects:")), stats_drawn, "{message}");
        }
    }

    /// An error is told from a message by more than its colour — the drawn text carries
    /// "Error: " and the amber token — while the stored message keeps the caller's words;
    /// the editor's own notices leave it in place, and a user's next action replaces it.
    #[test]
    fn test_an_error_is_prefixed_amber_and_held_against_notices_but_not_against_a_users_action() {
        let theme = EditorTheme::default();
        let input = input::InputHandler::new();
        let window = Vec2::new(1280.0, 720.0);
        let mut bar = StatusBar::new();
        bar.show_error("Play could not start the song");
        assert_eq!(bar.message(), Some("Play could not start the song"));
        assert_eq!(bar.shown_text().as_deref(), Some("Error: Play could not start the song"));

        let mut ui = UIContext::new();
        ui.begin_frame(&input, window);
        bar.render(&mut ui, window, &theme);
        let amber = ui.draw_list().commands().iter().any(|command| {
            matches!(command, ui::DrawCommand::TextPlaceholder { text, color, .. }
                if text == "Error: Play could not start the song" && *color == theme.status_error)
        });
        ui.end_frame();
        assert!(amber, "the error is drawn prefixed, in the status error colour");

        bar.show_notice("API batch committed by Play");
        assert_eq!(bar.message(), Some("Play could not start the song"), "a notice never displaces an error");
        bar.show_message("Saved");
        assert_eq!(bar.shown_text().as_deref(), Some("Saved"), "a user's action replaces it");
        bar.show_notice("Snapshot warning");
        assert_eq!(bar.message(), Some("Snapshot warning"), "with no error a notice shows like any message");
    }

    fn paddle_at_line_12() -> OpenRequest {
        OpenRequest { path: "scripts/paddle.rhai".into(), location: Location::Line { line: 12, column: None } }
    }

    /// A press then a release on the message, through the ui's interaction; returns what
    /// the release frame's render reported and the pointer it asked for while hovered.
    fn click_message(bar: &StatusBar) -> (Option<OpenRequest>, ui::CursorIcon) {
        let theme = EditorTheme::default();
        let mut input = input::InputHandler::new();
        let mut ui = UIContext::new();
        let on_message = Vec2::new(12.0, crate::test_support::WINDOW.y - STATUS_BAR_HEIGHT / 2.0);
        let window = crate::test_support::WINDOW;
        let (_, clicked) =
            crate::test_support::click_through(&mut ui, &mut input, on_message, |ui| bar.render(ui, window, &theme));
        let cursor = crate::test_support::move_to(&mut ui, &mut input, on_message, |ui| {
            bar.render(ui, window, &theme);
            ui.requested_cursor()
        });
        (clicked, cursor)
    }

    #[test]
    fn a_click_on_a_linked_error_reports_its_request_and_on_anything_else_nothing() {
        let mut bar = StatusBar::new();
        bar.show_linked_error("scripts/paddle.rhai:12: [Syntax] expected ';'", Some(paddle_at_line_12()));
        assert_eq!(click_message(&bar), (Some(paddle_at_line_12()), ui::CursorIcon::Pointer));

        bar.show_error("Failed to save");
        assert_eq!(click_message(&bar).0, None, "an unlinked error opens nothing");
        bar.show_message("Saved");
        assert_eq!(click_message(&bar), (None, ui::CursorIcon::Default), "nor does a plain message");
    }

    #[test]
    fn a_message_replacing_a_linked_error_drops_its_link() {
        let mut bar = StatusBar::new();
        bar.show_linked_error("scripts/paddle.rhai:12: [Syntax] expected ';'", Some(paddle_at_line_12()));
        bar.show_notice("API batch committed by Play");
        assert_eq!(bar.link(), Some(&paddle_at_line_12()), "a notice leaves the error and its link");
        bar.show_message("Saved");
        assert_eq!(bar.link(), None, "a click on \"Saved\" never opens a stale script");
        bar.show_linked_error("again", Some(paddle_at_line_12()));
        bar.show_error("Failed to save");
        assert_eq!(bar.link(), None, "an unlinked error drops it too");
    }

    /// Rest the pointer at `point` past the tooltip's delay; the text the tooltip band draws.
    fn tooltip_resting_at(bar: &StatusBar, point: Vec2) -> Vec<String> {
        let theme = EditorTheme::default();
        let window = crate::test_support::WINDOW;
        let mut input = input::InputHandler::new();
        let mut ui = UIContext::new();
        input.mouse_mut().update_position(point.x, point.y);
        for _ in 0..10 {
            ui.begin_frame_dt(&input, window, 0.1);
            bar.render(&mut ui, window, &theme);
            ui.end_frame();
            input.update();
        }
        let band = ui::UiLayer::Tooltip.depth_base()..ui::UiLayer::DragGhost.depth_base();
        ui.draw_list()
            .commands()
            .iter()
            .filter(|command| band.contains(&command.depth()))
            .filter_map(|command| match command {
                ui::DrawCommand::TextPlaceholder { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_click_to_open_tooltip_shows_over_the_message_and_not_over_the_stats_beside_it() {
        let mut bar = StatusBar::new();
        bar.show_linked_error("scripts/paddle.rhai:12: [Syntax] expected ';'", Some(paddle_at_line_12()));
        let y = crate::test_support::WINDOW.y - STATUS_BAR_HEIGHT / 2.0;
        let on_message = tooltip_resting_at(&bar, Vec2::new(12.0, y));
        assert!(on_message.iter().any(|text| text.contains("Click to open scripts/paddle.rhai at line 12")), "{on_message:?}");
        let on_stats = tooltip_resting_at(&bar, Vec2::new(crate::test_support::WINDOW.x / 2.0, y));
        assert!(on_stats.is_empty(), "a click on the stats opens nothing, so no tooltip says it does: {on_stats:?}");
    }

    #[test]
    fn the_tooltip_names_the_file_and_line_after_the_whole_text_when_shortened() {
        let mut bar = StatusBar::new();
        bar.show_linked_error("scripts/paddle.rhai:12: [Syntax] expected ';'", Some(paddle_at_line_12()));
        assert_eq!(bar.message_tooltip(false).as_deref(), Some("Click to open scripts/paddle.rhai at line 12"));
        assert_eq!(
            bar.message_tooltip(true).as_deref(),
            Some("Error: scripts/paddle.rhai:12: [Syntax] expected ';' — Click to open scripts/paddle.rhai at line 12")
        );
        bar.show_error("Failed to save");
        assert_eq!(bar.message_tooltip(false), None, "an unlinked message that fits needs none");
    }
}
