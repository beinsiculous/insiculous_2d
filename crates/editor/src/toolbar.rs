//! Editor toolbar with tool selection.
//!
//! The toolbar provides buttons for switching between editor tools
//! (Select, Move, Rotate, Scale) and displays the current tool state.

use glam::Vec2;
use ui::{Rect, UIContext};

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
    /// Button width: one size for every tool, wide enough for the longest
    /// tool name at the body font size, so the tools keep their positions
    /// as the selection changes.
    button_width: f32,
    /// Button height — the compact strip button, not a square.
    button_height: f32,
    /// Spacing between buttons
    spacing: f32,
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
            button_width: 64.0,
            button_height: 30.0,
            spacing: 6.0,
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

    /// Width of one tool button.
    pub fn button_width(&self) -> f32 {
        self.button_width
    }

    /// Height of one tool button — the strip sizes its band around it.
    pub fn button_height(&self) -> f32 {
        self.button_height
    }

    /// Gap between two tool buttons.
    pub fn spacing(&self) -> f32 {
        self.spacing
    }

    /// Distance from one button's left edge to the next one's.
    pub fn button_stride(&self) -> f32 {
        self.button_width + self.spacing
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
    /// button when some tools were shed, and handle tool selection.
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
        theme: &crate::EditorTheme,
        layout: &crate::toolbar_strip::StripLayout,
    ) -> Option<EditorTool> {
        let mut new_tool = None;

        for (index, &tool) in EditorTool::all().iter().take(layout.visible_tools).enumerate() {
            let button_bounds = self.button_bounds_at(layout.tools_origin, index);
            if self.render_tool_button(ui, theme, tool, button_bounds) {
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
            self.button_width,
            self.button_height,
        )
    }

    /// One tool button: its name, its shortcut as a small caption in the
    /// top-right corner (the compact strip has no room for the hint row the
    /// floating toolbar had), and the active state when it is the live tool.
    /// Returns true when clicked.
    fn render_tool_button(
        &mut self,
        ui: &mut UIContext,
        theme: &crate::EditorTheme,
        tool: EditorTool,
        bounds: Rect,
    ) -> bool {
        let is_selected = tool == self.current_tool;

        // Selection ring: an accent halo slightly larger than the button
        // (the button's own background is opaque, so anything drawn
        // directly underneath it would be invisible)
        if is_selected {
            ui.rect_rounded(bounds.expand(2.0), theme.toolbar_active, 5.0);
        }

        let id = format!("toolbar_{}", tool.name());
        let clicked = ui.button(id.as_str(), tool.name(), bounds);
        if clicked {
            self.current_tool = tool;
        }

        if is_selected {
            ui.rect_border(bounds, theme.accent_blue, 1.0, 4.0);
        }

        let hint_pos = Vec2::new(bounds.right() - 7.0, bounds.y + theme.fonts.small);
        ui.label_centered_styled(tool.shortcut(), hint_pos, theme.shortcut_hint, theme.fonts.small);

        clicked
    }

    /// The button that opens the shed tools' menu: a chevron, and the active
    /// state while the menu is open. Returns true when clicked.
    fn render_overflow_button(
        &self,
        ui: &mut UIContext,
        theme: &crate::EditorTheme,
        bounds: Rect,
    ) -> bool {
        if self.overflow_open {
            ui.rect_rounded(bounds.expand(2.0), theme.toolbar_active, 5.0);
        }
        let clicked = ui.button("toolbar_overflow", "", bounds);
        // Three dots rather than a glyph: the strip must read the same in
        // every font the editor can be running.
        let center = bounds.center();
        for offset in [-6.0, 0.0, 6.0] {
            ui.circle(Vec2::new(center.x + offset, center.y), 1.5, theme.accent_cyan);
        }
        clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play_controls::PlayControls;
    use crate::play_state::EditorPlayState;
    use crate::test_support::{frame, press_at, release};
    use crate::toolbar_strip::{self, StripLayout};
    use crate::{EditorAction, EditorBinding, EditorInputMapping, EditorTheme};

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

    /// The shortcut hint painted on each tool button and the binding
    /// table that actually switches tools are two tables; this holds them
    /// together so a rebind cannot leave a stale hint on screen.
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
            let key_name = format!("{key:?}");
            let hinted = key_name.strip_prefix("Key").unwrap_or(&key_name);
            assert_eq!(hinted, tool.shortcut(), "{tool:?}: the hint shows a key that does not select it");
        }
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
        let gap = Vec2::new(first.right() + toolbar.spacing() * 0.5, first.center().y);
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
