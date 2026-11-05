//! Effective allocations preserve the user's sizes while fitting the centre's budget.

use ui::Rect;
use crate::layout::PADDING;
use super::{DockArea, DockPosition, PanelId, HEADER_HEIGHT, MIN_CENTER_HEIGHT, MIN_CENTER_WIDTH, NARROW_TAB_LENGTH};

impl DockArea {
    pub(super) fn is_side(position: DockPosition) -> bool {
        matches!(position, DockPosition::Left | DockPosition::Right)
    }

    /// Allocate groups in registration order and give the remaining rectangle to the scene.
    /// Narrow side groups and height-constrained groups stay reachable through overlays.
    pub fn layout(&mut self) {
        self.settle_active_tabs();
        let side_width: f32 = self.groups.iter().enumerate()
            .filter(|(index, group)| self.group_visible(*index) && Self::is_side(group.position))
            .map(|(_, group)| group.effective_size()).sum();
        self.narrow = self.bounds.width - side_width < MIN_CENTER_WIDTH;
        let effective_sizes = self.vertical_sizes();
        if self.narrow_overlay.is_some_and(|index| index >= self.groups.len()
            || !self.group_visible(index) || !self.overlay_eligible(index)) {
            self.narrow_overlay = None;
        }
        // An open overflow menu belongs to a strip; a group that lost its strip to a
        // resize or a collapse would otherwise keep drawing the menu beside a tab it no
        // longer has.
        if self.overflow_group.is_some_and(|index| index >= self.groups.len()
            || !self.group_visible(index) || !self.has_tab_strip(index)) {
            self.overflow_group = None;
        }
        let mut remaining = self.bounds;
        for (index, size) in effective_sizes.into_iter().enumerate() {
            self.groups[index].bounds = Rect::default();
            if !self.group_visible(index) || (self.narrow && Self::is_side(self.groups[index].position)) { continue; }
            self.groups[index].bounds = allocate(&mut remaining, self.groups[index].position, size);
        }
        let mut slots = [0, 0];
        for index in 0..self.groups.len() {
            if !self.group_visible(index) { continue; }
            let group = &self.groups[index];
            if self.narrow && Self::is_side(group.position) {
                let slot = usize::from(group.position == DockPosition::Right);
                let x = if slot == 1 { remaining.right() - HEADER_HEIGHT } else { remaining.x };
                self.groups[index].bounds = Rect::new(x,
                    Self::narrow_top(remaining) + PADDING + slots[slot] as f32 * (NARROW_TAB_LENGTH + PADDING),
                    HEADER_HEIGHT, NARROW_TAB_LENGTH);
                slots[slot] += 1;
            }
            if self.narrow_overlay == Some(index) {
                self.groups[index].bounds = self.overlay_bounds(index, remaining);
            }
        }
        for panel in &mut self.panels {
            panel.bounds = Rect::default();
            panel.content = Rect::default();
        }
        if let Some(scene) = self.get_panel_mut(PanelId::SCENE_VIEW) {
            scene.bounds = remaining;
            scene.content = scene.expanded_content_bounds();
        }
        for index in 0..self.groups.len() {
            if !self.group_visible(index) { continue; }
            let group = &self.groups[index];
            let expanded = self.narrow_overlay == Some(index)
                || (!group.collapsed && !self.overlay_eligible(index));
            let (active, bounds) = (group.active, group.bounds);
            if let Some(panel) = self.get_panel_mut(active) {
                panel.bounds = bounds;
                if expanded { panel.content = panel.expanded_content_bounds(); }
            }
        }
        if self.focused_tab.is_some_and(|id| !self.get_panel(id).is_some_and(|panel| panel.visible)) {
            self.focused_tab = None;
        }
    }

    fn vertical_sizes(&mut self) -> Vec<f32> {
        let mut sizes: Vec<f32> = self.groups.iter().map(|group| group.effective_size()).collect();
        // Each edge's groups are allocated outside in, so on either edge the last allocated
        // touches the centre; ranked by that distance, not by registration order, or a
        // top group beside the centre would collapse before a bottom group at the edge.
        let mut ranked: Vec<(usize, usize)> = Vec::new();
        for edge in [DockPosition::Top, DockPosition::Bottom] {
            let on_edge: Vec<usize> = (0..self.groups.len())
                .filter(|index| self.group_visible(*index) && self.groups[*index].position == edge).collect();
            ranked.extend(on_edge.iter().enumerate().map(|(place, index)| (on_edge.len() - 1 - place, *index)));
        }
        ranked.sort_by_key(|(distance, _)| *distance);
        let mut excess = ranked.iter().map(|(_, index)| sizes[*index]).sum::<f32>()
            + MIN_CENTER_HEIGHT - self.bounds.height;
        // The groups touching the centre yield their surplus first.
        for &(_, index) in &ranked {
            if excess <= 0.0 { break; }
            let minimum = if self.groups[index].collapsed { HEADER_HEIGHT } else { self.groups[index].min_size };
            let shrink = excess.min((sizes[index] - minimum).max(0.0));
            sizes[index] -= shrink;
            excess -= shrink;
        }
        // Then the furthest from the centre collapse first.
        self.budget_collapsed.clear();
        for &(_, index) in ranked.iter().rev() {
            if excess <= 0.0 { break; }
            if !self.groups[index].collapsed {
                excess -= sizes[index] - HEADER_HEIGHT;
                sizes[index] = HEADER_HEIGHT;
                self.budget_collapsed.push(index);
            }
        }
        sizes
    }

    fn overlay_bounds(&self, index: usize, centre: Rect) -> Rect {
        let group = &self.groups[index];
        let top = Self::narrow_top(centre);
        let room = (self.bounds.bottom() - top).max(0.0);
        if Self::is_side(group.position) {
            let width = group.size.min(centre.width);
            let x = if group.position == DockPosition::Right { centre.right() - width } else { centre.x };
            Rect::new(x, top, width, room)
        } else {
            let height = group.size.min(room);
            let y = if group.position == DockPosition::Bottom { self.bounds.bottom() - height } else { top };
            Rect::new(centre.x, y, centre.width, height)
        }
    }

    fn narrow_top(centre: Rect) -> f32 {
        centre.y + HEADER_HEIGHT + crate::layout::TOOLBAR_STRIP_HEIGHT
    }
}

fn allocate(remaining: &mut Rect, edge: DockPosition, size: f32) -> Rect {
    let size = size.max(0.0);
    match edge {
        DockPosition::Left | DockPosition::Right => {
            let width = size.min(remaining.width);
            let x = if edge == DockPosition::Right { remaining.right() - width } else { remaining.x };
            let bounds = Rect::new(x, remaining.y, width, remaining.height);
            if edge == DockPosition::Left { remaining.x += width; }
            remaining.width -= width;
            bounds
        }
        DockPosition::Top | DockPosition::Bottom => {
            let height = size.min(remaining.height);
            let y = if edge == DockPosition::Bottom { remaining.bottom() - height } else { remaining.y };
            let bounds = Rect::new(remaining.x, y, remaining.width, height);
            if edge == DockPosition::Top { remaining.y += height; }
            remaining.height -= height;
            bounds
        }
        DockPosition::Center => Rect::default(),
    }
}
