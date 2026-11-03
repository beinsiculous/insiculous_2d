//! Where everything in the composer panel goes: a pure function of the panel's bounds, so
//! the render, the hit tests and the tests read one geometry.
//!
//! The transport runs along the top and the sidebar fills the rest, in either mode. **Full**
//! draws every sidebar section at once. **Compact** (below [`COMPACT_BELOW_WIDTH`]) shows
//! one section at a time behind a row of tabs. The native editor's minimum window is wider
//! than the threshold, so only a narrow browser reaches Compact.

use ui::Rect;

/// Below this panel width the composer is [`ComposerMode::Compact`].
pub const COMPACT_BELOW_WIDTH: f32 = 720.0;
/// The transport row along the panel's top.
pub const TRANSPORT_HEIGHT: f32 = 30.0;
/// A row of section tabs.
pub const TAB_STRIP_HEIGHT: f32 = 24.0;

/// See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerMode {
    Full,
    Compact,
}

impl ComposerMode {
    pub fn for_width(width: f32) -> Self {
        if width < COMPACT_BELOW_WIDTH {
            ComposerMode::Compact
        } else {
            ComposerMode::Full
        }
    }
}

/// The panel cut into its areas.
#[derive(Debug, Clone, PartialEq)]
pub struct ComposerLayout {
    pub mode: ComposerMode,
    pub transport: Rect,
    pub sidebar: Rect,
    /// The section tabs over the sidebar below the full width.
    pub section_tabs: Option<Rect>,
}

/// Lay the panel out: the transport, then the sidebar over the rest of the body — under its
/// section tabs in [`ComposerMode::Compact`].
pub fn compute(bounds: Rect) -> ComposerLayout {
    let mode = ComposerMode::for_width(bounds.width);
    let transport = Rect::new(bounds.x, bounds.y, bounds.width, TRANSPORT_HEIGHT.min(bounds.height));
    let body = Rect::new(
        bounds.x,
        transport.bottom(),
        bounds.width,
        (bounds.height - transport.height).max(0.0),
    );
    if mode == ComposerMode::Full {
        return ComposerLayout {
            mode,
            transport,
            sidebar: body,
            section_tabs: None,
        };
    }
    let tabs = Rect::new(body.x, body.y, body.width, TAB_STRIP_HEIGHT.min(body.height));
    ComposerLayout {
        mode,
        transport,
        sidebar: Rect::new(
            body.x,
            tabs.bottom(),
            body.width,
            (body.height - tabs.height).max(0.0),
        ),
        section_tabs: Some(tabs),
    }
}
