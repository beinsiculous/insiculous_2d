//! The Conductor panel's layout: the transport over the sidebar at every width, its section tabs
//! below the full width.

use ui::Rect;

use super::layout::{compute, ConductorMode, COMPACT_BELOW_WIDTH};

fn panel(width: f32) -> Rect {
    Rect::new(0.0, 400.0, width, 260.0)
}

#[test]
fn the_mode_follows_the_panel_width_at_the_threshold() {
    let cases = [
        (1280.0, ConductorMode::Full),
        (COMPACT_BELOW_WIDTH, ConductorMode::Full),
        (COMPACT_BELOW_WIDTH - 1.0, ConductorMode::Compact),
        (320.0, ConductorMode::Compact),
    ];
    for (width, mode) in cases {
        assert_eq!(ConductorMode::for_width(width), mode, "{width} px");
    }
}

#[test]
fn the_sidebar_fills_the_body_under_the_transport_at_every_width_and_its_tabs_show_below_the_full_width() {
    for width in [479.0, 480.0, 719.0, 720.0] {
        let bounds = panel(width);
        let layout = compute(bounds);
        assert_eq!(layout.transport.y, bounds.y, "{width} px: the transport on top");
        assert_eq!(layout.transport.width, width, "{width} px: the transport spans the panel");
        assert_eq!(layout.sidebar.x, bounds.x, "{width} px");
        assert_eq!(layout.sidebar.width, width, "{width} px: the sidebar spans the panel");
        assert_eq!(layout.sidebar.bottom(), bounds.bottom(), "{width} px: and runs to its bottom");
        match layout.section_tabs {
            Some(tabs) => {
                assert!(width < COMPACT_BELOW_WIDTH, "{width} px: tabs only below the full width");
                assert_eq!(tabs.y, layout.transport.bottom(), "{width} px: the tabs under the transport");
                assert_eq!(layout.sidebar.y, tabs.bottom(), "{width} px: the section under its tabs");
            }
            None => {
                assert!(width >= COMPACT_BELOW_WIDTH, "{width} px: every section drawn at the full width");
                assert_eq!(layout.sidebar.y, layout.transport.bottom(), "{width} px: the sidebar under the transport");
            }
        }
    }
}
