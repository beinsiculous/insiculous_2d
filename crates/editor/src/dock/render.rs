//! Group headers, tab overflow, narrow tabs and group resize handles.

use glam::Vec2;
use ui::{Rect, UIContext, WidgetState};
use crate::{theme::EditorTheme, menu::{Menu, MenuBar, MenuItem}};
use super::{DockArea, DockPosition, PanelId, TabGroup, DRAG_DWELL_FRAMES, HEADER_HEIGHT};

const CHEVRON_SIZE: f32 = 16.0;
/// The narrowest a tab shrinks to under pressure; past it the strip overflows into `»`.
pub(super) const MIN_TAB_WIDTH: f32 = 48.0;
/// The widest a tab grows: sized by its title, never to fill the strip, or the content
/// under a two-tab strip reads as belonging to whichever tab it happens to sit below.
pub(super) const MAX_TAB_WIDTH: f32 = 4.0 * MIN_TAB_WIDTH;
const TAB_LABEL_INSET: f32 = 6.0;

/// Shrink every tab in proportion to its slack above the minimum until the row fits;
/// a row whose tabs are all at the minimum is left for the overflow button.
fn fit_tab_widths(mut widths: Vec<f32>, available: f32) -> Vec<f32> {
    let excess = widths.iter().sum::<f32>() - available;
    let slack: f32 = widths.iter().map(|width| width - MIN_TAB_WIDTH).sum();
    if excess > 0.0 && slack > 0.0 {
        let share = excess / slack;
        for width in &mut widths {
            *width = if share >= 1.0 { MIN_TAB_WIDTH } else { *width - (*width - MIN_TAB_WIDTH) * share };
        }
    }
    widths
}

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

/// The one collapse button, reserved independently of the group's tabs.
pub(super) fn chevron_bounds(group: &TabGroup) -> Rect {
    let padding = (HEADER_HEIGHT - CHEVRON_SIZE) / 2.0;
    let vertical = group.bounds.width <= HEADER_HEIGHT && DockArea::is_side(group.position);
    let x = if vertical { group.bounds.x + padding } else { group.bounds.right() - CHEVRON_SIZE - padding };
    Rect::new(x, group.bounds.y + padding, CHEVRON_SIZE, CHEVRON_SIZE)
}

impl DockArea {
    /// Draw each visible group's header and return only active, expanded content rectangles.
    pub fn render(&mut self, ui: &mut UIContext, theme: &EditorTheme) -> Vec<(PanelId, Rect)> {
        self.render_tab_overflow(ui, theme);
        // A tab clicked this frame is revealed once every group's chrome has drawn, so no
        // group draws from a layout the click already changed.
        let mut clicked_tab = None;
        for index in 0..self.groups.len() {
            if !self.group_visible(index) { continue; }
            let group = self.groups[index].clone();
            let overlay = self.narrow_overlay == Some(index);
            let narrow_tab = self.narrow && Self::is_side(group.position) && !overlay;
            if narrow_tab {
                self.render_narrow_tab(ui, theme, index);
                continue;
            }
            if overlay { ui.begin_overlay_in(ui::UiLayer::Floating, group.bounds); }
            ui.panel_styled(group.bounds, theme.surface_1, theme.border_subtle, 1.0);
            let vertical_strip = group.collapsed && Self::is_side(group.position) && !overlay;
            if !vertical_strip { clicked_tab = self.render_tabs(ui, theme, index).or(clicked_tab); }
            if vertical_strip && self.focused_tab.is_some_and(|id| group.panels.contains(&id)) {
                ui.rect_border(group.bounds, theme.focus_ring, 2.0, 2.0);
            }
            let chevron = chevron_bounds(&group);
            let result = ui.interact(format!("group_collapse_{}", group.panels[0].0).as_str(), chevron, true);
            if result.state == WidgetState::Hovered { ui.rect_rounded(chevron, theme.menu_open_highlight, 3.0); }
            let collapsed = !overlay && (group.collapsed || self.budget_collapsed.contains(&index));
            draw_chevron(ui, chevron, collapsed, theme);
            ui.tooltip(chevron, if collapsed { "Expand" } else { "Collapse" });
            // The collapsed strip has no tabs to right-press, so the strip itself opens the
            // active tab's menu; registered after the chevron, which keeps first claim.
            if vertical_strip { self.request_tab_menu_on_right_press(ui, index, group.bounds); }
            if overlay { ui.end_overlay(); }
            if result.clicked { self.toggle_panel_collapsed(group.active); }
        }
        if let Some(scene) = self.get_panel(PanelId::SCENE_VIEW) {
            let header = Rect::new(scene.bounds.x, scene.bounds.y, scene.bounds.width, HEADER_HEIGHT);
            ui.rect(header, theme.surface_2);
            ui.label_in_bounds_styled(&scene.title, header, ui::TextAlign::Left, theme.text_primary, theme.fonts.body, 8.0);
        }
        if let Some(id) = clicked_tab { ui.clear_text_focus(); self.reveal(id); }
        self.panels.iter().filter(|panel| panel.content.width > 0.0 && panel.content.height > 0.0)
            .map(|panel| (panel.id, panel.content)).collect()
    }

    /// Compatibility hook: callers own each content clip scope.
    pub fn end_panel_content(&self, _ui: &mut UIContext, _panel_count: usize) {}

    /// Tab rectangles and the optional overflow button, all in UI points.
    pub fn tab_bounds(&self, id: PanelId) -> Option<Rect> {
        let index = self.group_of(id)?;
        if !self.has_tab_strip(index) { return Some(self.groups[index].bounds); }
        let (tabs, overflow) = self.tab_layout(index);
        tabs.into_iter().find_map(|(tab, bounds)| (tab == id).then_some(bounds)).or(overflow)
    }

    /// The tab under `point`: a strip's tab, or a narrow or collapsed side group's one
    /// vertical tab, which stands for its active panel. An open overlay floats above the
    /// strips it covers, so under it only the overlay's own tabs count.
    pub fn tab_at(&self, point: Vec2) -> Option<PanelId> {
        let covered_by_overlay = self.narrow_overlay.filter(|index| self.groups[*index].bounds.contains(point));
        (0..self.groups.len()).filter(|index| self.group_visible(*index)).find_map(|index| {
            if covered_by_overlay.is_some_and(|overlay| overlay != index) { return None; }
            if !self.has_tab_strip(index) {
                let group = &self.groups[index];
                return group.bounds.contains(point).then_some(group.active);
            }
            self.tab_layout(index).0.into_iter().find_map(|(id, bounds)| bounds.contains(point).then_some(id))
        })
    }

    /// The tab a dragged payload has rested on for [`DRAG_DWELL_FRAMES`], answered on that
    /// frame alone; called once a frame while a drag is live, with `end_drag` when it is not.
    pub fn tab_rested_on(&mut self, point: Vec2) -> Option<PanelId> {
        let hovered = self.tab_at(point);
        self.drag_dwell = match (hovered, self.drag_dwell) {
            (Some(id), Some((last, frames))) if last == id => Some((id, frames + 1)),
            (Some(id), _) => Some((id, 1)),
            (None, _) => None,
        };
        self.drag_dwell.filter(|(_, frames)| *frames == DRAG_DWELL_FRAMES).map(|(id, _)| id)
    }

    /// Forget the rest a drag had built up, so the next drag starts from nothing.
    pub fn end_drag(&mut self) { self.drag_dwell = None; }

    /// A tab's width before any pressure: its measured title between the label insets,
    /// never narrower than the minimum nor wider than four of them. A title the strips
    /// have not measured yet (a layout call before the first draw) counts as two minimums.
    fn natural_tab_width(&self, id: PanelId) -> f32 {
        let measured = self.title_widths.iter().find(|(tab, _)| *tab == id)
            .map_or(2.0 * MIN_TAB_WIDTH, |(_, width)| *width + 2.0 * TAB_LABEL_INSET);
        measured.clamp(MIN_TAB_WIDTH, MAX_TAB_WIDTH)
    }

    fn tab_layout(&self, index: usize) -> (Vec<(PanelId, Rect)>, Option<Rect>) {
        let group = &self.groups[index];
        let visible = self.visible_tabs(index);
        if visible.is_empty() { return (Vec::new(), None); }
        let width = (group.bounds.width - HEADER_HEIGHT).max(0.0);
        let overflow = visible.len() as f32 * MIN_TAB_WIDTH > width;
        let count = if overflow { ((width - HEADER_HEIGHT).max(0.0) / MIN_TAB_WIDTH).floor() as usize }
            else { visible.len() }.min(visible.len());
        let cursor = self.focused_tab.filter(|id| visible.contains(id)).unwrap_or(group.active);
        let cursor_index = visible.iter().position(|id| *id == cursor).unwrap_or(0);
        let start = if cursor_index >= count { cursor_index + 1 - count } else { 0 };
        let shown: Vec<PanelId> = visible.into_iter().skip(start).take(count).collect();
        let room = width - if overflow { HEADER_HEIGHT } else { 0.0 };
        let widths = fit_tab_widths(shown.iter().map(|id| self.natural_tab_width(*id)).collect(), room);
        let mut x = group.bounds.x;
        let tabs = shown.into_iter().zip(widths).map(|(id, tab_width)| {
            let bounds = Rect::new(x, group.bounds.y, tab_width, HEADER_HEIGHT);
            x += tab_width;
            (id, bounds)
        }).collect();
        let button = overflow.then(|| Rect::new(group.bounds.x + width - HEADER_HEIGHT, group.bounds.y, HEADER_HEIGHT, HEADER_HEIGHT));
        (tabs, button)
    }

    fn measure_titles(&mut self, ui: &UIContext, theme: &EditorTheme, index: usize) {
        for id in self.visible_tabs(index) {
            let Some(measured) = self.get_panel(id).map(|panel| ui.measure_text_styled(&panel.title, theme.fonts.body).x) else { continue; };
            match self.title_widths.iter_mut().find(|(tab, _)| *tab == id) {
                Some(entry) => entry.1 = measured,
                None => self.title_widths.push((id, measured)),
            }
        }
    }

    /// A right press on `bounds` asks the host for the active tab's action menu.
    fn request_tab_menu_on_right_press(&mut self, ui: &mut UIContext, index: usize, bounds: Rect) {
        let id = self.groups[index].active;
        let result = ui.interact(format!("group_tab_menu_{}", self.groups[index].panels[0].0).as_str(), bounds, true);
        if ui.mouse_right_just_pressed() && matches!(result.state, WidgetState::Hovered | WidgetState::Active) {
            self.focused_tab = Some(id);
            self.context_request = Some((id, Vec2::new(bounds.x, bounds.y + HEADER_HEIGHT)));
        }
    }

    /// Draw a group's strip and answer the tab clicked this frame, which the caller reveals
    /// once every group has drawn.
    fn render_tabs(&mut self, ui: &mut UIContext, theme: &EditorTheme, index: usize) -> Option<PanelId> {
        self.measure_titles(ui, theme, index);
        let (tabs, overflow) = self.tab_layout(index);
        let mut clicked = None;
        for (id, bounds) in tabs {
            let Some(panel) = self.get_panel(id) else { continue; };
            let active = self.groups[index].active == id;
            let result = ui.interact(format!("dock_tab_{}", id.0).as_str(), bounds, true);
            let background = if result.state == WidgetState::Hovered { theme.menu_open_highlight }
                else if active { theme.surface_2 } else { theme.surface_1 };
            ui.rect(bounds, background);
            // The active tab joins the content; inactive tabs retain the bottom separator.
            if !active { ui.rect(Rect::new(bounds.x, bounds.bottom() - 2.0, bounds.width, 2.0), theme.border_subtle); }
            let label = crate::ellipsize(&panel.title, (bounds.width - 12.0).max(0.0), |text| ui.measure_text_styled(text, theme.fonts.body).x);
            ui.push_clip_rect(bounds);
            ui.label_in_bounds_styled(&label, bounds, ui::TextAlign::Left, theme.text_primary, theme.fonts.body, 6.0);
            ui.pop_clip_rect();
            ui.tooltip(bounds, if panel.hint.is_empty() { &panel.title } else { panel.hint });
            if self.focused_tab == Some(id) { ui.rect_border(bounds, theme.focus_ring, 2.0, 2.0); }
            if ui.mouse_right_just_pressed() && matches!(result.state, WidgetState::Hovered | WidgetState::Active) {
                self.focused_tab = Some(id);
                self.context_request = Some((id, Vec2::new(bounds.x, bounds.bottom())));
            }
            // A click reveals, not merely selects: on a collapsed or budget-collapsed group a
            // bare activation moved the highlight behind a strip and opened nothing.
            if result.clicked { clicked = Some(id); }
        }
        if let Some(button) = overflow {
            if ui.button_styled(format!("dock_overflow_{index}").as_str(), "»", button, true) {
                self.overflow_group = (self.overflow_group != Some(index)).then_some(index);
            }
            ui.tooltip(button, "More tabs");
        }
        clicked
    }

    fn render_tab_overflow(&mut self, ui: &mut UIContext, theme: &EditorTheme) {
        let Some(index) = self.overflow_group.filter(|index| *index < self.groups.len()) else { return; };
        let (tabs, button) = self.tab_layout(index);
        let Some(button) = button else { self.overflow_group = None; return; };
        let hidden: Vec<PanelId> = self.visible_tabs(index).into_iter().filter(|id| !tabs.iter().any(|(tab, _)| tab == id)).collect();
        let menu = Menu::new("dock_overflow").with_items(hidden.iter().filter_map(|id| self.get_panel(*id))
            .map(|panel| MenuItem::action(&panel.title)).collect());
        let bounds = crate::overflow_menu::overflow_menu_bounds(button,
            menu.items.len() as f32 * crate::layout::ROW_HEIGHT + 8.0, ui.window_size(), 24.0);
        if (ui.mouse_just_pressed() || ui.mouse_right_just_pressed()) && !bounds.contains(ui.mouse_pos()) && !button.contains(ui.mouse_pos()) {
            self.overflow_group = None;
            return;
        }
        ui.begin_overlay_in(ui::UiLayer::Modal, bounds);
        let picked = MenuBar::render_dropdown_static(ui, &menu, bounds, theme);
        ui.end_overlay();
        if let Some(label) = picked {
            if let Some(id) = hidden.into_iter().find(|id| self.get_panel(*id).is_some_and(|panel| panel.title == label)) {
                self.reveal(id);
            }
            self.overflow_group = None;
        }
    }

    fn render_narrow_tab(&mut self, ui: &mut UIContext, theme: &EditorTheme, index: usize) {
        let group = &self.groups[index];
        let bounds = group.bounds;
        let id = group.active;
        ui.begin_overlay_in(ui::UiLayer::PanelChrome, bounds);
        ui.panel_styled(bounds, theme.surface_2, theme.border_subtle, 1.0);
        let result = ui.interact(format!("group_narrow_{}", group.panels[0].0).as_str(), bounds, true);
        if let Some(panel) = self.get_panel(id) {
            if let Some(initial) = panel.title.chars().next() {
                ui.label_centered_styled(&initial.to_string(), Vec2::new(bounds.center().x, bounds.y + HEADER_HEIGHT), theme.text_secondary, theme.fonts.body);
            }
            ui.tooltip(bounds, &panel.title);
        }
        draw_chevron(ui, Rect::new(bounds.x + 4.0, bounds.bottom() - 20.0, 16.0, 16.0), true, theme);
        if self.focused_tab.is_some_and(|tab| group.panels.contains(&tab)) { ui.rect_border(bounds, theme.focus_ring, 2.0, 2.0); }
        if ui.mouse_right_just_pressed() && matches!(result.state, WidgetState::Hovered | WidgetState::Active) {
            self.focused_tab = Some(id);
            self.context_request = Some((id, Vec2::new(bounds.x, bounds.y + HEADER_HEIGHT)));
        }
        ui.end_overlay();
        if result.clicked { self.open_narrow_overlay(id); }
    }

    /// Resize a group's edge, clamped to its largest member's minimum.
    pub fn handle_resize(&mut self, ui: &mut UIContext, theme: &EditorTheme) {
        for index in 0..self.groups.len() {
            if !self.group_visible(index) || self.groups[index].collapsed || self.overlay_eligible(index) { continue; }
            let group = &self.groups[index];
            let bounds = self.resize_handle_bounds(group);
            let result = ui.interact(format!("resize_group_{}", group.panels[0].0).as_str(), bounds, true);
            if result.state == WidgetState::Hovered || result.dragging {
                ui.rect(bounds, theme.accent_cyan);
                ui.request_cursor(if Self::is_side(group.position) { ui::CursorIcon::ColResize } else { ui::CursorIcon::RowResize });
            }
            if result.dragging {
                self.groups[index].size = resized_size(group.position, ui.mouse_pos(), group.bounds, group.min_size, self.bounds);
                self.layout();
            }
        }
    }

    pub(super) fn resize_handle_bounds(&self, group: &TabGroup) -> Rect {
        let bounds = group.bounds;
        let handle = self.resize_handle_size;
        match group.position {
            DockPosition::Left => Rect::new(bounds.right() - handle, bounds.y, handle * 2.0, bounds.height),
            DockPosition::Right => Rect::new(bounds.x - handle, bounds.y, handle * 2.0, bounds.height),
            DockPosition::Top => Rect::new(bounds.x, bounds.bottom() - handle, bounds.width, handle * 2.0),
            DockPosition::Bottom => Rect::new(bounds.x, bounds.y - handle, bounds.width, handle * 2.0),
            DockPosition::Center => Rect::default(),
        }
    }
}
