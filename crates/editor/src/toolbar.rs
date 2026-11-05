//! The scene view's tool group, and the segmented control every group on
//! the toolbar strip is drawn as.
//!
//! A segment is a row of square icon buttons on one rounded background with
//! a one-pixel divider between neighbours. The tools are one segment; the
//! play controls and the view toggles draw theirs with the same buttons, so
//! the strip's three groups read as one toolbar.

use glam::Vec2;
use ui::{Color, Rect, UIContext, WidgetState};

use crate::icons::{draw_icon, Icon};
use crate::theme::EditorTheme;

/// Side of every button on the toolbar strip: square, room for a 16 pt
/// icon and its margin, and over the 24 pt target WCAG 2.5.8 asks for.
pub const TOOL_BUTTON_SIZE: f32 = 28.0;

/// Space between two neighbouring buttons of a segment; the segment's
/// one-pixel divider sits at its centre.
pub const SEGMENT_DIVIDER_GAP: f32 = 4.0;

/// Side of the box a strip icon is drawn in, centred in its button.
pub const ICON_BOX_SIZE: f32 = 20.0;

/// Width of a strip icon's strokes, in points.
pub const ICON_STROKE_WEIGHT: f32 = 1.5;

/// Corner radius of a segment's background.
const SEGMENT_CORNER_RADIUS: f32 = 4.0;

/// How far a divider stops short of the segment's top and bottom.
const DIVIDER_INSET: f32 = 5.0;

/// A strip button's tooltip: its name, its shortcut in parentheses where one
/// is bound, and one sentence on what it does — "Move (W) — Drag the
/// gizmo's arrows…". Every group on the strip builds its tooltips here.
pub fn tooltip_text(name: &str, shortcut: Option<&str>, hint: &str) -> String {
    match shortcut {
        Some(key) => format!("{name} ({key}) — {hint}"),
        None => format!("{name} — {hint}"),
    }
}

/// How a strip button looks while the pointer is elsewhere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ButtonLook {
    /// The segment's own background.
    Plain,
    /// A fill of its own: Play's and Stop's tints.
    Tinted(Color),
    /// On — the live tool, an on toggle, the open overflow menu: filled in
    /// `toolbar_active` and ringed in `accent_blue`, so the state is never
    /// told by colour alone.
    On,
}

/// Draw a segment's background behind `buttons` — one row, left to right —
/// and a divider at the centre of each gap between neighbours. Call before
/// the buttons, which draw over it.
pub(crate) fn draw_segment(ui: &mut UIContext, theme: &EditorTheme, buttons: &[Rect]) {
    if let (Some(first), Some(last)) = (buttons.first(), buttons.last()) {
        let background = Rect::new(first.x, first.y, last.right() - first.x, first.height);
        ui.rect_rounded(background, theme.toolbar_segment, SEGMENT_CORNER_RADIUS);
        for neighbours in buttons.windows(2) {
            let divider_x = (neighbours[0].right() + neighbours[1].x) / 2.0;
            ui.line(
                Vec2::new(divider_x, background.y + DIVIDER_INSET),
                Vec2::new(divider_x, background.bottom() - DIVIDER_INSET),
                theme.separator,
                1.0,
            );
        }
    }
}

/// One icon button of a segment: its fill, its icon in `icon_stroke`, and
/// its tooltip. A hovered or pressed button that is not on wears the
/// chevrons' hover fill. Returns true on the click's release frame.
pub(crate) fn segment_button(
    ui: &mut UIContext,
    theme: &EditorTheme,
    id: &str,
    icon: Icon,
    bounds: Rect,
    look: ButtonLook,
    tooltip: &str,
) -> bool {
    let result = ui.interact(id, bounds, true);
    let hovered = result.state == WidgetState::Hovered || result.dragging;
    let corner_radius = SEGMENT_CORNER_RADIUS - 1.0;
    match look {
        ButtonLook::On => {
            ui.rect_rounded(bounds, theme.toolbar_active, corner_radius);
            ui.rect_border(bounds, theme.accent_blue, 1.0, corner_radius);
        }
        _ if hovered => ui.rect_rounded(bounds, theme.menu_open_highlight, corner_radius),
        ButtonLook::Tinted(fill) => ui.rect_rounded(bounds, fill, corner_radius),
        ButtonLook::Plain => {}
    }
    let icon_bounds = Rect::new(
        bounds.center().x - ICON_BOX_SIZE / 2.0,
        bounds.center().y - ICON_BOX_SIZE / 2.0,
        ICON_BOX_SIZE,
        ICON_BOX_SIZE,
    );
    draw_icon(ui, icon, icon_bounds, theme.icon_stroke, ICON_STROKE_WEIGHT);
    ui.tooltip(bounds, tooltip);
    result.clicked
}

/// Available editor tools for manipulating entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EditorTool {
    /// Select and click entities
    Select,
    /// Move/translate entities — the default, so a fresh editor shows a
    /// gizmo the moment something is selected.
    #[default]
    Move,
    /// Rotate entities
    Rotate,
    /// Scale entities uniformly or non-uniformly
    Scale,
}

impl EditorTool {
    /// Get the display name for this tool.
    pub fn name(&self) -> &'static str {
        match self {
            EditorTool::Select => "Select",
            EditorTool::Move => "Move",
            EditorTool::Rotate => "Rotate",
            EditorTool::Scale => "Scale",
        }
    }

    /// Get the keyboard shortcut hint for this tool.
    pub fn shortcut(&self) -> &'static str {
        match self {
            EditorTool::Select => "Q",
            EditorTool::Move => "W",
            EditorTool::Rotate => "E",
            EditorTool::Scale => "R",
        }
    }

    /// One sentence on what picking the tool lets you do — the tail of its
    /// [`tooltip`](Self::tooltip), after the name and the shortcut.
    pub fn hint(&self) -> &'static str {
        match self {
            EditorTool::Select => "Click an entity to select it, or drag a box across several.",
            EditorTool::Move => "Drag the gizmo's arrows to slide the selection along an axis.",
            EditorTool::Rotate => "Drag the ring around the selection to turn it.",
            EditorTool::Scale => "Drag the gizmo's handles to resize the selection.",
        }
    }

    /// The icon the tool's button carries.
    pub fn icon(&self) -> Icon {
        match self {
            EditorTool::Select => Icon::Select,
            EditorTool::Move => Icon::Move,
            EditorTool::Rotate => Icon::Rotate,
            EditorTool::Scale => Icon::Scale,
        }
    }

    /// The tool button's tooltip: name, shortcut and hint in
    /// [`tooltip_text`]'s shape — the button itself carries only its icon.
    pub fn tooltip(&self) -> String {
        tooltip_text(self.name(), Some(self.shortcut()), self.hint())
    }

    /// Get all available tools.
    pub fn all() -> &'static [EditorTool] {
        &[
            EditorTool::Select,
            EditorTool::Move,
            EditorTool::Rotate,
            EditorTool::Scale,
        ]
    }
}

/// Editor toolbar widget.
///
/// Renders the tool buttons of the scene view's toolbar strip: the tools
/// that fit, and an overflow button opening a menu of the ones that did
/// not. [`crate::toolbar_strip`] decides how many fit; the toolbar draws
/// them and reports what was clicked.
#[derive(Debug, Clone)]
pub struct Toolbar {
    /// Current selected tool
    current_tool: EditorTool,
    /// Whether the overflow menu of shed tools is open.
    overflow_open: bool,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

impl Toolbar {
    /// Create a new toolbar with default settings.
    pub fn new() -> Self {
        Self {
            current_tool: EditorTool::default(),
            overflow_open: false,
        }
    }

    /// Get the currently selected tool.
    pub fn current_tool(&self) -> EditorTool {
        self.current_tool
    }

    /// Set the current tool.
    pub fn set_tool(&mut self, tool: EditorTool) {
        self.current_tool = tool;
    }

    /// Width of one tool button: [`TOOL_BUTTON_SIZE`], one size for every
    /// tool, so the tools keep their positions as the selection changes.
    pub fn button_width(&self) -> f32 {
        TOOL_BUTTON_SIZE
    }

    /// Distance from one button's left edge to the next one's.
    pub fn button_stride(&self) -> f32 {
        TOOL_BUTTON_SIZE + SEGMENT_DIVIDER_GAP
    }

    /// Whether the overflow menu of shed tools is open.
    pub fn is_overflow_open(&self) -> bool {
        self.overflow_open
    }

    /// Open the overflow menu if it is closed, close it if it is open — what
    /// a click on its button does.
    pub fn toggle_overflow(&mut self) {
        self.overflow_open = !self.overflow_open;
    }

    /// Close the overflow menu. The Escape cascade's first step: an open
    /// menu is the most specific live thing on screen.
    pub fn close_overflow(&mut self) {
        self.overflow_open = false;
    }

    /// Render the tool buttons the strip has room for, plus the overflow
    /// button when some tools were shed, as one segment, and handle tool
    /// selection.
    ///
    /// Call inside the strip's scope ([`crate::toolbar_strip::begin`]) so the
    /// buttons land on the strip's band. The shed tools' menu is a separate
    /// call — [`crate::overflow_menu::render_overflow_menu`] — because
    /// it belongs on the floating band, and overlay scopes cannot nest.
    ///
    /// Returns the newly selected tool if changed.
    pub fn render(
        &mut self,
        ui: &mut UIContext,
        theme: &EditorTheme,
        layout: &crate::toolbar_strip::StripLayout,
    ) -> Option<EditorTool> {
        let visible: Vec<(EditorTool, Rect)> = EditorTool::all()
            .iter()
            .take(layout.visible_tools)
            .enumerate()
            .map(|(index, &tool)| (tool, self.button_bounds_at(layout.tools_origin, index)))
            .collect();
        let mut segment: Vec<Rect> = visible.iter().map(|&(_, bounds)| bounds).collect();
        segment.extend(layout.overflow_button);
        draw_segment(ui, theme, &segment);

        let mut new_tool = None;
        for (tool, bounds) in visible {
            if self.render_tool_button(ui, theme, tool, bounds) {
                new_tool = Some(tool);
            }
        }

        if let Some(overflow) = layout.overflow_button {
            if self.render_overflow_button(ui, theme, overflow) {
                self.toggle_overflow();
            }
        } else {
            self.overflow_open = false;
        }

        new_tool
    }

    /// Bounds of the `index`th visible tool button, from the row's origin.
    pub fn button_bounds_at(&self, origin: Vec2, index: usize) -> Rect {
        Rect::new(
            origin.x + index as f32 * self.button_stride(),
            origin.y,
            TOOL_BUTTON_SIZE,
            TOOL_BUTTON_SIZE,
        )
    }

    /// One tool button: its icon, its tooltip, and the on look when it is
    /// the live tool. Returns true when clicked.
    fn render_tool_button(
        &mut self,
        ui: &mut UIContext,
        theme: &EditorTheme,
        tool: EditorTool,
        bounds: Rect,
    ) -> bool {
        let look = if tool == self.current_tool { ButtonLook::On } else { ButtonLook::Plain };
        let id = format!("toolbar_{}", tool.name());
        let clicked = segment_button(ui, theme, &id, tool.icon(), bounds, look, &tool.tooltip());
        if clicked {
            self.current_tool = tool;
        }
        clicked
    }

    /// The button that opens the shed tools' menu, on while the menu is
    /// open. Returns true when clicked.
    fn render_overflow_button(&self, ui: &mut UIContext, theme: &EditorTheme, bounds: Rect) -> bool {
        let look = if self.overflow_open { ButtonLook::On } else { ButtonLook::Plain };
        let tooltip = tooltip_text("More", None, "The tools and view toggles the strip has no room for.");
        segment_button(ui, theme, "toolbar_overflow", Icon::More, bounds, look, &tooltip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play_controls::PlayControls;
    use crate::play_state::EditorPlayState;
    use crate::test_support::{frame, press_at, release};
    use crate::toolbar_strip::{self, StripLayout};
    use crate::{EditorAction, EditorBinding, EditorInputMapping};

    /// A strip across a comfortable scene panel, where every tool shows.
    fn wide_strip(toolbar: &Toolbar) -> (Rect, StripLayout) {
        let strip = toolbar_strip::split(Rect::new(0.0, 0.0, 1200.0, 600.0)).0;
        let laid_out = toolbar_strip::layout(
            strip,
            toolbar,
            &PlayControls::new(),
            EditorPlayState::Editing,
        );
        assert_eq!(laid_out.visible_tools, EditorTool::all().len(), "fixture: every tool shows");
        (strip, laid_out)
    }

    /// The shortcut a tool's tooltip names and the binding table that
    /// actually switches tools are two tables; this holds them together so
    /// a rebind cannot leave a stale shortcut on screen — and holds every
    /// tooltip to naming one, since the button no longer shows the key.
    #[test]
    fn test_tool_shortcut_hints_match_the_editor_bindings() {
        let mapping = EditorInputMapping::new();
        let tools = [
            (EditorTool::Select, EditorAction::ToolSelect),
            (EditorTool::Move, EditorAction::ToolMove),
            (EditorTool::Rotate, EditorAction::ToolRotate),
            (EditorTool::Scale, EditorAction::ToolScale),
        ];
        assert_eq!(tools.map(|(tool, _)| tool).as_slice(), EditorTool::all());
        for (tool, action) in tools {
            let bindings = mapping.get_bindings(action);
            let [EditorBinding::Chord { key, ctrl: false, shift: false }] = bindings else {
                panic!("{tool:?} must have exactly one bare-key chord, got {bindings:?}");
            };
            let tooltip = tool.tooltip();
            let named = tooltip
                .strip_prefix(tool.name())
                .and_then(|rest| rest.strip_prefix(" ("))
                .and_then(|rest| rest.split_once(')'))
                .map(|(shortcut, _)| shortcut);
            let Some(named) = named else {
                panic!("{tool:?}'s tooltip names no shortcut: {tooltip:?}");
            };
            let key_name = format!("{key:?}");
            let bound = key_name.strip_prefix("Key").unwrap_or(&key_name);
            assert_eq!(named, bound, "{tool:?}: the tooltip names a key that does not select it");
        }
    }

    /// The live tool is the one button filled, and it is ringed as well, so
    /// the state never rests on the fill's colour alone.
    #[test]
    fn test_the_live_tool_is_the_one_button_filled_and_ringed() {
        let mut toolbar = Toolbar::new();
        toolbar.set_tool(EditorTool::Rotate);
        let theme = EditorTheme::default();
        let (_, laid_out) = wide_strip(&toolbar);
        let rotate = toolbar.button_bounds_at(laid_out.tools_origin, 2);
        let mut ui = UIContext::new();
        let input = input::InputHandler::new();

        frame(&mut ui, &input, |ui| toolbar.render(ui, &theme, &laid_out));

        let commands = ui.draw_list().commands();
        let filled: Vec<Rect> = commands
            .iter()
            .filter_map(|command| match command {
                ui::DrawCommand::Rect { bounds, color, .. } if *color == theme.toolbar_active => Some(*bounds),
                _ => None,
            })
            .collect();
        assert_eq!(filled, vec![rotate], "exactly one fill, on the live tool");
        let ringed = commands.iter().any(|command| {
            matches!(command, ui::DrawCommand::RectBorder { bounds, color, .. }
                if *bounds == rotate && *color == theme.accent_blue)
        });
        assert!(ringed, "the live tool is ringed too");
    }

    /// Press in the gap between two tool buttons: the strip's band claims
    /// the gesture, so viewport picking underneath never sees the click.
    /// Press on a button: the click fires on the RELEASE frame, and the
    /// release frame stays widget-owned — the `WidgetState::Active` footgun,
    /// where the release frame is Hovered.
    #[test]
    fn test_a_gap_press_is_claimed_by_the_strip_and_a_button_click_fires_on_release() {
        let mut toolbar = Toolbar::new();
        let theme = EditorTheme::default();
        let (strip, laid_out) = wide_strip(&toolbar);
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();

        let first = toolbar.button_bounds_at(laid_out.tools_origin, 0);
        let gap = Vec2::new(first.right() + SEGMENT_DIVIDER_GAP * 0.5, first.center().y);
        let move_center = toolbar.button_bounds_at(laid_out.tools_origin, 1).center();

        let render = |ui: &mut UIContext, toolbar: &mut Toolbar| {
            toolbar_strip::begin(ui, strip, &theme);
            let picked = toolbar.render(ui, &theme, &laid_out);
            toolbar_strip::end(ui);
            picked
        };

        let picked = press_at(&mut ui, &mut input, gap, |ui| render(ui, &mut toolbar));
        assert_eq!(picked, None, "a gap press selects no tool");
        assert!(
            ui.is_input_blocked_at(gap),
            "a press on the strip must not fall through to viewport picking"
        );
        release(&mut ui, &mut input, |ui| render(ui, &mut toolbar));

        let picked = press_at(&mut ui, &mut input, move_center, |ui| render(ui, &mut toolbar));
        assert_eq!(picked, None, "clicks fire on release, not on press");
        assert!(ui.wants_mouse());
        let (picked, widget_owned) =
            release(&mut ui, &mut input, |ui| (render(ui, &mut toolbar), ui.wants_mouse()));
        assert_eq!(picked, Some(EditorTool::Move), "the second button is Move");
        assert!(widget_owned, "the release frame stays widget-owned");
    }

    /// The same click with raw input timing: press frame then release frame
    /// with NO `input.update()` between them, the way a fast click lands
    /// when the release arrives in the very next frame. The harness's
    /// inserted update must not be what makes the click register.
    #[test]
    fn test_button_click_registers_when_release_follows_press_without_an_input_update() {
        use input::prelude::MouseButton;
        let mut toolbar = Toolbar::new();
        let theme = EditorTheme::default();
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();
        let (strip, laid_out) = wide_strip(&toolbar);
        let rotate_center = toolbar.button_bounds_at(laid_out.tools_origin, 2).center();
        let render = |ui: &mut UIContext, toolbar: &mut Toolbar| {
            toolbar_strip::begin(ui, strip, &theme);
            let picked = toolbar.render(ui, &theme, &laid_out);
            toolbar_strip::end(ui);
            picked
        };

        input.mouse_mut().update_position(rotate_center.x, rotate_center.y);
        input.mouse_mut().handle_button_press(MouseButton::Left);
        let pressed = frame(&mut ui, &input, |ui| render(ui, &mut toolbar));
        input.mouse_mut().handle_button_release(MouseButton::Left);
        let released = frame(&mut ui, &input, |ui| render(ui, &mut toolbar));

        assert_eq!(pressed, None, "the press frame selects nothing");
        assert_eq!(released, Some(EditorTool::Rotate), "the release frame fires the click");
    }
}
