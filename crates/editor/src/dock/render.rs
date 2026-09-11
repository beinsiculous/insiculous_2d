//! Dock panel rendering: chrome, headers, collapse chevrons, resize grabbers.

use glam::Vec2;
use ui::{Rect, UIContext, WidgetState};

use crate::theme::EditorTheme;

use super::{DockArea, DockPanel, DockPosition, PanelId, HEADER_HEIGHT};

/// Size of the square collapse-chevron hit area inside a header.
const CHEVRON_SIZE: f32 = 16.0;

/// Compute a panel's new size from a resize drag, clamped to
/// `[min_size, half the dock bounds]` so a panel can never swallow the
/// scene view.
pub(crate) fn resized_size(
    position: DockPosition,
    mouse: Vec2,
    panel_bounds: Rect,
    min_size: f32,
    dock_bounds: Rect,
) -> f32 {
    let raw = match position {
        DockPosition::Left => mouse.x - panel_bounds.x,
        DockPosition::Right => panel_bounds.x + panel_bounds.width - mouse.x,
        DockPosition::Top => mouse.y - panel_bounds.y,
        DockPosition::Bottom => panel_bounds.y + panel_bounds.height - mouse.y,
        _ => return min_size,
    };
    let max = match position {
        DockPosition::Left | DockPosition::Right => dock_bounds.width * 0.5,
        _ => dock_bounds.height * 0.5,
    };
    raw.clamp(min_size, max.max(min_size))
}

/// Bounds of the collapse chevron button inside a panel's header (or its
/// collapsed strip).
pub(super) fn chevron_bounds(panel: &DockPanel) -> Rect {
    let pad = (super::HEADER_HEIGHT - CHEVRON_SIZE) / 2.0;
    if panel.collapsed
        && matches!(panel.position, DockPosition::Left | DockPosition::Right)
    {
        // Vertical strip: chevron sits near the top.
        Rect::new(panel.bounds.x + pad, panel.bounds.y + pad, CHEVRON_SIZE, CHEVRON_SIZE)
    } else {
        // Expanded header (or collapsed horizontal strip): right-aligned.
        Rect::new(
            panel.bounds.x + panel.bounds.width - CHEVRON_SIZE - pad,
            panel.bounds.y + pad,
            CHEVRON_SIZE,
            CHEVRON_SIZE,
        )
    }
}

/// Draw the collapse chevron with line primitives (no font-coverage risk):
/// ▾ when expanded (click to collapse), ▸ when collapsed (click to expand).
fn draw_chevron(ui: &mut UIContext, bounds: Rect, collapsed: bool, theme: &EditorTheme) {
    let center = bounds.center();
    let color = theme.accent_cyan;
    if collapsed {
        // Pointing right
        ui.line(Vec2::new(center.x - 2.0, center.y - 4.0), Vec2::new(center.x + 2.0, center.y), color, 2.0);
        ui.line(Vec2::new(center.x + 2.0, center.y), Vec2::new(center.x - 2.0, center.y + 4.0), color, 2.0);
    } else {
        // Pointing down
        ui.line(Vec2::new(center.x - 4.0, center.y - 2.0), Vec2::new(center.x, center.y + 2.0), color, 2.0);
        ui.line(Vec2::new(center.x, center.y + 2.0), Vec2::new(center.x + 4.0, center.y - 2.0), color, 2.0);
    }
}

impl DockArea {
    /// Render all panels.
    ///
    /// Returns the content bounds for each visible, expanded panel. The
    /// caller should:
    /// 1. Render content within each bounds
    /// 2. Call `end_panel_content(ui)` after rendering each panel's content
    pub fn render(&mut self, ui: &mut UIContext, theme: &EditorTheme) -> Vec<(PanelId, Rect)> {
        let mut content_areas = Vec::new();
        let mut toggled: Option<PanelId> = None;
        let mut tab_clicked: Option<PanelId> = None;
        let mut overlay_closed = false;
        let narrow_overlay = self.narrow_overlay();

        for panel in &self.panels {
            if !panel.visible {
                continue;
            }

            // Narrow mode: a side panel is either the one overlay or a tab at
            // its edge. The overlay's chrome rides the floating band, above
            // the viewport it covers.
            if self.narrow && DockArea::is_side(panel.position) {
                if narrow_overlay == Some(panel.id) {
                    ui.begin_overlay_in(ui::UiLayer::Floating, panel.bounds);
                    render_panel_frame(ui, panel, theme);
                    // The tab that opened the overlay is gone while it is
                    // open, so its chevron is the way back to a bare viewport.
                    if render_chevron_button(ui, panel, theme) {
                        overlay_closed = true;
                    }
                    ui.end_overlay();
                    content_areas.push((panel.id, panel.expanded_content_bounds()));
                } else if render_narrow_tab(ui, panel, theme) {
                    tab_clicked = Some(panel.id);
                }
                continue;
            }

            if panel.collapsed && panel.is_collapsible() {
                if render_collapsed_strip(ui, panel, theme) {
                    toggled = Some(panel.id);
                }
                continue;
            }

            render_panel_frame(ui, panel, theme);

            if panel.is_collapsible() && render_chevron_button(ui, panel, theme) {
                toggled = Some(panel.id);
            }

            // Track content area (caller will push/pop clip rect around each panel's content)
            let content = panel.content_bounds();
            content_areas.push((panel.id, content));
        }

        if let Some(id) = toggled {
            self.toggle_panel_collapsed(id);
        }
        if let Some(id) = tab_clicked {
            self.open_narrow_overlay(id);
        }
        if overlay_closed {
            self.close_narrow_overlay();
        }

        content_areas
    }

    /// No-op kept for API compatibility. Clip rects are now managed per-panel by the caller.
    pub fn end_panel_content(&self, _ui: &mut UIContext, _panel_count: usize) {
        // Clip rects are now pushed/popped around each panel's content individually
        // by the caller, so this is no longer needed.
    }

    /// Handle resize dragging for panels, drawing a grabber line on hover/drag.
    ///
    /// Call this AFTER panel content has been rendered so the grabber draws
    /// on top of it.
    pub fn handle_resize(&mut self, ui: &mut UIContext, theme: &EditorTheme) {
        for i in 0..self.panels.len() {
            let panel = &self.panels[i];
            if !panel.visible || !panel.resizable || panel.collapsed {
                continue;
            }
            // A narrow-mode side panel has no edge to drag: its width is its
            // own, and the centre is not sharing the row with it.
            if self.narrow && DockArea::is_side(panel.position) {
                continue;
            }

            let resize_bounds = self.resize_handle_bounds(panel);

            // Create unique ID for resize handle
            let id = format!("resize_handle_{}", panel.id.0);
            let result = ui.interact(id.as_str(), resize_bounds, true);

            if result.state == WidgetState::Hovered || result.dragging {
                draw_resize_grabber(ui, &self.panels[i], theme);
            }

            if result.dragging {
                let mouse_pos = ui.mouse_pos();
                let dock_bounds = self.bounds;
                let panel = &mut self.panels[i];
                panel.size = resized_size(
                    panel.position,
                    mouse_pos,
                    panel.bounds,
                    panel.min_size,
                    dock_bounds,
                );

                // Re-layout after resize
                self.layout();
            }
        }
    }

    /// Get the resize handle bounds for a panel.
    fn resize_handle_bounds(&self, panel: &DockPanel) -> Rect {
        match panel.position {
            DockPosition::Left => Rect::new(
                panel.bounds.x + panel.bounds.width - self.resize_handle_size,
                panel.bounds.y,
                self.resize_handle_size * 2.0,
                panel.bounds.height,
            ),
            DockPosition::Right => Rect::new(
                panel.bounds.x - self.resize_handle_size,
                panel.bounds.y,
                self.resize_handle_size * 2.0,
                panel.bounds.height,
            ),
            DockPosition::Top => Rect::new(
                panel.bounds.x,
                panel.bounds.y + panel.bounds.height - self.resize_handle_size,
                panel.bounds.width,
                self.resize_handle_size * 2.0,
            ),
            DockPosition::Bottom => Rect::new(
                panel.bounds.x,
                panel.bounds.y - self.resize_handle_size,
                panel.bounds.width,
                self.resize_handle_size * 2.0,
            ),
            _ => Rect::default(),
        }
    }
}

/// Render a collapsed panel as a slim strip (header chrome only, no content).
/// Returns true if the expand chevron was clicked.
fn render_collapsed_strip(ui: &mut UIContext, panel: &DockPanel, theme: &EditorTheme) -> bool {
    ui.panel_styled(panel.bounds, theme.surface_2, theme.border_subtle, 1.0);
    draw_panel_chrome(ui, &panel.bounds, theme);

    // Horizontal strips (Top/Bottom) keep their title; vertical strips are
    // too narrow for horizontal text, so they stay chevron-only.
    if matches!(panel.position, DockPosition::Top | DockPosition::Bottom) {
        ui.label_in_bounds_styled(
            &panel.title,
            panel.bounds,
            ui::TextAlign::Left,
            theme.accent_cyan,
            theme.fonts.body,
            8.0,
        );
    }

    render_chevron_button(ui, panel, theme)
}

/// Draw the collapse/expand chevron and return true when clicked.
fn render_chevron_button(ui: &mut UIContext, panel: &DockPanel, theme: &EditorTheme) -> bool {
    let bounds = chevron_bounds(panel);
    let id = format!("panel_collapse_{}", panel.id.0);
    let result = ui.interact(id.as_str(), bounds, true);
    if result.state == WidgetState::Hovered || result.dragging {
        ui.rect_rounded(bounds, theme.menu_open_highlight, 3.0);
    }
    draw_chevron(ui, bounds, panel.collapsed, theme);
    result.clicked
}

/// Draw the resize grabber: a 2px accent line along the resizable edge with
/// three center dots.
fn draw_resize_grabber(ui: &mut UIContext, panel: &DockPanel, theme: &EditorTheme) {
    let bounds = panel.bounds;
    let color = theme.accent_cyan;
    let (start, end, center, along_y) = match panel.position {
        DockPosition::Left => (
            Vec2::new(bounds.x + bounds.width, bounds.y),
            Vec2::new(bounds.x + bounds.width, bounds.y + bounds.height),
            Vec2::new(bounds.x + bounds.width, bounds.y + bounds.height / 2.0),
            true,
        ),
        DockPosition::Right => (
            Vec2::new(bounds.x, bounds.y),
            Vec2::new(bounds.x, bounds.y + bounds.height),
            Vec2::new(bounds.x, bounds.y + bounds.height / 2.0),
            true,
        ),
        DockPosition::Top => (
            Vec2::new(bounds.x, bounds.y + bounds.height),
            Vec2::new(bounds.x + bounds.width, bounds.y + bounds.height),
            Vec2::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height),
            false,
        ),
        DockPosition::Bottom => (
            Vec2::new(bounds.x, bounds.y),
            Vec2::new(bounds.x + bounds.width, bounds.y),
            Vec2::new(bounds.x + bounds.width / 2.0, bounds.y),
            false,
        ),
        _ => return,
    };

    ui.line(start, end, color, 2.0);
    for offset in [-8.0, 0.0, 8.0] {
        let dot = if along_y {
            Vec2::new(center.x, center.y + offset)
        } else {
            Vec2::new(center.x + offset, center.y)
        };
        ui.circle(dot, 1.5, color);
    }
}

/// Panel-header separator: a thin subtle separator along the header's bottom edge.
fn draw_panel_chrome(ui: &mut UIContext, header_bounds: &Rect, theme: &EditorTheme) {
    ui.rect(
        Rect::new(
            header_bounds.x,
            header_bounds.y + header_bounds.height - 1.0,
            header_bounds.width,
            1.0,
        ),
        theme.border_subtle,
    );
}

/// Draw a panel's frame: background, header, title and header flair. Shared
/// by the docked panels and the narrow-mode overlay, which differs only in
/// the band it is drawn on.
fn render_panel_frame(ui: &mut UIContext, panel: &DockPanel, theme: &EditorTheme) {
    // The scene view shows game content directly; every other panel gets the
    // opaque background so game sprites never bleed through.
    if panel.id != PanelId::SCENE_VIEW {
        ui.panel_styled(panel.bounds, theme.surface_1, theme.border_subtle, 1.0);
    }

    let header_bounds = Rect::new(
        panel.bounds.x,
        panel.bounds.y,
        panel.bounds.width,
        HEADER_HEIGHT,
    );
    ui.rect_rounded(header_bounds, theme.surface_2, 0.0);
    ui.label_in_bounds_styled(
        &panel.title,
        header_bounds,
        ui::TextAlign::Left,
        theme.text_primary,
        theme.fonts.body,
        8.0,
    );
    draw_panel_chrome(ui, &header_bounds, theme);
}

/// Draw a closed side panel's narrow-mode tab — the header tab at its edge
/// that opens it — and return true when it is clicked. On the chrome band, so
/// it stays visible and grabbable over the viewport it sits on.
fn render_narrow_tab(ui: &mut UIContext, panel: &DockPanel, theme: &EditorTheme) -> bool {
    let bounds = panel.bounds;
    ui.begin_overlay_in(ui::UiLayer::PanelChrome, bounds);
    ui.panel_styled(bounds, theme.surface_2, theme.border_subtle, 1.0);

    let id = format!("panel_narrow_tab_{}", panel.id.0);
    let result = ui.interact(id.as_str(), bounds, true);
    if result.state == WidgetState::Hovered || result.dragging {
        ui.rect_rounded(bounds, theme.menu_open_highlight, 3.0);
    }

    // The tab is a vertical strip: too narrow for the title, so it carries
    // the panel's initial and a chevron pointing into the viewport.
    if let Some(initial) = panel.title.chars().next() {
        ui.label_centered_styled(
            &initial.to_string(),
            Vec2::new(bounds.center().x, bounds.y + HEADER_HEIGHT),
            theme.text_secondary,
            theme.fonts.body,
        );
    }
    draw_tab_chevron(ui, bounds, panel.position, theme);
    ui.end_overlay();

    result.clicked
}

/// The arrow on a narrow-mode tab, pointing the way the panel will open.
fn draw_tab_chevron(ui: &mut UIContext, bounds: Rect, position: DockPosition, theme: &EditorTheme) {
    let center = Vec2::new(bounds.center().x, bounds.bottom() - 12.0);
    let direction = if matches!(position, DockPosition::Right) { -1.0 } else { 1.0 };
    let color = theme.text_secondary;
    ui.line(
        Vec2::new(center.x - 2.0 * direction, center.y - 4.0),
        Vec2::new(center.x + 2.0 * direction, center.y),
        color,
        2.0,
    );
    ui.line(
        Vec2::new(center.x + 2.0 * direction, center.y),
        Vec2::new(center.x - 2.0 * direction, center.y + 4.0),
        color,
        2.0,
    );
}
