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

    /// Set the button size.
    pub fn with_button_size(mut self, size: f32) -> Self {
        self.button_size = size;
        self
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

    #[test]
    fn test_toolbar_position_follows_scene_content() {
        let pos = toolbar_position_for(Rect::new(200.0, 48.0, 550.0, 600.0));
        assert_eq!(pos, Vec2::new(216.0, 56.0));

        // Left panel hidden: scene content starts at x = 0
        let pos = toolbar_position_for(Rect::new(0.0, 48.0, 750.0, 600.0));
        assert_eq!(pos, Vec2::new(16.0, 56.0));
    }

    #[test]
    fn test_toolbar_set_position() {
        let mut toolbar = Toolbar::new();
        toolbar.set_position(Vec2::new(300.0, 60.0));
        assert_eq!(toolbar.bounds().x, 300.0);
        assert_eq!(toolbar.bounds().y, 60.0);
    }

    #[test]
    fn test_editor_tool_default_is_move_so_a_gizmo_shows() {
        let tool = EditorTool::default();
        assert_eq!(tool, EditorTool::Move);
    }

    #[test]
    fn test_editor_tool_names() {
        assert_eq!(EditorTool::Select.name(), "Select");
        assert_eq!(EditorTool::Move.name(), "Move");
        assert_eq!(EditorTool::Rotate.name(), "Rotate");
        assert_eq!(EditorTool::Scale.name(), "Scale");
    }

    #[test]
    fn test_editor_tool_shortcuts() {
        assert_eq!(EditorTool::Select.shortcut(), "Q");
        assert_eq!(EditorTool::Move.shortcut(), "W");
        assert_eq!(EditorTool::Rotate.shortcut(), "E");
        assert_eq!(EditorTool::Scale.shortcut(), "R");
    }

    #[test]
    fn test_editor_tool_all() {
        let tools = EditorTool::all();
        assert_eq!(tools.len(), 4);
        assert!(tools.contains(&EditorTool::Select));
        assert!(tools.contains(&EditorTool::Move));
        assert!(tools.contains(&EditorTool::Rotate));
        assert!(tools.contains(&EditorTool::Scale));
    }

    #[test]
    fn test_toolbar_new() {
        let toolbar = Toolbar::new();
        assert_eq!(toolbar.current_tool(), EditorTool::Move);
    }

    #[test]
    fn test_toolbar_with_position() {
        let toolbar = Toolbar::new().with_position(Vec2::new(100.0, 50.0));
        assert_eq!(toolbar.position, Vec2::new(100.0, 50.0));
    }

    #[test]
    fn test_toolbar_set_tool() {
        let mut toolbar = Toolbar::new();
        toolbar.set_tool(EditorTool::Move);
        assert_eq!(toolbar.current_tool(), EditorTool::Move);
    }

    #[test]
    fn test_toolbar_chrome_bounds_cover_background_and_hint_row() {
        let toolbar = Toolbar::new();
        let chrome = toolbar.chrome_bounds();
        let bg = toolbar.bounds().expand(4.0);
        assert!(chrome.contains(Vec2::new(bg.x, bg.y)), "background top-left is chrome");
        // Shortcut hint baseline sits 12px below the buttons
        assert!(
            chrome.contains(Vec2::new(38.0, 10.0 + 56.0 + 12.0)),
            "hint row below the buttons is chrome"
        );
    }

    #[test]
    fn test_toolbar_background_press_claims_mouse_gesture() {
        use input::prelude::MouseButton;
        let mut toolbar = Toolbar::new();
        let theme = crate::EditorTheme::default();
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();

        // Press in the 6px gap between the first two buttons — visually
        // toolbar chrome, but no button contains the point.
        input.mouse_mut().update_position(68.0, 30.0);
        input.mouse_mut().handle_button_press(MouseButton::Left);
        ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
        assert!(toolbar.render(&mut ui, &theme).is_none());
        assert!(
            ui.wants_mouse(),
            "a press on toolbar chrome must not fall through to viewport picking"
        );
        ui.end_frame();
    }

    #[test]
    fn test_toolbar_button_click_survives_chrome_interact() {
        use input::prelude::MouseButton;
        let mut toolbar = Toolbar::new();
        let theme = crate::EditorTheme::default();
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();

        // Press on the Move button (second button, center x = 72 + 28)
        input.mouse_mut().update_position(100.0, 38.0);
        input.mouse_mut().handle_button_press(MouseButton::Left);
        ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
        assert!(toolbar.render(&mut ui, &theme).is_none(), "clicks fire on release");
        assert!(ui.wants_mouse());
        ui.end_frame();

        // Release: the button must still win the click over the chrome rect
        input.update();
        input.mouse_mut().handle_button_release(MouseButton::Left);
        ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
        assert_eq!(toolbar.render(&mut ui, &theme), Some(EditorTool::Move));
        assert!(ui.wants_mouse(), "release frame stays widget-owned");
        ui.end_frame();
    }

    #[test]
    fn test_toolbar_bounds() {
        let toolbar = Toolbar::new();
        let bounds = toolbar.bounds();

        // 4 tools * (56 + 6) - 6 = 242
        assert_eq!(bounds.width, 242.0);
        assert_eq!(bounds.height, 56.0);
    }
}
