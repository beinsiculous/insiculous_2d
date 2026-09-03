//! Toast notifications for achievements: visual style, active queue, and
//! top-right HUD rendering.

use glam::Vec2;
use common::Color;
use ui::{Rect, UIContext};

/// Default time (seconds) a toast stays visible before fading out.
pub const DEFAULT_TOAST_DURATION: f32 = 4.0;

/// Visual styling for achievement toasts.
///
/// Colors carry their base alpha; the fade-out over a toast's last second
/// multiplies that alpha at draw time. Override via
/// [`AchievementManager::set_toast_style`](super::AchievementManager::set_toast_style).
#[derive(Debug, Clone, PartialEq)]
pub struct ToastStyle {
    /// Toast panel width in pixels.
    pub width: f32,
    /// Toast panel height in pixels.
    pub height: f32,
    /// Margin from the window's top-right corner.
    pub margin: f32,
    /// Vertical spacing between stacked toasts.
    pub spacing: f32,
    /// Panel background color.
    pub background: Color,
    /// Panel border color.
    pub border: Color,
    /// Panel border stroke width.
    pub border_width: f32,
    /// "Achievement Unlocked!" header color.
    pub title_color: Color,
    /// Achievement name text color.
    pub name_color: Color,
    /// Achievement description text color.
    pub description_color: Color,
    /// Header font size.
    pub title_size: f32,
    /// Achievement name font size.
    pub name_size: f32,
    /// Description font size.
    pub description_size: f32,
}

impl Default for ToastStyle {
    fn default() -> Self {
        Self {
            width: 320.0,
            height: 72.0,
            margin: 16.0,
            spacing: 8.0,
            background: Color::new(0.08, 0.08, 0.12, 0.92),
            border: Color::new(1.0, 0.82, 0.2, 1.0),
            border_width: 2.0,
            title_color: Color::new(1.0, 0.82, 0.2, 1.0),
            name_color: Color::new(1.0, 1.0, 1.0, 1.0),
            description_color: Color::new(0.8, 0.8, 0.85, 1.0),
            title_size: 14.0,
            name_size: 16.0,
            description_size: 12.0,
        }
    }
}

/// Multiply a style color's base alpha by the toast's fade factor.
fn faded(color: Color, fade: f32) -> Color {
    Color::new(color.r, color.g, color.b, color.a * fade)
}

/// Active toast being displayed.
#[derive(Debug, Clone)]
pub(super) struct Toast {
    pub achievement_id: String,
    pub name: String,
    pub description: String,
    pub remaining: f32,
}

/// Queue of active achievement toasts displayed on screen.
#[derive(Debug, Clone)]
pub(super) struct ToastQueue {
    pub toasts: Vec<Toast>,
    pub duration: f32,
    pub style: ToastStyle,
}

impl ToastQueue {
    pub fn new() -> Self {
        Self {
            toasts: Vec::new(),
            duration: DEFAULT_TOAST_DURATION,
            style: ToastStyle::default(),
        }
    }

    pub fn push(&mut self, achievement_id: String, name: String, description: String) {
        self.toasts.push(Toast {
            achievement_id,
            name,
            description,
            remaining: self.duration,
        });
    }

    pub fn clear(&mut self) {
        self.toasts.clear();
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    pub fn tick(&mut self, delta_time: f32) {
        for toast in &mut self.toasts {
            toast.remaining -= delta_time;
        }
        self.toasts.retain(|t| t.remaining > 0.0);
    }

    pub fn draw(&self, ui: &mut UIContext, window_size: Vec2) {
        let style = &self.style;

        for (i, toast) in self.toasts.iter().enumerate() {
            let alpha = (toast.remaining / 1.0).clamp(0.0, 1.0);
            let x = window_size.x - style.width - style.margin;
            let y = style.margin + (style.height + style.spacing) * i as f32;

            let bg = faded(style.background, alpha);
            let border = faded(style.border, alpha);
            ui.panel_styled(Rect::new(x, y, style.width, style.height), bg, border, style.border_width);

            ui.label_styled(
                "Achievement Unlocked!",
                Vec2::new(x + 12.0, y + 10.0),
                faded(style.title_color, alpha),
                style.title_size,
            );
            ui.label_styled(
                &toast.name,
                Vec2::new(x + 12.0, y + 30.0),
                faded(style.name_color, alpha),
                style.name_size,
            );
            ui.label_styled(
                &toast.description,
                Vec2::new(x + 12.0, y + 52.0),
                faded(style.description_color, alpha),
                style.description_size,
            );
            let _ = toast.achievement_id; // reserved for future icon lookup
        }
    }
}
