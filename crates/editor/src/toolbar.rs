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
    /// gizmo the moment something is selected (audit §4.5)
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
/// Renders a horizontal bar with tool selection buttons.
#[derive(Debug, Clone)]
pub struct Toolbar {
    /// Current selected tool
    current_tool: EditorTool,
    /// Position of the toolbar
    position: Vec2,
    /// Button size
    button_size: f32,
    /// Spacing between buttons
    spacing: f32,
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
            position: Vec2::new(10.0, 10.0),
            // Wide enough for the longest label ("Rotate"/"Select") at the
            // body font size — 40px caused the labels to overflow each other
            button_size: 56.0,
            spacing: 6.0,
        }
    }

    /// Set the toolbar position.
    pub fn with_position(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    /// Set the toolbar position in place (used to follow the scene view).
    pub fn set_position(&mut self, position: Vec2) {
        self.position = position;
    }

    /// Get the currently selected tool.
    pub fn current_tool(&self) -> EditorTool {
        self.current_tool
    }

    /// Set the current tool.
    pub fn set_tool(&mut self, tool: EditorTool) {
        self.current_tool = tool;
    }

    /// Get the toolbar bounds (for layout purposes).
    pub fn bounds(&self) -> Rect {
        let tools = EditorTool::all();
        let width = tools.len() as f32 * (self.button_size + self.spacing) - self.spacing;
        Rect::new(self.position.x, self.position.y, width, self.button_size)
    }

    /// Full chrome footprint: the background panel plus the shortcut-hint row
    /// below the buttons. Everything inside consumes mouse gestures so clicks
    /// on toolbar chrome never fall through to viewport picking.
    pub fn chrome_bounds(&self) -> Rect {
        let bg = self.bounds().expand(4.0);
        // Hint baselines sit 12px below the buttons; 16px covers descenders.
        Rect::new(bg.x, bg.y, bg.width, bg.height + 16.0)
    }

    /// Render the toolbar and handle tool selection.
    ///
    /// Returns the newly selected tool if changed.
    pub fn render(&mut self, ui: &mut UIContext, theme: &crate::EditorTheme) -> Option<EditorTool> {
        let tools = EditorTool::all();
        let mut new_tool = None;

        // Draw toolbar background
        let bounds = self.bounds();
        let bg_bounds = bounds.expand(4.0);
        ui.panel(bg_bounds);

        // Draw tool buttons
        for (i, &tool) in tools.iter().enumerate() {
            let x = self.position.x + i as f32 * (self.button_size + self.spacing);
            let button_bounds = Rect::new(x, self.position.y, self.button_size, self.button_size);

            let is_selected = tool == self.current_tool;

            // Selection ring: an accent halo slightly larger than the button
            // (the button's own background is opaque, so anything drawn
            // directly underneath it would be invisible)
            if is_selected {
                ui.rect_rounded(button_bounds.expand(2.0), theme.toolbar_active, 5.0);
            }

            // Draw button (will use default styling with hover effect)
            let id = format!("toolbar_{}", tool.name());
            if ui.button(id.as_str(), tool.name(), button_bounds) {
                self.current_tool = tool;
                new_tool = Some(tool);
            }

            // Accent border on the active tool, over the button chrome
            if is_selected {
                ui.rect_border(button_bounds, theme.accent_cyan, 1.0, 4.0);
            }

            // Draw shortcut hint below button (baseline sits a line below the
            // button's bottom edge so the glyphs don't rise into the button)
            let hint_pos = Vec2::new(
                button_bounds.center().x,
                button_bounds.y + button_bounds.height + 12.0,
            );
            ui.label_centered_styled(tool.shortcut(), hint_pos, theme.shortcut_hint, theme.fonts.small);
        }

        // Consume-only: a press on the background/border/hint chrome (not on
        // a button) must still claim the mouse gesture, or viewport picking
        // underneath treats the click as its own. Registered AFTER the
        // buttons so they win the active-widget slot.
        ui.interact("toolbar_chrome", self.chrome_bounds(), true);

        new_tool
    }
}

/// Where the toolbar should sit for a given scene-view content area:
/// tucked into the top-left corner, below the panel header.
pub fn toolbar_position_for(scene_content: Rect) -> Vec2 {
    Vec2::new(scene_content.x + 16.0, scene_content.y + 8.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{frame, press_at, release};
    use crate::{EditorAction, EditorBinding, EditorInputMapping, EditorTheme};

    /// The toolbar tucks into the scene view's top-left corner and follows
    /// it when the left panel is hidden.
    #[test]
    fn test_toolbar_position_follows_scene_content() {
        let docked = toolbar_position_for(Rect::new(200.0, 48.0, 550.0, 600.0));
        let left_hidden = toolbar_position_for(Rect::new(0.0, 48.0, 750.0, 600.0));
        assert_eq!(docked, Vec2::new(216.0, 56.0));
        assert_eq!(left_hidden, Vec2::new(16.0, 56.0));
    }

    /// The shortcut hint painted under each tool button and the binding
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

    /// Press on toolbar chrome (the gap between buttons): the toolbar claims
    /// the gesture so viewport picking underneath never sees the click.
    /// Press on a button: the click fires on the RELEASE frame and must win
    /// over the consume-only chrome rect registered after it — the
    /// `WidgetState::Active` footgun, where the release frame is Hovered.
    #[test]
    fn test_toolbar_button_click_survives_chrome_interact() {
        let mut toolbar = Toolbar::new();
        let theme = EditorTheme::default();
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();

        // Chrome press in the 6px gap between the first two buttons.
        let picked = press_at(&mut ui, &mut input, Vec2::new(68.0, 30.0), |ui| toolbar.render(ui, &theme));
        assert_eq!(picked, None, "a gap press selects no tool");
        assert!(ui.wants_mouse(), "a press on toolbar chrome must not fall through to picking");
        release(&mut ui, &mut input, |ui| toolbar.render(ui, &theme));

        // Button press on Move (second button, center x = 72 + 28).
        let picked = press_at(&mut ui, &mut input, Vec2::new(100.0, 38.0), |ui| toolbar.render(ui, &theme));
        assert_eq!(picked, None, "clicks fire on release, not on press");
        assert!(ui.wants_mouse());
        let (picked, widget_owned) = release(&mut ui, &mut input, |ui| (toolbar.render(ui, &theme), ui.wants_mouse()));
        assert_eq!(picked, Some(EditorTool::Move), "the button wins the click over the chrome rect");
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
        let rotate_center = Vec2::new(10.0 + 2.0 * 62.0 + 28.0, 38.0);

        input.mouse_mut().update_position(rotate_center.x, rotate_center.y);
        input.mouse_mut().handle_button_press(MouseButton::Left);
        let pressed = frame(&mut ui, &input, |ui| toolbar.render(ui, &theme));
        input.mouse_mut().handle_button_release(MouseButton::Left);
        let released = frame(&mut ui, &input, |ui| toolbar.render(ui, &theme));

        assert_eq!(pressed, None, "the press frame selects nothing");
        assert_eq!(released, Some(EditorTool::Rotate), "the release frame fires the click");
    }
}
