//! A tab dragged to another place in the dock: the press and its threshold, the target under
//! the pointer and the chip and target drawn while the drag is live. The target is resolved
//! each frame from that frame's layout and never remembered, so nothing the drag resolves
//! can be stale. The drop is one `DockAction`, the Window menu's own, which the host runs
//! before the dock draws.

use glam::Vec2;
use ui::{InteractionResult, Rect, UIContext, UiLayer, WidgetState};
use crate::{drag_drop::DRAG_THRESHOLD, theme::EditorTheme};
use super::{DockAction, DockArea, DockPosition, PanelId, HEADER_HEIGHT};

/// How far in from each side of the dock a drop makes a new group on that edge, in points.
pub(super) const EDGE_ZONE_DEPTH: f32 = 48.0;
/// The chip's height, and its offset from the pointer so the target under it stays visible.
const CHIP_HEIGHT: f32 = 24.0;
const CHIP_OFFSET: Vec2 = Vec2::new(12.0, 16.0);
/// The insertion bar's stroke and the width of the caps that make it read as a bar, not a line.
const BAR_WIDTH: f32 = 3.0;
const BAR_CAP_WIDTH: f32 = 10.0;

/// A left press on a tab, from its press frame until the button is up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TabPress {
    panel: PanelId,
    press_point: Vec2,
    phase: PressPhase,
}

/// Where a press stands. Every phase past `Held` is the latch: its release is never the tab's
/// click, so a drag that moved nothing reveals nothing and changes no saved `active`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PressPhase {
    /// Under the threshold: the release is the tab's click.
    Held,
    /// Past the threshold: a drag of the panel.
    Dragging,
    /// Dropped or cancelled with the button still down: the release does nothing.
    Ended,
}

impl TabPress {
    /// The panel this press drags, while the drag is live.
    pub(super) fn dragging(self) -> Option<PanelId> {
        (self.phase == PressPhase::Dragging).then_some(self.panel)
    }

    /// Whether the press is, or may still become, a drag: not yet dropped or cancelled.
    pub(super) fn pending(self) -> bool {
        self.phase != PressPhase::Ended
    }

    /// The press after Escape: latched whether or not it had become a drag, so its release
    /// neither drops nor clicks.
    pub(super) fn ended(self) -> TabPress {
        TabPress { phase: PressPhase::Ended, ..self }
    }

    /// The press after a cancel: a live drag ends latched, and a press still under the
    /// threshold is forgotten, so its release stays the tab's click and it never becomes a drag.
    pub(super) fn cancelled(self) -> Option<TabPress> {
        (self.phase != PressPhase::Held).then_some(TabPress { phase: PressPhase::Ended, ..self })
    }
}

/// Where a dragged tab would land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DropTarget {
    /// After this drawn tab, in its group.
    AfterTab(PanelId),
    /// Over a strip's empty run: after its last drawn tab.
    StripEnd(PanelId),
    /// A collapsed side strip, which draws no tabs: after its active panel.
    CollapsedStrip(PanelId),
    /// A new group at the end of this edge.
    Edge(DockPosition),
}

impl DropTarget {
    /// The panel the dropped one would follow; none for an edge.
    fn follows(self) -> Option<PanelId> {
        match self {
            Self::AfterTab(tab) | Self::StripEnd(tab) | Self::CollapsedStrip(tab) => Some(tab),
            Self::Edge(_) => None,
        }
    }

    /// The one action a drop here asks of the host.
    pub(super) fn action(self, dragged: PanelId) -> DockAction {
        match self {
            Self::AfterTab(tab) | Self::StripEnd(tab) | Self::CollapsedStrip(tab) => DockAction::GroupWith(dragged, tab),
            Self::Edge(edge) => DockAction::MoveTo(dragged, edge),
        }
    }
}

/// One visible group as this frame lays it out, as the target resolution reads it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct GroupView {
    /// The group's allocation, its strip along the top.
    pub(super) bounds: Rect,
    /// The tabs its strip draws, in order; empty for a group drawn without a strip.
    pub(super) tabs: Vec<(PanelId, Rect)>,
    /// For a collapsed side group, drawn as a vertical strip, the active panel it stands for.
    pub(super) collapsed_strip: Option<PanelId>,
    /// Whether the group is open as the overlay floating above the others.
    pub(super) overlay: bool,
}

/// The dock as one frame lays it out for a drag.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DropFrame {
    pub(super) dock_bounds: Rect,
    pub(super) groups: Vec<GroupView>,
    /// The edges the dragged panel may move to: a zone that would refuse is no target.
    pub(super) open_edges: Vec<DockPosition>,
}

impl DropFrame {
    fn tab_bounds(&self, id: PanelId) -> Option<Rect> {
        self.groups.iter().flat_map(|group| group.tabs.iter())
            .find_map(|(tab, bounds)| (*tab == id).then_some(*bounds))
    }
}

/// The target under `pointer` for a drag of `dragged`, in precedence: a drawn tab (any group's,
/// whatever its collapse) or a strip's empty run, then a collapsed side strip, then an edge zone
/// the panel may move to. A tab or a strip wins over the zone it sits in, and the dragged
/// panel's own tab is no target. Anywhere else there is none.
pub(super) fn resolve_target(pointer: Vec2, dragged: PanelId, frame: &DropFrame) -> Option<DropTarget> {
    // An open overlay floats above whatever it covers: under it only its own strip counts.
    if let Some(overlay) = frame.groups.iter().find(|group| group.overlay && group.bounds.contains(pointer)) {
        return group_claim(pointer, dragged, overlay).flatten();
    }
    if let Some(claim) = frame.groups.iter().find_map(|group| group_claim(pointer, dragged, group)) {
        return claim;
    }
    edge_zone(pointer, frame.dock_bounds).into_iter().flatten()
        .find(|edge| frame.open_edges.contains(edge)).map(DropTarget::Edge)
}

/// A group's claim on the pointer: `None` when the group does not hold it, `Some(None)` when it
/// does and offers nothing (a drop after the dragged panel itself), otherwise its target.
fn group_claim(pointer: Vec2, dragged: PanelId, group: &GroupView) -> Option<Option<DropTarget>> {
    let target = if let Some(active) = group.collapsed_strip {
        if !group.bounds.contains(pointer) { return None; }
        DropTarget::CollapsedStrip(active)
    } else {
        let &(last, _) = group.tabs.last()?;
        let strip = Rect::new(group.bounds.x, group.bounds.y, group.bounds.width, HEADER_HEIGHT);
        if !strip.contains(pointer) { return None; }
        group.tabs.iter().find(|(_, bounds)| bounds.contains(pointer))
            .map_or(DropTarget::StripEnd(last), |(tab, _)| DropTarget::AfterTab(*tab))
    };
    Some((target.follows() != Some(dragged)).then_some(target))
}

/// The edges whose zone holds `pointer`, the nearer side edge first, so a corner goes to the
/// side; the top or bottom zone takes the corner only where the side refuses.
fn edge_zone(pointer: Vec2, dock: Rect) -> [Option<DockPosition>; 2] {
    if !dock.contains(pointer) { return [None, None]; }
    let (from_left, from_right) = (pointer.x - dock.x, dock.right() - pointer.x);
    let (from_top, from_bottom) = (pointer.y - dock.y, dock.bottom() - pointer.y);
    let side = (from_left.min(from_right) < EDGE_ZONE_DEPTH)
        .then_some(if from_left <= from_right { DockPosition::Left } else { DockPosition::Right });
    let end = (from_top.min(from_bottom) < EDGE_ZONE_DEPTH)
        .then_some(if from_top <= from_bottom { DockPosition::Top } else { DockPosition::Bottom });
    [side, end]
}

/// The band a drop on `edge` shows: a new group's header along that edge.
pub(super) fn edge_band(dock: Rect, edge: DockPosition) -> Rect {
    match edge {
        DockPosition::Left => Rect::new(dock.x, dock.y, HEADER_HEIGHT, dock.height),
        DockPosition::Right => Rect::new(dock.right() - HEADER_HEIGHT, dock.y, HEADER_HEIGHT, dock.height),
        DockPosition::Top => Rect::new(dock.x, dock.y, dock.width, HEADER_HEIGHT),
        DockPosition::Bottom => Rect::new(dock.x, dock.bottom() - HEADER_HEIGHT, dock.width, HEADER_HEIGHT),
        DockPosition::Center => Rect::default(),
    }
}

/// The name an edge band carries; the Window menu's Move to items (`menu/window.rs`) use the
/// same four words, which `drag_tests.rs` holds together.
pub(super) fn edge_name(edge: DockPosition) -> &'static str {
    match edge {
        DockPosition::Left => "Left",
        DockPosition::Right => "Right",
        DockPosition::Top => "Top",
        DockPosition::Bottom => "Bottom",
        DockPosition::Center => "Center",
    }
}

impl DockArea {
    /// Take the drop of a drag released this frame, resolved against this frame's layout, as
    /// the action the Window menu would run; the host runs it before the dock draws, so the
    /// release frame lays out and draws the new groups. A release over no target, and a dragged
    /// panel hidden or gone by the release, take nothing. Either way the release is no click.
    pub fn take_tab_drop(&mut self, ui: &UIContext) -> Option<DockAction> {
        let press = self.tab_press.as_mut().filter(|press| press.phase == PressPhase::Dragging)?;
        if !ui.mouse_just_released() { return None; }
        press.phase = PressPhase::Ended;
        let dragged = press.panel;
        let present = self.get_panel(dragged).is_some_and(|panel| panel.visible) && self.group_of(dragged).is_some();
        if !present { return None; }
        resolve_target(ui.mouse_pos(), dragged, &self.drop_frame(dragged)).map(|target| target.action(dragged))
    }

    /// Remember a left press that landed on `id`'s tab this frame. While the narrow overlay is
    /// open no drag starts, so a press there stays the click it always was.
    pub(super) fn note_tab_press(&mut self, ui: &UIContext, id: PanelId, result: InteractionResult) {
        if ui.mouse_just_pressed() && result.state == WidgetState::Active && self.narrow_overlay.is_none() {
            self.tab_press = Some(TabPress { panel: id, press_point: ui.mouse_pos(), phase: PressPhase::Held });
        }
    }

    /// Start the drag once a held press has moved the threshold from its press point. Crossing
    /// it clears text focus as a tab's click does: a focused field owns the keyboard ahead of
    /// the dock's keys, and Escape must reach the drag.
    pub(super) fn follow_tab_press(&mut self, ui: &mut UIContext) {
        let Some(press) = self.tab_press.as_mut() else { return; };
        if press.phase == PressPhase::Held && ui.mouse_down()
            && ui.mouse_pos().distance(press.press_point) >= DRAG_THRESHOLD {
            press.phase = PressPhase::Dragging;
            ui.clear_text_focus();
        }
    }

    /// Whether `id`'s press is latched, so its release is not the tab's click.
    pub(super) fn tab_press_latched(&self, id: PanelId) -> bool {
        self.tab_press.is_some_and(|press| press.panel == id && press.phase != PressPhase::Held)
    }

    /// Draw a live drag's target and chip; once the button is up, forget the press.
    pub(super) fn finish_tab_press_frame(&mut self, ui: &mut UIContext, theme: &EditorTheme) {
        if ui.mouse_down() { self.draw_tab_drag(ui, theme); } else { self.tab_press = None; }
    }

    fn draw_tab_drag(&self, ui: &mut UIContext, theme: &EditorTheme) {
        let Some(dragged) = self.dragged_tab() else { return; };
        let Some(panel) = self.get_panel(dragged).filter(|panel| panel.visible) else { return; };
        if self.group_of(dragged).is_none() { return; }
        let pointer = ui.mouse_pos();
        let frame = self.drop_frame(dragged);
        let target = resolve_target(pointer, dragged, &frame);
        let chip = chip_bounds(ui, theme, &panel.title, pointer);
        ui.begin_overlay_in(UiLayer::DragGhost, chip);
        if let Some(target) = target { draw_target(ui, theme, &frame, target); }
        ui.rect_rounded(chip, theme.surface_3, 4.0);
        ui.rect_border(chip, theme.accent_blue, 1.0, 4.0);
        ui.label_in_bounds_styled(&panel.title, chip, ui::TextAlign::Center, theme.text_primary, theme.fonts.small, 0.0);
        ui.end_overlay();
    }

    /// Whether a move of the panel to `edge` would change nothing: it is the only visible member
    /// of the dock's last group, which sits on that edge, so the move detaches that group and
    /// pushes it back last. Last among every group, not among the edge's: the groups are
    /// allocated in their order, and re-pushing an earlier one hands its corners to the others.
    fn rests_alone_at(&self, id: PanelId, edge: DockPosition) -> bool {
        let Some(index) = self.group_of(id) else { return false; };
        let group = &self.groups[index];
        group.position == edge && index + 1 == self.groups.len()
            && group.panels.iter().all(|member| *member == id || !self.get_panel(*member).is_some_and(|panel| panel.visible))
    }

    /// This frame's layout as the target resolution reads it. A narrow side group's one tab is
    /// neither a strip nor a collapsed strip, so it is no target.
    pub(super) fn drop_frame(&self, dragged: PanelId) -> DropFrame {
        let groups = (0..self.groups.len()).filter(|index| self.group_visible(*index)).filter_map(|index| {
            let group = &self.groups[index];
            let overlay = self.narrow_overlay == Some(index);
            if self.narrow && Self::is_side(group.position) && !overlay { return None; }
            let collapsed_strip = (!self.has_tab_strip(index)).then_some(group.active);
            let tabs = if collapsed_strip.is_some() { Vec::new() } else { self.tab_layout(index).0 };
            Some(GroupView { bounds: group.bounds, tabs, collapsed_strip, overlay })
        }).collect();
        let open_edges = [DockPosition::Left, DockPosition::Right, DockPosition::Top, DockPosition::Bottom]
            .into_iter().filter(|edge| self.can_move_panel_to_edge(dragged, *edge) && !self.rests_alone_at(dragged, *edge)).collect();
        DropFrame { dock_bounds: self.bounds, groups, open_edges }
    }
}

/// The chip beside the pointer, below and to the right of it, turned to the other side of the
/// pointer where the window has no room.
fn chip_bounds(ui: &UIContext, theme: &EditorTheme, title: &str, pointer: Vec2) -> Rect {
    let width = (ui.measure_text_styled(title, theme.fonts.small).x + 16.0).max(48.0);
    let window = ui.window_size();
    let x = if pointer.x + CHIP_OFFSET.x + width <= window.x { pointer.x + CHIP_OFFSET.x }
        else { pointer.x - CHIP_OFFSET.x - width };
    let y = if pointer.y + CHIP_OFFSET.y + CHIP_HEIGHT <= window.y { pointer.y + CHIP_OFFSET.y }
        else { pointer.y - CHIP_OFFSET.y - CHIP_HEIGHT };
    Rect::new(x.max(0.0), y.max(0.0), width, CHIP_HEIGHT)
}

/// The one target under the pointer, told by shape and text, never by colour alone: an
/// insertion bar after the tab the panel would follow, a ring round a collapsed strip, a
/// band along an edge with the edge's name in it.
fn draw_target(ui: &mut UIContext, theme: &EditorTheme, frame: &DropFrame, target: DropTarget) {
    match target {
        DropTarget::AfterTab(tab) | DropTarget::StripEnd(tab) => {
            let Some(bounds) = frame.tab_bounds(tab) else { return; };
            let x = bounds.right();
            ui.rect(Rect::new(x - BAR_WIDTH / 2.0, bounds.y, BAR_WIDTH, bounds.height), theme.focus_ring);
            for y in [bounds.y, bounds.bottom() - 2.0] {
                ui.rect(Rect::new(x - BAR_CAP_WIDTH / 2.0, y, BAR_CAP_WIDTH, 2.0), theme.focus_ring);
            }
        }
        DropTarget::CollapsedStrip(active) => {
            let Some(group) = frame.groups.iter().find(|group| group.collapsed_strip == Some(active)) else { return; };
            ui.rect(group.bounds, theme.selection_fill);
            ui.rect_border(group.bounds, theme.focus_ring, 2.0, 2.0);
        }
        DropTarget::Edge(edge) => {
            let band = edge_band(frame.dock_bounds, edge);
            ui.rect(band, theme.selection_fill);
            ui.rect_border(band, theme.focus_ring, 2.0, 2.0);
            let name = edge_name(edge);
            let width = ui.measure_text_styled(name, theme.fonts.small).x + 12.0;
            // A side band is a header's height wide, narrower than its name: centred on it, the
            // tag ran off the window's edge, so it is kept inside the dock.
            let dock = frame.dock_bounds;
            let x = (band.center().x - width / 2.0).clamp(dock.x, (dock.right() - width).max(dock.x));
            let tag = Rect::new(x, band.center().y - 10.0, width, 20.0);
            ui.rect_rounded(tag, theme.surface_4, 3.0);
            ui.rect_border(tag, theme.popup_border, 1.0, 3.0);
            ui.label_in_bounds_styled(name, tag, ui::TextAlign::Center, theme.text_primary, theme.fonts.small, 0.0);
        }
    }
}
