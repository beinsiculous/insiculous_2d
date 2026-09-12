//! The scene view's toolbar strip: an opaque band across the top of the
//! scene panel holding the tools on the left and the play controls at the
//! centre.
//!
//! The strip owns two things the rest of the editor asks it for: the
//! **split** of the scene panel's content area into the band and the
//! viewport below it (so no overlay, scissor or pick ever maps through the
//! band), and the **layout** of the two widget groups inside the band at any
//! width, down to viewports too narrow to show every tool.

use glam::Vec2;
use ui::{Rect, UIContext};

use crate::layout::{PADDING, TOOLBAR_STRIP_HEIGHT};
use crate::play_controls::PlayControls;
use crate::play_state::EditorPlayState;
use crate::theme::EditorTheme;
use crate::toolbar::Toolbar;

/// Gap between the tool group and the play controls, and between the play
/// controls and the group that joins them at the right.
/// The play controls are a group of their own among the tools, so they
/// stand off further than the shared [`crate::layout::GAP`].
const GROUP_GAP: f32 = 8.0;

/// Width of the button that opens the shed tools' menu. Square at the
/// button height, plus room for the chevron beside the glyph.
const OVERFLOW_BUTTON_WIDTH: f32 = 34.0;

/// Total width of the strip's right view-controls group:
/// 4 toggles (24px) at 4px gaps, an 8px gap, and Reset Layout (24px).
pub const VIEW_GROUP_WIDTH: f32 = crate::view_toggles::VIEW_GROUP_WIDTH;

/// The width below which the strip can no longer hold its groups side by
/// side at their natural positions: the padding, the overflow button, the
/// play controls in their widest state, and the view group.
///
/// Also the dock's minimum centre width — below it the side panels stop
/// taking an edge allocation (see [`crate::dock`]), because a centre
/// narrower than this cannot show the play controls at all.
pub const TOOLBAR_STRIP_MIN_WIDTH: f32 = PADDING * 2.0
    + OVERFLOW_BUTTON_WIDTH
    + GROUP_GAP
    + PlayControls::LEAD_WIDTH
    + PlayControls::WIDEST_CONTENT_WIDTH
    + GROUP_GAP
    + VIEW_GROUP_WIDTH;

/// Split a scene panel's content area into the toolbar strip (top) and the
/// viewport (below it).
///
/// The two rects are disjoint and tile the input exactly, so a point is in
/// one or the other and never in both or in neither. A content area shorter
/// than the strip yields the whole of it as strip and a zero-height
/// viewport — the editor stays usable while a panel is dragged shut.
pub fn split(content: Rect) -> (Rect, Rect) {
    let strip_height = TOOLBAR_STRIP_HEIGHT.min(content.height.max(0.0));
    let strip = Rect::new(content.x, content.y, content.width, strip_height);
    let viewport = Rect::new(
        content.x,
        content.y + strip_height,
        content.width,
        (content.height - strip_height).max(0.0),
    );
    (strip, viewport)
}

/// Enter the strip's draw band and paint its chrome. Every widget drawn
/// before the matching [`end`] lands on [`ui::UiLayer::PanelChrome`], above
/// the game world and the panel content, and presses inside the strip stop
/// at it instead of reaching viewport picking underneath.
///
/// A scope rather than one render call because the band and the blocking
/// rect have to cover the strip's *widgets*, and overlay scopes cannot nest.
pub fn begin(ui: &mut UIContext, strip: Rect, theme: &EditorTheme) {
    ui.begin_overlay_in(ui::UiLayer::PanelChrome, strip);
    // Opaque: the game world renders behind the strip, not through it.
    ui.rect(strip, theme.surface_1);
    ui.rect(
        Rect::new(strip.x, strip.bottom() - 1.0, strip.width, 1.0),
        theme.border_subtle,
    );
}

/// Leave the strip's band. Pair with [`begin`].
pub fn end(ui: &mut UIContext) {
    ui.end_overlay();
}

/// Where the strip's groups sit, and how much of the tool and view groups fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripLayout {
    /// The strip this layout was made for — the chrome floor a popup hung
    /// from the strip may shift up to, and no further.
    pub strip: Rect,
    /// Top-left corner of the first tool button.
    pub tools_origin: Vec2,
    /// How many tools are shown as buttons, in tool order. The rest are
    /// reachable through [`overflow_button`](Self::overflow_button).
    pub visible_tools: usize,
    /// The button that opens the shed tools/views menu, when any tool or view group was shed.
    pub overflow_button: Option<Rect>,
    /// Top-left corner of the play controls' first button.
    pub play_controls_origin: Vec2,
    /// Top-left corner of the right view-toggles group, or `None` if shed into overflow.
    pub view_group: Option<Vec2>,
}

/// Lay the tool group, play controls, and view toggles group out inside `strip`.
///
/// The play controls are the sprint's primary control, so they are placed
/// first and never clip: at or above [`TOOLBAR_STRIP_MIN_WIDTH`] they sit at
/// the strip's centre so they stay put as the play state and the tool set
/// change — as long as the centre leaves them whole; where it would not, and
/// below the minimum, they are laid out from the strip's right edge.
/// The right view group sits at the right edge when at or above the minimum,
/// or sheds as a unit into the overflow menu when below it.
pub fn layout(
    strip: Rect,
    toolbar: &Toolbar,
    play_controls: &PlayControls,
    play_state: EditorPlayState,
) -> StripLayout {
    let button_top = strip.y + ((strip.height - toolbar.button_height()) * 0.5).max(0.0);
    let left = strip.x + PADDING;
    let right = strip.right() - PADDING;

    let (view_group, from_right) = if strip.width >= TOOLBAR_STRIP_MIN_WIDTH {
        let origin = Vec2::new(right - VIEW_GROUP_WIDTH, button_top);
        let from_right = right - VIEW_GROUP_WIDTH - GROUP_GAP - play_controls.content_width(play_state);
        (Some(origin), from_right)
    } else {
        let from_right = right - play_controls.content_width(play_state);
        (None, from_right)
    };

    let play_x = if strip.width >= TOOLBAR_STRIP_MIN_WIDTH {
        strip.center().x.min(from_right)
    } else {
        from_right
    }
    .max(left + PlayControls::LEAD_WIDTH);

    // The separator the play controls draw to their left belongs to them.
    let tools_limit = play_x - PlayControls::LEAD_WIDTH - GROUP_GAP;
    let (visible_tools, overflow_button) =
        fit_tools(toolbar, left, tools_limit, button_top, view_group.is_none());

    StripLayout {
        strip,
        tools_origin: Vec2::new(left, button_top),
        visible_tools,
        overflow_button,
        play_controls_origin: Vec2::new(play_x, button_top),
        view_group,
    }
}

/// How many tool buttons fit between `left` and `limit`, and where the
/// overflow button goes when some do not or when the view group has shed.
/// Shedding takes from the end of the tool order, so the tools keep their
/// positions as the strip narrows.
fn fit_tools(
    toolbar: &Toolbar,
    left: f32,
    limit: f32,
    top: f32,
    view_group_shed: bool,
) -> (usize, Option<Rect>) {
    let total = crate::toolbar::EditorTool::all().len();
    let stride = toolbar.button_stride();
    let available = limit - left;

    if !view_group_shed && fitting_count(available, stride, toolbar.spacing()) >= total {
        return (total, None);
    }

    // One tool in the menu is worth no less than one on the strip, so the
    // overflow button's own width comes out of the tool group's budget.
    // If all tools fit but the view group has shed, the overflow button is placed
    // after the last tool.
    let max_tools = if view_group_shed { total } else { total.saturating_sub(1) };
    let visible = fitting_count(
        available - OVERFLOW_BUTTON_WIDTH - toolbar.spacing(),
        stride,
        toolbar.spacing(),
    )
    .min(max_tools);
    let overflow = Rect::new(
        left + visible as f32 * stride,
        top,
        OVERFLOW_BUTTON_WIDTH,
        toolbar.button_height(),
    );
    (visible, Some(overflow))
}

/// How many buttons of `stride` (button plus the gap after it) fit in
/// `available` px, counting that the last button needs no trailing gap.
fn fitting_count(available: f32, stride: f32, spacing: f32) -> usize {
    if available <= 0.0 || stride <= 0.0 {
        return 0;
    }
    (((available + spacing) / stride).floor().max(0.0)) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::HEADER_HEIGHT;
    use crate::toolbar::EditorTool;

    /// A comfortable scene panel and the one a 390px page produces once the
    /// dock has gone narrow (the centre takes the whole width).
    const COMFORTABLE: Rect = Rect { x: 200.0, y: 48.0, width: 800.0, height: 600.0 };
    /// Narrow widths below the 391px minimum width (where the view group sheds).
    const NARROW: Rect = Rect { x: 0.0, y: 48.0, width: 390.0, height: 560.0 };
    const ANDROID: Rect = Rect { x: 0.0, y: 48.0, width: 360.0, height: 560.0 };
    const JUST_OVER_THE_MINIMUM: Rect =
        Rect { x: 0.0, y: 48.0, width: TOOLBAR_STRIP_MIN_WIDTH + 7.0, height: 560.0 };

    fn strip_of(content: Rect) -> Rect {
        split(content).0
    }

    /// The split tiles the content area: the two rects are disjoint, they
    /// cover it exactly, and the viewport is what every overlay and the
    /// scissor map through — so nothing can draw into the band.
    #[test]
    fn test_split_tiles_the_content_area_into_a_strip_and_the_viewport() {
        let (strip, viewport) = split(COMFORTABLE);

        assert_eq!(strip.height, TOOLBAR_STRIP_HEIGHT);
        assert_eq!(strip.y, COMFORTABLE.y, "the strip starts at the content top");
        assert_eq!(viewport.y, strip.bottom(), "the viewport starts where the strip ends");
        assert_eq!(strip.height + viewport.height, COMFORTABLE.height, "the two tile the height");
        assert_eq!((strip.x, strip.width), (COMFORTABLE.x, COMFORTABLE.width));
        assert_eq!((viewport.x, viewport.width), (COMFORTABLE.x, COMFORTABLE.width));
        assert!(!viewport.contains(strip.center()), "the rects are disjoint");
        assert!(!strip.contains(viewport.center()));
    }

    /// A content area shorter than the strip: the strip shrinks to what
    /// there is and the viewport goes to zero height, never negative.
    #[test]
    fn test_split_never_yields_a_negative_viewport() {
        let squashed = Rect::new(0.0, 0.0, 400.0, HEADER_HEIGHT);
        let (strip, viewport) = split(squashed);

        assert_eq!(strip.height, HEADER_HEIGHT);
        assert_eq!(viewport.height, 0.0);
        assert_eq!(strip.height + viewport.height, squashed.height);
    }

    /// At a comfortable width every tool is a button, nothing sheds, and
    /// the play controls sit at the strip's centre — the same x in every
    /// play state, so they do not move under the pointer when Play is
    /// clicked.
    #[test]
    fn test_every_tool_shows_and_the_play_controls_hold_the_centre_when_there_is_room() {
        let strip = strip_of(COMFORTABLE);
        let toolbar = Toolbar::new();
        let controls = PlayControls::new();

        let editing = layout(strip, &toolbar, &controls, EditorPlayState::Editing);
        assert_eq!(editing.visible_tools, EditorTool::all().len());
        assert_eq!(editing.overflow_button, None);
        assert!(editing.view_group.is_some());
        assert_eq!(editing.play_controls_origin.x, strip.center().x);

        for state in [EditorPlayState::Playing, EditorPlayState::Paused] {
            let laid_out = layout(strip, &toolbar, &controls, state);
            assert_eq!(
                laid_out.play_controls_origin, editing.play_controls_origin,
                "{state:?}: the play controls must not move when the state changes"
            );
        }
    }

    /// The groups laid out in one strip never overlap, and every
    /// button stays inside the strip — at a comfortable width, at the widths
    /// 390px and 360px produce (below the 391px min where view group sheds),
    /// and in the band just over the minimum where the centre is preferred.
    /// Reachability, not merely non-overlap: every tool is a button or an
    /// entry in the overflow menu.
    #[test]
    fn test_the_groups_never_overlap_or_clip_at_the_narrow_width_or_a_comfortable_one() {
        let toolbar = Toolbar::new();
        let controls = PlayControls::new();

        for content in [COMFORTABLE, NARROW, ANDROID, JUST_OVER_THE_MINIMUM] {
            let strip = strip_of(content);
            for state in [EditorPlayState::Editing, EditorPlayState::Playing, EditorPlayState::Paused] {
                let laid_out = layout(strip, &toolbar, &controls, state);
                let tools = tool_group_bounds(&toolbar, &laid_out);
                let play = play_group_bounds(&controls, &laid_out, state);

                assert!(
                    contains_rect(strip, play),
                    "{state:?} at {}px: the play controls clip ({play:?} outside {strip:?})",
                    content.width
                );
                if let Some(tools) = tools {
                    assert!(
                        contains_rect(strip, tools),
                        "{state:?} at {}px: the tool group clips",
                        content.width
                    );
                    assert!(
                        tools.right() <= play.x,
                        "{state:?} at {}px: the groups overlap",
                        content.width
                    );
                }
                if let Some(view_origin) = laid_out.view_group {
                    let view_rect = Rect::new(view_origin.x, view_origin.y, VIEW_GROUP_WIDTH, toolbar.button_height());
                    assert!(
                        contains_rect(strip, view_rect),
                        "{state:?} at {}px: view group clips",
                        content.width
                    );
                    assert!(
                        play.right() <= view_rect.x,
                        "{state:?} at {}px: play controls overlap view group",
                        content.width
                    );
                }
                assert_eq!(
                    laid_out.visible_tools < EditorTool::all().len() || laid_out.view_group.is_none(),
                    laid_out.overflow_button.is_some(),
                    "{state:?} at {}px: a shed tool or view group must have a menu to live in",
                    content.width
                );
            }
        }
    }

    /// At the exact minimum width (391px) the view group shows whole; one pixel under (390px),
    /// the view group sheds and the overflow button exists even if all tools fit.
    #[test]
    fn test_view_group_shows_whole_at_minimum_and_sheds_one_pixel_under() {
        let toolbar = Toolbar::new();
        let controls = PlayControls::new();

        let at_min = strip_of(Rect::new(0.0, 0.0, TOOLBAR_STRIP_MIN_WIDTH, 560.0));
        let layout_at_min = layout(at_min, &toolbar, &controls, EditorPlayState::Editing);
        assert!(layout_at_min.view_group.is_some(), "view group shows whole at minimum width");

        let under_min = strip_of(Rect::new(0.0, 0.0, TOOLBAR_STRIP_MIN_WIDTH - 1.0, 560.0));
        let layout_under = layout(under_min, &toolbar, &controls, EditorPlayState::Editing);
        assert_eq!(layout_under.view_group, None, "view group sheds one pixel under minimum");
        assert!(layout_under.overflow_button.is_some(), "overflow button present when view group sheds");
    }

    /// Below the minimum width the play controls are laid out from the
    /// right edge and the tool group is what sheds — it never wins space
    /// from the control the visitor came to press.
    #[test]
    fn test_below_the_minimum_width_the_play_controls_take_the_right_edge_and_tools_shed() {
        let strip = strip_of(Rect::new(0.0, 0.0, TOOLBAR_STRIP_MIN_WIDTH - 60.0, 560.0));
        let toolbar = Toolbar::new();
        let controls = PlayControls::new();

        let laid_out = layout(strip, &toolbar, &controls, EditorPlayState::Paused);
        let play = play_group_bounds(&controls, &laid_out, EditorPlayState::Paused);

        assert!(laid_out.visible_tools < EditorTool::all().len(), "tools shed first");
        assert!(laid_out.overflow_button.is_some(), "the shed tools get a menu");
        assert_eq!(laid_out.view_group, None, "view group sheds");
        assert!(contains_rect(strip, play), "the play controls still fit whole");
        assert!(
            play.right() <= strip.right() && play.right() > strip.center().x,
            "the play controls are laid out from the right edge"
        );
    }

    /// The keep/discard dialog renders first in the frame with a Modal scrim;
    /// the strip opens its own scope after it. A Resume click behind the scrim
    /// must not fire, or the session resumes while the dialog still asks.
    #[test]
    fn test_a_modal_scrim_keeps_the_strips_play_controls_inert() {
        use crate::confirm_dialog::ConfirmDialog;
        use crate::test_support::{press_at, release};

        let theme = EditorTheme::default();
        let window = Vec2::new(1200.0, 700.0);
        let strip = strip_of(COMFORTABLE);
        let mut toolbar = Toolbar::new();
        let mut controls = PlayControls::new();
        let dialog = ConfirmDialog::keep_paused_edits(1);
        let laid_out = layout(strip, &toolbar, &controls, EditorPlayState::Paused);
        controls.position = laid_out.play_controls_origin;
        let resume = Vec2::new(controls.position.x + 4.0, controls.position.y + controls.height * 0.5);
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();

        let render = |ui: &mut UIContext, toolbar: &mut Toolbar, controls: &mut PlayControls| {
            dialog.render(ui, window, &theme);
            begin(ui, strip, &theme);
            let tool = toolbar.render(ui, &theme, &laid_out);
            let action = controls.render(ui, EditorPlayState::Paused, false, &theme);
            end(ui);
            (tool, action)
        };
        press_at(&mut ui, &mut input, resume, |ui| render(ui, &mut toolbar, &mut controls));
        let (tool, action) =
            release(&mut ui, &mut input, |ui| render(ui, &mut toolbar, &mut controls));

        assert_eq!(action, None, "Resume behind the scrim must not fire");
        assert_eq!(tool, None, "no tool changes behind the scrim");
    }

    /// Bounds of the visible tool buttons, or `None` when all of them shed.
    fn tool_group_bounds(toolbar: &Toolbar, laid_out: &StripLayout) -> Option<Rect> {
        let right = match (laid_out.visible_tools, laid_out.overflow_button) {
            (0, None) => return None,
            (_, Some(overflow)) => overflow.right(),
            (count, None) => {
                laid_out.tools_origin.x + count as f32 * toolbar.button_stride() - toolbar.spacing()
            }
        };
        Some(Rect::new(
            laid_out.tools_origin.x,
            laid_out.tools_origin.y,
            right - laid_out.tools_origin.x,
            toolbar.button_height(),
        ))
    }

    /// Bounds of the play controls in `state`, the separator included.
    fn play_group_bounds(
        controls: &PlayControls,
        laid_out: &StripLayout,
        state: EditorPlayState,
    ) -> Rect {
        let mut positioned = controls.clone();
        positioned.position = laid_out.play_controls_origin;
        positioned.chrome_bounds(state)
    }

    /// Whether `inner` lies wholly inside `outer` (tolerant of the
    /// half-pixel a centred button can land on).
    fn contains_rect(outer: Rect, inner: Rect) -> bool {
        inner.x >= outer.x - 0.5
            && inner.y >= outer.y - 0.5
            && inner.right() <= outer.right() + 0.5
            && inner.bottom() <= outer.bottom() + 0.5
    }
}
